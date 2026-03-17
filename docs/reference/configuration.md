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
**Valid values:** `rust`, `python`, `typescript`, `javascript`, `go`, `java`, `c`, `cpp`, `oc` (Objective-C), `swift`, `kotlin`, `lua`, `dart`, `shell`, `ruby`, `php`, `scala`, `csharp`

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

### `[hook]`

Configure git hook behavior and source overrides.

```toml
[hook]
timeout = 60
parallel = true
output_width = 0   # 0 = auto-detect terminal width
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `timeout` | Integer | `60` | Hook timeout in seconds |
| `parallel` | Boolean | `true` | Enable parallel execution |
| `output_width` | Integer | `0` | Output box width (0 = auto, min 50, max 120) |

---

### Three-Tier Hook Resolution

When `linthis hook install` generates or resolves a hook, it applies a three-tier resolution system (highest → lowest priority):

| Tier | Description |
|------|-------------|
| **Tier 1** | Fixed-path auto-discovery — `hooks/<type>/<event>` in project root (for git hooks) or `hooks/agent/plugins/<id>/` (for agent plugins) |
| **Tier 2** | TOML source mapping — `[hook.git]`, `[hook.agent.plugins._default]`, `[hook.agent.stop]` entries in `.linthis/config.toml` |
| **Tier 3** | Built-in generator — the default generated script or rules content (unchanged fallback) |

Tier 1 takes effect automatically without any configuration — just place a file at the expected path. Tier 2 allows fine-grained source overrides via TOML. Tier 3 is always the fallback.

---

### `HookSource` — Source Specification

All `[hook.*]` override entries use a `source = { ... }` field with one of five variants:

```toml
# Variant 1 — File: local path relative to project root
source = { file = "hooks/git/pre-commit" }

# Variant 2 — Plugin: path inside an installed plugin cache
source = { plugin = "my-plugin", file = "hooks/git/pre-commit" }

# Variant 3 — Marketplace: file inside a plugin from a named marketplace repo
source = { marketplace = "corp", plugin = "linthis-official", file = "hooks/agent/plugins/lt/lint" }

# Variant 4 — Url: direct HTTP/HTTPS download (files only)
source = { url = "https://example.com/hooks/pre-commit" }

# Variant 5 — Git: clone a git repo and use the given path
source = { git = "https://github.com/org/repo.git", ref = "v1.0", path = "hooks/git/pre-commit" }
```

| Variant | Required fields | Optional fields | Notes |
|---------|-----------------|-----------------|-------|
| `File` | `file` | — | Path relative to project root |
| `Plugin` | `plugin`, `file` | — | Plugin must be added via `linthis plugin add` |
| `Marketplace` | `marketplace`, `plugin`, `file` | — | Marketplace URL defined in `[hook.marketplaces]` |
| `Url` | `url` | — | Files only, not directories |
| `Git` | `git`, `path` | `ref` | Clones on first use, cached locally |

---

### `[hook.marketplaces]`

Named marketplace repositories used by `HookSource::Marketplace`. The key `"default"` is used when no `marketplace` field is given.

```toml
[hook.marketplaces]
default = "https://github.com/linthis-group/marketplace.git"
corp    = "https://github.com/mycompany/linthis-marketplace.git"
```

| Field | Type | Description |
|-------|------|-------------|
| `<name>` | String | Git URL of the marketplace repository |

---

### `[hook.git]`

Override the hook script for git hooks (Tier 2). Key is the event name.

```toml
[hook.git]
pre-commit = { source = { plugin = "my-plugin", file = "hooks/git/pre-commit" } }
pre-push   = { source = { file = "hooks/git/pre-push" } }
commit-msg = { source = { url = "https://example.com/hooks/commit-msg" } }
```

| Key | Description |
|-----|-------------|
| `pre-commit` | Script to run for the pre-commit event |
| `pre-push` | Script to run for the pre-push event |
| `commit-msg` | Script to run for the commit-msg event |

Similar override sections exist for other hook types:
- `[hook.git-with-agent]` — git hooks with AI fix fallback
- `[hook.prek]` — prek hook scripts
- `[hook.prek-with-agent]` — prek hooks with AI fix fallback

---

### `[hook.agent.plugins._default]`

Override agent plugin bundles (Tier 2). Each entry points to a directory containing skill, command, memory, and hook subdirectories. Key is the plugin ID.

```toml
[hook.agent.plugins._default]
"lt.lint"   = { source = { plugin = "my-plugin", file = "hooks/agent/plugins/lt/lint" } }
"lt.cmsg"   = { source = { plugin = "my-plugin", file = "hooks/agent/plugins/lt/cmsg" } }
"lt.review" = { source = { plugin = "my-plugin", file = "hooks/agent/plugins/lt/review" } }
```

The resolved directory must contain one or more of:

```
<plugin-dir>/
├── skills/<skill_name>/SKILL.md    — skill instruction file (e.g., skills/lt-lint/SKILL.md)
├── commands/                        — slash command files (optional)
├── memories/TOPLEVEL.md             — memory section injected into CLAUDE.md etc. (optional)
└── hooks/hooks.json                 — stop hook settings (optional)
```

---

### `[hook.agent.stop]`

Override the agent Stop Hook settings file (Tier 2). Key format is `<provider>.<filename-stem>`.

```toml
[hook.agent.stop]
"claude.settings" = { source = { plugin = "my-plugin", file = "hooks/agent/hook/stop/claude/settings.json" } }
```

| Key format | Example | Description |
|------------|---------|-------------|
| `<provider>.<stem>` | `claude.settings` | Overrides `.claude/settings.json` for the Claude provider |

---

### `[hook.agent.skill-names]`

Configure custom skill directory names for each hook event. This allows teams with existing skill directories to map linthis events to their custom names instead of the defaults.

```toml
[hook.agent.skill-names]
pre-commit = "my-team-lint"     # default: "lt-lint"
commit-msg = "my-team-cmsg"     # default: "lt-cmsg"
pre-push = "my-team-review"     # default: "lt-review"
```

The values are used as:
- **Directory names** under `.claude/skills/` and `.codebuddy/skills/` (e.g., `.claude/skills/my-team-lint/SKILL.md`)
- **Base file names** for flat-file providers (Gemini: `my-team-lint.md`, Cursor: `my-team-lint.mdc`, etc.)

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `pre-commit` | String | `"lt-lint"` | Skill name for the pre-commit (lint) event |
| `commit-msg` | String | `"lt-cmsg"` | Skill name for the commit-msg event |
| `pre-push` | String | `"lt-review"` | Skill name for the pre-push (review) event |

Without this config, the defaults are used (backward compatible).

---

## Commit Message Validation (`[cmsg]`)

Configure commit message validation rules used by `linthis cmsg`.

```toml
[cmsg]
commit_msg_pattern = "^(feat|fix|docs|style|refactor|perf|test|build|ci|chore|revert)(\\(.+\\))?: .{1,72}"
require_ticket = false
ticket_pattern = "\\[\\w+-\\d+\\]"
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
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
fn_length = 80
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | Boolean | `true` | Enable/disable C++ linting |
| `max_complexity` | Integer | Global | Override max complexity |
| `excludes` | Array | `[]` | Additional exclude patterns |
| `linelength` | Integer | `80` | Line length for cpplint |
| `cpplint_filter` | String | - | Cpplint filter rules |
| `clang_tidy_ignored_checks` | Array | `[]` | Clang-tidy checks to ignore |
| `fn_length` | Integer | `80` | Max Objective-C method SLOC (non-blank, non-comment lines) |
| `rules` | RulesConfig | - | Language-specific rule overrides |

---

### `[oc]` / `[objectivec]`

Objective-C specific options. Includes all fields from `[cpp]`, plus `fn_length` for method length checking.

```toml
[oc]
linelength = 150
cpplint_filter = "-build/header_guard"
fn_length = 80  # Max method SLOC (non-blank, non-comment lines). Default: 80
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `fn_length` | Integer | `80` | Max Objective-C method SLOC (non-blank, non-comment lines) |

All other fields same as `[cpp]`.

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

### `[c]`

C-specific options.

```toml
[c]
enabled = true
linelength = 120
```

Same fields as `[cpp]`.

---

### `[swift]`

Swift-specific options.

```toml
[swift]
enabled = true
excludes = ["Pods/**"]
```

Same fields as `[rust]`.

---

### `[kotlin]`

Kotlin-specific options.

```toml
[kotlin]
enabled = true
excludes = ["*Test.kt"]
```

Same fields as `[rust]`.

---

### `[lua]`

Lua-specific options.

```toml
[lua]
enabled = true
```

Same fields as `[rust]`.

---

### `[dart]`

Dart-specific options.

```toml
[dart]
enabled = true
excludes = [".dart_tool/**"]
```

Same fields as `[rust]`.

---

### `[shell]`

Shell/Bash-specific options.

```toml
[shell]
enabled = true
excludes = ["*.bak.sh"]
```

Same fields as `[rust]`.

---

### `[ruby]`

Ruby-specific options.

```toml
[ruby]
enabled = true
excludes = ["vendor/**"]
```

Same fields as `[rust]`.

---

### `[php]`

PHP-specific options.

```toml
[php]
enabled = true
excludes = ["vendor/**"]
```

Same fields as `[rust]`.

---

### `[scala]`

Scala-specific options.

```toml
[scala]
enabled = true
excludes = ["target/**"]
```

Same fields as `[rust]`.

---

### `[csharp]`

C#-specific options.

```toml
[csharp]
enabled = true
excludes = ["bin/**", "obj/**"]
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

## Review Configuration

### `[review]`

Configure the AI-powered code review feature.

```toml
[review]
enabled = true
auto_fix = false
provider = "claude-cli"
retention_days = 30

[review.reviewers]
default = ["alice", "bob"]
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | Boolean | `true` | Enable/disable review feature |
| `auto_fix` | Boolean | `false` | Enable auto-fix mode (create fix branch + PR/MR) |
| `provider` | String | - | AI provider override for review |
| `retention_days` | Integer | `30` | Retention days for review artifacts |

### `[review.reviewers]`

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `default` | Array | `[]` | Default reviewers (platform usernames) |

### `[review.platforms.<name>]`

Configure custom Git platforms for PR/MR creation.

```toml
[review.platforms.my-gitlab]
pr_create = "glab mr create --title '{title}' --description '{body}'"
pr_list = "glab mr list"
reviewer_flag = "--reviewer"
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `pr_create` | String | Yes | Command template for creating PR/MR |
| `pr_list` | String | No | Command template for listing PRs/MRs |
| `reviewer_flag` | String | No | Flag name for specifying reviewers (default: `--reviewer`) |
| `install_cmd` | String | No | Install command for the CLI tool |
| `install_hint` | String | No | Human-readable install hint shown when tool is missing |

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
[hook]
timeout = 120

# Hook source overrides (Tier 2)
[hook.git]
pre-commit = { source = { plugin = "company", file = "hooks/git/pre-commit" } }

[hook.agent.plugins._default]
"lt.lint" = { source = { plugin = "company", file = "hooks/agent/plugins/lt/lint" } }

[hook.agent.stop]
"claude.settings" = { source = { plugin = "company", file = "hooks/agent/hook/stop/claude/settings.json" } }

# Custom skill directory names (optional)
[hook.agent.skill-names]
pre-commit = "custom-lint"

# Language-specific overrides
[rust]
max_complexity = 15

[python]
excludes = ["*_test.py"]

[cpp]
linelength = 120
cpplint_filter = "-build/c++11"
```
