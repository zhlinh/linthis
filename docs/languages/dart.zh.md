# Dart 语言指南

linthis 使用 **dart analyze** 进行代码检查，使用 **dart format** 进行代码格式化。这两个工具都包含在 Dart SDK 中。

## 支持的文件扩展名

- `.dart`

## 必需工具

### Dart SDK

代码检查和格式化工具都包含在 Dart SDK 中。

```bash
# macOS
brew install dart

# Linux (Ubuntu/Debian)
sudo apt install dart

# Windows
choco install dart-sdk

# 或安装 Flutter（包含 Dart）
# https://flutter.dev/docs/get-started/install

# 验证安装
dart --version
```

## 配置

### 基本示例

```toml
# .linthis/config.toml

[dart]
max_complexity = 15
excludes = [".dart_tool/**", "build/**", "*.g.dart"]
```

### 禁用特定规则

```toml
[dart.rules]
disable = [
    "prefer_single_quotes",
    "lines_longer_than_80_chars"
]
```

## 自定义规则

```toml
[[rules.custom]]
code = "dart/no-print"
pattern = "\\bprint\\s*\\("
message = "Use logging instead of print()"
severity = "warning"
languages = ["dart"]

[[rules.custom]]
code = "dart/no-debug-print"
pattern = "debugPrint\\s*\\("
message = "Remove debugPrint before release"
severity = "warning"
languages = ["dart"]
```

## CLI 用法

```bash
# 仅检查 Dart 文件
linthis -c --lang dart

# 仅格式化 Dart 文件
linthis -f --lang dart
```

## 分析选项

创建 `analysis_options.yaml`：

```yaml
include: package:lints/recommended.yaml
# 或对于 Flutter：
# include: package:flutter_lints/flutter.yaml

analyzer:
  exclude:
    - "**/*.g.dart"
    - "**/*.freezed.dart"
  errors:
    invalid_annotation_target: ignore

linter:
  rules:
    - prefer_const_constructors
    - prefer_const_declarations
    - prefer_final_fields
    - avoid_print
    - prefer_single_quotes
```

## 常见问题

### Dart 未找到

```
Warning: No dart linter available for dart files
  Install: https://dart.dev/get-dart
```

### 生成的代码被检查

将生成的文件添加到排除项：

```toml
[dart]
excludes = [
    "*.g.dart",
    "*.freezed.dart",
    "*.mocks.dart",
    ".dart_tool/**"
]
```

### Flutter 特定检查

对于 Flutter 项目，使用 `package:flutter_lints`：

```yaml
# analysis_options.yaml
include: package:flutter_lints/flutter.yaml
```

### 构建输出

将构建目录添加到排除项：

```toml
[dart]
excludes = ["build/**", ".dart_tool/**"]
```

## 最佳实践

1. **使用 analysis_options.yaml**：标准 Dart/Flutter 配置
2. **排除生成的代码**：将 `*.g.dart`、`*.freezed.dart` 添加到排除项
3. **使用推荐的检查**：从 `package:lints/recommended.yaml` 开始
4. **空安全**：确保所有代码都是空安全的
5. **Flutter 检查**：Flutter 项目使用 `package:flutter_lints`
