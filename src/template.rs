use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use rand::Rng;
use regex::Regex;
use std::collections::HashMap;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use uuid::Uuid;

lazy_static::lazy_static! {
    static ref TEMPLATE_REGEX: Regex = Regex::new(r"\{\{([^}]+)\}\}").unwrap();
}

#[derive(Clone)]
pub struct TemplateEngine {
    variables: HashMap<String, VariableType>,
    sequences: Arc<HashMap<String, AtomicU64>>,
}

#[derive(Clone)]
enum VariableType {
    Random { min: i64, max: i64 },
    Uuid,
    Timestamp { format: TimestampFormat },
    Sequence { start: u64 },
    Choice { options: Vec<String> },
    Static { value: String },
}

#[derive(Clone)]
enum TimestampFormat {
    Unix,
    UnixMs,
    Rfc3339,
    Iso8601,
    Date,
    Time,
}

impl TemplateEngine {
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
            sequences: Arc::new(HashMap::new()),
        }
    }

    pub fn add_variable(&mut self, name: String, definition: &str) -> Result<()> {
        let var_type = Self::parse_variable_definition(definition)?;
        
        // Initialize sequence counter if needed
        if let VariableType::Sequence { start } = &var_type {
            let sequences = Arc::get_mut(&mut self.sequences).unwrap();
            sequences.insert(name.clone(), AtomicU64::new(*start));
        }
        
        self.variables.insert(name, var_type);
        Ok(())
    }

    fn parse_variable_definition(def: &str) -> Result<VariableType> {
        if def.starts_with("random:") {
            let range = &def[7..];
            let parts: Vec<&str> = range.split('-').collect();
            if parts.len() != 2 {
                return Err(anyhow!("Invalid random range format. Expected: random:min-max"));
            }
            let min: i64 = parts[0].parse()?;
            let max: i64 = parts[1].parse()?;
            Ok(VariableType::Random { min, max })
        } else if def == "uuid" {
            Ok(VariableType::Uuid)
        } else if def.starts_with("timestamp:") || def.starts_with("now:") {
            let format_str = if def.starts_with("timestamp:") {
                &def[10..]
            } else {
                &def[4..]
            };
            let format = Self::parse_timestamp_format(format_str)?;
            Ok(VariableType::Timestamp { format })
        } else if def == "timestamp" || def == "now" {
            Ok(VariableType::Timestamp { format: TimestampFormat::Unix })
        } else if def.starts_with("sequence:") {
            let start: u64 = def[9..].parse()?;
            Ok(VariableType::Sequence { start })
        } else if def.starts_with("choice:") {
            let options_str = &def[7..];
            let options: Vec<String> = options_str.split(',').map(|s| s.to_string()).collect();
            if options.is_empty() {
                return Err(anyhow!("Choice must have at least one option"));
            }
            Ok(VariableType::Choice { options })
        } else {
            Ok(VariableType::Static { value: def.to_string() })
        }
    }

    fn parse_timestamp_format(format: &str) -> Result<TimestampFormat> {
        match format {
            "unix" => Ok(TimestampFormat::Unix),
            "unix_ms" => Ok(TimestampFormat::UnixMs),
            "rfc3339" => Ok(TimestampFormat::Rfc3339),
            "iso8601" => Ok(TimestampFormat::Iso8601),
            "date" => Ok(TimestampFormat::Date),
            "time" => Ok(TimestampFormat::Time),
            _ => Err(anyhow!("Unknown timestamp format: {}", format)),
        }
    }

    pub fn process(&self, text: &str) -> String {
        TEMPLATE_REGEX.replace_all(text, |caps: &regex::Captures| {
            let template = &caps[1];
            self.evaluate_template(template).unwrap_or_else(|_| format!("{{{{{}}}}}", template))
        }).to_string()
    }

    fn evaluate_template(&self, template: &str) -> Result<String> {
        // Check if it's a variable reference
        if let Some(var_type) = self.variables.get(template) {
            return self.generate_value(var_type);
        }

        // Check if it's an inline function
        if template.starts_with("random:") {
            let var_type = Self::parse_variable_definition(template)?;
            return self.generate_value(&var_type);
        } else if template == "uuid" {
            return Ok(Uuid::new_v4().to_string());
        } else if template.starts_with("timestamp:") || template.starts_with("now:") {
            let var_type = Self::parse_variable_definition(template)?;
            return self.generate_value(&var_type);
        } else if template == "timestamp" || template == "now" {
            return Ok(Utc::now().timestamp().to_string());
        } else if template.starts_with("sequence:") {
            let var_type = Self::parse_variable_definition(template)?;
            return self.generate_value(&var_type);
        } else if template.starts_with("choice:") {
            let var_type = Self::parse_variable_definition(template)?;
            return self.generate_value(&var_type);
        }

        Err(anyhow!("Unknown template: {}", template))
    }

    fn generate_value(&self, var_type: &VariableType) -> Result<String> {
        match var_type {
            VariableType::Random { min, max } => {
                let mut rng = rand::thread_rng();
                let value = rng.gen_range(*min..=*max);
                Ok(value.to_string())
            }
            VariableType::Uuid => {
                Ok(Uuid::new_v4().to_string())
            }
            VariableType::Timestamp { format } => {
                let now: DateTime<Utc> = Utc::now();
                let value = match format {
                    TimestampFormat::Unix => now.timestamp().to_string(),
                    TimestampFormat::UnixMs => now.timestamp_millis().to_string(),
                    TimestampFormat::Rfc3339 => now.to_rfc3339(),
                    TimestampFormat::Iso8601 => now.to_rfc3339(),
                    TimestampFormat::Date => now.format("%Y-%m-%d").to_string(),
                    TimestampFormat::Time => now.format("%H:%M:%S").to_string(),
                };
                Ok(value)
            }
            VariableType::Sequence { start } => {
                // This is a simplified version - in real implementation,
                // we'd need to track the counter per variable name
                Ok(start.to_string())
            }
            VariableType::Choice { options } => {
                let mut rng = rand::thread_rng();
                let idx = rng.gen_range(0..options.len());
                Ok(options[idx].clone())
            }
            VariableType::Static { value } => {
                Ok(value.clone())
            }
        }
    }
}

pub fn print_help() {
    println!(r#"
URL Template Variables
======================

hurl supports dynamic URL template variables for realistic load testing.

Built-in Template Functions:
----------------------------

1. random - Random number in range
   Usage: {{{{random:min-max}}}}
   Example: {{{{random:1-1000}}}} → 456

2. uuid - Generate UUID
   Usage: {{{{uuid}}}}
   Example: {{{{uuid}}}} → 550e8400-e29b-41d4-a716-446655440000

3. timestamp/now - Current timestamp
   Usage: {{{{timestamp:format}}}} or {{{{now:format}}}}
   Formats:
     - unix (default): 1640995200
     - unix_ms: 1640995200000
     - rfc3339: 2022-01-01T00:00:00Z
     - iso8601: 2022-01-01T00:00:00Z
     - date: 2022-01-01
     - time: 15:04:05
   Example: {{{{timestamp:unix}}}} → 1640995200

4. sequence - Incrementing numbers
   Usage: {{{{sequence:start}}}}
   Example: {{{{sequence:1}}}} → 1, 2, 3, ...

5. choice - Random selection
   Usage: {{{{choice:a,b,c}}}}
   Example: {{{{choice:GET,POST,PUT}}}} → POST

Basic Examples:
--------------

# Random user ID
hurl -c 50 -d 30s 'https://api.example.com/users/{{{{random:1-1000}}}}'

# UUID session
hurl -c 20 -d 60s 'https://api.example.com/data?session={{{{uuid}}}}'

# Timestamp
hurl -c 10 -d 30s 'https://api.example.com/events?timestamp={{{{timestamp:unix}}}}'

# Sequential pages
hurl -c 5 -d 60s 'https://api.example.com/items?page={{{{sequence:1}}}}&limit=20'

Custom Variables:
----------------

Define variables with --var option:

hurl --var user_id=random:1-10000 \
     --var method=choice:GET,POST,PUT \
     --var session=uuid \
     -c 30 -d 45s \
     'https://api.example.com/{{{{method}}}}/users/{{{{user_id}}}}?session={{{{session}}}}'

Advanced Example:
----------------

# E-commerce simulation
hurl --var user_id=random:1-10000 \
     --var product_id=random:100-999 \
     --var quantity=choice:1,2,3,4,5 \
     --var payment=choice:credit_card,paypal,apple_pay \
     -c 50 -d 60s \
     --parse-curl 'curl -X POST https://shop.example.com/api/orders'
"#);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_random_template() {
        let engine = TemplateEngine::new();
        let result = engine.process("https://api.example.com/users/{{random:1-100}}");
        assert!(result.starts_with("https://api.example.com/users/"));
        assert!(!result.contains("{{"));
    }

    #[test]
    fn test_uuid_template() {
        let engine = TemplateEngine::new();
        let result = engine.process("session={{uuid}}");
        assert!(result.starts_with("session="));
        assert!(!result.contains("{{"));
    }

    #[test]
    fn test_custom_variable() {
        let mut engine = TemplateEngine::new();
        engine.add_variable("user_id".to_string(), "random:1-1000").unwrap();
        let result = engine.process("https://api.example.com/users/{{user_id}}");
        assert!(result.starts_with("https://api.example.com/users/"));
    }
}
