# linthis Documentation

Welcome to the linthis documentation. This guide covers installation, configuration, and usage of linthis - a fast, cross-platform multi-language linting aggregator.

## Quick Links

- [Main README](../README.md) - Installation and quick start
- [Configuration Reference](reference/configuration.md) - Complete config options
- [Plugin Development Guide](plugins/creating-plugins.md) - Create custom plugins

## Getting Started

### Installation

```bash
# Using cargo
cargo install linthis

# Using pip (Python)
pip install linthis
```

### Basic Usage

```bash
# Run linting and formatting on current directory
linthis

# Check only (no formatting)
linthis -c

# Format only
linthis -f

# Check specific files
linthis -c src/main.rs lib/utils.py

# Output as JSON
linthis --output json
```

### Initialize Configuration

```bash
# Create .linthis/config.toml with defaults
linthis init
```

## Documentation Index

### Reference

- [Configuration Reference](reference/configuration.md) - All configuration options

### Guides

#### Language Guides

- [Rust](guides/languages/rust.md)
- [Python](guides/languages/python.md)
- [C++](guides/languages/cpp.md)
- [TypeScript](guides/languages/typescript.md)
- [JavaScript](guides/languages/javascript.md)
- [Go](guides/languages/go.md)
- [Java](guides/languages/java.md)
- [Objective-C](guides/languages/objc.md)
- [Swift](guides/languages/swift.md)
- [Kotlin](guides/languages/kotlin.md)
- [Lua](guides/languages/lua.md)
- [Dart](guides/languages/dart.md)

### Plugins

- [Creating Plugins](plugins/creating-plugins.md)

### Features

- [Auto Sync](AUTO_SYNC.md) - Automatic plugin synchronization
- [Self Update](SELF_UPDATE.md) - Automatic linthis updates
- [Global Hooks](GLOBAL_HOOKS.md) - Git hooks configuration

## API Documentation

For Rust library API documentation, run:

```bash
cargo doc --open
```

Or view online at [docs.rs/linthis](https://docs.rs/linthis).

## Supported Languages

| Language | Linter | Formatter |
|----------|--------|-----------|
| Rust | clippy | rustfmt |
| Python | ruff | ruff |
| TypeScript | eslint | prettier |
| JavaScript | eslint | prettier |
| Go | golangci-lint | gofmt |
| Java | checkstyle | google-java-format |
| C++ | cpplint, clang-tidy | clang-format |
| Objective-C | cpplint, clang-tidy | clang-format |
| Swift | swiftlint | swift-format |
| Kotlin | ktlint, detekt | ktlint |
| Lua | luacheck | stylua |
| Dart | dart analyze | dart format |

## Contributing

See the [main README](../README.md) for contribution guidelines.

## License

MIT License - see [LICENSE](../LICENSE) for details.
