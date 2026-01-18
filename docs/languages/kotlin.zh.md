# Kotlin 语言指南

linthis 使用 **ktlint** 进行代码检查和格式化，可选使用 **detekt** 进行额外的静态分析。

## 支持的文件扩展名

- `.kt`
- `.kts`

## 必需工具

### 代码检查和格式化：ktlint

```bash
# macOS
brew install ktlint

# 或通过 curl
curl -sSLO https://github.com/pinterest/ktlint/releases/download/1.0.1/ktlint
chmod a+x ktlint

# 验证安装
ktlint --version
```

### 可选：detekt（额外静态分析）

```bash
# macOS
brew install detekt

# 验证安装
detekt --version
```

## 配置

### 基本示例

```toml
# .linthis/config.toml

[kotlin]
max_complexity = 15
excludes = ["build/**", "*.generated.kt"]
```

### 禁用特定规则

```toml
[kotlin.rules]
disable = [
    "max-line-length",
    "no-wildcard-imports"
]
```

## 自定义规则

```toml
[[rules.custom]]
code = "kotlin/no-println"
pattern = "println\\s*\\("
message = "Use logging framework instead of println()"
severity = "warning"
languages = ["kotlin"]

[[rules.custom]]
code = "kotlin/no-lateinit"
pattern = "lateinit\\s+var"
message = "Consider using lazy or nullable instead of lateinit"
severity = "info"
languages = ["kotlin"]
```

## CLI 用法

```bash
# 仅检查 Kotlin 文件
linthis -c --lang kotlin

# 仅格式化 Kotlin 文件
linthis -f --lang kotlin
```

## Ktlint 配置

为 ktlint 创建 `.editorconfig`：

```ini
[*.{kt,kts}]
indent_size = 4
max_line_length = 120
ktlint_code_style = ktlint_official
```

或使用 ktlint 特定配置文件 `.ktlint`：

```
disabled_rules=no-wildcard-imports
```

## Detekt 配置

创建 `detekt.yml`：

```yaml
complexity:
  LongMethod:
    threshold: 60
  ComplexMethod:
    threshold: 15

style:
  MaxLineLength:
    maxLineLength: 120
```

## 常见问题

### Ktlint 未找到

```
Warning: No kotlin linter available for kotlin files
  Install: brew install ktlint
```

### 构建输出被检查

添加到排除项：

```toml
[kotlin]
excludes = ["build/**", "out/**", ".gradle/**"]
```

### 生成的代码

将生成的文件添加到排除项：

```toml
[kotlin]
excludes = ["*.generated.kt", "generated/**"]
```

## 最佳实践

1. **使用 .editorconfig**：配置 ktlint 的标准方式
2. **Android 项目**：排除生成的代码和构建输出
3. **协程**：注意 suspend 函数的复杂度
4. **KDoc 注释**：为公共 API 使用 KDoc 文档
