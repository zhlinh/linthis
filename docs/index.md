# linthis

[![Crates.io](https://img.shields.io/crates/v/linthis.svg)](https://crates.io/crates/linthis)
[![PyPI](https://img.shields.io/pypi/v/linthis.svg)](https://pypi.org/project/linthis/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A fast, cross-platform multi-language linter and formatter written in Rust.

## Features

- **Single Command**: Run both linting and formatting simultaneously
- **Multi-Language Support**: Rust, Python, TypeScript, JavaScript, Go, Java, C++, Swift, Kotlin, Lua, Dart, Shell, Ruby, PHP, Scala, C# and more
- **Auto-Detection**: Automatically detect programming languages used in your project
- **Flexible Configuration**: Support for project config, global config, and CLI parameters
- **Plugin System**: Share and reuse configurations via Git repositories
- **Format Presets**: Support for popular code styles like Google, Airbnb, Standard
- **Parallel Processing**: Leverage multi-core CPU for faster file processing

## Quick Links

- [Installation](getting-started/installation.md) - Get linthis installed
- [Quick Start](getting-started/quickstart.md) - Start using linthis in minutes
- [Configuration](getting-started/configuration.md) - Configure linthis for your project
- [Languages](languages/index.md) - Supported programming languages
- [CLI Reference](reference/cli.md) - Complete command-line reference

## Example Usage

```bash
# Check and format current directory
linthis

# Check only (no formatting)
linthis --check-only

# Format only (no checking)
linthis --format-only

# Check Git staged files
linthis --staged
```

## Why linthis?

1. **Unified Interface**: One tool for all languages instead of managing multiple linters
2. **Fast**: Written in Rust with parallel processing support
3. **Easy Setup**: Works out of the box with sensible defaults
4. **Team Friendly**: Plugin system for sharing configurations across projects
