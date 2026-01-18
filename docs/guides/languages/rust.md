# Rust Language Guide

linthis uses **clippy** for linting and **rustfmt** for formatting Rust code.

## Supported File Extensions

- `.rs`

## Required Tools

### Linter: clippy

```bash
# Install via rustup (recommended)
rustup component add clippy

# Verify installation
cargo clippy --version
```

### Formatter: rustfmt

```bash
# Install via rustup (recommended)
rustup component add rustfmt

# Verify installation
rustfmt --version
```

## Configuration

### Basic Example

```toml
# .linthis/config.toml

[rust]
max_complexity = 15
excludes = ["target/**"]
```

### Disable Specific Clippy Lints

```toml
[rust.rules]
disable = [
    "clippy::needless_return",
    "clippy::too_many_arguments",
    "clippy::type_complexity"
]
```

### Change Severity

```toml
[rust.rules.severity]
"clippy::unwrap_used" = "error"    # Treat unwrap as error
"clippy::todo" = "warning"         # Keep TODO as warning
```

## Custom Rules

```toml
[[rules.custom]]
code = "rust/no-println"
pattern = "println!"
message = "Use log macros instead of println!"
severity = "warning"
suggestion = "Use log::info!, log::debug!, etc."
languages = ["rust"]
```

## CLI Usage

```bash
# Check Rust files only
linthis -c --lang rust

# Format Rust files only
linthis -f --lang rust

# Check specific file
linthis -c src/main.rs
```

## Clippy Configuration

linthis respects your `.clippy.toml` or `clippy.toml` configuration file:

```toml
# clippy.toml
cognitive-complexity-threshold = 15
too-many-arguments-threshold = 8
```

## Common Issues

### Clippy not found

```
Warning: No rust linter available for rust files
  Install: rustup component add clippy
```

**Solution**: Run `rustup component add clippy`

### Slow first run

Clippy compiles your project on first run. Subsequent runs use incremental compilation.

### Conflicting with project clippy config

linthis uses the same clippy configuration as `cargo clippy`. Your project's `clippy.toml` settings are respected.

## Best Practices

1. **Use workspace-level config**: Place `.linthis/config.toml` at workspace root
2. **CI integration**: Run `linthis -c --lang rust` in CI for consistent checks
3. **Complexity limits**: Set `max_complexity = 15` for maintainable code
4. **Unwrap handling**: Consider treating `clippy::unwrap_used` as error in production code
