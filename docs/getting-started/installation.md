# Installation

There are multiple ways to install linthis depending on your environment.

## Method 1: Install via PyPI (Recommended for Python users)

```bash
# Using pip
pip install linthis

# Using uv (recommended)
# pip install uv
uv pip install linthis
```

## Method 2: Install via Cargo (Recommended for Rust users)

```bash
cargo install linthis
```

## Method 3: Build from Source

```bash
git clone https://github.com/zhlinh/linthis.git
cd linthis
cargo build --release
```

The binary will be available at `target/release/linthis`.

## Verify Installation

After installation, verify that linthis is working:

```bash
linthis --version
```

## System Requirements

- **Operating Systems**: macOS, Linux, Windows
- **Architecture**: x86_64, arm64

## Language-Specific Tools

linthis wraps existing language-specific tools. For each language you want to lint/format, you'll need the underlying tools installed:

| Language | Required Tools |
|----------|---------------|
| Rust | `rustfmt`, `clippy` |
| Python | `ruff` or `black`, `flake8`, `pylint` |
| JavaScript/TypeScript | `eslint`, `prettier` |
| Go | `gofmt`, `golangci-lint` |
| Java | `checkstyle`, `google-java-format` |
| C++ | `clang-format`, `cpplint` |

See [Languages](../languages/index.md) for detailed setup instructions for each language.

## Next Steps

- [Quick Start](quickstart.md) - Learn the basics
- [Configuration](configuration.md) - Configure linthis for your project
