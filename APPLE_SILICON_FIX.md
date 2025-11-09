# Apple Silicon 交叉编译修复指南

## 问题

在 Apple Silicon (M1/M2/M3) Mac 上使用 `cross` 工具进行交叉编译时，会遇到以下错误：

```
docker: no matching manifest for linux/arm64/v8 in the manifest list entries
```

这是因为 `cross` 工具的 Docker 镜像不支持 ARM64 主机架构。

## 解决方案

我们提供了两种解决方案：

### 方案 1: 使用 cargo-zigbuild (推荐)

`cargo-zigbuild` 使用 Zig 编译器进行交叉编译，不依赖 Docker，在 Apple Silicon 上工作完美。

#### 安装步骤

```bash
# 1. 重新安装交叉编译工具（会自动检测 Apple Silicon 并安装 zigbuild）
make install-cross
```

这将自动：
- 通过 Homebrew 安装 `zig` 编译器
- 安装 `cargo-zigbuild` 工具
- 添加所有必要的 Rust target

#### 使用方法

安装完成后，Makefile 会自动使用 `cargo-zigbuild` 而不是 `cross`：

```bash
# 编译 Linux x86_64
make cross-linux

# 编译 Linux ARM64
make cross-arm

# 编译 Windows x86_64
make cross-windows

# 编译所有平台
make cross-all
```

### 方案 2: 使用 Cross.toml 配置

如果你仍然想使用 `cross`，可以使用项目中的 `Cross.toml` 配置文件，它指定了支持 ARM64 的 Docker 镜像：

```bash
# 使用 edge 版本的镜像
cross build --release --target x86_64-unknown-linux-gnu
```

但是这种方法可能仍然会遇到问题，因为 Docker 镜像的支持有限。

## 验证安装

```bash
# 检查 zig 是否安装
zig version

# 检查 cargo-zigbuild 是否安装
cargo zigbuild --version

# 检查 Rust targets
rustup target list --installed
```

## 测试交叉编译

```bash
# 测试编译 Linux x86_64
make cross-linux

# 如果成功，你会看到：
# Building for Linux x86_64...
# Using cargo-zigbuild (Apple Silicon detected)...
# Binary: target/x86_64-unknown-linux-gnu/release/hurl
```

## 常见问题

### Q: 为什么不能使用 cross？

A: `cross` 工具依赖 Docker 镜像，而大多数镜像只为 x86_64 主机构建。虽然有一些 `edge` 版本的镜像支持 ARM64，但支持不完整且不稳定。

### Q: cargo-zigbuild 和 cross 有什么区别？

A: 
- `cross` 使用 Docker 容器提供完整的交叉编译环境
- `cargo-zigbuild` 使用 Zig 编译器作为链接器，不需要 Docker
- 在 Apple Silicon 上，`cargo-zigbuild` 更可靠且速度更快

### Q: 如果 Homebrew 安装 zig 失败怎么办？

A: 可以手动下载 zig：

```bash
# 从官网下载
# https://ziglang.org/download/

# 或使用其他包管理器
# macports:
sudo port install zig

# 或直接下载二进制
curl -L https://ziglang.org/download/0.11.0/zig-macos-aarch64-0.11.0.tar.xz | tar xJ
sudo mv zig-macos-aarch64-0.11.0 /usr/local/zig
export PATH="/usr/local/zig:$PATH"
```

### Q: 编译后的二进制文件能在目标平台上运行吗？

A: 是的！`cargo-zigbuild` 生成的二进制文件与原生编译的二进制文件完全兼容。

## 手动安装步骤

如果 `make install-cross` 失败，可以手动执行：

```bash
# 1. 安装 zig
brew install zig

# 2. 安装 cargo-zigbuild
cargo install cargo-zigbuild

# 3. 添加 Rust targets
rustup target add x86_64-unknown-linux-gnu
rustup target add aarch64-unknown-linux-gnu
rustup target add x86_64-pc-windows-gnu

# 4. 测试编译
cargo zigbuild --release --target x86_64-unknown-linux-gnu
```

## 性能对比

在 Apple Silicon 上：

| 工具 | 编译时间 | 依赖 | 稳定性 |
|------|---------|------|--------|
| cross | ❌ 不可用 | Docker | ❌ 镜像不兼容 |
| cargo-zigbuild | ✅ 快速 | Zig | ✅ 稳定 |
| 原生编译 | ✅ 最快 | 无 | ✅ 仅限 macOS |

## 更多信息

- [cargo-zigbuild GitHub](https://github.com/rust-cross/cargo-zigbuild)
- [Zig 官网](https://ziglang.org/)
- [Cross GitHub](https://github.com/cross-rs/cross)
