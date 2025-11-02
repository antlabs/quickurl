use anyhow::{anyhow, Result};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct CurlCommand {
    pub url: String,
    pub method: String,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
}

impl CurlCommand {
    pub fn new(url: String) -> Self {
        Self {
            url,
            method: "GET".to_string(),
            headers: HashMap::new(),
            body: None,
        }
    }
}

pub fn parse_curl_command(cmd: &str) -> Result<CurlCommand> {
    let cmd = cmd.trim();
    
    // Remove leading "curl" if present
    let cmd = if cmd.starts_with("curl ") {
        &cmd[5..]
    } else if cmd.starts_with("curl") {
        &cmd[4..]
    } else {
        cmd
    };

    let mut url = String::new();
    let mut method = "GET".to_string();
    let mut headers = HashMap::new();
    let mut body: Option<String> = None;

    // Tokenize the command
    let tokens = tokenize_curl_command(cmd)?;
    let mut i = 0;

    while i < tokens.len() {
        let token = &tokens[i];

        match token.as_str() {
            "-X" | "--request" => {
                i += 1;
                if i < tokens.len() {
                    method = tokens[i].to_uppercase();
                }
            }
            "-H" | "--header" => {
                i += 1;
                if i < tokens.len() {
                    parse_header(&tokens[i], &mut headers)?;
                }
            }
            "-d" | "--data" | "--data-raw" | "--data-binary" => {
                i += 1;
                if i < tokens.len() {
                    body = Some(tokens[i].clone());
                    if method == "GET" {
                        method = "POST".to_string();
                    }
                }
            }
            "--data-urlencode" => {
                i += 1;
                if i < tokens.len() {
                    body = Some(tokens[i].clone());
                    if method == "GET" {
                        method = "POST".to_string();
                    }
                }
            }
            "-u" | "--user" => {
                i += 1;
                if i < tokens.len() {
                    let auth = format!("Basic {}", base64_encode(&tokens[i]));
                    headers.insert("Authorization".to_string(), auth);
                }
            }
            "-A" | "--user-agent" => {
                i += 1;
                if i < tokens.len() {
                    headers.insert("User-Agent".to_string(), tokens[i].clone());
                }
            }
            "-e" | "--referer" => {
                i += 1;
                if i < tokens.len() {
                    headers.insert("Referer".to_string(), tokens[i].clone());
                }
            }
            "--compressed" => {
                headers.insert("Accept-Encoding".to_string(), "gzip, deflate".to_string());
            }
            _ => {
                // If it doesn't start with -, it's likely the URL
                if !token.starts_with('-') && url.is_empty() {
                    url = token.clone();
                }
            }
        }

        i += 1;
    }

    if url.is_empty() {
        return Err(anyhow!("No URL found in curl command"));
    }

    Ok(CurlCommand {
        url,
        method,
        headers,
        body,
    })
}

fn tokenize_curl_command(cmd: &str) -> Result<Vec<String>> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_quote = false;
    let mut quote_char = ' ';
    let mut escaped = false;

    for ch in cmd.chars() {
        if escaped {
            current.push(ch);
            escaped = false;
            continue;
        }

        if ch == '\\' {
            escaped = true;
            continue;
        }

        if ch == '"' || ch == '\'' {
            if !in_quote {
                in_quote = true;
                quote_char = ch;
            } else if ch == quote_char {
                in_quote = false;
            } else {
                current.push(ch);
            }
            continue;
        }

        if ch.is_whitespace() && !in_quote {
            if !current.is_empty() {
                tokens.push(current.clone());
                current.clear();
            }
        } else {
            current.push(ch);
        }
    }

    if !current.is_empty() {
        tokens.push(current);
    }

    Ok(tokens)
}

fn parse_header(header: &str, headers: &mut HashMap<String, String>) -> Result<()> {
    if let Some(pos) = header.find(':') {
        let key = header[..pos].trim().to_string();
        let value = header[pos + 1..].trim().to_string();
        headers.insert(key, value);
    }
    Ok(())
}

fn base64_encode(s: &str) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(s.as_bytes())
}

pub fn parse_curl_file(path: &std::path::Path) -> Result<Vec<CurlCommand>> {
    let content = std::fs::read_to_string(path)?;
    let mut commands = Vec::new();

    for line in content.lines() {
        let line = line.trim();
        
        // Skip empty lines and comments
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        match parse_curl_command(line) {
            Ok(cmd) => commands.push(cmd),
            Err(e) => {
                eprintln!("Warning: Failed to parse line: {} - {}", line, e);
            }
        }
    }

    if commands.is_empty() {
        return Err(anyhow!("No valid curl commands found in file"));
    }

    Ok(commands)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_get() {
        let cmd = "curl https://example.com";
        let parsed = parse_curl_command(cmd).unwrap();
        assert_eq!(parsed.url, "https://example.com");
        assert_eq!(parsed.method, "GET");
    }

    #[test]
    fn test_parse_post_with_data() {
        let cmd = r#"curl -X POST -H "Content-Type: application/json" -d '{"key":"value"}' https://api.example.com"#;
        let parsed = parse_curl_command(cmd).unwrap();
        assert_eq!(parsed.url, "https://api.example.com");
        assert_eq!(parsed.method, "POST");
        assert_eq!(parsed.headers.get("Content-Type").unwrap(), "application/json");
        assert_eq!(parsed.body.unwrap(), r#"{"key":"value"}"#);
    }

    #[test]
    fn test_parse_with_headers() {
        let cmd = r#"curl -H "Authorization: Bearer token123" https://api.example.com"#;
        let parsed = parse_curl_command(cmd).unwrap();
        assert_eq!(parsed.headers.get("Authorization").unwrap(), "Bearer token123");
    }
}
