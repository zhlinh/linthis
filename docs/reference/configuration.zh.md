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

### `[hook]`

配置 git hook 行为和来源覆盖。

```toml
[hook]
timeout = 60
parallel = true
output_width = 0   # 0 = 自动检测终端宽度
```

| 字段 | 类型 | 默认值 | 描述 |
|-----|-----|-------|------|
| `timeout` | 整数 | `60` | Hook 超时秒数 |
| `parallel` | 布尔值 | `true` | 启用并行执行 |
| `output_width` | 整数 | `0` | 输出框宽度（0=自动，最小 50，最大 120） |

---

### 三层 Hook 解析机制

`linthis hook install` 生成或解析 hook 时，按三层优先级（由高到低）执行：

| 层级 | 描述 |
|------|------|
| **第 1 层** | 固定路径自动发现——项目根目录的 `hooks/<type>/<event>`（git hook）或 `hooks/agent/plugins/<id>/`（agent 插件） |
| **第 2 层** | TOML 来源映射——`.linthis/config.toml` 中的 `[hook.git]`、`[hook.agent.plugins._default]`、`[hook.agent.stop]` 条目 |
| **第 3 层** | 内置生成器——默认的内置生成脚本或规则内容（兜底） |

第 1 层无需配置，只需在约定路径放置文件即可自动生效。第 2 层通过 TOML 实现细粒度来源覆盖。第 3 层始终作为兜底。

---

### `HookSource` — 来源规格

所有 `[hook.*]` 覆盖条目均使用 `source = { ... }` 字段，支持以下五种变体：

```toml
# 变体 1 — File：相对于项目根目录的本地路径
source = { file = "hooks/git/pre-commit" }

# 变体 2 — Plugin：已安装插件缓存中的路径
source = { plugin = "my-plugin", file = "hooks/git/pre-commit" }

# 变体 3 — Marketplace：来自命名市场仓库的插件文件
source = { marketplace = "corp", plugin = "linthis-official", file = "hooks/agent/plugins/lt/lint" }

# 变体 4 — Url：直接 HTTP/HTTPS 下载（仅文件，不支持目录）
source = { url = "https://example.com/hooks/pre-commit" }

# 变体 5 — Git：克隆 git 仓库并使用指定路径
source = { git = "https://github.com/org/repo.git", ref = "v1.0", path = "hooks/git/pre-commit" }
```

| 变体 | 必填字段 | 可选字段 | 说明 |
|------|---------|---------|------|
| `File` | `file` | — | 相对于项目根目录的路径 |
| `Plugin` | `plugin`、`file` | — | 插件须通过 `linthis plugin add` 添加 |
| `Marketplace` | `marketplace`、`plugin`、`file` | — | 市场 URL 在 `[hook.marketplaces]` 中定义 |
| `Url` | `url` | — | 仅支持文件，不支持目录 |
| `Git` | `git`、`path` | `ref` | 首次使用时克隆，本地缓存 |

---

### `[hook.marketplaces]`

`HookSource::Marketplace` 使用的命名市场仓库。键 `"default"` 在未指定 `marketplace` 字段时使用。

```toml
[hook.marketplaces]
default = "https://github.com/linthis-group/marketplace.git"
corp    = "https://github.com/mycompany/linthis-marketplace.git"
```

---

### `[hook.git]`

覆盖 git hook 脚本（第 2 层）。键为事件名称。

```toml
[hook.git]
pre-commit = { source = { plugin = "my-plugin", file = "hooks/git/pre-commit" } }
pre-push   = { source = { file = "hooks/git/pre-push" } }
commit-msg = { source = { url = "https://example.com/hooks/commit-msg" } }
```

其他 hook 类型也有对应的覆盖节：
- `[hook.git-with-agent]` — 带 AI 修复兜底的 git hook
- `[hook.prek]` — prek hook 脚本
- `[hook.prek-with-agent]` — 带 AI 修复兜底的 prek hook

---

### `[hook.agent.plugins._default]`

覆盖 agent 插件包（第 2 层）。每个条目指向包含技能、命令、记忆和 hook 子目录的目录。键为插件 ID。

```toml
[hook.agent.plugins._default]
"lt.lint"   = { source = { plugin = "my-plugin", file = "hooks/agent/plugins/lt/lint" } }
"lt.cmsg"   = { source = { plugin = "my-plugin", file = "hooks/agent/plugins/lt/cmsg" } }
"lt.review" = { source = { plugin = "my-plugin", file = "hooks/agent/plugins/lt/review" } }
```

解析后的目录须包含以下一个或多个子目录：

```
<plugin-dir>/
├── skills/<skill_name>/SKILL.md    — 技能指令文件（如 skills/lt-lint/SKILL.md）
├── commands/                        — 斜杠命令文件（可选）
├── memories/TOPLEVEL.md             — 注入 CLAUDE.md 等文件的记忆段落（可选）
└── hooks/hooks.json                 — stop hook 设置（可选）
```

---

### `[hook.agent.stop]`

覆盖 agent Stop Hook 设置文件（第 2 层）。键格式为 `<provider>.<filename-stem>`。

```toml
[hook.agent.stop]
"claude.settings" = { source = { plugin = "my-plugin", file = "hooks/agent/hook/stop/claude/settings.json" } }
```

---

### `[hook.agent.skill-names]`

配置每个 hook 事件的自定义技能目录名。允许已有自定义技能目录的团队将 linthis 事件映射到其自定义名称，而非使用默认值。

```toml
[hook.agent.skill-names]
pre-commit = "my-team-lint"     # 默认值: "lt-lint"
commit-msg = "my-team-cmsg"     # 默认值: "lt-cmsg"
pre-push = "my-team-review"     # 默认值: "lt-review"
```

值的用途：
- **目录名** 位于 `.claude/skills/` 和 `.codebuddy/skills/` 下（如 `.claude/skills/my-team-lint/SKILL.md`）
- **基础文件名** 用于平铺文件提供商（Gemini: `my-team-lint.md`、Cursor: `my-team-lint.mdc` 等）

| 字段 | 类型 | 默认值 | 描述 |
|-----|-----|-------|------|
| `pre-commit` | 字符串 | `"lt-lint"` | pre-commit（lint）事件的技能名称 |
| `commit-msg` | 字符串 | `"lt-cmsg"` | commit-msg 事件的技能名称 |
| `pre-push` | 字符串 | `"lt-review"` | pre-push（review）事件的技能名称 |

未配置时使用默认值（向后兼容）。

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
fn_length = 80
```

| 字段 | 类型 | 默认值 | 描述 |
|-----|-----|-------|------|
| `enabled` | 布尔值 | `true` | 启用/禁用 C++ 检查 |
| `max_complexity` | 整数 | 全局值 | 覆盖最大复杂度 |
| `excludes` | 数组 | `[]` | 额外排除模式 |
| `linelength` | 整数 | `80` | cpplint 行长度 |
| `cpplint_filter` | 字符串 | - | Cpplint 过滤规则 |
| `clang_tidy_ignored_checks` | 数组 | `[]` | 要忽略的 Clang-tidy 检查 |
| `fn_length` | 整数 | `80` | Objective-C 方法最大 SLOC（非空非注释行数） |
| `rules` | RulesConfig | - | 语言特定规则覆盖 |

---

### `[oc]` / `[objectivec]`

Objective-C 特定选项。包含 `[cpp]` 所有字段，另增 `fn_length` 用于方法长度检查。

```toml
[oc]
linelength = 150
cpplint_filter = "-build/header_guard"
fn_length = 80  # 方法最大 SLOC（非空非注释行数），默认 80
```

| 字段 | 类型 | 默认值 | 描述 |
|-----|-----|-------|------|
| `fn_length` | 整数 | `80` | Objective-C 方法最大 SLOC（非空非注释行数） |

其余字段与 `[cpp]` 相同。

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

## Review 配置

### `[review]`

配置 AI 代码审查功能。

```toml
[review]
enabled = true
auto_fix = false
provider = "claude-cli"
retention_days = 30

[review.reviewers]
default = ["alice", "bob"]
```

| 字段 | 类型 | 默认值 | 描述 |
|-----|-----|-------|------|
| `enabled` | 布尔值 | `true` | 启用/禁用 review 功能 |
| `auto_fix` | 布尔值 | `false` | 启用自动修复模式（创建修复分支 + PR/MR） |
| `provider` | 字符串 | - | review 使用的 AI 提供商 |
| `retention_days` | 整数 | `30` | review 产物保留天数 |

### `[review.reviewers]`

| 字段 | 类型 | 默认值 | 描述 |
|-----|-----|-------|------|
| `default` | 数组 | `[]` | 默认审查人（平台用户名） |

### `[review.platforms.<name>]`

配置自定义 Git 平台以支持 PR/MR 创建。

```toml
[review.platforms.my-gitlab]
pr_create = "glab mr create --title '{title}' --description '{body}'"
pr_list = "glab mr list"
reviewer_flag = "--reviewer"
```

| 字段 | 类型 | 必填 | 描述 |
|-----|-----|-----|------|
| `pr_create` | 字符串 | 是 | 创建 PR/MR 的命令模板 |
| `pr_list` | 字符串 | 否 | 列出 PR/MR 的命令模板 |
| `reviewer_flag` | 字符串 | 否 | 指定审查人的参数名（默认：`--reviewer`） |
| `install_cmd` | 字符串 | 否 | CLI 工具的安装命令 |
| `install_hint` | 字符串 | 否 | 工具缺失时显示的安装提示 |

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
[hook]
timeout = 120

# Hook 来源覆盖（第 2 层）
[hook.git]
pre-commit = { source = { plugin = "company", file = "hooks/git/pre-commit" } }

[hook.agent.plugins._default]
"lt.lint" = { source = { plugin = "company", file = "hooks/agent/plugins/lt/lint" } }

[hook.agent.stop]
"claude.settings" = { source = { plugin = "company", file = "hooks/agent/hook/stop/claude/settings.json" } }

# 自定义技能目录名（可选）
[hook.agent.skill-names]
pre-commit = "custom-lint"

# 语言特定覆盖
[rust]
max_complexity = 15

[python]
excludes = ["*_test.py"]

[cpp]
linelength = 120
cpplint_filter = "-build/c++11"
```
