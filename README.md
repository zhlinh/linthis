# linthis

[![Crates.io](https://img.shields.io/crates/v/linthis.svg)](https://crates.io/crates/linthis)
[![PyPI](https://img.shields.io/pypi/v/linthis.svg)](https://pypi.org/project/linthis/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A fast, cross-platform multi-language linter and formatter written in Rust.

## Features

- 🚀 **Single Command**: Run both linting and formatting simultaneously
- 🌍 **Multi-Language Support**: Rust, Python, TypeScript, JavaScript, Go, Java, C++, Swift, Kotlin, Lua, and more
- 🎯 **Auto-Detection**: Automatically detect programming languages used in your project
- ⚙️ **Flexible Configuration**: Support for project config, global config, and CLI parameters
- 📦 **Plugin System**: Share and reuse configurations via Git repositories
- 🎨 **Format Presets**: Support for popular code styles like Google, Airbnb, Standard
- ⚡ **Parallel Processing**: Leverage multi-core CPU for faster file processing

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

### Initialize Configuration (Optional)

```bash
# Create project configuration file
linthis init

# Create global configuration file
linthis init -g

# Create global git hook template (for all new repos)
linthis init -g --hook-type git

# Initialize with pre-commit hooks (project-level)
linthis init --hook-type prek
linthis init --hook-type pre-commit
linthis init --hook-type git

# Force overwrite existing files
linthis init --force
linthis init --hook-type prek -f
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

### Add Plugin

```bash
# Add plugin to project config (.linthis.toml)
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
linthis -l python -i src/
linthis --check-only
linthis --staged
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

Create `.linthis.toml` in your project root:

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
2. **Project Config**: `.linthis.toml`
3. **Global Config**: `~/.linthis/config.toml`
4. **Plugin Config**: Plugins in sources array (later ones override earlier ones)
5. **Built-in Defaults**

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

### Initialize Configuration File

Use the `init` subcommand to explicitly create configuration files:

```bash
# Create project config (.linthis.toml)
linthis init

# Create global config (~/.linthis/config.toml)
linthis init -g
linthis init --global

# Backward compatible: can also use --init flag
linthis --init
```

### Auto-Create Configuration Files

When using the `config` command, configuration files are automatically created if they don't exist:

- **Project Config**: Creates `.linthis.toml` in current directory
- **Global Config**: Creates `config.toml` in `~/.linthis/` directory

All modifications preserve TOML file format and comments.

## Command Line Options

### Main Command Options

| Short | Long                    | Description                              | Example                 |
| ----- | ----------------------- | ---------------------------------------- | ----------------------- |
| `-i`  | `--include`             | Specify files or directories to check    | `-i src -i lib`         |
| `-e`  | `--exclude`             | Exclude patterns (can be used multiple times) | `-e "*.test.js"`        |
| `-c`  | `--check-only`          | Check only, no formatting                | `-c`                    |
| `-f`  | `--format-only`         | Format only, no checking                 | `-f`                    |
| `-s`  | `--staged`              | Check only Git staged files              | `-s`                    |
| `-l`  | `--lang`                | Specify languages (comma-separated)      | `-l python,rust`        |
| `-o`  | `--output`              | Output format: human, json, github-actions | `-o json`               |
| `-v`  | `--verbose`             | Verbose output                           | `-v`                    |
| `-q`  | `--quiet`               | Quiet mode (errors only)                 | `-q`                    |
|       | `--config`              | Specify config file path                 | `--config custom.toml`  |
|       | `--init`                | Initialize .linthis.toml config file     | `--init`                |
|       | `--preset`              | Format preset                            | `--preset google`       |
|       | `--no-default-excludes` | Disable default exclude rules            | `--no-default-excludes` |
|       | `--no-gitignore`        | Disable .gitignore rules                 | `--no-gitignore`        |
|       | `--no-plugin`           | Skip loading plugins, use default config | `--no-plugin`           |

### Plugin Management Subcommands

| Command                    | Short | Long        | Description               |
| -------------------------- | ----- | ----------- | ------------------------- |
| `plugin add <alias> <url>` | `-g`  | `--global`  | Add to global config      |
|                            |       | `--ref`     | Specify Git reference     |
| `plugin remove <alias>`    | `-g`  | `--global`  | Remove from global config |
| `plugin list`              | `-g`  | `--global`  | Show global config plugins|
|                            | `-v`  | `--verbose` | Show detailed info        |
| `plugin clean`             |       | `--all`     | Clean all caches          |
| `plugin init <name>`       |       |             | Initialize new plugin     |
| `plugin validate <path>`   |       |             | Validate plugin structure |

### Configuration Management Subcommands

| Command                         | Short | Long        | Description                     |
| ------------------------------- | ----- | ----------- | ------------------------------- |
| `config add <field> <value>`    | `-g`  | `--global`  | Add value to array field        |
| `config remove <field> <value>` | `-g`  | `--global`  | Remove value from array field   |
| `config clear <field>`          | `-g`  | `--global`  | Clear array field               |
| `config set <field> <value>`    | `-g`  | `--global`  | Set scalar field value          |
| `config unset <field>`          | `-g`  | `--global`  | Remove scalar field             |
| `config get <field>`            | `-g`  | `--global`  | Get field value                 |
| `config list`                   | `-g`  | `--global`  | List all configuration          |
|                                 | `-v`  | `--verbose` | Show detailed info (including empty values) |

**Supported array fields**: `includes`, `excludes`, `languages`
**Supported scalar fields**: `max_complexity`, `preset`, `verbose`

### Init Subcommand

| Command | Short | Long       | Description                        |
| ------- | ----- | ---------- | ---------------------------------- |
| `init`  | `-g`  | `--global` | Create global config file          |
|         |       | `--hook`   | Initialize pre-commit hooks        |
|         | `-i`  | `--interactive` | Interactive mode for hooks setup |
|         | `-f`  | `--force`  | Force overwrite existing files     |

**Created configuration files**:
- Without `-g`: Creates `.linthis.toml` (current directory)
- With `-g`: Creates `~/.linthis/config.toml` (global config)

**Hook options**:
- `prek`: Rust-based pre-commit tool (faster)
- `pre-commit`: Python-based standard tool
- `git`: Traditional git hook

## Supported Languages

| Language   | Linter               | Formatter          |
| ---------- | -------------------- | ------------------ |
| Rust       | clippy               | rustfmt            |
| Python     | pylint, flake8, ruff | black, ruff        |
| TypeScript | eslint               | prettier           |
| JavaScript | eslint               | prettier           |
| Go         | golangci-lint        | gofmt              |
| Java       | checkstyle           | google-java-format |
| C++        | cpplint, cppcheck    | clang-format       |
| Swift      | swiftlint            | swift-format       |
| Kotlin     | detekt               | ktlint             |
| Lua        | luacheck             | stylua             |
| Dart       | dart analyze         | dart format        |

## Usage Scenarios

### Pre-commit Hook

#### Method 1: Global Hook Template (One-time Setup)

Set up a global Git hook template that applies to all new repositories:

```bash
# Create global hook template
linthis init -g --hook-type git

# All new repos will automatically include the hook
git init new-project
cd new-project
# .git/hooks/pre-commit is already set up!
```

For existing repositories:
```bash
cd existing-project
git init  # Re-apply template
```

**Features:**
- 🎯 **Smart Detection**: Only runs if project has linthis config
- 🔗 **Hook Chaining**: Supports `.git/hooks/pre-commit.local` for project-specific hooks
- 🚫 **Zero Interference**: Projects without linthis config are not affected
- ⚡ **One-time Setup**: Works for all your new repositories

**Pros:**
- One-time setup for all your projects
- No need to configure hooks per project
- Perfect for personal development
- Won't interfere with other projects or hook tools

**Cons:**
- Not shared with team members
- Requires manual setup on each machine

See [Global Hooks Guide](docs/GLOBAL_HOOKS.md) for details.

#### Method 2: Using prek (Recommended for Teams)

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

#### Method 3: Traditional Git Hook (Project-level)

Add to `.git/hooks/pre-commit`:

```bash
#!/bin/sh
linthis --staged --check-only
```

Or use linthis to create it automatically:
```bash
linthis init --hook-type git
```

#### Method 4: Using pre-commit Framework

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
- Windows: `%LOCALAPPDATA%\linthis\plugins`

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
