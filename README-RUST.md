# hurl - Rust Implementation

This is a Rust implementation of **hurl**, a modern, high-performance HTTP benchmarking tool inspired by **wrk**, with native support for parsing **curl** commands.

## Status

This is an initial implementation that provides the core functionality described in the main README.md. The following features are implemented:

### ✅ Implemented Features

- ✅ Basic HTTP load testing
- ✅ Parse curl commands with `--parse-curl` option
- ✅ Multiple curl commands from file with load distribution strategies
- ✅ Detailed statistics (requests, latency, throughput)
- ✅ Per-endpoint statistics for multi-endpoint tests
- ✅ Configurable connections, threads, and duration
- ✅ Latency distribution analysis
- ✅ Load strategies: random, round-robin
- ✅ Template variables (random, uuid, timestamp, sequence, choice)
- ✅ Custom variable definitions with `--var`
- ✅ Batch testing with YAML/JSON configuration
- ✅ Mock HTTP server with configurable routes
- ✅ Rate limiting
- ✅ Multiple HTTP methods (GET, POST, PUT, DELETE, etc.)
- ✅ Custom headers and request bodies

### 🚧 Planned Features

- ⏳ Live Terminal UI with real-time charts (placeholder implemented)
- ⏳ Advanced async I/O optimizations
- ⏳ Additional output formats
- ⏳ More comprehensive error handling

## Building

```bash
# Build in debug mode
cargo build

# Build optimized release version
cargo build --release

# The binary will be at target/release/hurl
```

## Installation

```bash
# Install from local source
cargo install --path .

# Or run directly
cargo run --release -- [OPTIONS] <URL>
```

## Quick Start

```bash
# Simple GET request
cargo run --release -- -c 100 -d 30s https://httpbin.org/get

# Parse a curl command
cargo run --release -- --parse-curl "curl -X POST -H 'Content-Type: application/json' -d '{\"name\":\"test\"}' https://httpbin.org/post" -c 50 -d 10s

# Multiple endpoints from file
cargo run --release -- --parse-curl-file examples/endpoints.txt -c 100 -d 30s

# With template variables
cargo run --release -- --var user_id=random:1-1000 -c 50 -d 30s 'https://httpbin.org/anything/{{user_id}}'

# Batch testing
cargo run --release -- --batch-config examples/batch-config.yaml

# Start mock server
cargo run --release -- --mock-server --mock-config examples/mock-server.yaml
```

## Usage Examples

### Basic Load Test

```bash
cargo run --release -- -c 100 -d 30s -t 4 https://httpbin.org/get
```

### POST Request with JSON

```bash
cargo run --release -- -c 50 -d 10s -X POST \
  -H "Content-Type: application/json" \
  --data '{"key":"value"}' \
  https://httpbin.org/post
```

### Using Curl Commands

```bash
cargo run --release -- --parse-curl \
  "curl -X POST -H 'Authorization: Bearer token123' -H 'Content-Type: application/json' -d '{\"user\":\"test\"}' https://httpbin.org/post" \
  -c 50 -d 10s --latency
```

### Template Variables

```bash
# Random user IDs
cargo run --release -- -c 50 -d 30s 'https://httpbin.org/anything/user/{{random:1-1000}}'

# UUID sessions
cargo run --release -- -c 20 -d 60s 'https://httpbin.org/anything?session={{uuid}}'

# Custom variables
cargo run --release -- \
  --var user_id=random:1-10000 \
  --var action=choice:view,edit,delete \
  -c 30 -d 45s \
  'https://httpbin.org/anything/users/{{user_id}}/{{action}}'
```

### Batch Testing

Create a batch configuration file:

```yaml
version: "1.0"
tests:
  - name: "Health Check"
    curl: 'curl https://httpbin.org/get'
    connections: 10
    duration: "10s"
    
  - name: "POST Test"
    curl: 'curl -X POST https://httpbin.org/post -d "test=data"'
    connections: 20
    duration: "15s"
```

Run the batch tests:

```bash
cargo run --release -- --batch-config batch-tests.yaml
```

### Mock Server

Start a mock server for testing:

```bash
# Simple mock server
cargo run --release -- --mock-server --mock-port 8080

# With configuration file
cargo run --release -- --mock-server --mock-config examples/mock-server.yaml
```

Then test against it:

```bash
# In another terminal
cargo run --release -- -c 100 -d 30s http://localhost:8080/fast
```

## Command Line Options

See the main README.md for a complete list of options. All options from the Go version are supported.

## Architecture

The Rust implementation is organized into the following modules:

- **cli**: Command-line argument parsing using clap
- **curl_parser**: Parsing curl commands into HTTP requests
- **engine**: Core benchmarking engine with async workers
- **stats**: Statistics collection and reporting
- **template**: Template variable processing
- **batch**: Batch testing configuration and execution
- **mock_server**: Mock HTTP server for testing
- **ui**: Terminal UI (placeholder for future implementation)

## Performance

The Rust implementation uses:

- **tokio**: Async runtime for efficient I/O
- **reqwest**: High-performance HTTP client
- **hdrhistogram**: Accurate latency measurements
- **rayon**: Parallel processing where applicable

Performance characteristics:

- Low memory footprint
- Efficient async I/O
- Minimal allocation during testing
- Multi-threaded worker architecture

## Testing

```bash
# Run unit tests
cargo test

# Run with verbose output
cargo test -- --nocapture

# Run specific test
cargo test test_parse_curl_command
```

## Development

```bash
# Check code
cargo check

# Format code
cargo fmt

# Run clippy lints
cargo clippy

# Build documentation
cargo doc --open
```

## Differences from Go Version

This Rust implementation aims to be functionally equivalent to the Go version, with some differences:

1. **Live UI**: Currently a placeholder (planned for future implementation)
2. **HTTP Client**: Uses reqwest instead of a custom pulse library
3. **Performance**: Different performance characteristics due to Rust's async model
4. **Error Handling**: Uses Rust's Result type and anyhow for error handling

## Contributing

Contributions are welcome! Areas that need work:

- Live terminal UI implementation using ratatui
- Performance optimizations
- Additional output formats
- More comprehensive tests
- Documentation improvements

## License

MIT License - see LICENSE file for details.
