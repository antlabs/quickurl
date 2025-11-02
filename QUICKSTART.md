# Quick Start Guide - hurl (Rust Implementation)

## Build

```bash
# Build release version (optimized)
cargo build --release

# The binary will be at: target/release/hurl
```

## Basic Usage

### 1. Simple GET Request

```bash
./target/release/hurl -c 100 -d 30s https://httpbin.org/get
```

Options:
- `-c 100`: Use 100 concurrent connections
- `-d 30s`: Run test for 30 seconds

### 2. POST Request with JSON

```bash
./target/release/hurl -c 50 -d 10s \
  -X POST \
  -H "Content-Type: application/json" \
  --data '{"key":"value"}' \
  https://httpbin.org/post
```

### 3. Parse Curl Command

```bash
./target/release/hurl --parse-curl \
  "curl -X POST -H 'Content-Type: application/json' -d '{\"name\":\"test\"}' https://httpbin.org/post" \
  -c 50 -d 10s
```

### 4. Multiple Endpoints from File

Create a file `endpoints.txt`:
```
curl https://httpbin.org/get
curl -X POST https://httpbin.org/post -d "test=data"
curl https://httpbin.org/uuid
```

Run the test:
```bash
./target/release/hurl --parse-curl-file endpoints.txt -c 100 -d 30s
```

### 5. Template Variables

```bash
# Random user IDs
./target/release/hurl -c 50 -d 30s \
  'https://httpbin.org/anything/user/{{random:1-1000}}'

# UUID sessions
./target/release/hurl -c 20 -d 60s \
  'https://httpbin.org/anything?session={{uuid}}'

# Custom variables
./target/release/hurl \
  --var user_id=random:1-10000 \
  --var action=choice:view,edit,delete \
  -c 30 -d 45s \
  'https://httpbin.org/anything/users/{{user_id}}/{{action}}'
```

### 6. Batch Testing

Create `batch-config.yaml`:
```yaml
version: "1.0"
tests:
  - name: "GET Test"
    curl: 'curl https://httpbin.org/get'
    connections: 10
    duration: "10s"
    
  - name: "POST Test"
    curl: 'curl -X POST https://httpbin.org/post -d "test=data"'
    connections: 20
    duration: "15s"
```

Run batch tests:
```bash
./target/release/hurl --batch-config batch-config.yaml
```

### 7. Mock Server

Start a mock server:
```bash
./target/release/hurl --mock-server --mock-config examples/mock-server.yaml
```

In another terminal, test against it:
```bash
./target/release/hurl -c 100 -d 30s http://localhost:8080/fast
```

## Common Options

- `-c, --connections <N>`: Number of connections (default: 10)
- `-d, --duration <TIME>`: Test duration (e.g., 10s, 5m, 1h)
- `-t, --threads <N>`: Number of threads (default: 2)
- `-R, --rate <N>`: Rate limit in requests/sec (0=unlimited)
- `--latency`: Show detailed latency percentiles
- `--verbose`: Verbose output
- `-X, --method <METHOD>`: HTTP method (GET, POST, etc.)
- `-H, --header <HEADER>`: Add custom header

## Output Example

```
Running 30s test @ https://httpbin.org/get
  2 threads and 100 connections

2847 requests in 30.02s, 1.23MB read
Requests/sec:   94.83
Transfer/sec:   0.04MB

Latency Stats:
  Avg:      1054.23ms
  Min:      234.56ms
  Max:      3456.78ms
  Stdev:    456.78ms

Status Code Distribution:
  [200] 2847 (100.00%)
```

## Installation

To install globally:

```bash
cargo install --path .
```

Then you can use `hurl` directly:

```bash
hurl -c 100 -d 30s https://httpbin.org/get
```

## Next Steps

- Read [README-RUST.md](README-RUST.md) for detailed documentation
- Check [examples/](examples/) directory for configuration examples
- See main [README.md](README.md) for complete feature list
