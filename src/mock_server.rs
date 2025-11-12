use crate::cli::Args;
use anyhow::Result;
use http_body_util::{BodyExt, Full};
use hyper::body::Bytes;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Error as HyperError, Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::signal;
use tokio::time::{sleep, Instant};
use tracing::{info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockConfig {
    pub port: Option<u16>,
    pub routes: Option<Vec<RouteConfig>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteConfig {
    pub path: String,
    #[serde(default = "default_method")]
    pub method: String,
    #[serde(default = "default_status_code")]
    pub status_code: u16,
    pub response: Option<String>,
    pub delay: Option<String>,
    #[serde(default)]
    pub echo: bool,
}

fn default_method() -> String {
    "GET".to_string()
}

fn default_status_code() -> u16 {
    200
}

#[derive(Debug, Clone)]
struct Route {
    path: String,
    method: Method,
    status_code: StatusCode,
    response: Option<String>,
    delay: Option<std::time::Duration>,
    echo: bool,
}

#[derive(Debug, Clone)]
struct MockServerState {
    routes: Vec<Route>,
}

fn parse_duration_string(s: &str) -> Result<std::time::Duration> {
    let s = s.trim();

    if s.ends_with("ms") {
        let num: u64 = s[..s.len() - 2].parse()?;
        Ok(std::time::Duration::from_millis(num))
    } else if s.ends_with('s') {
        let num: u64 = s[..s.len() - 1].parse()?;
        Ok(std::time::Duration::from_secs(num))
    } else if s.ends_with('m') {
        let num: u64 = s[..s.len() - 1].parse()?;
        Ok(std::time::Duration::from_secs(num * 60))
    } else if s.ends_with('h') {
        let num: u64 = s[..s.len() - 1].parse()?;
        Ok(std::time::Duration::from_secs(num * 3600))
    } else {
        // Default to seconds
        let num: u64 = s.parse()?;
        Ok(std::time::Duration::from_secs(num))
    }
}

fn load_config_file(path: &PathBuf) -> Result<MockConfig> {
    let content = std::fs::read_to_string(path)?;

    if path.extension().and_then(|s| s.to_str()) == Some("json") {
        let config: MockConfig = serde_json::from_str(&content)?;
        Ok(config)
    } else {
        let config: MockConfig = serde_yaml::from_str(&content)?;
        Ok(config)
    }
}

fn parse_routes(config_routes: Option<Vec<RouteConfig>>) -> Result<Vec<Route>> {
    let mut routes = Vec::new();

    if let Some(config_routes) = config_routes {
        for route_config in config_routes {
            let method = match route_config.method.to_uppercase().as_str() {
                "GET" => Method::GET,
                "POST" => Method::POST,
                "PUT" => Method::PUT,
                "DELETE" => Method::DELETE,
                "PATCH" => Method::PATCH,
                "HEAD" => Method::HEAD,
                "OPTIONS" => Method::OPTIONS,
                _ => {
                    warn!(
                        "Unknown HTTP method: {}, defaulting to GET",
                        route_config.method
                    );
                    Method::GET
                }
            };

            let status_code =
                StatusCode::from_u16(route_config.status_code).unwrap_or(StatusCode::OK);

            let delay = route_config
                .delay
                .as_ref()
                .and_then(|d| parse_duration_string(d).ok());

            routes.push(Route {
                path: route_config.path,
                method,
                status_code,
                response: route_config.response,
                delay,
                echo: route_config.echo,
            });
        }
    }

    Ok(routes)
}

fn build_server_state(args: &Args) -> Result<MockServerState> {
    let routes = if let Some(config_path) = &args.mock_config {
        // Load from config file
        let config = load_config_file(config_path)?;
        let port = config.port.unwrap_or(args.mock_port);
        if port != args.mock_port && args.mock_port != 8080 {
            warn!(
                "Port in config file ({}) differs from command line ({}), using config file port",
                port, args.mock_port
            );
        }
        parse_routes(config.routes)?
    } else {
        // Build from command line arguments
        let mut routes = Vec::new();

        // If we have command line args, create a default route
        if args.mock_response.is_some() || args.mock_delay.is_some() || args.mock_status != 200 {
            let delay = args
                .mock_delay
                .as_ref()
                .and_then(|d| parse_duration_string(d).ok());

            let status_code = StatusCode::from_u16(args.mock_status).unwrap_or(StatusCode::OK);

            routes.push(Route {
                path: "*".to_string(), // Match all paths
                method: Method::GET,
                status_code,
                response: args.mock_response.clone(),
                delay,
                echo: false,
            });
        }

        routes
    };

    Ok(MockServerState { routes })
}

async fn handle_request(
    req: Request<hyper::body::Incoming>,
    state: Arc<MockServerState>,
) -> Result<Response<Full<Bytes>>> {
    let start_time = Instant::now();
    let method = req.method().clone();
    let uri = req.uri().clone();
    let path = uri.path();
    let query = uri.query().unwrap_or("");

    // Collect headers
    let headers: HashMap<String, String> = req
        .headers()
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
        .collect();

    // Read body
    let body_bytes = req.collect().await?.to_bytes();
    let body_str = String::from_utf8_lossy(&body_bytes).to_string();

    // Find matching route
    let matched_route = state
        .routes
        .iter()
        .find(|route| route.method == method && (route.path == "*" || route.path == path));

    let (status_code, response_body, echo_mode) = if let Some(route) = matched_route {
        // Apply delay if configured
        if let Some(delay) = route.delay {
            sleep(delay).await;
        }

        if route.echo {
            (route.status_code, None, true)
        } else {
            (route.status_code, route.response.clone(), false)
        }
    } else {
        // Default handler - echo mode
        (StatusCode::OK, None, true)
    };

    // Build response body
    let response_body = if echo_mode {
        // Echo mode - return request details
        let echo_response = serde_json::json!({
            "method": method.as_str(),
            "path": path,
            "query": query,
            "headers": headers,
            "body": body_str,
        });
        serde_json::to_string_pretty(&echo_response)?
    } else {
        response_body.unwrap_or_else(|| r#"{"message": "OK"}"#.to_string())
    };

    // Calculate delay time
    let elapsed = start_time.elapsed();

    // Log request
    info!(
        "{} {} -> {} ({}ms)",
        method,
        path,
        status_code.as_u16(),
        elapsed.as_millis()
    );

    // Build response
    let response = Response::builder()
        .status(status_code)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(response_body)))?;

    Ok(response)
}

pub async fn run(args: Args) -> Result<()> {
    let state = Arc::new(build_server_state(&args)?);

    // Determine port
    let port = if let Some(config_path) = &args.mock_config {
        let config = load_config_file(config_path)?;
        config.port.unwrap_or(args.mock_port)
    } else {
        args.mock_port
    };

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = TcpListener::bind(addr).await?;

    info!("Mock server listening on http://0.0.0.0:{}", port);
    info!("Press Ctrl+C to stop");

    if state.routes.is_empty() {
        info!("No routes configured, all requests will be handled by default echo handler");
    } else {
        info!("Configured {} route(s)", state.routes.len());
        for route in &state.routes {
            info!("  {} {} -> {}", route.method, route.path, route.status_code);
        }
    }

    loop {
        tokio::select! {
            result = listener.accept() => {
                match result {
                    Ok((stream, _)) => {
                        let io = TokioIo::new(stream);
                        let state_clone = state.clone();

                        tokio::task::spawn(async move {
                            let service = service_fn(move |req| {
                                let state = state_clone.clone();
                                async move {
                                    match handle_request(req, state).await {
                                        Ok(response) => Ok::<Response<Full<Bytes>>, HyperError>(response),
                                        Err(e) => {
                                            warn!("Error handling request: {}", e);
                                            let error_response = Response::builder()
                                                .status(StatusCode::INTERNAL_SERVER_ERROR)
                                                .header("Content-Type", "application/json")
                                                .body(Full::new(Bytes::from(format!(r#"{{"error": "{}"}}"#, e))))
                                                .unwrap();
                                            Ok(error_response)
                                        }
                                    }
                                }
                            });

                            if let Err(err) = http1::Builder::new()
                                .serve_connection(io, service)
                                .await
                            {
                                warn!("Error serving connection: {}", err);
                            }
                        });
                    }
                    Err(e) => {
                        warn!("Failed to accept connection: {}", e);
                    }
                }
            }
            _ = signal::ctrl_c() => {
                info!("Shutting down mock server...");
                break;
            }
        }
    }

    Ok(())
}
