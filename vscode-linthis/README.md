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

   # Or Using cargo
   cargo install linthis

   ```

2. Install this extension from the VS Code Marketplace

## Extension Settings

| Setting | Default | Description |
|---------|---------|-------------|
| `linthis.enable` | `true` | Enable/disable the extension |
| `linthis.lintOnSave` | `true` | Run lint on file save |
| `linthis.formatOnSave` | `false` | Format document on save |
| `linthis.executablePath` | `"linthis"` | Path to linthis executable |
| `linthis.extraArgs` | `[]` | Extra arguments for LSP server |
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

## Development

```bash
# Install dependencies
npm install

# Build
npm run build

# Watch mode
npm run watch

# Package extension
npx vsce package
```

## License

MIT
