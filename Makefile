.PHONY: build release test check clean install run help

# Default target
help:
	@echo "Available targets:"
	@echo "  build    - Build debug version"
	@echo "  release  - Build optimized release version"
	@echo "  test     - Run tests"
	@echo "  check    - Check code without building"
	@echo "  clean    - Clean build artifacts"
	@echo "  install  - Install binary globally"
	@echo "  run      - Run with example (requires ARGS)"
	@echo "  fmt      - Format code"
	@echo "  clippy   - Run clippy lints"

# Build debug version
build:
	cargo build

# Build release version
release:
	cargo build --release

# Run tests
test:
	cargo test

# Check code
check:
	cargo check

# Format code
fmt:
	cargo fmt

# Run clippy
clippy:
	cargo clippy -- -D warnings

# Clean build artifacts
clean:
	cargo clean

# Install globally
install:
	cargo install --path .

# Run with arguments (use: make run ARGS="-c 10 -d 5s https://httpbin.org/get")
run:
	cargo run --release -- $(ARGS)

# Quick test against httpbin
test-httpbin:
	cargo run --release -- -c 10 -d 5s https://httpbin.org/get

# Test curl parsing
test-curl:
	cargo run --release -- --parse-curl "curl -X POST https://httpbin.org/post -d 'test=data'" -c 5 -d 3s

# Test batch config
test-batch:
	cargo run --release -- --batch-config examples/batch-config.yaml

# Start mock server
mock-server:
	cargo run --release -- --mock-server --mock-config examples/mock-server.yaml
