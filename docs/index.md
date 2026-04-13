# linthis

[![Crates.io](https://img.shields.io/crates/v/linthis.svg)](https://crates.io/crates/linthis)
[![PyPI](https://img.shields.io/pypi/v/linthis.svg)](https://pypi.org/project/linthis/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A fast, cross-platform multi-language linter and formatter written in Rust.

## Features

- **Single Command**: Run linting, formatting, security, and complexity checks simultaneously
- **Multi-Language Support**: Rust, Python, TypeScript, JavaScript, Go, Java, C++, Swift, Kotlin, Lua, Dart, Shell, Ruby, PHP, Scala, C# and more
- **Auto-Detection**: Automatically detect programming languages used in your project
- **Security Scanning (SAST)**: Built-in secrets detection + OpenGrep/Semgrep, Bandit, Gosec, Flawfinder
- **Complexity Analysis**: Cyclomatic/cognitive complexity per function with threshold enforcement
- **Flexible Configuration**: Support for project config, global config, and CLI parameters with dotted key paths
- **Plugin System**: Share and reuse configurations via Git repositories (HTTPS with SSH auto-fallback)
- **Format Presets**: Support for popular code styles like Google, Airbnb, Standard
- **Parallel Processing**: Leverage multi-core CPU for faster file processing with per-file caching
- **Fix Commit Mode**: Three modes for auto-fix handling — squash / dirty / fixup
- **Self-Update**: `linthis update` with install method detection (cargo, pip, uv, pipx)
- **Global Data Storage**: Runtime data stored in `~/.linthis/projects/` — keeps project directory clean

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
linthis -c
linthis --check-only

# Format only (no checking)
linthis -f
linthis --format-only

# Check Git staged files
linthis -s
linthis --staged
```

## Video Tutorials

See linthis in action with short video demos:

1. [Quick Start](getting-started/videos.md#episode-1-quick-start) — Install and run your first check (20s)
2. [Multi-Language](getting-started/videos.md#episode-2-multi-language-support) — Auto-detect 18+ languages (15s)
3. [Plugin System](getting-started/videos.md#episode-3-plugin-system) — Share configs across teams (20s)
4. [AI Fix](getting-started/videos.md#episode-4-ai-powered-fix) — AI-powered code fixes (20s)
5. [Git Hooks](getting-started/videos.md#episode-5-git-hooks) — Automatic pre-commit linting (15s)
6. [Editor Integration](getting-started/videos.md#episode-6-editor-integration) — VS Code, JetBrains, Neovim, Claude Code (15s)
7. [AI Agent Hook](getting-started/videos.md#episode-7-ai-agent-hook) — AI agent integration (20s)

## Why linthis?

1. **Unified Interface**: One tool for all languages instead of managing multiple linters
2. **Fast**: Written in Rust with parallel processing support
3. **Easy Setup**: Works out of the box with sensible defaults
4. **Team Friendly**: Plugin system for sharing configurations across projects
