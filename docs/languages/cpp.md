# C++ Language Guide

linthis uses **cpplint** and **clang-tidy** for linting, and **clang-format** for formatting C/C++ code.

## Supported File Extensions

- `.c`, `.cc`, `.cpp`, `.cxx`
- `.h`, `.hpp`, `.hxx`

## Required Tools

### Linter: cpplint

```bash
# Install via pip
pip install cpplint

# Verify installation
cpplint --version
```

### Linter: clang-tidy (optional, additional checks)

```bash
# macOS
brew install llvm

# Ubuntu/Debian
sudo apt install clang-tidy

# Windows
choco install llvm

# Verify installation
clang-tidy --version
```

### Formatter: clang-format

```bash
# macOS
brew install clang-format

# Ubuntu/Debian
sudo apt install clang-format

# Windows
choco install llvm

# Verify installation
clang-format --version
```

## Configuration

### Basic Example

```toml
# .linthis/config.toml

[cpp]
max_complexity = 25
linelength = 120
excludes = ["build/**", "third_party/**"]
```

### Cpplint Filter

Disable specific cpplint checks:

```toml
[cpp]
cpplint_filter = "-build/c++11,-whitespace/tab,-build/header_guard"
```

### Clang-tidy Checks

Ignore specific clang-tidy checks:

```toml
[cpp]
clang_tidy_ignored_checks = [
    "modernize-use-trailing-return-type",
    "readability-magic-numbers"
]
```

### Disable Specific Rules

```toml
[cpp.rules]
disable = [
    "build/include_order",
    "whitespace/braces"
]
```

## Custom Rules

```toml
[[rules.custom]]
code = "cpp/no-raw-pointer-new"
pattern = "new\\s+\\w+"
message = "Consider using smart pointers instead of raw new"
severity = "warning"
suggestion = "Use std::make_unique or std::make_shared"
languages = ["cpp"]

[[rules.custom]]
code = "cpp/no-goto"
pattern = "\\bgoto\\b"
message = "Avoid using goto"
severity = "error"
languages = ["cpp"]
```

## CLI Usage

```bash
# Check C++ files only
linthis -c --lang cpp

# Format C++ files only
linthis -f --lang cpp
```

## Clang-Format Configuration

Create `.clang-format`:

```yaml
BasedOnStyle: Google
IndentWidth: 4
ColumnLimit: 120
AllowShortFunctionsOnASingleLine: Inline
BreakBeforeBraces: Attach
```

## Cpplint Configuration

Create `CPPLINT.cfg` in your project root:

```
linelength=120
filter=-build/c++11,-whitespace/tab
```

## Common Issues

### .h files detected as wrong language

linthis uses smart detection for `.h` files. If misdetected, you can force the language:

```bash
linthis -c --lang cpp src/
```

### Cpplint not found

```
Warning: No cpp linter available for cpp files
  Install: pip install cpplint
```

### Third-party code being checked

Add to excludes:

```toml
[cpp]
excludes = ["third_party/**", "external/**", "vendor/**"]
```

## Best Practices

1. **Use .clang-format**: Consistent formatting across your project
2. **Line length**: 120 characters is common for modern displays
3. **Header guards**: Consider using `#pragma once` if cpplint complains
4. **Smart pointers**: Enable warnings for raw pointer usage
