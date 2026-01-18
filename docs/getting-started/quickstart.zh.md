# 快速开始

本指南将帮助您在几分钟内开始使用 linthis。

## 初始化配置（可选）

```bash
# 创建项目配置文件
linthis init

# 创建全局配置文件
linthis init -g

# 安装 pre-commit hooks
linthis hook install --type git
```

## 基本用法

### 检查和格式化

```bash
# 检查和格式化当前目录（默认行为）
linthis

# 检查和格式化指定目录
linthis -i src/
linthis --include src/ --include lib/
```

### 仅检查

```bash
# 仅检查，不格式化
linthis -c
linthis --check-only
```

### 仅格式化

```bash
# 仅格式化，不检查
linthis -f
linthis --format-only
```

### Git 暂存文件

```bash
# 检查 Git 暂存文件（适用于 pre-commit hook）
linthis -s
linthis --staged
```

## 指定语言

```bash
# 检查特定语言
linthis -l python
linthis --lang rust

# 检查多种语言
linthis -l python,rust,cpp
linthis --lang "python,javascript,go"
```

## 排除文件

```bash
# 排除特定模式
linthis -e "*.test.js" -e "dist/**"
linthis --exclude "target/**" --exclude "node_modules/**"
```

## 输出格式

```bash
# 人类可读输出（默认）
linthis

# JSON 输出
linthis -o json

# GitHub Actions 格式
linthis -o github-actions
```

## 常见工作流

### Pre-commit Hook

```bash
# 安装 git hook
linthis hook install --type git

# 或与 pre-commit 框架一起使用
linthis hook install --type pre-commit
```

### CI/CD

```bash
# 在 CI 中，使用仅检查模式，错误时非零退出
linthis --check-only --output github-actions
```

## 下一步

- [配置](configuration.md) - 了解配置选项
- [插件系统](../features/plugins.md) - 跨项目共享配置
- [CLI 参考](../reference/cli.md) - 完整命令参考
