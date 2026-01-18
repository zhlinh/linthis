# linthis

[![Crates.io](https://img.shields.io/crates/v/linthis.svg)](https://crates.io/crates/linthis)
[![PyPI](https://img.shields.io/pypi/v/linthis.svg)](https://pypi.org/project/linthis/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

一个用 Rust 编写的快速、跨平台多语言代码检查器和格式化器。

## 功能特性

- **单一命令**：同时运行代码检查和格式化
- **多语言支持**：Rust、Python、TypeScript、JavaScript、Go、Java、C++、Swift、Kotlin、Lua、Dart、Shell、Ruby、PHP、Scala、C# 等
- **自动检测**：自动检测项目中使用的编程语言
- **灵活配置**：支持项目配置、全局配置和 CLI 参数
- **插件系统**：通过 Git 仓库共享和复用配置
- **格式化预设**：支持 Google、Airbnb、Standard 等流行代码风格
- **并行处理**：利用多核 CPU 加速文件处理

## 快速链接

- [安装](getting-started/installation.md) - 安装 linthis
- [快速开始](getting-started/quickstart.md) - 几分钟内开始使用 linthis
- [配置](getting-started/configuration.md) - 为您的项目配置 linthis
- [语言支持](languages/index.md) - 支持的编程语言
- [CLI 参考](reference/cli.md) - 完整的命令行参考

## 使用示例

```bash
# 检查和格式化当前目录
linthis

# 仅检查（不格式化）
linthis --check-only

# 仅格式化（不检查）
linthis --format-only

# 检查 Git 暂存文件
linthis --staged
```

## 为什么选择 linthis？

1. **统一界面**：一个工具管理所有语言，无需管理多个 linter
2. **快速**：使用 Rust 编写，支持并行处理
3. **易于设置**：开箱即用，具有合理的默认值
4. **团队友好**：插件系统用于跨项目共享配置
