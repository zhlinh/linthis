# 安装

根据您的环境，有多种方式安装 linthis。

## 方式 1：通过 PyPI 安装（推荐 Python 用户）

```bash
# 使用 pip
pip install linthis

# 使用 uv（推荐）
# pip install uv
uv pip install linthis
```

## 方式 2：通过 Cargo 安装（推荐 Rust 用户）

```bash
cargo install linthis
```

## 方式 3：从源码构建

```bash
git clone https://github.com/zhlinh/linthis.git
cd linthis
cargo build --release
```

二进制文件将位于 `target/release/linthis`。

## 验证安装

安装后，验证 linthis 是否正常工作：

```bash
linthis --version
```

## 系统要求

- **操作系统**：macOS、Linux、Windows
- **架构**：x86_64、arm64

## 语言特定工具

linthis 封装了现有的语言特定工具。对于您想要检查/格式化的每种语言，您需要安装底层工具：

| 语言 | 所需工具 |
|-----|---------|
| Rust | `rustfmt`、`clippy` |
| Python | `ruff` 或 `black`、`flake8`、`pylint` |
| JavaScript/TypeScript | `eslint`、`prettier` |
| Go | `gofmt`、`golangci-lint` |
| Java | `checkstyle`、`google-java-format` |
| C++ | `clang-format`、`cpplint` |

请参阅[语言支持](../languages/index.md)了解每种语言的详细设置说明。

## 下一步

- [快速开始](quickstart.md) - 学习基础知识
- [配置](configuration.md) - 为您的项目配置 linthis
