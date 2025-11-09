use anyhow::{anyhow, Result};
use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::client::conn::http1;
use hyper::{Method, Request, Uri};
use hyper_rustls::{HttpsConnector, HttpsConnectorBuilder};
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::rt::TokioExecutor;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

type HttpsConn = HttpsConnector<HttpConnector>;

/// 客户端状态 - 每个 worker 维护一个，用于连接复用
pub struct ClientState {
    /// HTTP/1.1 连接的 SendRequest（保持连接复用）
    pub send_request: Option<http1::SendRequest<Full<Bytes>>>,
}

impl ClientState {
    pub fn new() -> Self {
        Self {
            send_request: None,
        }
    }
}

impl Default for ClientState {
    fn default() -> Self {
        Self::new()
    }
}

/// 高性能 HTTP 客户端，基于 hyper 1.4
/// 参考 oha 的优化策略：
/// 1. 直接管理 HTTP/1.1 连接，避免连接池开销
/// 2. 流式处理响应体，不完整缓存
/// 3. 连接复用，减少握手开销
pub struct HttpClient {
    connector: Arc<HttpsConn>,
    timeout: Duration,
}

impl HttpClient {
    /// 创建新的 HTTP 客户端
    /// 
    /// # 参数
    /// - `timeout`: 请求超时时间
    /// - `pool_size`: 连接池大小
    /// - `enable_http2`: 是否启用 HTTP/2（默认只使用 HTTP/1.1）
    pub fn new(timeout: Duration, _pool_size: usize, enable_http2: bool) -> Result<Self> {
        // 初始化 rustls crypto provider（只需要初始化一次）
        let _ = rustls::crypto::ring::default_provider().install_default();
        
        // 根据参数决定是否启用 HTTP/2，构建连接器
        let connector = if enable_http2 {
            // 启用 HTTP/1.1 和 HTTP/2
            HttpsConnectorBuilder::new()
                .with_native_roots()
                .map_err(|e| anyhow!("Failed to load native certs: {}", e))?
                .https_or_http()
                .enable_http1()
                .enable_http2()
                .build()
        } else {
            // 只启用 HTTP/1.1
            HttpsConnectorBuilder::new()
                .with_native_roots()
                .map_err(|e| anyhow!("Failed to load native certs: {}", e))?
                .https_or_http()
                .enable_http1()
                .build()
        };

        Ok(Self {
            connector: Arc::new(connector),
            timeout,
        })
    }

    /// 发送 HTTP 请求 - 使用 oha 的优化策略
    /// 
    /// # 参数
    /// - `state`: 客户端状态，用于连接复用
    /// - `method`: HTTP 方法
    /// - `url`: 目标 URL
    /// - `headers`: 请求头
    /// - `body`: 请求体
    pub async fn request(
        &self,
        state: &mut ClientState,
        method: &str,
        url: &str,
        headers: &HashMap<String, String>,
        body: Option<&str>,
    ) -> Result<(u16, usize)> {
        let do_req = async {
            // 解析 URL
            let uri: Uri = url.parse().map_err(|e| anyhow!("Invalid URL: {}", e))?;

            // 构建 HTTP 方法
            let http_method = match method.to_uppercase().as_str() {
                "GET" => Method::GET,
                "POST" => Method::POST,
                "PUT" => Method::PUT,
                "DELETE" => Method::DELETE,
                "HEAD" => Method::HEAD,
                "PATCH" => Method::PATCH,
                "OPTIONS" => Method::OPTIONS,
                _ => Method::GET,
            };

            // 构建请求体
            let body_data = if let Some(data) = body {
                Full::new(Bytes::from(data.to_string()))
            } else {
                Full::new(Bytes::new())
            };

            // 构建请求
            let mut request = Request::builder()
                .method(http_method)
                .uri(uri.clone());

            // 添加 Host header（HTTP/1.1 必需）
            if let Some(host) = uri.host() {
                let host_value = if let Some(port) = uri.port_u16() {
                    format!("{}:{}", host, port)
                } else {
                    host.to_string()
                };
                request = request.header("Host", host_value);
            }

            // 添加 headers
            for (key, value) in headers {
                request = request.header(key, value);
            }

            let request = request
                .body(body_data)
                .map_err(|e| anyhow!("Failed to build request: {}", e))?;

            // 获取或创建连接（关键优化：连接复用）
            let mut send_request = if let Some(sr) = state.send_request.take() {
                sr
            } else {
                // 建立新连接
                self.establish_connection(&uri).await?
            };

            // 检查连接是否可用，如果不可用则重连（oha 的策略）
            while send_request.ready().await.is_err() {
                send_request = self.establish_connection(&uri).await?;
            }

            // 发送请求
            match send_request.send_request(request).await {
                Ok(res) => {
                    let (parts, mut stream) = res.into_parts();
                    let status = parts.status.as_u16();

                    // 流式读取响应体（关键优化：不完整缓存）
                    let mut len_bytes = 0;
                    while let Some(chunk) = stream.frame().await {
                        if let Ok(frame) = chunk {
                            len_bytes += frame.data_ref().map(|d| d.len()).unwrap_or_default();
                        }
                    }

                    // 保存连接以便复用（关键优化：连接复用）
                    state.send_request = Some(send_request);

                    Ok::<_, anyhow::Error>((status, len_bytes))
                }
                Err(e) => {
                    // 即使出错也保存连接，下次会重连
                    state.send_request = Some(send_request);
                    Err(anyhow!("Request failed: {}", e))
                }
            }
        };

        // 超时控制
        if self.timeout.as_secs() > 0 {
            tokio::select! {
                res = do_req => res,
                _ = tokio::time::sleep(self.timeout) => {
                    Err(anyhow!("Request timeout"))
                }
            }
        } else {
            do_req.await
        }
    }

    /// 建立 HTTP/1.1 连接
    async fn establish_connection(
        &self,
        uri: &Uri,
    ) -> Result<http1::SendRequest<Full<Bytes>>> {
        // 通过 connector 建立 TCP 连接
        use tower::Service;
        let mut connector = self.connector.as_ref().clone();
        let stream = connector.call(uri.clone()).await
            .map_err(|e| anyhow!("Failed to connect: {}", e))?;

        // 创建 HTTP/1.1 handshake
        let (send_request, conn) = http1::handshake(stream)
            .await
            .map_err(|e| anyhow!("Failed to handshake: {}", e))?;

        // 在后台运行连接
        tokio::spawn(async move {
            if let Err(_e) = conn.await {
                // 连接错误，静默处理
            }
        });

        Ok(send_request)
    }

}

/// 连接池管理器
pub struct ConnectionPool {
    clients: Vec<Arc<HttpClient>>,
    next_index: std::sync::atomic::AtomicUsize,
}

impl ConnectionPool {
    /// 创建连接池
    /// 
    /// # 参数
    /// - `pool_size`: 连接池中客户端数量
    /// - `timeout`: 请求超时时间
    /// - `connections_per_client`: 每个客户端的连接数
    /// - `enable_http2`: 是否启用 HTTP/2
    pub fn new(
        pool_size: usize,
        timeout: Duration,
        connections_per_client: usize,
        enable_http2: bool,
    ) -> Result<Self> {
        let mut clients = Vec::with_capacity(pool_size);
        
        for _ in 0..pool_size {
            let client = HttpClient::new(timeout, connections_per_client, enable_http2)?;
            clients.push(Arc::new(client));
        }

        Ok(Self {
            clients,
            next_index: std::sync::atomic::AtomicUsize::new(0),
        })
    }

    /// 获取一个客户端（轮询）
    pub fn get_client(&self) -> Arc<HttpClient> {
        let index = self.next_index.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.clients[index % self.clients.len()].clone()
    }
}
