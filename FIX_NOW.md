# 🔧 立即修复 Apple Silicon 交叉编译问题

## 快速修复（3 步）

你遇到的错误是因为在 Apple Silicon Mac 上 `cross` 工具不兼容。

### 步骤 1: 安装 Zig

```bash
brew install zig
```

### 步骤 2: 安装 cargo-zigbuild

```bash
cargo install cargo-zigbuild
```

### 步骤 3: 现在可以交叉编译了！

```bash
# 编译 Linux x86_64
make cross-linux

# 编译所有平台
make cross-all
```

## 工作原理

Makefile 已经更新，会自动检测你的 Apple Silicon Mac 并使用 `cargo-zigbuild` 而不是 `cross`。

## 验证安装

```bash
# 检查 zig
zig version

# 检查 cargo-zigbuild  
cargo zigbuild --version
```

## 如果遇到问题

查看详细文档：
- [APPLE_SILICON_FIX.md](APPLE_SILICON_FIX.md) - 完整的故障排除指南
- [CROSS_COMPILE.md](CROSS_COMPILE.md) - 交叉编译完整文档

---

**提示**: 安装完成后，所有 `make cross-*` 命令都会自动使用 zigbuild！
