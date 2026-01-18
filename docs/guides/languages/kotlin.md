# Kotlin Language Guide

linthis uses **ktlint** for both linting and formatting, with optional **detekt** for additional static analysis.

## Supported File Extensions

- `.kt`
- `.kts`

## Required Tools

### Linter & Formatter: ktlint

```bash
# macOS
brew install ktlint

# Or via curl
curl -sSLO https://github.com/pinterest/ktlint/releases/download/1.0.1/ktlint
chmod a+x ktlint

# Verify installation
ktlint --version
```

### Optional: detekt (additional static analysis)

```bash
# macOS
brew install detekt

# Verify installation
detekt --version
```

## Configuration

### Basic Example

```toml
# .linthis/config.toml

[kotlin]
max_complexity = 15
excludes = ["build/**", "*.generated.kt"]
```

### Disable Specific Rules

```toml
[kotlin.rules]
disable = [
    "max-line-length",
    "no-wildcard-imports"
]
```

## Custom Rules

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

## CLI Usage

```bash
# Check Kotlin files only
linthis -c --lang kotlin

# Format Kotlin files only
linthis -f --lang kotlin
```

## Ktlint Configuration

Create `.editorconfig` for ktlint:

```ini
[*.{kt,kts}]
indent_size = 4
max_line_length = 120
ktlint_code_style = ktlint_official
```

Or use ktlint-specific config file `.ktlint`:

```
disabled_rules=no-wildcard-imports
```

## Detekt Configuration

Create `detekt.yml`:

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

## Common Issues

### Ktlint not found

```
Warning: No kotlin linter available for kotlin files
  Install: brew install ktlint
```

### Build outputs being checked

Add to excludes:

```toml
[kotlin]
excludes = ["build/**", "out/**", ".gradle/**"]
```

### Generated code

Add generated files to excludes:

```toml
[kotlin]
excludes = ["*.generated.kt", "generated/**"]
```

## Best Practices

1. **Use .editorconfig**: Standard way to configure ktlint
2. **Android projects**: Exclude generated code and build outputs
3. **Coroutines**: Be mindful of complexity in suspend functions
4. **KDoc comments**: Use KDoc for public API documentation
