# Objective-C 语言指南

linthis 使用 **cpplint** 和 **clang-tidy** 进行代码检查，使用 **clang-format** 进行代码格式化。

## 支持的文件扩展名

- `.m`
- `.mm`
- `.h`（根据内容自动检测）

## 必需工具

与 C++ 相同 - 请参阅 [C++ 语言指南](cpp.md)。

### 代码检查：cpplint

```bash
pip install cpplint
```

### 格式化：clang-format

```bash
# macOS
brew install clang-format

# 或安装 LLVM
brew install llvm
```

## 配置

### 基本示例

```toml
# .linthis/config.toml

[oc]
linelength = 120
excludes = ["Pods/**", "Carthage/**"]
cpplint_filter = "-build/header_guard,-runtime/int"
```

可替代的键名：

```toml
[objectivec]
linelength = 120
```

### Clang-tidy 忽略检查

```toml
[oc]
clang_tidy_ignored_checks = [
    "clang-analyzer-osx.cocoa.RetainCount",
    "clang-analyzer-osx.cocoa.SelfInit"
]
```

## 自定义规则

```toml
[[rules.custom]]
code = "oc/no-nslog"
pattern = "NSLog\\s*\\("
message = "Remove NSLog before release"
severity = "warning"
languages = ["oc"]

[[rules.custom]]
code = "oc/arc-bridge"
pattern = "__bridge"
message = "Review ARC bridge usage"
severity = "info"
languages = ["oc"]
```

## CLI 用法

```bash
# 仅检查 Objective-C 文件
linthis -c --lang oc

# 仅格式化 Objective-C 文件
linthis -f --lang oc
```

## Clang-Format 配置

创建 `.clang-format`：

```yaml
BasedOnStyle: Google
Language: ObjC
ObjCSpaceAfterProperty: true
ObjCSpaceBeforeProtocolList: true
ColumnLimit: 120
```

## 头文件检测

linthis 在 `.h` 文件包含以下内容时自动检测为 Objective-C：
- `#import` 或 `@import` 语句
- `@interface`、`@implementation`、`@protocol`
- NS 类型（NSString、NSArray 等）
- Objective-C 方法语法（`+ (`、`- (`）

## 常见问题

### .h 文件被检测为 C++

如果 `.h` 文件被错误检测，确保它们包含 Objective-C 模式或显式指定：

```bash
linthis -c --lang oc src/
```

### Pods/Carthage 被检查

添加到排除项：

```toml
[oc]
excludes = ["Pods/**", "Carthage/**", "Vendor/**"]
```

## 最佳实践

1. **使用 .clang-format**：ObjC 特定风格的一致格式化
2. **排除依赖**：将 Pods/Carthage 添加到排除项
3. **ARC 考虑**：审查 `__bridge` 使用警告
4. **NSLog 清理**：发布前移除调试日志
