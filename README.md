# linthis

A fast, cross-platform multi-language linter and formatter written in Rust.

## Features

- Multi-language support: C++, Objective-C, Java, Python, Rust, Go, JavaScript, TypeScript
- Fast parallel processing
- Git integration (check staged files)
- Configurable via YAML/TOML
- Compatible with CodeCC configuration format

## Supported Tools

| Language | Linter | Formatter |
|----------|--------|-----------|
| Python | [ruff](https://github.com/astral-sh/ruff) | [ruff](https://github.com/astral-sh/ruff) |
| Rust | clippy | rustfmt |
| TypeScript/JavaScript | eslint | prettier |
| Go | go vet | gofmt |
| C/C++ | - | clang-format |
| Java | - | - |

### Python: Powered by Ruff

For Python, linthis uses **ruff** - an extremely fast Python linter and formatter written in Rust. Ruff provides:

- **10-100x faster** than flake8 + black
- **800+ built-in rules** from flake8, isort, pyupgrade, and more
- **Black-compatible** formatting
- **Zero configuration** - works out of the box

## Installation

```bash
cargo install linthis
```

### Prerequisites

Install the linters/formatters for the languages you want to check:

```bash
# Python (ruff - recommended, 10-100x faster)
pip install ruff

# TypeScript/JavaScript
npm install -g eslint prettier

# Go (included with Go installation)

# Rust (included with Rust installation)

# C/C++
# Install clang-format via your package manager
```

## Usage

```bash
# Check current directory
linthis

# Check specific files or directories
linthis src/ tests/

# Check only (no formatting)
linthis --check-only

# Format only (no linting)
linthis --format-only

# Check only staged files
linthis --staged

# Specify languages
linthis --lang python,rust

# Verbose output
linthis --verbose

# Initialize linter configs
linthis --init-configs

# Benchmark Python tools (ruff vs flake8+black)
linthis --benchmark
```

### Benchmark Mode

Compare the speed of ruff vs legacy tools (flake8 + black):

```bash
linthis --benchmark path/to/python/project
```

Example output:
```
Python Linting/Formatting Benchmark (100 files)
┌──────────────┬─────────────┬─────────────┬──────────────┐
│ Tool         │ Lint (ms)   │ Format (ms) │ Total (ms)   │
├──────────────┼─────────────┼─────────────┼──────────────┤
│ flake8+black │        5234 │        3421 │         8655 │
│ ruff         │         312 │         198 │          510 │
├──────────────┼─────────────┼─────────────┼──────────────┤
│ Speedup      │       16.8x │       17.3x │        17.0x │
└──────────────┴─────────────┴─────────────┴──────────────┘
```

## Configuration

Create `.linthis.yml` or `.linthis.toml` in your project root:

```yaml
languages:
  - python
  - rust
  - typescript

max_complexity: 20

source:
  test_source:
    filepath_regex:
      - ".*/test/.*"
  third_party_source:
    filepath_regex:
      - ".*/third_party/.*"
```

### Generate Default Configs

Generate configuration files for all supported linters/formatters:

```bash
linthis --init-configs
```

This creates:
- `ruff.toml` - Python linting/formatting (ruff)
- `pyproject.toml` - Python project config with ruff settings
- `.clang-format` - C/C++ formatting
- `.prettierrc` - JavaScript/TypeScript formatting
- `.eslintrc.json` - JavaScript/TypeScript linting
- `rustfmt.toml` - Rust formatting

## Migration from flake8/black

If you're currently using flake8 and black for Python, switching to ruff is seamless:

1. **Install ruff**: `pip install ruff`
2. **Run linthis**: It will automatically use ruff
3. **Optional**: Generate ruff config with `linthis --init-configs`

Ruff is compatible with most flake8 rules and produces Black-compatible formatting.

## License

MIT
