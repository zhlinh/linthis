# Quick Start

This guide will help you get started with linthis in minutes.

## Initialize Configuration (Optional)

```bash
# Create project configuration file
linthis init

# Create global configuration file
linthis init -g

# Install pre-commit hooks
linthis hook install --type git
```

## Basic Usage

### Check and Format

```bash
# Check and format current directory (default behavior)
linthis

# Check and format specific directories
linthis -i src/
linthis --include src/ --include lib/
```

### Check Only

```bash
# Check only, no formatting
linthis -c
linthis --check-only
```

### Format Only

```bash
# Format only, no checking
linthis -f
linthis --format-only
```

### Git Staged Files

```bash
# Check Git staged files (suitable for pre-commit hook)
linthis -s
linthis --staged
```

## Specify Languages

```bash
# Check specific language
linthis -l python
linthis --lang rust

# Check multiple languages
linthis -l python,rust,cpp
linthis --lang "python,javascript,go"
```

## Exclude Files

```bash
# Exclude specific patterns
linthis -e "*.test.js" -e "dist/**"
linthis --exclude "target/**" --exclude "node_modules/**"
```

## Output Formats

```bash
# Human-readable output (default)
linthis

# JSON output
linthis -o json

# GitHub Actions format
linthis -o github-actions
```

## Common Workflows

### Pre-commit Hook

```bash
# Install git hook
linthis hook install --type git

# Or use with pre-commit framework
linthis hook install --type pre-commit
```

### CI/CD

```bash
# In CI, use check-only mode with non-zero exit on errors
linthis --check-only --output github-actions
```

## Next Steps

- [Configuration](configuration.md) - Learn about configuration options
- [Plugin System](../features/plugins.md) - Share configurations across projects
- [CLI Reference](../reference/cli.md) - Complete command reference
