# Linthis for VS Code

Multi-language linter and formatter extension for Visual Studio Code.

## Features

- **Multi-language Support**: Lint and format 18+ programming languages with a single extension
- **Real-time Diagnostics**: See lint issues as you type
- **On-save Linting**: Automatically lint files when saved
- **LSP-based**: Powered by the linthis language server

## Supported Languages

| Language | Linter | Formatter |
|----------|--------|-----------|
| Rust | clippy | rustfmt |
| Python | ruff | ruff/black |
| TypeScript | ESLint | Prettier |
| JavaScript | ESLint | Prettier |
| Go | golangci-lint | gofmt |
| Java | checkstyle | google-java-format |
| C++ | clang-tidy | clang-format |
| Swift | SwiftLint | swift-format |
| Kotlin | Detekt | ktlint |
| Objective-C | clang-tidy | clang-format |
| Lua | luacheck | stylua |
| Dart | dart analyze | dart format |
| Shell | shellcheck | shfmt |
| Ruby | rubocop | rubocop |
| PHP | phpcs | php-cs-fixer |
| Scala | scalafix | scalafmt |
| C# | dotnet format | dotnet format |

## Requirements

- [linthis](https://github.com/zhlinh/linthis) CLI must be installed and available in PATH
- Respective language tools (linters/formatters) should be installed

## Installation

1. Install linthis CLI:
   ```bash
   # using pip
   pip install linthis
   # Or use uv (Recommand)
   # pip install uv
   uv pip install linthis

   # Or Using cargo
   cargo install linthis

   ```

2. Install this extension from the VS Code Marketplace

## Extension Settings

| Setting | Default | Description |
|---------|---------|-------------|
| `linthis.enable` | `true` | Enable/disable the extension |
| `linthis.lintOnSave` | `true` | Run lint on file save |
| `linthis.lintOnOpen` | `true` | Run lint when opening a file |
| `linthis.formatOnSave` | `false` | Format document on save |
| `linthis.executable.path` | `""` | Path to linthis executable (auto-detect if empty) |
| `linthis.executable.additionalArguments` | `""` | Additional arguments for linthis CLI |
| `linthis.usePlugin` | `""` | Plugin(s) to use directly (see [Plugin Configuration](#plugin-configuration)) |
| `linthis.trace.server` | `"off"` | Trace LSP communication |

## Commands

| Command | Description |
|---------|-------------|
| `Linthis: Run Lint` | Manually trigger linting |
| `Linthis: Format Document` | Format the current document |
| `Linthis: Restart Language Server` | Restart the LSP server |

## Configuration

Create a `.linthis.toml` file in your project root:

```toml
[languages.python]
linter = "ruff"
formatter = "ruff"

[languages.typescript]
linter = "eslint"
formatter = "prettier"
```

## Plugin Configuration

Plugins provide pre-configured lint rules for teams or organizations. You can specify plugins in two ways:

### 1. Via Settings (Recommended for team-wide configuration)

Add the `linthis.usePlugin` setting in your VS Code settings:

```json
{
  "linthis.usePlugin": "https://github.com/org/linthis-plugin.git"
}
```

**Supported formats:**

| Format | Example |
|--------|---------|
| Git URL | `https://github.com/org/plugin.git` |
| Git URL with version | `https://github.com/org/plugin.git@v1.0` |
| Local path | `./my-plugin` or `/absolute/path/to/plugin` |
| Multiple plugins | `plugin1,plugin2` (comma-separated) |

### 2. Via Config File

Add the plugin to your `.linthis.toml`:

```toml
[plugin]
use = "https://github.com/org/linthis-plugin.git@v1.0"
```

### Plugin Priority

When both settings and config file specify plugins:
- The `linthis.usePlugin` setting takes precedence
- This allows workspace-level overrides without modifying the config file

## License

MIT
