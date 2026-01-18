# Contributing

Thank you for your interest in contributing to linthis!

## Getting Started

### Prerequisites

- Rust 1.75+ (stable)
- Cargo

### Clone and Build

```bash
git clone https://github.com/zhlinh/linthis.git
cd linthis
cargo build
```

### Run Tests

```bash
cargo test
```

### Run Clippy

```bash
cargo clippy
```

## Development Workflow

1. Fork the repository
2. Create a feature branch: `git checkout -b feature/my-feature`
3. Make your changes
4. Run tests: `cargo test`
5. Run clippy: `cargo clippy`
6. Commit your changes: `git commit -m "feat: add my feature"`
7. Push to your fork: `git push origin feature/my-feature`
8. Open a Pull Request

## Code Style

- Follow Rust standard conventions
- Use `rustfmt` for formatting
- Use `clippy` for linting
- Write tests for new features
- Document public APIs

## Commit Messages

We follow [Conventional Commits](https://www.conventionalcommits.org/):

- `feat:` - New feature
- `fix:` - Bug fix
- `docs:` - Documentation changes
- `refactor:` - Code refactoring
- `test:` - Test changes
- `chore:` - Build/tooling changes

Examples:

```
feat(python): add support for mypy
fix(cli): handle empty file list correctly
docs: update installation instructions
```

## Adding Language Support

To add support for a new language:

1. Create checker in `src/checkers/<language>.rs`
2. Create formatter in `src/formatters/<language>.rs`
3. Add language variant to `Language` enum in `src/lib.rs`
4. Update extension mappings
5. Add install hints
6. Write tests
7. Create documentation in `docs/languages/<language>.md`

See existing implementations for reference.

## Project Structure

```
src/
├── main.rs          # CLI entry point
├── lib.rs           # Main library
├── checkers/        # Language checkers
├── formatters/      # Language formatters
├── config/          # Configuration handling
├── plugin/          # Plugin system
├── cli/             # CLI commands
└── utils/           # Utility functions
```

## Testing

### Unit Tests

```bash
cargo test
```

### Integration Tests

```bash
cargo test --test integration
```

### Specific Test

```bash
cargo test test_name
```

## Documentation

- Update docs in `docs/` directory
- Use MkDocs for local preview: `mkdocs serve`
- Keep README.md in sync with major changes

## Questions?

- Open an issue for bugs or feature requests
- Start a discussion for questions
- Check existing issues before creating new ones

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
