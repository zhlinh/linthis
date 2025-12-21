# linthis

[![Crates.io](https://img.shields.io/crates/v/linthis.svg)](https://crates.io/crates/linthis)
[![PyPI](https://img.shields.io/pypi/v/linthis.svg)](https://pypi.org/project/linthis/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A fast, cross-platform multi-language linter and formatter written in Rust.

## Installation

### From PyPI (Python users)

```bash
# Using pip
pip install linthis

# Using uv (recommended)
uv pip install linthis
```

### From crates.io (Rust users)

```bash
cargo install linthis
```

## Features

- Single command for both linting and formatting
- Multi-language support (Rust, Python, TypeScript, JavaScript, Go, Java, C++)
- Auto-detection of project languages
- Configurable exclusion patterns
- Hierarchical configuration (built-in, user, project, CLI)
- Format presets (Google, Airbnb, Standard)
- Parallel file processing for performance

## Usage

```bash
# Run both lint and format checks
linthis

# Lint only (no formatting)
linthis --check-only

# Format only (no linting)
linthis --format-only

# Specify files or directories
linthis src/main.rs src/lib.rs

# Check only staged files
linthis --staged
```

## Configuration

Create `.linthis.toml` in your project root:

```toml
# Languages to check (omit for auto-detection)
languages = ["rust", "python"]

# Files to exclude
exclude = [
    "target/**",
    "node_modules/**",
    "*.generated.rs"
]

# Maximum cyclomatic complexity
max_complexity = 20

# Formatting preset
format_preset = "google"
```

## Supported Languages

| Language | Linter | Formatter |
|----------|--------|-----------|
| Rust | clippy | rustfmt |
| Python | pylint, flake8 | black |
| TypeScript | eslint | prettier |
| JavaScript | eslint | prettier |
| Go | golint, go vet | gofmt |
| Java | checkstyle | google-java-format |
| C++ | cpplint, cppcheck | clang-format |

## License

MIT
