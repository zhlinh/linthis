# 更新日志

本项目的所有重要更改都将记录在此文件中。

格式基于[Keep a Changelog](https://keepachangelog.com/en/1.0.0/)，
本项目遵循[语义化版本](https://semver.org/spec/v2.0.0.html)。

## [未发布]

### 新增

- Shell/Bash 语言支持（ShellCheck + shfmt）
- Ruby 语言支持（RuboCop）
- PHP 语言支持（phpcs + php-cs-fixer）
- Scala 语言支持（scalafix + scalafmt）
- C# 语言支持（dotnet format）
- 监视模式，持续文件监控
- 使用 Material 主题的 MkDocs 文档

### 更改

- 改进了缺少工具时的错误消息

### 修复

- 修复了 Windows 上的文件模式匹配

## [0.0.10] - 2024-XX-XX

### 新增

- Lua 语言支持（luacheck + stylua）
- Dart 语言支持（dart analyze + dart format）
- 插件自动同步功能
- 自更新功能
- 从 ESLint、Prettier、Black 迁移配置

### 更改

- 改进了并行处理性能
- 增强了插件缓存机制

### 修复

- 修复了 gitignore 模式处理
- 修复了暂存文件检测

## [0.0.1] - 2024-XX-XX

### 新增

- 初始发布
- 支持 Rust、Python、TypeScript、JavaScript、Go、Java、C++、Swift、Kotlin、Objective-C
- 插件系统
- Git hooks 集成
- 格式化预设（Google、Airbnb、Standard）
- 配置管理 CLI
- JSON 和 GitHub Actions 输出格式
