use anyhow::{anyhow, Result};
use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::{Method, Request, Uri};
use hyper_rustls::HttpsConnectorBuilder;
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

type HttpsConnector = hyper_rustls::HttpsConnector<hyper_util::client::legacy::connect::HttpConnector>;

/// 高性能 HTTP 客户端，基于 hyper 1.4
pub struct HttpClient {
    client: Client<HttpsConnector, Full<Bytes>>,
    timeout: Duration,
}

impl HttpClient {
    /// 创建新的 HTTP 客户端
    /// 
    /// # 参数
    /// - `timeout`: 请求超时时间
    /// - `pool_size`: 连接池大小
    /// - `enable_http2`: 是否启用 HTTP/2（默认只使用 HTTP/1.1）
    pub async fn new(timeout: Duration, pool_size: usize, enable_http2: bool) -> Result<Self> {
        // 初始化 rustls crypto provider（只需要初始化一次）
        let _ = rustls::crypto::ring::default_provider().install_default();
        
        // 根据参数决定是否启用 HTTP/2，构建不同的连接器
        let https = if enable_http2 {
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

        // 创建 HTTP 客户端
        let client = Client::builder(TokioExecutor::new())
            .pool_idle_timeout(Duration::from_secs(90))
            .pool_max_idle_per_host(pool_size)
            .build(https);

        Ok(Self {
            client,
            timeout,
        })
    }

    /// 发送 HTTP 请求
    pub async fn request(
        &self,
        method: &str,
        url: &str,
        headers: &HashMap<String, String>,
        body: Option<&str>,
    ) -> Result<(u16, usize)> {
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
            .uri(uri);

        // 添加 headers
        for (key, value) in headers {
            request = request.header(key, value);
        }

        let request = request
            .body(body_data)
            .map_err(|e| anyhow!("Failed to build request: {}", e))?;

        // 发送请求（带超时）
        let response = tokio::time::timeout(self.timeout, self.client.request(request))
            .await
            .map_err(|_| anyhow!("Request timeout"))?
            .map_err(|e| anyhow!("Request failed: {}", e))?;

        // 获取状态码
        let status = response.status().as_u16();

        // 读取响应体
        let body = response.into_body();
        let bytes = body
            .collect()
            .await
            .map_err(|e| anyhow!("Failed to read response body: {}", e))?
            .to_bytes();

        Ok((status, bytes.len()))
    }

    /// 发送 HTTP 请求（流式读取，不完整读取 body）
    pub async fn request_streaming(
        &self,
        method: &str,
        url: &str,
        headers: &HashMap<String, String>,
        body: Option<&str>,
    ) -> Result<(u16, usize)> {
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
            .uri(uri);

        // 添加 headers
        for (key, value) in headers {
            request = request.header(key, value);
        }

        let request = request
            .body(body_data)
            .map_err(|e| anyhow!("Failed to build request: {}", e))?;

        // 发送请求（带超时）
        let response = tokio::time::timeout(self.timeout, self.client.request(request))
            .await
            .map_err(|_| anyhow!("Request timeout"))?
            .map_err(|e| anyhow!("Request failed: {}", e))?;

        // 获取状态码
        let status = response.status().as_u16();

        // 流式读取响应体（只读取第一个 chunk 或者直接丢弃）
        let mut body = response.into_body();
        let mut total_size = 0;

        // 读取所有数据块来计算大小
        while let Some(chunk) = body.frame().await {
            if let Ok(frame) = chunk {
                if let Some(data) = frame.data_ref() {
                    total_size += data.len();
                }
            }
        }

        Ok((status, total_size))
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
    pub async fn new(
        pool_size: usize,
        timeout: Duration,
        connections_per_client: usize,
        enable_http2: bool,
    ) -> Result<Self> {
        let mut clients = Vec::with_capacity(pool_size);
        
        for _ in 0..pool_size {
            let client = HttpClient::new(timeout, connections_per_client, enable_http2).await?;
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
