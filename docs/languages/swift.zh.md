# Swift 语言指南

linthis 使用 **swiftlint** 进行代码检查，使用 **swift-format** 进行代码格式化。

## 支持的文件扩展名

- `.swift`

## 必需工具

### 代码检查：swiftlint

```bash
# macOS（推荐）
brew install swiftlint

# 或通过 Mint
mint install realm/SwiftLint

# 验证安装
swiftlint version
```

### 格式化：swift-format

```bash
# macOS
brew install swift-format

# 或从源码构建
git clone https://github.com/apple/swift-format.git
cd swift-format
swift build -c release

# 验证安装
swift-format --version
```

## 配置

### 基本示例

```toml
# .linthis/config.toml

[swift]
max_complexity = 15
excludes = ["Pods/**", "Carthage/**", ".build/**"]
```

### 禁用特定规则

```toml
[swift.rules]
disable = [
    "line_length",
    "trailing_whitespace"
]
```

## 自定义规则

```toml
[[rules.custom]]
code = "swift/no-print"
pattern = "\\bprint\\s*\\("
message = "Use os_log or Logger instead of print()"
severity = "warning"
languages = ["swift"]

[[rules.custom]]
code = "swift/force-unwrap"
pattern = "\\!(?![=])"
message = "Avoid force unwrapping"
severity = "warning"
suggestion = "Use optional binding or nil coalescing"
languages = ["swift"]
```

## CLI 用法

```bash
# 仅检查 Swift 文件
linthis -c --lang swift

# 仅格式化 Swift 文件
linthis -f --lang swift
```

## SwiftLint 配置

创建 `.swiftlint.yml`：

```yaml
disabled_rules:
  - trailing_whitespace
  - line_length

opt_in_rules:
  - empty_count
  - closure_spacing

line_length:
  warning: 120
  error: 150

excluded:
  - Pods
  - Carthage
  - .build
```

## Swift-Format 配置

创建 `.swift-format`：

```json
{
  "version": 1,
  "lineLength": 120,
  "indentation": {
    "spaces": 4
  },
  "lineBreakBeforeControlFlowKeywords": false,
  "lineBreakBeforeEachArgument": true
}
```

## 常见问题

### SwiftLint 未找到

```
Warning: No swift linter available for swift files
  Install: brew install swiftlint
```

**注意**：SwiftLint 主要只支持 macOS。

### SPM 包被检查

添加到排除项：

```toml
[swift]
excludes = [".build/**", "Package.resolved"]
```

### Xcode 管理的文件

将生成的文件添加到排除项：

```toml
[swift]
excludes = ["*.generated.swift", "R.generated.swift"]
```

## 最佳实践

1. **使用 .swiftlint.yml**：按项目配置规则
2. **强制解包检测**：对 `!` 使用发出警告
3. **SPM 排除**：排除 `.build` 目录
4. **行长度**：120 字符对 Swift 项目较为常见
