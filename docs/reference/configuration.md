# Configuration Reference

Complete reference for all linthis configuration options.

## Configuration Hierarchy

Configuration is loaded from multiple sources with the following precedence (highest to lowest):

1. **CLI arguments** - Command-line flags override all config files
2. **Project config** - `.linthis/config.toml` in project root
3. **User config** - `~/.linthis/config.toml` for global settings
4. **Built-in defaults** - Sensible defaults for all options

## Configuration File Formats

Supported formats:
- TOML (`.toml`) - Recommended
- YAML (`.yml`, `.yaml`)
- JSON (`.json`)

---

## Top-Level Options

### `languages`

Languages to check. If empty, auto-detects based on file extensions.

```toml
languages = ["rust", "python", "typescript"]
```

**Type:** Array of strings
**Default:** `[]` (auto-detect)
**Valid values:** `rust`, `python`, `typescript`, `javascript`, `go`, `java`, `cpp`, `oc` (Objective-C), `swift`, `kotlin`, `lua`, `dart`

---

### `includes`

File patterns to include (glob patterns). If empty, checks all files.

```toml
includes = ["src/**", "lib/**"]
```

**Type:** Array of strings (glob patterns)
**Default:** `[]` (all files)

---

### `excludes`

File patterns to exclude (glob patterns). Added to built-in excludes.

```toml
excludes = ["*.generated.rs", "vendor/**"]
```

**Type:** Array of strings (glob patterns)
**Default:** `[]`
**Alias:** `exclude`

Built-in excludes (always applied):
- `**/node_modules/**`
- `**/target/**`
- `**/.git/**`
- `**/vendor/**`
- `**/__pycache__/**`

---

### `max_complexity`

Maximum cyclomatic complexity allowed per function.

```toml
max_complexity = 20
```

**Type:** Integer
**Default:** `20`

---

### `preset`

Code style preset to use.

```toml
preset = "google"
```

**Type:** String
**Default:** `null` (none)
**Valid values:** `google`, `standard`, `airbnb`

---

### `verbose`

Enable verbose output with additional details.

```toml
verbose = true
```

**Type:** Boolean
**Default:** `false`

---

## Plugin Configuration

### `[plugins]`

Configure external plugins for additional linters or rules.

```toml
[plugins]
sources = [
    { name = "official" },
    { name = "custom-rules", url = "https://github.com/user/lts-plugin.git", ref = "main" }
]
```

### Plugin Source Fields

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `name` | String | Yes | - | Plugin name (for registry lookup) |
| `url` | String | No | - | Git repository URL |
| `ref` | String | No | `main` | Git ref (tag, branch, commit) |
| `enabled` | Boolean | No | `true` | Enable/disable this plugin |

---

## Rules Configuration

### `[rules]`

Configure rule disabling, severity overrides, and custom rules.

```toml
[rules]
disable = ["E501", "whitespace/*"]

[rules.severity]
"W0612" = "error"
"E0001" = "info"
```

### `rules.disable`

Disable specific rule codes. Supports exact codes and prefix patterns.

```toml
[rules]
disable = [
    "E501",           # Exact rule code
    "whitespace/*",   # All rules starting with "whitespace"
    "clippy::needless_*"  # All clippy needless_* rules
]
```

**Type:** Array of strings
**Default:** `[]`

---

### `[rules.severity]`

Override severity level for specific rules.

```toml
[rules.severity]
"W0612" = "error"    # Upgrade warning to error
"E0001" = "info"     # Downgrade error to info
"todo" = "off"       # Disable rule entirely
```

**Type:** Map of string to severity
**Valid severities:** `error`, `warning`, `info`, `off`

---

### `[[rules.custom]]`

Define custom regex-based lint rules.

```toml
[[rules.custom]]
code = "custom/no-fixme"
pattern = "FIXME|XXX"
message = "Found FIXME/XXX comment that needs attention"
severity = "warning"
suggestion = "Address the issue or convert to TODO"
languages = ["rust", "python"]
extensions = ["rs", "py"]
enabled = true
```

### Custom Rule Fields

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `code` | String | Yes | - | Unique rule code (e.g., `custom/no-print`) |
| `pattern` | String | Yes | - | Regex pattern to match |
| `message` | String | Yes | - | Error message to display |
| `severity` | String | No | `warning` | Severity level (`error`, `warning`, `info`) |
| `suggestion` | String | No | - | Fix suggestion text |
| `extensions` | Array | No | `[]` | File extensions filter (empty = all) |
| `languages` | Array | No | `[]` | Language filter (empty = all) |
| `enabled` | Boolean | No | `true` | Enable/disable this rule |

---

## Performance Configuration

### `[performance]`

Tune performance-related settings.

```toml
[performance]
large_file_threshold = 1048576  # 1MB
skip_large_files = false
cache_max_age_days = 7
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `large_file_threshold` | Integer | `1048576` (1MB) | File size threshold in bytes |
| `skip_large_files` | Boolean | `false` | Skip (instead of warn) for large files |
| `cache_max_age_days` | Integer | `7` | Plugin cache expiry in days |

---

## Git Hooks Configuration

### `[hooks]`

Configure git hook behavior.

```toml
[hooks]
timeout = 60
parallel = true
commit_msg_pattern = "^(feat|fix|docs|style|refactor|perf|test|build|ci|chore|revert)(\\(.+\\))?: .{1,72}"
require_ticket = false
ticket_pattern = "\\[\\w+-\\d+\\]"
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `timeout` | Integer | `60` | Hook timeout in seconds |
| `parallel` | Boolean | `true` | Enable parallel execution |
| `commit_msg_pattern` | String | Conventional Commits | Regex for valid commit messages |
| `require_ticket` | Boolean | `false` | Require ticket reference |
| `ticket_pattern` | String | - | Regex for ticket format (e.g., `[JIRA-123]`) |

---

## Language-Specific Configuration

### `[rust]`

Rust-specific options.

```toml
[rust]
enabled = true
max_complexity = 15
excludes = ["target/**"]

[rust.rules]
disable = ["clippy::needless_return"]
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | Boolean | `true` | Enable/disable Rust linting |
| `max_complexity` | Integer | Global | Override max complexity |
| `excludes` | Array | `[]` | Additional exclude patterns |
| `rules` | RulesConfig | - | Language-specific rule overrides |

---

### `[python]`

Python-specific options.

```toml
[python]
enabled = true
max_complexity = 10
excludes = ["*_test.py", "test_*.py"]
```

Same fields as `[rust]`.

---

### `[cpp]`

C++ specific options with cpplint/clang-tidy support.

```toml
[cpp]
enabled = true
max_complexity = 25
linelength = 120
cpplint_filter = "-build/c++11,-whitespace/tab"
clang_tidy_ignored_checks = ["clang-analyzer-osx.cocoa.RetainCount"]
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | Boolean | `true` | Enable/disable C++ linting |
| `max_complexity` | Integer | Global | Override max complexity |
| `excludes` | Array | `[]` | Additional exclude patterns |
| `linelength` | Integer | `80` | Line length for cpplint |
| `cpplint_filter` | String | - | Cpplint filter rules |
| `clang_tidy_ignored_checks` | Array | `[]` | Clang-tidy checks to ignore |
| `rules` | RulesConfig | - | Language-specific rule overrides |

---

### `[oc]` / `[objectivec]`

Objective-C specific options. Same fields as `[cpp]`.

```toml
[oc]
linelength = 150
cpplint_filter = "-build/header_guard"
```

---

### `[typescript]`

TypeScript-specific options.

```toml
[typescript]
enabled = true
excludes = ["*.d.ts"]
```

Same fields as `[rust]`.

---

### `[javascript]`

JavaScript-specific options.

```toml
[javascript]
enabled = true
excludes = ["*.min.js"]
```

Same fields as `[rust]`.

---

### `[go]`

Go-specific options.

```toml
[go]
enabled = true
max_complexity = 15
```

Same fields as `[rust]`.

---

### `[java]`

Java-specific options.

```toml
[java]
enabled = true
excludes = ["*Test.java"]
```

Same fields as `[rust]`.

---

## Source Configuration (CodeCC Compatibility)

### `[source]`

Configure source path exclusions (compatible with CodeCC `.code.yml`).

```toml
[source.test_source]
filepath_regex = [".*_test\\.py$", "test_.*\\.py$"]

[source.auto_generate_source]
filepath_regex = ["generated/.*"]

[source.third_party_source]
filepath_regex = ["vendor/.*", "third_party/.*"]
```

---

## Auto-Sync Configuration

### `[plugin_auto_sync]`

Configure automatic plugin synchronization.

```toml
[plugin_auto_sync]
enabled = true
interval_hours = 24
```

See [Auto Sync Documentation](../features/auto-sync.md) for details.

---

### `[self_auto_update]`

Configure automatic self-updates.

```toml
[self_auto_update]
enabled = false
check_interval_hours = 24
```

See [Self Update Documentation](../features/self-update.md) for details.

---

## Complete Example

```toml
# .linthis/config.toml

# Global settings
languages = ["rust", "python", "typescript"]
excludes = ["*.generated.*", "vendor/**"]
max_complexity = 20
preset = "google"

# Plugin configuration
[plugins]
sources = [
    { name = "official" },
    { name = "company-rules", url = "https://github.com/company/linthis-plugin.git" }
]

# Rules configuration
[rules]
disable = ["E501", "whitespace/*"]

[rules.severity]
"W0612" = "error"
"todo" = "warning"

[[rules.custom]]
code = "custom/no-console"
pattern = "console\\.(log|warn|error)"
message = "console.log found in code"
severity = "warning"
languages = ["typescript", "javascript"]

# Performance tuning
[performance]
large_file_threshold = 2097152  # 2MB
skip_large_files = true

# Git hooks
[hooks]
timeout = 120
require_ticket = true
ticket_pattern = "\\[PROJ-\\d+\\]"

# Language-specific overrides
[rust]
max_complexity = 15

[python]
excludes = ["*_test.py"]

[cpp]
linelength = 120
cpplint_filter = "-build/c++11"
```
