# 配置参考

linthis 所有配置选项的完整参考。

## 配置层次

配置从多个来源加载，优先级如下（从高到低）：

1. **CLI 参数** - 命令行参数覆盖所有配置文件
2. **项目配置** - 项目根目录下的 `.linthis/config.toml`
3. **用户配置** - `~/.linthis/config.toml` 用于全局设置
4. **内置默认值** - 所有选项的合理默认值

## 配置文件格式

支持的格式：
- TOML (`.toml`) - 推荐
- YAML (`.yml`、`.yaml`)
- JSON (`.json`)

---

## 顶级选项

### `languages`

要检查的语言。如果为空，根据文件扩展名自动检测。

```toml
languages = ["rust", "python", "typescript"]
```

**类型：** 字符串数组
**默认值：** `[]`（自动检测）
**有效值：** `rust`、`python`、`typescript`、`javascript`、`go`、`java`、`cpp`、`oc`（Objective-C）、`swift`、`kotlin`、`lua`、`dart`

---

### `includes`

要包含的文件模式（glob 模式）。如果为空，检查所有文件。

```toml
includes = ["src/**", "lib/**"]
```

**类型：** 字符串数组（glob 模式）
**默认值：** `[]`（所有文件）

---

### `excludes`

要排除的文件模式（glob 模式）。添加到内置排除项。

```toml
excludes = ["*.generated.rs", "vendor/**"]
```

**类型：** 字符串数组（glob 模式）
**默认值：** `[]`
**别名：** `exclude`

内置排除项（始终应用）：
- `**/node_modules/**`
- `**/target/**`
- `**/.git/**`
- `**/vendor/**`
- `**/__pycache__/**`

---

### `max_complexity`

每个函数允许的最大圈复杂度。

```toml
max_complexity = 20
```

**类型：** 整数
**默认值：** `20`

---

### `preset`

要使用的代码风格预设。

```toml
preset = "google"
```

**类型：** 字符串
**默认值：** `null`（无）
**有效值：** `google`、`standard`、`airbnb`

---

### `verbose`

启用详细输出，显示额外细节。

```toml
verbose = true
```

**类型：** 布尔值
**默认值：** `false`

---

## 插件配置

### `[plugins]`

配置额外的 linter 或规则的外部插件。

```toml
[plugins]
sources = [
    { name = "official" },
    { name = "custom-rules", url = "https://github.com/user/lts-plugin.git", ref = "main" }
]
```

### 插件源字段

| 字段 | 类型 | 必需 | 默认值 | 描述 |
|-----|-----|-----|-------|------|
| `name` | 字符串 | 是 | - | 插件名称（用于注册表查找） |
| `url` | 字符串 | 否 | - | Git 仓库 URL |
| `ref` | 字符串 | 否 | `main` | Git 引用（标签、分支、提交） |
| `enabled` | 布尔值 | 否 | `true` | 启用/禁用此插件 |

---

## 规则配置

### `[rules]`

配置规则禁用、严重性覆盖和自定义规则。

```toml
[rules]
disable = ["E501", "whitespace/*"]

[rules.severity]
"W0612" = "error"
"E0001" = "info"
```

### `rules.disable`

禁用特定规则代码。支持精确代码和前缀模式。

```toml
[rules]
disable = [
    "E501",           # 精确规则代码
    "whitespace/*",   # 以 "whitespace" 开头的所有规则
    "clippy::needless_*"  # 所有 clippy needless_* 规则
]
```

**类型：** 字符串数组
**默认值：** `[]`

---

### `[rules.severity]`

覆盖特定规则的严重性级别。

```toml
[rules.severity]
"W0612" = "error"    # 将警告升级为错误
"E0001" = "info"     # 将错误降级为信息
"todo" = "off"       # 完全禁用规则
```

**类型：** 字符串到严重性的映射
**有效严重性：** `error`、`warning`、`info`、`off`

---

### `[[rules.custom]]`

定义自定义的基于正则表达式的 lint 规则。

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

### 自定义规则字段

| 字段 | 类型 | 必需 | 默认值 | 描述 |
|-----|-----|-----|-------|------|
| `code` | 字符串 | 是 | - | 唯一规则代码（如 `custom/no-print`） |
| `pattern` | 字符串 | 是 | - | 要匹配的正则表达式模式 |
| `message` | 字符串 | 是 | - | 要显示的错误消息 |
| `severity` | 字符串 | 否 | `warning` | 严重性级别（`error`、`warning`、`info`） |
| `suggestion` | 字符串 | 否 | - | 修复建议文本 |
| `extensions` | 数组 | 否 | `[]` | 文件扩展名过滤器（空 = 所有） |
| `languages` | 数组 | 否 | `[]` | 语言过滤器（空 = 所有） |
| `enabled` | 布尔值 | 否 | `true` | 启用/禁用此规则 |

---

## 性能配置

### `[performance]`

调整性能相关设置。

```toml
[performance]
large_file_threshold = 1048576  # 1MB
skip_large_files = false
cache_max_age_days = 7
```

| 字段 | 类型 | 默认值 | 描述 |
|-----|-----|-------|------|
| `large_file_threshold` | 整数 | `1048576`（1MB） | 文件大小阈值（字节） |
| `skip_large_files` | 布尔值 | `false` | 跳过（而不是警告）大文件 |
| `cache_max_age_days` | 整数 | `7` | 插件缓存过期天数 |

---

## Git Hooks 配置

### `[hooks]`

配置 git hook 行为。

```toml
[hooks]
timeout = 60
parallel = true
commit_msg_pattern = "^(feat|fix|docs|style|refactor|perf|test|build|ci|chore|revert)(\\(.+\\))?: .{1,72}"
require_ticket = false
ticket_pattern = "\\[\\w+-\\d+\\]"
```

| 字段 | 类型 | 默认值 | 描述 |
|-----|-----|-------|------|
| `timeout` | 整数 | `60` | Hook 超时秒数 |
| `parallel` | 布尔值 | `true` | 启用并行执行 |
| `commit_msg_pattern` | 字符串 | 约定式提交 | 有效提交消息的正则表达式 |
| `require_ticket` | 布尔值 | `false` | 要求工单引用 |
| `ticket_pattern` | 字符串 | - | 工单格式的正则表达式（如 `[JIRA-123]`） |

---

## 语言特定配置

### `[rust]`

Rust 特定选项。

```toml
[rust]
enabled = true
max_complexity = 15
excludes = ["target/**"]

[rust.rules]
disable = ["clippy::needless_return"]
```

| 字段 | 类型 | 默认值 | 描述 |
|-----|-----|-------|------|
| `enabled` | 布尔值 | `true` | 启用/禁用 Rust 检查 |
| `max_complexity` | 整数 | 全局值 | 覆盖最大复杂度 |
| `excludes` | 数组 | `[]` | 额外排除模式 |
| `rules` | RulesConfig | - | 语言特定规则覆盖 |

---

### `[python]`

Python 特定选项。

```toml
[python]
enabled = true
max_complexity = 10
excludes = ["*_test.py", "test_*.py"]
```

与 `[rust]` 相同的字段。

---

### `[cpp]`

C++ 特定选项，支持 cpplint/clang-tidy。

```toml
[cpp]
enabled = true
max_complexity = 25
linelength = 120
cpplint_filter = "-build/c++11,-whitespace/tab"
clang_tidy_ignored_checks = ["clang-analyzer-osx.cocoa.RetainCount"]
```

| 字段 | 类型 | 默认值 | 描述 |
|-----|-----|-------|------|
| `enabled` | 布尔值 | `true` | 启用/禁用 C++ 检查 |
| `max_complexity` | 整数 | 全局值 | 覆盖最大复杂度 |
| `excludes` | 数组 | `[]` | 额外排除模式 |
| `linelength` | 整数 | `80` | cpplint 行长度 |
| `cpplint_filter` | 字符串 | - | Cpplint 过滤规则 |
| `clang_tidy_ignored_checks` | 数组 | `[]` | 要忽略的 Clang-tidy 检查 |
| `rules` | RulesConfig | - | 语言特定规则覆盖 |

---

### `[oc]` / `[objectivec]`

Objective-C 特定选项。与 `[cpp]` 相同的字段。

```toml
[oc]
linelength = 150
cpplint_filter = "-build/header_guard"
```

---

### `[typescript]`

TypeScript 特定选项。

```toml
[typescript]
enabled = true
excludes = ["*.d.ts"]
```

与 `[rust]` 相同的字段。

---

### `[javascript]`

JavaScript 特定选项。

```toml
[javascript]
enabled = true
excludes = ["*.min.js"]
```

与 `[rust]` 相同的字段。

---

### `[go]`

Go 特定选项。

```toml
[go]
enabled = true
max_complexity = 15
```

与 `[rust]` 相同的字段。

---

### `[java]`

Java 特定选项。

```toml
[java]
enabled = true
excludes = ["*Test.java"]
```

与 `[rust]` 相同的字段。

---

## 源配置（CodeCC 兼容）

### `[source]`

配置源路径排除（兼容 CodeCC `.code.yml`）。

```toml
[source.test_source]
filepath_regex = [".*_test\\.py$", "test_.*\\.py$"]

[source.auto_generate_source]
filepath_regex = ["generated/.*"]

[source.third_party_source]
filepath_regex = ["vendor/.*", "third_party/.*"]
```

---

## 自动同步配置

### `[plugin_auto_sync]`

配置自动插件同步。

```toml
[plugin_auto_sync]
enabled = true
interval_hours = 24
```

详见[自动同步文档](../features/auto-sync.md)。

---

### `[self_auto_update]`

配置自动自更新。

```toml
[self_auto_update]
enabled = false
check_interval_hours = 24
```

详见[自更新文档](../features/self-update.md)。

---

## 完整示例

```toml
# .linthis/config.toml

# 全局设置
languages = ["rust", "python", "typescript"]
excludes = ["*.generated.*", "vendor/**"]
max_complexity = 20
preset = "google"

# 插件配置
[plugins]
sources = [
    { name = "official" },
    { name = "company-rules", url = "https://github.com/company/linthis-plugin.git" }
]

# 规则配置
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

# 性能调优
[performance]
large_file_threshold = 2097152  # 2MB
skip_large_files = true

# Git hooks
[hooks]
timeout = 120
require_ticket = true
ticket_pattern = "\\[PROJ-\\d+\\]"

# 语言特定覆盖
[rust]
max_complexity = 15

[python]
excludes = ["*_test.py"]

[cpp]
linelength = 120
cpplint_filter = "-build/c++11"
```
