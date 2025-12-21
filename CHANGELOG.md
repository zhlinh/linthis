# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.0.1] - 2025-12-22

### Added

- Initial release of linthis
- Single command for both linting and formatting
- Multi-language support:
  - Rust (clippy + rustfmt)
  - Python (pylint/flake8 + black)
  - TypeScript (eslint + prettier)
  - JavaScript (eslint + prettier)
  - Go (golint/go vet + gofmt)
  - Java (checkstyle + google-java-format)
  - C++ (cpplint/cppcheck + clang-format)
- Auto-detection of project languages based on file extensions
- Configurable exclusion patterns via `.linthis.toml`
- Hierarchical configuration (built-in, user, project, CLI)
- Format presets (Google, Airbnb, Standard)
- Parallel file processing for performance
- `--check-only` mode for lint without formatting
- `--format-only` mode for format without linting
- `--staged` mode for checking only staged files

### Installation

```bash
# From PyPI
pip install linthis

# From crates.io
cargo install linthis
```

[Unreleased]: https://github.com/zhlinh/linthis/compare/v0.0.1...HEAD
[0.0.1]: https://github.com/zhlinh/linthis/releases/tag/v0.0.1
