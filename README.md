# linthis

[![Crates.io](https://img.shields.io/crates/v/linthis.svg)](https://crates.io/crates/linthis)
[![PyPI](https://img.shields.io/pypi/v/linthis.svg)](https://pypi.org/project/linthis/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A fast, cross-platform multi-language linter and formatter written in Rust.

## Features

- 🚀 **Single Command**: Run linting, formatting, security, and complexity checks simultaneously
- 🌍 **Multi-Language Support**: Rust, Python, TypeScript, JavaScript, Go, Java, C++, Swift, Kotlin, Lua, and more
- 🎯 **Auto-Detection**: Automatically detect programming languages used in your project
- 🔒 **Security Scanning (SAST)**: Built-in secrets detection + OpenGrep/Semgrep, Bandit, Gosec, Flawfinder integration
- 📊 **Complexity Analysis**: Cyclomatic/cognitive complexity per function with threshold enforcement
- ⚙️ **Flexible Configuration**: Support for project config, global config, and CLI parameters with dotted key paths
- 📦 **Plugin System**: Share and reuse configurations via Git repositories (HTTPS with SSH auto-fallback)
- 🎨 **Format Presets**: Support for popular code styles like Google, Airbnb, Standard
- ⚡ **Parallel Processing**: Leverage multi-core CPU for faster file processing with per-file caching
- 🤖 **AI Code Review**: `linthis review` analyzes diffs with AI and creates PR/MR automatically
- 💾 **Backup & Undo**: `linthis backup undo` / `linthis backup redo` to restore changes, with git patch files for review
- 🔄 **Fix Commit Mode**: Three modes for handling auto-fixes — `squash` (merge into commit), `dirty` (leave for review), `fixup` (separate commit)
- 🔌 **Plugin Hook Bundling**: Plugins can ship custom git and agent hook scripts — auto-installed when the plugin is added
- 🆙 **Self-Update**: `linthis update` with install method detection (cargo, pip, uv, pipx)
- 📝 **Auto .gitignore**: Generates .gitignore when missing, blocks commits with files that should be ignored
- 🗂️ **Global Data Storage**: Runtime data (results, backups, cache) stored in `~/.linthis/projects/` — keeps project directory clean

## Installation

### Method 1: Install via PyPI (Recommended for Python users)

```bash
# Using pip
pip install linthis

# Using uv (recommended)
# pip install uv
uv pip install linthis
```

### Method 2: Install via Cargo (Recommended for Rust users)

```bash
cargo install linthis
```

### Method 3: Build from Source

```bash
git clone https://github.com/zhlinh/linthis.git
cd linthis
cargo build --release
```

## Quick Start

Install linthis, add a plugin, set up hooks, and run your first check — all in under a minute.

```bash
# 1. Install
pip install linthis

# 2. Add team plugin (-g is user scope, use your team's plugin URL)
linthis plugin add -g sample https://github.com/zhlinh/linthis-plugin-template

# 3. Install hooks (-g is user scope)
linthis hook install -g                                           # git pre-commit hook
linthis hook install -g --type git-with-agent --provider claude  # git hook + AI auto-fix on failure
linthis hook install -g --type agent --provider claude            # AI agent hook (Claude, Cursor, etc.)

# 4. Run lint check
linthis -i src/

# 5. Check staged files before commit
git add src/main.py
linthis -s
```

<video src="docs/assets/videos/QuickStart-en.mp4" controls width="100%"></video>

> See more video tutorials in the [Video Tutorials](docs/getting-started/videos.md) page.

### Initialize Configuration (Optional)

```bash
# Create project configuration file
linthis init

# Create global configuration file
linthis init -g

# Project-level hooks
linthis hook install                                          # git pre-commit hook
linthis hook install --type git-with-agent --provider claude  # git hook + AI auto-fix on failure
linthis hook install --type agent --provider claude           # AI agent rules (Claude Code)
linthis hook install --type prek                              # prek pre-commit hook
linthis hook install --event pre-push                         # git pre-push hook
linthis hook install --event commit-msg                       # commit message format hook

# Global hooks (apply to all repos on this machine)
linthis hook install --global                                 # global git pre-commit
linthis hook install --global --type git-with-agent --provider claude  # global + AI auto-fix
linthis hook install --type agent --provider claude --global  # AI agent rules (user home)
linthis hook install --global --event commit-msg              # global commit message format hook

# Force overwrite existing files
linthis init --force
linthis hook install --force
```

### Basic Usage

```bash
# Check and format current directory (default behavior)
linthis

# Check and format specific directories
linthis -i src/
linthis --include src/ --include lib/

# Check only, no formatting
linthis -c
linthis --check-only

# Format only, no checking
linthis -f
linthis --format-only

# Check Git staged files (suitable for pre-commit hook)
linthis -s
linthis --staged

# Check all locally modified files (staged + unstaged)
linthis -m
linthis --modified

# Run specific checks (default: lint + security + complexity)
linthis --checks lint,security     # Only lint + security
linthis --checks all               # All checks
linthis --checks lint              # Only lint (no security/complexity)
```

### Specify Languages

```bash
# Check specific language
linthis -l python
linthis --lang rust

# Check multiple languages
linthis -l python,rust,cpp
linthis --lang "python,javascript,go"
```

### Exclude Files

```bash
# Exclude specific patterns
linthis -e "*.test.js" -e "dist/**"
linthis --exclude "target/**" --exclude "node_modules/**"
```

## Plugin System

linthis supports Git-based configuration plugins for easy sharing of code standards across projects and teams.

<video src="docs/assets/videos/PluginSystem-en.mp4" controls width="100%"></video>

### Add Plugin

```bash
# Add plugin to project config (.linthis/config.toml)
linthis plugin add <alias> <git-url>

# Example: Add a custom plugin
linthis plugin add myplugin https://github.com/zhlinh/linthis-plugin.git

# Add to global config (~/.linthis/config.toml)
linthis plugin add -g <alias> <git-url>
linthis plugin add --global <alias> <git-url>
```

### Use Plugin

Plugins are automatically loaded when running linthis. After adding a plugin:

```bash
# Plugin configs are auto-loaded
linthis

# Combine with other options
linthis -i src/
# Check only
linthis -c
# Format only
linthis -f
# Check and format files staged
linthis -s
```

### Remove Plugin

```bash
# Remove plugin from project config
linthis plugin remove <alias>
linthis plugin remove myplugin

# Remove plugin from global config
linthis plugin remove -g <alias>
linthis plugin remove --global myplugin

# Supports flexible parameter ordering
linthis plugin remove --global myplugin
```

### View and Manage Plugins

```bash
# View project config plugins
linthis plugin list

# View global config plugins
linthis plugin list -g
linthis plugin list --global

# Sync (update) plugins
linthis plugin sync          # Sync local plugins
linthis plugin sync --global # Sync global plugins

# Initialize new plugin
linthis plugin init my-config

# Validate plugin structure
linthis plugin validate /path/to/plugin

# Clean plugin cache
linthis plugin clean          # Interactive cleanup
linthis plugin clean --all    # Clean all caches
```

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
    { name = "myplugin", url = "https://github.com/zhlinh/linthis-plugin.git", ref = "main" }
]

# Checks to run (default: lint + security + complexity)
[checks]
run = ["lint", "security", "complexity"]

# Security check settings
[checks.security]
scan_type = "sast"    # sca, sast, all
fail_on = "high"      # critical, high, medium, low

# Complexity check settings
[checks.complexity]
threshold = 15
fail_on_high = true

# Language-specific configuration
# [rust]
# max_complexity = 15

# [python]
# excludes = ["*_test.py"]
```

### Global Configuration

Global configuration file is located at `~/.linthis/config.toml`, with the same format as project config.

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

## Configuration Management

linthis provides a `config` subcommand for convenient command-line configuration management without manual TOML editing.

### Array Field Operations

Supported array fields: `includes`, `excludes`, `languages`

#### Add Values (add)

```bash
# Add to project config
linthis config add includes "src/**"
linthis config add excludes "*.log"
linthis config add languages "rust"

# Add to global config (-g or --global)
linthis config add -g includes "lib/**"
linthis config add --global excludes "node_modules/**"

# Add multiple values (automatically deduped)
linthis config add includes "src/**"
linthis config add includes "lib/**"
```

#### Remove Values (remove)

```bash
# Remove from project config
linthis config remove excludes "*.log"
linthis config remove languages "python"

# Remove from global config
linthis config remove -g includes "lib/**"
linthis config remove --global excludes "target/**"
```

#### Clear Field (clear)

```bash
# Clear project config field
linthis config clear languages
linthis config clear includes

# Clear global config field
linthis config clear -g excludes
linthis config clear --global languages
```

### Scalar Field Operations

Supported scalar fields: `max_complexity`, `preset`, `verbose`

#### Set Value (set)

```bash
# Set complexity limit
linthis config set max_complexity 15
linthis config set max_complexity 30 -g

# Set format preset (google, standard, airbnb)
linthis config set preset google
linthis config set preset airbnb --global

# Set verbose output
linthis config set verbose true
linthis config set verbose false -g
```

#### Unset Value (unset)

```bash
# Remove field from project config
linthis config unset max_complexity
linthis config unset preset

# Remove field from global config
linthis config unset -g verbose
linthis config unset --global max_complexity
```

### Query Operations

#### Get Single Field Value (get)

```bash
# View project config field
linthis config get includes
linthis config get max_complexity
linthis config get preset

# View global config field
linthis config get -g excludes
linthis config get --global languages
```

#### List All Configuration (list)

```bash
# List project config
linthis config list

# List global config
linthis config list -g
linthis config list --global

# Verbose mode (show all fields including empty values)
linthis config list -v
linthis config list --verbose
linthis config list --global --verbose
```

### Configuration Management Examples

```bash
# Initialize project config
linthis config add includes "src/**"
linthis config add includes "lib/**"
linthis config add excludes "target/**"
linthis config add excludes "*.log"
linthis config add languages "rust"
linthis config add languages "python"
linthis config set max_complexity 20
linthis config set preset google

# View config
linthis config list

# Adjust config
linthis config set max_complexity 15
linthis config remove excludes "*.log"
linthis config add excludes "*.tmp"

# Set global defaults
linthis config set -g max_complexity 20
linthis config add -g excludes "node_modules/**"
linthis config add -g excludes ".git/**"
```

### Configuration Migration

linthis can automatically detect and migrate existing linter/formatter configurations to linthis format.

#### Supported Tools

| Tool     | Detected Files                                                                                                         |
| -------- | ---------------------------------------------------------------------------------------------------------------------- |
| ESLint   | `.eslintrc.js`, `.eslintrc.json`, `.eslintrc.yml`, `.eslintrc`, `eslint.config.js`, `package.json[eslintConfig]`       |
| Prettier | `.prettierrc`, `.prettierrc.json`, `.prettierrc.yml`, `.prettierrc.js`, `prettier.config.js`, `package.json[prettier]` |
| Black    | `pyproject.toml[tool.black]`                                                                                           |
| isort    | `pyproject.toml[tool.isort]`                                                                                           |

#### Migration Commands

```bash
# Auto-detect and migrate all configs
linthis config migrate

# Migrate specific tool only
linthis config migrate --from eslint
linthis config migrate --from prettier
linthis config migrate --from black
linthis config migrate --from isort

# Preview changes without applying
linthis config migrate --dry-run

# Create backup of original files
linthis config migrate --backup

# Verbose output
linthis config migrate --verbose
```

#### Migration Output

Migrated configurations are placed in `.linthis/configs/{language}/`:

- ESLint → `.linthis/configs/javascript/.eslintrc.js`
- Prettier → `.linthis/configs/javascript/prettierrc.js`
- Black/isort → `.linthis/configs/python/ruff.toml`

### Initialize Configuration File

Use the `init` subcommand to explicitly create configuration files:

```bash
# Create project config (.linthis/config.toml)
linthis init

# Create global config (~/.linthis/config.toml)
linthis init -g
linthis init --global

# Backward compatible: can also use --init flag
linthis --init
```

### Auto-Create Configuration Files

When using the `config` command, configuration files are automatically created if they don't exist:

- **Project Config**: Creates `.linthis/config.toml` in current directory
- **Global Config**: Creates `config.toml` in `~/.linthis/` directory

All modifications preserve TOML file format and comments.

## Command Line Options

### Main Command Options

| Short | Long                    | Description                                          | Example                 |
| ----- | ----------------------- | ---------------------------------------------------- | ----------------------- |
| `-i`  | `--include`             | Specify files or directories to check                | `-i src -i lib`         |
| `-e`  | `--exclude`             | Exclude patterns (can be used multiple times)        | `-e "*.test.js"`        |
| `-c`  | `--check-only`          | Check only, no formatting                            | `-c`                    |
| `-f`  | `--format-only`         | Format only, no checking                             | `-f`                    |
| `-s`  | `--staged`              | Check only Git staged files                          | `-s`                    |
| `-m`  | `--modified`            | Check all locally modified files (staged + unstaged) | `-m`                    |
| `-l`  | `--lang`                | Specify languages (comma-separated)                  | `-l python,rust`        |
| `-o`  | `--output`              | Output format: human, json, github-actions           | `-o json`               |
| `-v`  | `--verbose`             | Verbose output                                       | `-v`                    |
| `-q`  | `--quiet`               | Quiet mode (errors only)                             | `-q`                    |
|       | `--config`              | Specify config file path                             | `--config custom.toml`  |
|       | `--init`                | Initialize .linthis/config.toml config file          | `--init`                |
|       | `--preset`              | Format preset                                        | `--preset google`       |
|       | `--no-default-excludes` | Disable default exclude rules                        | `--no-default-excludes` |
|       | `--no-gitignore`        | Disable .gitignore rules                             | `--no-gitignore`        |
|       | `--no-plugin`           | Skip loading plugins, use default config             | `--no-plugin`           |

### Plugin Management Subcommands

| Command                    | Short | Long        | Description                |
| -------------------------- | ----- | ----------- | -------------------------- |
| `plugin add <alias> <url>` | `-g`  | `--global`  | Add to global config       |
|                            |       | `--ref`     | Specify Git reference      |
| `plugin remove <alias>`    | `-g`  | `--global`  | Remove from global config  |
| `plugin list`              | `-g`  | `--global`  | Show global config plugins |
|                            | `-v`  | `--verbose` | Show detailed info         |
| `plugin clean`             |       | `--all`     | Clean all caches           |
| `plugin init <name>`       |       |             | Initialize new plugin      |
| `plugin validate <path>`   |       |             | Validate plugin structure  |

### Configuration Management Subcommands

| Command                         | Short | Long        | Description                                 |
| ------------------------------- | ----- | ----------- | ------------------------------------------- |
| `config add <field> <value>`    | `-g`  | `--global`  | Add value to array field                    |
| `config remove <field> <value>` | `-g`  | `--global`  | Remove value from array field               |
| `config clear <field>`          | `-g`  | `--global`  | Clear array field                           |
| `config set <field> <value>`    | `-g`  | `--global`  | Set scalar field value                      |
| `config unset <field>`          | `-g`  | `--global`  | Remove scalar field                         |
| `config get <field>`            | `-g`  | `--global`  | Get field value                             |
| `config list`                   | `-g`  | `--global`  | List all configuration                      |
|                                 | `-v`  | `--verbose` | Show detailed info (including empty values) |
| `config migrate`                |       | `--from`    | Migrate from specific tool                  |
|                                 |       | `--dry-run` | Preview changes without applying            |
|                                 |       | `--backup`  | Create backup of original files             |
|                                 | `-v`  | `--verbose` | Show detailed output                        |

**Supported array fields**: `includes`, `excludes`, `languages`
**Supported scalar fields**: `max_complexity`, `preset`, `verbose`

### Init Subcommand

| Command | Short | Long          | Description                      |
| ------- | ----- | ------------- | -------------------------------- |
| `init`  | `-g`  | `--global`    | Create global config file        |
|         |       | `--with-hook` | Also install git hook after init |
|         |       | `--force`     | Force overwrite existing files   |

**Created configuration files**:

- Without `-g`: Creates `.linthis/config.toml` (current directory)
- With `-g`: Creates `~/.linthis/config.toml` (global config)

### Hook Subcommand

<video src="docs/assets/videos/GitHooks-en.mp4" controls width="100%"></video>

| Command          | Short | Long            | Description                                                                                                                                             |
| ---------------- | ----- | --------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `hook install`   |       | `--type`        | Hook type (git/git-with-agent/agent/prek/prek-with-agent)                                                                                               |
|                  |       | `--event`       | Hook event (pre-commit/pre-push/commit-msg)                                                                                                             |
|                  | `-g`  | `--global`      | Install globally: agent type → user home dir; others → `~/.config/git/hooks/` + `core.hooksPath`                                                        |
|                  |       | `--provider`    | AI provider: `claude`/`codex`/`gemini`/`cursor`/`droid`/`auggie`/`codebuddy`. Supports `provider/model` shorthand (e.g. `claude/opus`). For `--type agent`: installs rules files. For `*-with-agent`: uses headless CLI to auto-fix. |
|                  |       | `--provider-args` | Extra arguments passed to the AI agent CLI (e.g. `"--model opus"`)                                                                                    |
|                  | `-c`  | `--check-only`  | Hook only runs check                                                                                                                                    |
|                  | `-f`  | `--format-only` | Hook only runs format                                                                                                                                   |
|                  |       | `--force`       | Force overwrite existing hook                                                                                                                           |
|                  | `-y`  | `--yes`         | Non-interactive mode                                                                                                                                    |
| `hook uninstall` |       | `--event`       | Hook event to uninstall                                                                                                                                 |
|                  | `-g`  | `--global`      | Uninstall global hook                                                                                                                                   |
|                  |       | `--all`         | Uninstall all hooks                                                                                                                                     |
|                  | `-y`  | `--yes`         | Non-interactive mode                                                                                                                                    |
| `hook status`    |       |                 | Show git hook status (Project Hooks and Global Hooks sections)                                                                                          |
| `hook check`     |       |                 | Check for hook conflicts                                                                                                                                |
| `hook sync`      |       |                 | Re-sync all installed hooks and agent skills                                                                                                            |
|                  | `-g`  | `--global`      | Sync global hooks                                                                                                                                       |

**Hook types**:

- `git`: Traditional git hook (default)
- `git-with-agent`: git hook + AI agent auto-fix on failure
- `agent`: AI agent hook (Claude, Cursor, Windsurf, etc.)
- `prek`: Rust-based pre-commit tool (faster)
- `prek-with-agent`: prek hook + AI agent auto-fix on failure

**Global hooks**: Use `-g` / `--global` with any hook type. For `agent` type, installs rules to the user home directory. For all other types, installs to `~/.config/git/hooks/` and sets `git config --global core.hooksPath`. Local hooks take priority over global hooks.

<video src="docs/assets/videos/AgentHook-en.mp4" controls width="100%"></video>

**Hook events**:

- `pre-commit`: Run before commit (default, checks staged files)
- `pre-push`: Run before push (checks all files)
- `commit-msg`: Validate commit message format (calls `linthis cmsg "$1"`)

Each hook event generates a separate per-event skill file for agent integrations (e.g., `lt-lint` for pre-commit, `lt-cmsg` for commit-msg, `lt-review` for pre-push). All skills belong to a single unified plugin bundle `lt`.

### cmsg Subcommand

Validate commit message format directly — without going through a hook.

| Command | Description |
| ------- | ----------- |
| `cmsg <msg-or-file>` | Validate a commit message string or file path |
| `cmsg <file> --auto-fix` | AI rewrite on failure (writes result back to file) |

```bash
# Validate a message string directly
linthis cmsg "feat: add new feature"
linthis cmsg "fix(api): handle null response"

# Validate from a file (git hook usage)
linthis cmsg .git/COMMIT_EDITMSG

# AI rewrite on failure (writes result back to file)
linthis cmsg .git/COMMIT_EDITMSG --auto-fix
linthis cmsg .git/COMMIT_EDITMSG --auto-fix --provider claude-cli

# Install the commit-msg hook (calls `linthis cmsg "$1"` automatically)
linthis hook install --event commit-msg
```

The default format follows [Conventional Commits](https://www.conventionalcommits.org/):
`type(scope)?: description` — where `type` is one of `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `build`, `ci`, `chore`, `revert`.

The pattern is configurable via `.linthis/config.toml`:

```toml
[cmsg]
commit_msg_pattern = "^(feat|fix|docs|...)\\(\\S+\\)?: .{1,72}"
```

### format Subcommand

Format files with automatic backup and undo support.

```bash
linthis format                        # Format all files (with backup)
linthis format -s                     # Format staged files
linthis format -m                     # Format modified files
linthis format -i src/main.rs         # Format specific file
linthis format --undo                 # Undo last format (restore from backup)
linthis format --list-backups         # List available backups
```

A backup is created before each format operation. Use `--undo` to revert.

> **Note:** When running `linthis -s` (staged mode), formatted files are **automatically re-staged** — no manual `git add` needed.

### review Subcommand

AI-powered code review with PR/MR creation.

```bash
linthis review                        # Review current branch vs remote
linthis review --auto-fix             # Review + auto-fix + create PR
linthis review -r alice -r bob        # Specify reviewers
linthis review --base main            # Diff against main branch
linthis review --background           # Run in background (non-blocking)
linthis review --status               # Check background review status
linthis review --no-pr                # Generate Markdown report only
```

Supported platforms: **GitHub** (`gh`), **GitLab** (`glab`).

Configure in `.linthis/config.toml`:

```toml
[review]
enabled = true
provider = "claude-cli"

[review.reviewers]
default = ["alice", "bob"]
```

## Supported Languages

| Language    | Linter                        | Formatter          |
| ----------- | ----------------------------- | ------------------ |
| Rust        | clippy                        | rustfmt            |
| Python      | ruff, pylint, flake8          | ruff, black        |
| TypeScript  | eslint                        | prettier           |
| JavaScript  | eslint                        | prettier           |
| Go          | golangci-lint                 | gofmt              |
| Java        | checkstyle                    | google-java-format |
| C           | clang-tidy, cppcheck          | clang-format       |
| C++         | clang-tidy, cpplint, cppcheck | clang-format       |
| Objective-C | clang-tidy                    | clang-format       |
| Swift       | swiftlint                     | swift-format       |
| Kotlin      | ktlint, detekt                | ktlint             |
| Lua         | luacheck                      | stylua             |
| Dart        | dart analyze                  | dart format        |
| Shell/Bash  | shellcheck                    | shfmt              |
| Ruby        | rubocop                       | rubocop            |
| PHP         | phpcs                         | php-cs-fixer       |
| Scala       | scalafix                      | scalafmt           |
| C#          | dotnet format                 | dotnet format      |

## Editor Plugins

linthis provides official plugins for popular editors, offering seamless integration with format-on-save, manual lint/format commands, and configurable settings.

<video src="docs/assets/videos/EditorSkills-en.mp4" controls width="100%"></video>

### VSCode

Install from [VS Marketplace](https://marketplace.visualstudio.com/items?itemName=zhlinh.linthis) or search "linthis" in VSCode Extensions.

**Features:**

- Format on Save (configurable)
- Manual Lint/Format commands via Command Palette
- Configurable executable path and additional arguments
- Status bar integration

**Installation via Command Palette:**

```
ext install zhlinh.linthis
```

**Configuration (settings.json):**

```json
{
  "linthis.formatOnSave": true,
  "linthis.executable.path": "",
  "linthis.executable.additionalArguments": ""
}
```

📁 Source: [vscode-linthis](./vscode-linthis/)

### JetBrains (IntelliJ IDEA, WebStorm, PyCharm, etc.)

Install from [JetBrains Marketplace](https://plugins.jetbrains.com/plugin/XXXXX-linthis) or search "linthis" in IDE Settings → Plugins.

**Features:**

- Format on Save (configurable)
- Manual Lint/Format via Tools menu
- Configurable executable path and additional arguments
- Settings UI in Preferences → Tools → Linthis

**Installation:**

1. Open Settings/Preferences → Plugins
2. Search for "linthis"
3. Click Install and restart IDE

**Configuration:**

- Settings → Tools → Linthis
- Enable/disable Format on Save
- Set custom executable path
- Add additional command-line arguments

📁 Source: [jetbrains-linthis](./jetbrains-linthis/)

### Neovim

Install using your favorite plugin manager. Distributed via GitHub.

#### lazy.nvim (Recommended)

```lua
-- For monorepo (plugin in subdirectory)
{
  "zhlinh/linthis",
  subdir = "nvim-linthis",
  config = function()
    require("linthis").setup()
  end,
}

-- For standalone repository
{
  "zhlinh/nvim-linthis",
  config = function()
    require("linthis").setup()
  end,
}
```

#### packer.nvim

```lua
-- For monorepo
use {
  "zhlinh/linthis",
  rtp = "nvim-linthis",
  config = function()
    require("linthis").setup()
  end,
}
```

#### vim-plug

```vim
" For monorepo
Plug 'zhlinh/linthis', { 'rtp': 'nvim-linthis' }
```

**Features:**

- Format on Save (configurable)
- Commands: `:LinthisLint`, `:LinthisFormat`, `:LinthisLintFormat`
- Configurable via `setup()` options

**Configuration:**

```lua
require("linthis").setup({
  format_on_save = true,
  executable = "linthis",
  additional_args = {},
})
```

📁 Source: [nvim-linthis](./nvim-linthis/)

## Usage Scenarios

### Pre-commit Hook

#### Method 1: Using prek (Recommended for Teams)

[prek](https://prek.j178.dev) is a high-performance Git hooks manager written in Rust, fully compatible with pre-commit config format but much faster.

Install prek:

```bash
# Using cargo
cargo install prek

# Or using pip
pip install prek
```

Create `.pre-commit-config.yaml` in your project:

```yaml
# .pre-commit-config.yaml
repos:
  - repo: local
    hooks:
      - id: linthis
        name: linthis
        entry: linthis --staged --check-only
        language: system
        pass_filenames: false
```

Install hook:

```bash
prek install
```

#### Method 2: Traditional Git Hook (Project-level)

Add to `.git/hooks/pre-commit`:

```bash
#!/bin/sh
linthis --staged --check-only
```

Or use linthis to create it automatically:

```bash
linthis hook install --type git
```

#### Method 3: Using pre-commit Framework

Using the [pre-commit](https://pre-commit.com/) framework:

```yaml
# .pre-commit-config.yaml
repos:
  - repo: local
    hooks:
      - id: linthis
        name: linthis
        entry: linthis --staged --check-only
        language: system
        pass_filenames: false
```

### CI/CD Integration

#### GitHub Actions

```yaml
name: Lint

on: [push, pull_request]

jobs:
  lint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Install linthis
        run: pip install linthis
      - name: Run linthis
        run: linthis --check-only --output github-actions
```

#### GitLab CI

```yaml
lint:
  image: rust:latest
  script:
    - cargo install linthis
    - linthis --check-only
```

## Creating Custom Plugins

### 1. Initialize Plugin

```bash
linthis plugin init my-company-standards
cd my-company-standards
```

### 2. Edit Plugin Configuration

Edit `linthis-plugin.toml`:

```toml
[plugin]
name = "my-company-standards"
version = "1.0.0"
description = "My company's coding standards"

["language.python"]
config_count = 2

["language.python".tools.flake8]
priority = "P0"
files = [".flake8"]

["language.python".tools.black]
priority = "P1"
files = ["pyproject.toml"]
```

### 3. Add Configuration Files

```bash
mkdir -p python
# Add your config files to corresponding language directories
cp /path/to/.flake8 python/
cp /path/to/pyproject.toml python/
```

### 3b. (Optional) Bundle Hook Overrides

Create `linthis-hook.toml` in the plugin root to ship custom git/agent hooks. Use `plugin = "self"` — it is replaced with the user's alias when they add the plugin.

```toml
# linthis-hook.toml — bundled hook overrides
[hook.git]
pre-commit = { source = { plugin = "self", file = "hooks/git/pre-commit" } }

[hook.agent.plugins._default]
"lt" = { source = { plugin = "self", file = "hooks/agent/plugins/lt" } }

[hook.agent.stop]
"claude.settings" = { source = { plugin = "self", file = "hooks/agent/hook/stop/claude/settings.json" } }
```

Place the referenced files in the plugin repo. When users run `linthis plugin add company <url>`, these entries are automatically merged into their `.linthis/config.toml`.

### 4. Publish to Git

```bash
git init
git add .
git commit -m "feat: Initial commit of my company coding standards"
git remote add origin git@github.com:mycompany/linthis-standards.git
git push -u origin main
```

### 5. Use Your Plugin

```bash
linthis plugin add company https://github.com/mycompany/linthis-standards.git
linthis  # Plugin configs are auto-loaded
```

## FAQ

### Q: How to specify multiple paths?

```bash
linthis -i src -i lib -i tests
```

### Q: How to check only specific file types?

```bash
linthis -l python  # Only check Python files
```

### Q: Where is the plugin cache?

- macOS: `~/Library/Caches/linthis/plugins`
- Linux: `~/.cache/linthis/plugins`
- Windows: `%LOCALAPPDATA%\linthis\cache\plugins`

### Q: How to update plugins?

```bash
linthis plugin sync          # Sync local plugins
linthis plugin sync --global # Sync global plugins
```

### Q: What is the plugin Git reference (ref) used for?

The ref can specify:

- Branch name: `--ref main`
- Tag: `--ref v1.0.0`
- Commit hash: `--ref abc1234`

This allows you to lock plugin versions or use development versions.

## Documentation

- [Plugin Auto-Sync](docs/AUTO_SYNC.md) - Automatic plugin synchronization (inspired by oh-my-zsh)
- [Self Auto-Update](docs/SELF_UPDATE.md) - Automatic self-update functionality

## Development

### Build

```bash
cargo build
```

### Test

```bash
cargo test
```

### Release

```bash
cargo build --release
```

## Contributing

Issues and Pull Requests are welcome!

## License

MIT License - See [LICENSE](LICENSE) file for details
