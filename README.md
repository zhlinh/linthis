# linthis

A fast, cross-platform multi-language linter and formatter written in Rust.

## Features

- Multi-language support: C++, Objective-C, Java, Python, Rust, Go, JavaScript, TypeScript
- Fast parallel processing
- Git integration (check staged files)
- Configurable via YAML/TOML
- Compatible with CodeCC configuration format

## Installation

```bash
cargo install linthis
```

## Usage

```bash
# Check current directory
linthis

# Check specific files or directories
linthis src/ tests/

# Format mode (auto-fix)
linthis --format

# Check only staged files
linthis --staged

# Specify languages
linthis --lang cpp,java

# Verbose output
linthis --verbose
```

## Configuration

Create `.linthis.yml` or `.linthis.toml` in your project root:

```yaml
languages:
  - cpp
  - java
  - python

max_complexity: 20

source:
  test_source:
    filepath_regex:
      - ".*/test/.*"
  third_party_source:
    filepath_regex:
      - ".*/third_party/.*"
```

## License

MIT
