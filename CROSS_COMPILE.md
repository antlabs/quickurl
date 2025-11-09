# 交叉编译指南 / Cross-Compilation Guide

## 概述 / Overview

quickurl 支持交叉编译到多个平台，包括 Linux、Windows 和 macOS。

quickurl supports cross-compilation to multiple platforms including Linux, Windows, and macOS.

## 支持的目标平台 / Supported Target Platforms

| 平台 / Platform | 架构 / Architecture | Target Triple |
|----------------|---------------------|---------------|
| Linux | x86_64 | `x86_64-unknown-linux-gnu` |
| Linux | ARM64 | `aarch64-unknown-linux-gnu` |
| Windows | x86_64 | `x86_64-pc-windows-gnu` |
| macOS | Intel (x86_64) | `x86_64-apple-darwin` |
| macOS | Apple Silicon (ARM64) | `aarch64-apple-darwin` |
| macOS | Universal Binary | Both architectures |

## 安装交叉编译工具 / Install Cross-Compilation Tools

### ⚠️ Apple Silicon (M1/M2/M3) 用户注意

如果你使用的是 Apple Silicon Mac，推荐使用 **cargo-zigbuild** 而不是 `cross`，因为 `cross` 的 Docker 镜像在 ARM64 主机上存在兼容性问题。

Makefile 会自动检测你的架构并使用正确的工具。

If you're on Apple Silicon Mac, we recommend **cargo-zigbuild** instead of `cross` due to Docker image compatibility issues on ARM64 hosts.

The Makefile will automatically detect your architecture and use the correct tool.

### 方法 1: 使用 Makefile (推荐)

```bash
make install-cross
```

**在 Apple Silicon 上，这将安装：**
- `zig` 编译器（通过 Homebrew）
- `cargo-zigbuild` 工具
- 所有必要的 Rust target

**在 Intel Mac/Linux 上，这将安装：**
- `cross` 工具（用于 Linux 和 Windows 交叉编译）
- 所有必要的 Rust target

**On Apple Silicon, this will install:**
- `zig` compiler (via Homebrew)
- `cargo-zigbuild` tool
- All necessary Rust targets

**On Intel Mac/Linux, this will install:**
- `cross` tool (for Linux and Windows cross-compilation)
- All necessary Rust targets

### 方法 2: 手动安装

#### Apple Silicon 用户

```bash
# 安装 zig 编译器
brew install zig

# 安装 cargo-zigbuild
cargo install cargo-zigbuild

# 添加目标平台
rustup target add x86_64-unknown-linux-gnu
rustup target add aarch64-unknown-linux-gnu
rustup target add x86_64-pc-windows-gnu
rustup target add x86_64-apple-darwin
rustup target add aarch64-apple-darwin
```

#### Intel Mac / Linux 用户

```bash
# 安装 cross 工具
cargo install cross

# 添加目标平台
rustup target add x86_64-unknown-linux-gnu
rustup target add aarch64-unknown-linux-gnu
rustup target add x86_64-pc-windows-gnu
rustup target add x86_64-apple-darwin
rustup target add aarch64-apple-darwin
```

## 使用方法 / Usage

### 编译到 Linux x86_64

```bash
make cross-linux
# 或 / or
cross build --release --target x86_64-unknown-linux-gnu
```

输出 / Output: `target/x86_64-unknown-linux-gnu/release/quickurl`

### 编译到 Linux ARM64

```bash
make cross-arm
# 或 / or
cross build --release --target aarch64-unknown-linux-gnu
```

输出 / Output: `target/aarch64-unknown-linux-gnu/release/quickurl`

### 编译到 Windows x86_64

```bash
make cross-windows
# 或 / or
cross build --release --target x86_64-pc-windows-gnu
```

输出 / Output: `target/x86_64-pc-windows-gnu/release/quickurl.exe`

### 编译到 macOS (Intel)

```bash
make cross-macos-intel
# 或 / or
cargo build --release --target x86_64-apple-darwin
```

输出 / Output: `target/x86_64-apple-darwin/release/quickurl`

### 编译到 macOS (Apple Silicon)

```bash
make cross-macos-arm
# 或 / or
cargo build --release --target aarch64-apple-darwin
```

输出 / Output: `target/aarch64-apple-darwin/release/quickurl`

### 编译 macOS Universal Binary (通用二进制)

```bash
make cross-macos
```

这将创建一个同时支持 Intel 和 Apple Silicon 的通用二进制文件。

This creates a universal binary that works on both Intel and Apple Silicon Macs.

输出 / Output: `target/universal-apple-darwin/release/quickurl`

### 编译所有平台

```bash
make cross-all
```

这将为所有支持的平台构建二进制文件。

This builds binaries for all supported platforms.

## Docker 支持 / Docker Support

`cross` 工具使用 Docker 容器进行交叉编译，因此需要：

The `cross` tool uses Docker containers for cross-compilation, so you need:

1. 安装 Docker / Install Docker
2. 确保 Docker daemon 正在运行 / Ensure Docker daemon is running

```bash
# 检查 Docker 是否运行 / Check if Docker is running
docker ps
```

## 注意事项 / Notes

### macOS 交叉编译

- macOS 目标平台可以直接使用 `cargo` 而不需要 `cross`
- 创建 Universal Binary 需要 `lipo` 工具（macOS 自带）
- 在 macOS 上编译其他 macOS 架构不需要 Docker

- macOS targets can use `cargo` directly without `cross`
- Creating Universal Binaries requires `lipo` tool (included with macOS)
- Cross-compiling between macOS architectures doesn't require Docker

### Linux/Windows 交叉编译

- 需要 Docker
- 使用 `cross` 工具自动处理工具链和依赖
- 首次编译会下载 Docker 镜像（可能较大）

- Requires Docker
- Uses `cross` tool to automatically handle toolchains and dependencies
- First build will download Docker images (may be large)

### 依赖问题

如果遇到依赖问题，可能需要在 `Cross.toml` 中配置：

If you encounter dependency issues, you may need to configure in `Cross.toml`:

```toml
[target.x86_64-unknown-linux-gnu]
image = "ghcr.io/cross-rs/x86_64-unknown-linux-gnu:latest"

[target.aarch64-unknown-linux-gnu]
image = "ghcr.io/cross-rs/aarch64-unknown-linux-gnu:latest"
```

## 测试交叉编译的二进制文件 / Testing Cross-Compiled Binaries

### Linux 二进制文件

```bash
# 在 Linux 机器上
./target/x86_64-unknown-linux-gnu/release/quickurl --version

# 或使用 Docker
docker run --rm -v $(pwd):/app -w /app ubuntu:latest \
  ./target/x86_64-unknown-linux-gnu/release/quickurl --version
```

### Windows 二进制文件

```bash
# 在 Windows 机器上
.\target\x86_64-pc-windows-gnu\release\quickurl.exe --version

# 或使用 Wine (Linux/macOS)
wine target/x86_64-pc-windows-gnu/release/quickurl.exe --version
```

### macOS 二进制文件

```bash
# 在 macOS 上
./target/x86_64-apple-darwin/release/quickurl --version
./target/aarch64-apple-darwin/release/quickurl --version
./target/universal-apple-darwin/release/quickurl --version

# 检查 Universal Binary 的架构
lipo -info target/universal-apple-darwin/release/quickurl
# 输出应该显示: Architectures in the fat file: ... are: x86_64 arm64
```

## 发布流程 / Release Workflow

构建所有平台的发布版本：

Build release versions for all platforms:

```bash
# 1. 清理之前的构建
make clean

# 2. 构建所有平台
make cross-all

# 3. 创建发布目录
mkdir -p releases

# 4. 复制并重命名二进制文件
cp target/x86_64-unknown-linux-gnu/release/quickurl releases/quickurl-linux-x86_64
cp target/aarch64-unknown-linux-gnu/release/quickurl releases/quickurl-linux-arm64
cp target/x86_64-pc-windows-gnu/release/quickurl.exe releases/quickurl-windows-x86_64.exe
cp target/universal-apple-darwin/release/quickurl releases/quickurl-macos-universal

# 5. 创建压缩包
cd releases
tar -czf quickurl-linux-x86_64.tar.gz quickurl-linux-x86_64
tar -czf quickurl-linux-arm64.tar.gz quickurl-linux-arm64
zip quickurl-windows-x86_64.zip quickurl-windows-x86_64.exe
tar -czf quickurl-macos-universal.tar.gz quickurl-macos-universal
```

## 性能优化 / Performance Optimization

所有交叉编译都使用 release 模式，包含以下优化：

All cross-compilations use release mode with the following optimizations:

```toml
[profile.release]
opt-level = 3          # 最高优化级别
lto = true             # 链接时优化
codegen-units = 1      # 单个代码生成单元（更好的优化）
```

## 故障排除 / Troubleshooting

### Docker 权限问题

```bash
# Linux: 添加用户到 docker 组
sudo usermod -aG docker $USER
# 需要重新登录
```

### cross 工具未找到

```bash
cargo install cross --force
```

### 目标平台未安装

```bash
rustup target add <target-triple>
```

### 编译错误

1. 确保 Docker 正在运行
2. 更新 cross 工具: `cargo install cross --force`
3. 清理并重新构建: `make clean && make cross-all`

## 参考资源 / References

- [Rust Platform Support](https://doc.rust-lang.org/nightly/rustc/platform-support.html)
- [cross GitHub](https://github.com/cross-rs/cross)
- [rustup Documentation](https://rust-lang.github.io/rustup/)
