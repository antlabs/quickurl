use crate::cli::Args;
use anyhow::Result;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server, StatusCode};
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

#[derive(Debug, Deserialize, Serialize, Clone)]
struct MockConfig {
    #[serde(default = "default_port")]
    port: u16,
    routes: Vec<RouteConfig>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct RouteConfig {
    path: String,
    #[serde(default = "default_method")]
    method: String,
    #[serde(default = "default_status")]
    status_code: u16,
    #[serde(default)]
    response: String,
    #[serde(default)]
    delay: Option<String>,
    #[serde(default)]
    echo: bool,
}

fn default_port() -> u16 { 8080 }
fn default_method() -> String { "GET".to_string() }
fn default_status() -> u16 { 200 }

pub async fn run(args: Args) -> Result<()> {
    let config = if let Some(config_path) = &args.mock_config {
        load_mock_config(config_path)?
    } else {
        // Create simple config from args
        MockConfig {
            port: args.mock_port,
            routes: vec![RouteConfig {
                path: "/".to_string(),
                method: "GET".to_string(),
                status_code: args.mock_status,
                response: args.mock_response.unwrap_or_else(|| "{}".to_string()),
                delay: args.mock_delay.clone(),
                echo: false,
            }],
        }
    };

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    let config = Arc::new(config);

    println!("Mock HTTP Server starting on http://{}", addr);
    println!("Routes:");
    for route in &config.routes {
        println!("  {} {} -> {} (delay: {:?})", 
            route.method, 
            route.path, 
            route.status_code,
            route.delay
        );
    }
    println!("\nPress Ctrl+C to stop\n");

    let make_svc = make_service_fn(move |_conn| {
        let config = config.clone();
        async move {
            Ok::<_, Infallible>(service_fn(move |req| {
                handle_request(req, config.clone())
            }))
        }
    });

    let server = Server::bind(&addr).serve(make_svc);

    if let Err(e) = server.await {
        eprintln!("Server error: {}", e);
    }

    Ok(())
}

async fn handle_request(
    req: Request<Body>,
    config: Arc<MockConfig>,
) -> Result<Response<Body>, Infallible> {
    let method = req.method().clone();
    let path = req.uri().path().to_string();
    
    println!("{} {}", method, path);

    // Find matching route
    let route = config.routes.iter().find(|r| {
        r.path == path && r.method.to_uppercase() == method.as_str()
    });

    let route = match route {
        Some(r) => r,
        None => {
            // No route found, return 404
            return Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::from("Not Found"))
                .unwrap());
        }
    };

    // Apply delay if configured
    if let Some(delay_str) = &route.delay {
        if let Ok(delay) = parse_delay(delay_str) {
            sleep(delay).await;
        }
    }

    // Build response
    let response_body = if route.echo {
        // Echo request details
        let body_bytes = hyper::body::to_bytes(req.into_body()).await.unwrap();
        let body_str = String::from_utf8_lossy(&body_bytes);
        
        format!(
            r#"{{"method":"{}","path":"{}","body":{}}}"#,
            method,
            path,
            if body_str.is_empty() { "null".to_string() } else { format!("\"{}\"", body_str) }
        )
    } else {
        route.response.clone()
    };

    Ok(Response::builder()
        .status(StatusCode::from_u16(route.status_code).unwrap())
        .header("Content-Type", "application/json")
        .body(Body::from(response_body))
        .unwrap())
}

fn load_mock_config(path: &Path) -> Result<MockConfig> {
    let content = std::fs::read_to_string(path)?;
    
    if path.extension().and_then(|s| s.to_str()) == Some("json") {
        Ok(serde_json::from_str(&content)?)
    } else {
        Ok(serde_yaml::from_str(&content)?)
    }
}

fn parse_delay(s: &str) -> Result<Duration> {
    let s = s.trim();
    
    if s.ends_with("ms") {
        let num: u64 = s[..s.len() - 2].parse()?;
        Ok(Duration::from_millis(num))
    } else if s.ends_with('s') {
        let num: u64 = s[..s.len() - 1].parse()?;
        Ok(Duration::from_secs(num))
    } else {
        let num: u64 = s.parse()?;
        Ok(Duration::from_millis(num))
    }
}
