# Configuration

linthis can be configured through configuration files, CLI parameters, or both.

## Configuration Files

### Project Configuration

Use `linthis init` to create the configuration file:

```bash
linthis init
```

This creates `.linthis/config.toml` in your project root:

```toml
# Specify languages to check (omit for auto-detection)
languages = ["rust", "python", "javascript"]

# Exclude files and directories
excludes = [
    "target/**",
    "node_modules/**",
    "*.generated.rs",
    "dist/**"
]

# Maximum cyclomatic complexity
max_complexity = 20

# Format preset
preset = "google"  # Options: google, airbnb, standard

# Configure plugins
[plugins]
sources = [
    { name = "official" },
    { name = "myplugin", url = "https://github.com/user/plugin.git", ref = "main" }
]

# Language-specific configuration
# [rust]
# max_complexity = 15

# [python]
# excludes = ["*_test.py"]
```

### Global Configuration

Global configuration is located at `~/.linthis/config.toml`, with the same format as project config.

### Configuration Priority

Configuration merge priority (from high to low):

1. **CLI Parameters**: `--option value`
2. **Project Config**: `.linthis/config.toml`
3. **Global Config**: `~/.linthis/config.toml`
4. **Built-in Defaults**

For tool-specific configs (ruff.toml, .eslintrc.js, etc.), the priority is:

1. **Local manual configs** (highest) - ruff.toml, pyproject.toml, .eslintrc.js in project
2. **CLI plugin configs** - from `--use-plugin` option
3. **Project plugin configs** - from `.linthis/config.toml` plugins section
4. **Global plugin configs** - from `~/.linthis/config.toml` plugins
5. **Tool defaults** (lowest)

## Configuration Management Commands

### Array Field Operations

Supported array fields: `includes`, `excludes`, `languages`

```bash
# Add values
linthis config add includes "src/**"
linthis config add excludes "*.log"
linthis config add languages "rust"

# Add to global config
linthis config add -g includes "lib/**"

# Remove values
linthis config remove excludes "*.log"

# Clear field
linthis config clear languages
```

### Scalar Field Operations

Supported scalar fields: `max_complexity`, `preset`, `verbose`

```bash
# Set value
linthis config set max_complexity 15
linthis config set preset google

# Set in global config
linthis config set -g max_complexity 20

# Unset value
linthis config unset max_complexity
```

### Query Operations

```bash
# Get single field
linthis config get includes
linthis config get max_complexity

# List all configuration
linthis config list
linthis config list -g  # global config
linthis config list -v  # verbose (show empty fields)
```

## Configuration Migration

linthis can migrate existing linter/formatter configurations:

```bash
# Auto-detect and migrate all configs
linthis config migrate

# Migrate specific tool
linthis config migrate --from eslint
linthis config migrate --from prettier
linthis config migrate --from black

# Preview changes
linthis config migrate --dry-run

# Create backup
linthis config migrate --backup
```

### Supported Tools

| Tool | Detected Files |
|------|---------------|
| ESLint | `.eslintrc.js`, `.eslintrc.json`, `.eslintrc.yml`, `eslint.config.js` |
| Prettier | `.prettierrc`, `.prettierrc.json`, `.prettierrc.yml`, `prettier.config.js` |
| Black | `pyproject.toml[tool.black]` |
| isort | `pyproject.toml[tool.isort]` |

## Environment Variables

### `LINTHIS_SKIP` — bypass a specific hook

Git's `--no-verify` skips every hook at once. `LINTHIS_SKIP` lets you bypass only one while keeping the others active. Values are comma-separated and case-insensitive.

| Token | Hooks skipped |
|-------|---------------|
| `check` or `pc` | pre-commit + post-commit (they always run as a pair) |
| `pre-commit` | only pre-commit |
| `post-commit` | only post-commit |
| `cmsg`, `cm`, or `commit-msg` | commit-msg |
| `pp` or `pre-push` | pre-push |
| `all` | every hook |

```bash
# Skip commit-msg regex check but keep lint running
LINTHIS_SKIP=cm git commit -m "WIP: non-conventional message"

# Skip lint + complexity + security (pre-commit) but keep commit-msg
LINTHIS_SKIP=check git commit -m "feat: quick save"

# Combine multiple
LINTHIS_SKIP=cm,pp git push
```

Unknown tokens print an error and are not skipped — fail-safe by design.

### `LINTHIS_SKIP_CHECKS` — filter individual checks

Skips specific checks inside a hook (pre-commit / pre-push). Accepts full names or any **≥ 3-character prefix** (case-insensitive).

| Token | Effect |
|-------|--------|
| `lin` or `lint` | skip the lint check |
| `sec` or `security` | skip the security (SAST) check |
| `com` or `complexity` | skip the complexity check |

```bash
# Skip slow complexity check
LINTHIS_SKIP_CHECKS=com git commit -m "fix: bug"

# Only run security (skip lint and complexity)
LINTHIS_SKIP_CHECKS=lin,com git commit -m "fix: bug"
```

Shorter-than-3-char or unknown tokens print a warning and are ignored; the rest still apply.

Both variables are orthogonal and can be combined:

```bash
LINTHIS_SKIP=cm LINTHIS_SKIP_CHECKS=com git commit -m "WIP"
```

### `LINTHIS_AGENT_MAX_AUTO_FIX` — cap auto-fix on large error sets

The `git-with-agent` hook type invokes an AI agent (Claude / Codex / Gemini / …) to auto-fix failing lint on commit. With hundreds or thousands of issues this can block the terminal for many minutes with no visible progress.

`LINTHIS_AGENT_MAX_AUTO_FIX` sets a ceiling on `errors + warnings` — above it, auto-fix is skipped and the commit fails fast, pointing you at the interactive fix path where agent output streams live:

```bash
# Default: skip auto-fix if total errors+warnings > 100
git commit -m "..."

# Raise the cap for one commit
LINTHIS_AGENT_MAX_AUTO_FIX=500 git commit -m "..."

# Disable the cap entirely (not recommended — commit can hang for a long time)
LINTHIS_AGENT_MAX_AUTO_FIX=0 git commit -m "..."
```

When the hook decides to invoke the agent, its output (tool calls, file edits, etc.) now streams directly to your terminal — no more silent spinner. Cancel with Ctrl-C at any time.

## Next Steps

- [Plugin System](../features/plugins.md) - Share configurations
- [CLI Reference](../reference/cli.md) - All command options
