# linthis

[![Crates.io](https://img.shields.io/crates/v/linthis.svg)](https://crates.io/crates/linthis)
[![PyPI](https://img.shields.io/pypi/v/linthis.svg)](https://pypi.org/project/linthis/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

一个用 Rust 编写的快速、跨平台多语言代码检查器和格式化器。

## 功能特性

- **单一命令**：同时运行代码检查、格式化、安全扫描和复杂度分析
- **多语言支持**：Rust、Python、TypeScript、JavaScript、Go、Java、C++、Swift、Kotlin、Lua、Dart、Shell、Ruby、PHP、Scala、C# 等
- **自动检测**：自动检测项目中使用的编程语言
- **安全扫描（SAST）**：内置 secrets 检测 + OpenGrep/Semgrep、Bandit、Gosec、Flawfinder 集成
- **复杂度分析**：函数级圈复杂度/认知复杂度分析，支持阈值拦截
- **灵活配置**：支持项目配置、全局配置和 CLI 参数
- **插件系统**：通过 Git 仓库共享和复用配置
- **格式化预设**：支持 Google、Airbnb、Standard 等流行代码风格
- **并行处理**：利用多核 CPU 加速文件处理，per-file 缓存

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
linthis -c
linthis --check-only

# 仅格式化（不检查）
linthis -f
linthis --format-only

# 检查 Git 暂存文件
linthis -s
linthis --staged
```

## 视频教程

观看 linthis 功能演示短视频：

1. [快速开始](getting-started/videos.md#1) — 安装并运行首次检查（20秒）
2. [多语言支持](getting-started/videos.md#2) — 自动检测 18+ 种语言（15秒）
3. [插件系统](getting-started/videos.md#3) — 团队间共享配置（20秒）
4. [AI 修复](getting-started/videos.md#4-ai) — AI 驱动的代码修复（20秒）
5. [Git Hooks](getting-started/videos.md#5-git-hooks) — 自动 pre-commit 检查（15秒）
6. [编辑器集成](getting-started/videos.md#6) — VS Code、JetBrains、Neovim、Claude Code（15秒）
7. [AI Agent Hook](getting-started/videos.md#7-ai-agent-hook) — AI 编程助手集成（20秒）

## 为什么选择 linthis？

1. **统一界面**：一个工具管理所有语言，无需管理多个 linter
2. **快速**：使用 Rust 编写，支持并行处理
3. **易于设置**：开箱即用，具有合理的默认值
4. **团队友好**：插件系统用于跨项目共享配置
