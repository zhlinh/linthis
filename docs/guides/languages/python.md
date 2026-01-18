# Python Language Guide

linthis uses **ruff** for both linting and formatting Python code.

## Supported File Extensions

- `.py`
- `.pyw`

## Required Tools

### Linter & Formatter: ruff

```bash
# Install via pip
pip install ruff

# Or via pipx (isolated environment)
pipx install ruff

# Or via homebrew (macOS)
brew install ruff

# Verify installation
ruff --version
```

## Configuration

### Basic Example

```toml
# .linthis/config.toml

[python]
max_complexity = 10
excludes = ["*_test.py", "test_*.py", "venv/**"]
```

### Disable Specific Rules

```toml
[python.rules]
disable = [
    "E501",    # Line too long
    "W503",    # Line break before binary operator
    "F401"     # Unused import
]
```

### Change Severity

```toml
[python.rules.severity]
"F841" = "error"     # Unused variable as error
"E999" = "error"     # Syntax errors as error
```

## Custom Rules

```toml
[[rules.custom]]
code = "python/no-print"
pattern = "\\bprint\\s*\\("
message = "Use logging instead of print()"
severity = "warning"
suggestion = "import logging; logging.info(...)"
languages = ["python"]

[[rules.custom]]
code = "python/no-assert"
pattern = "\\bassert\\b"
message = "Avoid assert in production code"
severity = "info"
languages = ["python"]
```

## CLI Usage

```bash
# Check Python files only
linthis -c --lang python

# Format Python files only
linthis -f --lang python

# Check specific file
linthis -c src/main.py
```

## Ruff Configuration

linthis respects your `ruff.toml` or `pyproject.toml` configuration:

```toml
# ruff.toml
line-length = 100
target-version = "py311"

[lint]
select = ["E", "F", "W", "I", "N", "UP"]
ignore = ["E501"]

[lint.per-file-ignores]
"__init__.py" = ["F401"]
"tests/*" = ["S101"]
```

Or in `pyproject.toml`:

```toml
[tool.ruff]
line-length = 100

[tool.ruff.lint]
select = ["E", "F", "W"]
```

## Common Issues

### Ruff not found

```
Warning: No python linter available for python files
  Install: pip install ruff
```

**Solution**: Run `pip install ruff`

### Virtual environment files being checked

**Solution**: Add virtual environment to excludes:

```toml
[python]
excludes = ["venv/**", ".venv/**", "env/**"]
```

### Type hints not checked

Ruff doesn't do type checking. For type checking, use mypy separately or configure it in your CI.

## Best Practices

1. **Use pyproject.toml**: Keep ruff config in `pyproject.toml` for better tooling integration
2. **Exclude tests differently**: Consider different severity for test files
3. **Complexity limits**: Set `max_complexity = 10` for Python (lower than other languages due to Python's expressiveness)
4. **Import sorting**: Ruff can sort imports - enable with `select = ["I"]` in ruff config
