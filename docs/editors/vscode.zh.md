# VS Code 扩展

Linthis 官方 VS Code 扩展，通过 Language Server Protocol 提供实时代码检查、格式化和诊断功能。

## 安装

### 从 VS Code 市场安装

1. 打开 VS Code
2. 进入扩展面板（`Cmd/Ctrl + Shift + X`）
3. 搜索 "Linthis"
4. 点击"安装"

### 从源码安装

```bash
cd vscode-linthis
npm install
npm run build
# 按 F5 启动扩展开发主机
```

## 功能特性

### 🔍 实时代码检查（LSP）

- Language Server Protocol (LSP) 集成
- 实时诊断，即时反馈
- 保存时自动检查（可配置）
- 支持多种语言（Python、TypeScript、Rust、Go、Java、C/C++、Swift、Kotlin 等）

### 🎨 保存时格式化

- 保存文件时自动格式化
- 支持工作区或全局配置
- 静默操作，日志记录
- 启用时交互式提示

### 🛠️ 命令

- **Linthis: Lint Document** - 手动检查当前文件
- **Linthis: Format Document** - 手动格式化当前文件
- **Linthis: Restart Language Server** - 重启 LSP 服务器

## 配置

### 设置项

所有设置以 `linthis.` 为前缀：

#### 基础设置

```json
{
  "linthis.enable": true,
  "linthis.executablePath": "linthis"
}
```

- `linthis.enable`（布尔值，默认：`true`）- 启用/禁用扩展
- `linthis.executablePath`（字符串，默认：`"linthis"`）- linthis 可执行文件路径

#### 自动功能

```json
{
  "linthis.lintOnSave": true,
  "linthis.formatOnSave": false
}
```

- `linthis.lintOnSave`（布尔值，默认：`true`）- 保存时检查文档
- `linthis.formatOnSave`（布尔值，默认：`false`）- 保存时格式化文档

#### 高级设置

```json
{
  "linthis.extraArgs": [],
  "linthis.trace.server": "off"
}
```

- `linthis.extraArgs`（数组，默认：`[]`）- LSP 服务器额外参数
- `linthis.trace.server`（字符串，默认：`"off"`）- LSP 服务器跟踪级别

### 配置示例

#### 最小配置

```json
{
  "linthis.enable": true
}
```

#### 全自动模式

```json
{
  "linthis.enable": true,
  "linthis.lintOnSave": true,
  "linthis.formatOnSave": true
}
```

#### 自定义可执行文件路径

```json
{
  "linthis.executablePath": "/usr/local/bin/linthis"
}
```

## 工作原理

### LSP 集成

扩展通过 Language Server Protocol 与 linthis CLI 通信：

1. **启动**：扩展启动 `linthis lsp` 子进程
2. **文档事件**：VS Code 发送文档变化到 LSP 服务器
3. **诊断**：LSP 服务器运行检查工具并返回诊断
4. **显示**：诊断显示在 Problems 面板，带 `[linthis-ruff]` 前缀

### 保存时格式化

启用 `formatOnSave` 后：

1. 用户保存文件（`Cmd/Ctrl + S`）
2. VS Code 将文件保存到磁盘
3. 扩展触发 `linthis -f -i <file>`
4. 文件原地格式化
5. VS Code 检测变化并重新加载文件

### 保存时检查 vs 保存时格式化

| 功能 | 保存时检查 | 保存时格式化 |
|------|-----------|-------------|
| 触发时机 | 保存事件 | 保存之后 |
| 实现方式 | LSP 诊断 | CLI 格式化 |
| 结果 | Problems 面板 | 文件修改 |
| 默认值 | 启用 | 禁用 |

## 使用方法

### 日常工作流

1. **打开文件** - LSP 服务器自动启动
2. **编辑代码** - Problems 面板实时显示诊断
3. **保存文件**（`Cmd/Ctrl + S`）- 自动检查和格式化（如启用）
4. **查看问题**（`Cmd/Ctrl + Shift + M`）- 查看所有问题

### 手动命令

按 `Cmd/Ctrl + Shift + P` 打开命令面板：

- 输入 "Linthis: Lint Document" - 手动检查当前文件
- 输入 "Linthis: Format Document" - 手动格式化当前文件
- 输入 "Linthis: Restart Language Server" - 重启 LSP（如有问题）

### 设置保存时格式化

启用 `formatOnSave` 时会看到提示：

```
Linthis: Format on save enabled. Format all open files now?
[Format All]  [Skip]
```

- **Format All** - 立即格式化所有打开的代码文件
- **Skip** - 等待下次保存时再格式化

## 诊断来源

Problems 面板中的诊断带有来源前缀：

- `[linthis-ruff]` - Python 检查（ruff）
- `[linthis-eslint]` - JavaScript/TypeScript 检查（eslint）
- `[linthis-clippy]` - Rust 检查（clippy）
- `[linthis]` - 通用 linthis 诊断

这样可以清楚地看到每个问题来自哪个工具。

## 支持的语言

扩展支持与 linthis CLI 相同的语言：

- Python
- TypeScript / JavaScript（包括 React）
- Rust
- Go
- Java
- C / C++ / Objective-C
- Swift
- Kotlin
- Lua
- Dart
- Shell Script
- Ruby
- PHP
- Scala
- C#

## 故障排除

### 扩展无法工作

**检查 Output 面板**：
1. View → Output
2. 从下拉菜单选择 "Linthis"
3. 查找错误消息

**常见问题**：

- `Extension is disabled` - 检查 `linthis.enable` 设置
- `Language server not running` - 检查 `linthis` 是否在 PATH 中
- `Command not found` - 安装 linthis CLI：`cargo install --path .`

### 保存时没有诊断

**检查**：
1. `linthis.lintOnSave` 为 `true`
2. 文件类型受支持
3. LSP 服务器正在运行（检查 Output 面板）
4. 查看 **Problems 面板**，而不是 Output 面板

### 保存时格式化不工作

**检查**：
1. `linthis.formatOnSave` 为 `true`
2. 文件类型受支持
3. 文件没有语法错误
4. 检查 Output 面板的错误日志

### LSP 服务器超时

如果看到 "Language server start timeout"：

1. 检查 `linthis lsp` 是否能成功运行：
   ```bash
   linthis lsp
   ```
2. 重启扩展："Linthis: Restart Language Server"
3. 检查 `linthis.trace.server` 获取详细日志

## 输出日志

扩展记录日志到 Output 面板（"Linthis" 频道）：

### 启动日志
```
[info] Using linthis executable: linthis
[info] LSP arguments: lsp
[info] Starting language server...
[info] Language server started successfully
[info] Lint on save is enabled
[info] Format on save is enabled
```

### 格式化日志
```
[info] Format on save: /path/to/file.py
[info] Format completed
```

### 错误日志
```
[error] Format on save failed with exit code: 1
[error] stdout: formatting errors found
```

## 性能

### 小文件
- 检查：< 100ms
- 格式化：< 200ms
- 几乎即时反馈

### 大文件
- 检查：< 1s
- 格式化：1-3s
- 考虑对超大文件禁用自动格式化

### 优化建议

1. 使用 `.linthisignore` 排除大文件
2. 对大项目禁用 `formatOnSave`
3. 需要时使用手动命令
4. 必要时配置 LSP 超时

## 最佳实践

### 推荐设置

对于大多数项目：
```json
{
  "linthis.enable": true,
  "linthis.lintOnSave": true,
  "linthis.formatOnSave": true
}
```

### 按语言设置

禁用其他格式化器以避免冲突：

```json
{
  "[python]": {
    "editor.formatOnSave": false,
    "editor.defaultFormatter": "linthis.linthis"
  },
  "[typescript]": {
    "editor.formatOnSave": false,
    "editor.defaultFormatter": "linthis.linthis"
  }
}
```

### 工作区设置

在项目中创建 `.vscode/settings.json`：

```json
{
  "linthis.enable": true,
  "linthis.lintOnSave": true,
  "linthis.formatOnSave": true,
  "files.associations": {
    ".linthis.toml": "toml"
  }
}
```

## 开发

### 从源码构建

```bash
git clone https://github.com/lint-group/linthis
cd linthis/vscode-linthis
npm install
npm run build
```

### 测试

在 VS Code 中按 `F5` 启动扩展开发主机。

### 调试

1. 在 TypeScript 文件中设置断点
2. 按 `F5` 开始调试
3. 使用调试控制台查看变量

## 相关文档

- [Linthis CLI 文档](../reference/cli.zh.md)
- [配置参考](../reference/configuration.zh.md)
- [语言支持](../languages/index.zh.md)

## 反馈

报告问题：https://github.com/lint-group/linthis/issues
