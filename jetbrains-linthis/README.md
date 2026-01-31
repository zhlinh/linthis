# Linthis for JetBrains IDEs

Multi-language linter and formatter plugin for JetBrains IDEs (IntelliJ IDEA, PyCharm, WebStorm, GoLand, CLion, etc.).

## Features

- **Real-time Diagnostics**: See linting issues as you type
- **Format on Save**: Automatically format code when saving files
- **Multi-language Support**: 18+ programming languages supported
- **Quick Fixes**: Apply suggestions from linters
- **Configurable**: Customize via `.linthis.toml`

## Supported Languages

| Language              | Linter        | Formatter          |
| --------------------- | ------------- | ------------------ |
| Rust                  | clippy        | rustfmt            |
| Python                | ruff          | ruff/black         |
| TypeScript/JavaScript | ESLint        | Prettier           |
| Go                    | golangci-lint | gofmt              |
| Java                  | checkstyle    | google-java-format |
| C/C++                 | clang-tidy    | clang-format       |
| Objective-C           | clang-tidy    | clang-format       |
| Swift                 | SwiftLint     | swift-format       |
| Kotlin                | detekt        | ktlint             |
| Lua                   | luacheck      | stylua             |
| Dart                  | dart analyze  | dart format        |
| Shell/Bash            | shellcheck    | shfmt              |
| Ruby                  | rubocop       | rubocop            |
| PHP                   | phpcs         | php-cs-fixer       |
| Scala                 | scalafix      | scalafmt           |
| C#                    | dotnet format | dotnet format      |
| And more...           |               |                    |

## Requirements

### 1. Install Linthis CLI

```bash
# Using pip
pip install linthis
# Or use uv (recommended)
uv pip install 

# Or using cargo
cargo install linthis
```

### 2. Install LSP4IJ Plugin

This plugin requires the [LSP4IJ](https://plugins.jetbrains.com/plugin/23257-lsp4ij) plugin for Language Server Protocol support.

Install from JetBrains Marketplace:

1. Open Settings/Preferences → Plugins
2. Search for "LSP4IJ"
3. Click Install

### 3. Install Linthis Plugin

#### From JetBrains Marketplace (Coming Soon)

1. Open Settings/Preferences → Plugins
2. Search for "Linthis"
3. Click Install

#### From Local Build

1. Build the plugin: `./gradlew buildPlugin`
2. Install from disk: Settings → Plugins → ⚙️ → Install Plugin from Disk
3. Select `build/distributions/jetbrains-linthis-*.zip`

## Configuration

Create a `.linthis.toml` file in your project root:

```toml
[global]
# Enabled languages
enabled_languages = ["rust", "python", "typescript"]

# Format on save
format_on_save = true

# Cache settings
cache_enabled = true

[python]
# Python-specific settings
lint_tool = "ruff"
format_tool = "ruff"

[rust]
# Rust-specific settings
lint_tool = "clippy"
format_tool = "rustfmt"
```

## Usage

### Quick Start

1. **Install prerequisites** (LSP4IJ plugin + Linthis CLI)
2. **Install Linthis plugin**
3. **Open a supported file** - The LSP server starts automatically
4. **View diagnostics** - Issues appear in editor gutter and Problems view

### Automatic Features

Once installed, the plugin will automatically:

- **Start the LSP server** when you open a supported file
- **Show diagnostics** in the editor gutter and Problems view
- **Format on save** (if enabled in settings)
- **Lint on file open** (if enabled in settings)

### Menu Actions

Access linting and formatting actions from **Tools → Linthis**:

![Tools Menu](docs/images/tools-menu.png)

| Action                  | Description                                           |
| ----------------------- | ----------------------------------------------------- |
| **Lint Current File**   | Manually trigger linting on the active file           |
| **Format Current File** | Format the active file using the configured formatter |

Additional menu actions:

- **Code → Reformat Code**: Also triggers LSP-based formatting
- **View → Tool Windows → Problems**: Show all diagnostics

### Plugin Settings

Configure Linthis in **Settings/Preferences → Tools → Linthis**:

![Settings](docs/images/settings.png)

| Setting              | Default | Description                       |
| -------------------- | ------- | --------------------------------- |
| Lint on file open    | On      | Run linter when opening a file    |
| Lint on save         | On      | Run linter when saving a file     |
| Format on save       | Off     | Auto-format file when saving      |
| Linthis path         | (auto)  | Custom path to linthis executable |
| Additional arguments | (empty) | Extra arguments passed to linthis |

### Keyboard Shortcuts

To configure shortcuts:

1. Open **Settings/Preferences → Keymap**
2. Search for "Linthis"
3. Right-click on an action and select "Add Keyboard Shortcut"

Suggested shortcuts (not set by default to avoid conflicts):

- Lint Current File: `Ctrl+Shift+;` / `Cmd+Shift+;`
- Format Current File: `Ctrl+Shift+'` / `Cmd+Shift+'`

### Viewing Diagnostics

Linting issues are displayed in multiple places:

1. **Editor Gutter** - Icons indicate warnings/errors on specific lines
2. **Problems Tool Window** - View → Tool Windows → Problems
3. **Inline Highlights** - Underlines in the code editor
4. **Hover Tooltips** - Hover over highlighted code to see details

### LSP Server Status

Check the Language Server status at **View → Tool Windows → Language Servers**. You can:

- See if Linthis server is running
- View server logs
- Restart the server if needed

## Building from Source

```bash
# Clone the repository
git clone https://github.com/zhlinh/linthis.git
cd linthis/jetbrains-linthis

# Build the plugin
./gradlew buildPlugin

# Run in a sandbox IDE for testing
./gradlew runIde
```

## Troubleshooting

### Linthis not found

If you see "Linthis not found" notification:

1. Verify linthis is installed: `linthis --version`
2. Ensure linthis is in your PATH
3. Restart the IDE

### No diagnostics showing

1. Check if the file type is supported
2. Verify `.linthis.toml` configuration
3. View → Tool Windows → Language Servers to check server status

### LSP server not starting

1. Open Help → Diagnostic Tools → Debug Log Settings
2. Add `#com.linthis` to enable debug logging
3. Check idea.log for errors

## License

MIT License - see [LICENSE](../LICENSE) for details.

## Contributing

Contributions are welcome! Please see the [main repository](https://github.com/zhlinh/linthis) for guidelines.
