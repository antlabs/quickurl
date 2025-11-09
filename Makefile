.PHONY: build release test check clean install run help cross-linux cross-windows cross-macos cross-all

# Default target
help:
	@echo "Available targets:"
	@echo "  build         - Build debug version"
	@echo "  release       - Build optimized release version"
	@echo "  test          - Run tests"
	@echo "  check         - Check code without building"
	@echo "  clean         - Clean build artifacts"
	@echo "  install       - Install binary globally"
	@echo "  run           - Run with example (requires ARGS)"
	@echo "  fmt           - Format code"
	@echo "  clippy        - Run clippy lints"
	@echo ""
	@echo "Cross-compilation targets:"
	@echo "  cross-linux   - Build for Linux x86_64"
	@echo "  cross-windows - Build for Windows x86_64"
	@echo "  cross-macos   - Build for macOS (Intel & Apple Silicon)"
	@echo "  cross-arm     - Build for Linux ARM64"
	@echo "  cross-all     - Build for all platforms"

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

# Cross-compilation targets
# Note: On Apple Silicon, use cargo-zigbuild for better compatibility
# Install with: brew install zig && cargo install cargo-zigbuild

# Detect if we're on Apple Silicon
UNAME_M := $(shell uname -m)
ifeq ($(UNAME_M),arm64)
    USE_ZIG := 1
else
    USE_ZIG := 0
endif

# Linux x86_64
cross-linux:
	@echo "Building for Linux x86_64..."
ifeq ($(USE_ZIG),1)
	@echo "Using cargo-zigbuild (Apple Silicon detected)..."
	cargo zigbuild --release --target x86_64-unknown-linux-gnu
else
	cross build --release --target x86_64-unknown-linux-gnu
endif
	@echo "Binary: target/x86_64-unknown-linux-gnu/release/quickurl"

# Linux ARM64
cross-arm:
	@echo "Building for Linux ARM64..."
ifeq ($(USE_ZIG),1)
	@echo "Using cargo-zigbuild (Apple Silicon detected)..."
	cargo zigbuild --release --target aarch64-unknown-linux-gnu
else
	cross build --release --target aarch64-unknown-linux-gnu
endif
	@echo "Binary: target/aarch64-unknown-linux-gnu/release/quickurl"

# Windows x86_64
cross-windows:
	@echo "Building for Windows x86_64..."
ifeq ($(USE_ZIG),1)
	@echo "Using cargo-zigbuild (Apple Silicon detected)..."
	cargo zigbuild --release --target x86_64-pc-windows-gnu
else
	cross build --release --target x86_64-pc-windows-gnu
endif
	@echo "Binary: target/x86_64-pc-windows-gnu/release/quickurl.exe"

# macOS Intel
cross-macos-intel:
	@echo "Building for macOS Intel..."
	cargo build --release --target x86_64-apple-darwin
	@echo "Binary: target/x86_64-apple-darwin/release/quickurl"

# macOS Apple Silicon
cross-macos-arm:
	@echo "Building for macOS Apple Silicon..."
	cargo build --release --target aarch64-apple-darwin
	@echo "Binary: target/aarch64-apple-darwin/release/quickurl"

# macOS Universal (both architectures)
cross-macos: cross-macos-intel cross-macos-arm
	@echo "Creating macOS universal binary..."
	@mkdir -p target/universal-apple-darwin/release
	lipo -create \
		target/x86_64-apple-darwin/release/quickurl \
		target/aarch64-apple-darwin/release/quickurl \
		-output target/universal-apple-darwin/release/quickurl
	@echo "Universal binary: target/universal-apple-darwin/release/quickurl"

# Build for all platforms
cross-all: cross-linux cross-arm cross-windows cross-macos
	@echo ""
	@echo "All cross-compilation builds complete!"
	@echo "Binaries:"
	@echo "  Linux x86_64:   target/x86_64-unknown-linux-gnu/release/quickurl"
	@echo "  Linux ARM64:    target/aarch64-unknown-linux-gnu/release/quickurl"
	@echo "  Windows x86_64: target/x86_64-pc-windows-gnu/release/quickurl.exe"
	@echo "  macOS Universal: target/universal-apple-darwin/release/quickurl"

# Install cross-compilation tools
install-cross:
	@echo "Installing cross-compilation tools..."
ifeq ($(USE_ZIG),1)
	@echo "Apple Silicon detected - installing cargo-zigbuild..."
	@if ! command -v zig &> /dev/null; then \
		echo "Installing zig via Homebrew..."; \
		brew install zig; \
	else \
		echo "zig already installed"; \
	fi
	cargo install cargo-zigbuild
else
	@echo "Installing cross..."
	cargo install cross
endif
	@echo "Adding Rust targets..."
	rustup target add x86_64-unknown-linux-gnu
	rustup target add aarch64-unknown-linux-gnu
	rustup target add x86_64-pc-windows-gnu
	rustup target add x86_64-apple-darwin
	rustup target add aarch64-apple-darwin
	@echo "Cross-compilation tools installed!"
