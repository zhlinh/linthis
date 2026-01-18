# Dart Language Guide

linthis uses **dart analyze** for linting and **dart format** for formatting Dart code. Both tools are included with the Dart SDK.

## Supported File Extensions

- `.dart`

## Required Tools

### Dart SDK

Both linter and formatter are included with Dart SDK.

```bash
# macOS
brew install dart

# Linux (Ubuntu/Debian)
sudo apt install dart

# Windows
choco install dart-sdk

# Or install Flutter (includes Dart)
# https://flutter.dev/docs/get-started/install

# Verify installation
dart --version
```

## Configuration

### Basic Example

```toml
# .linthis/config.toml

[dart]
max_complexity = 15
excludes = [".dart_tool/**", "build/**", "*.g.dart"]
```

### Disable Specific Rules

```toml
[dart.rules]
disable = [
    "prefer_single_quotes",
    "lines_longer_than_80_chars"
]
```

## Custom Rules

```toml
[[rules.custom]]
code = "dart/no-print"
pattern = "\\bprint\\s*\\("
message = "Use logging instead of print()"
severity = "warning"
languages = ["dart"]

[[rules.custom]]
code = "dart/no-debug-print"
pattern = "debugPrint\\s*\\("
message = "Remove debugPrint before release"
severity = "warning"
languages = ["dart"]
```

## CLI Usage

```bash
# Check Dart files only
linthis -c --lang dart

# Format Dart files only
linthis -f --lang dart
```

## Analysis Options

Create `analysis_options.yaml`:

```yaml
include: package:lints/recommended.yaml
# Or for Flutter:
# include: package:flutter_lints/flutter.yaml

analyzer:
  exclude:
    - "**/*.g.dart"
    - "**/*.freezed.dart"
  errors:
    invalid_annotation_target: ignore

linter:
  rules:
    - prefer_const_constructors
    - prefer_const_declarations
    - prefer_final_fields
    - avoid_print
    - prefer_single_quotes
```

## Common Issues

### Dart not found

```
Warning: No dart linter available for dart files
  Install: https://dart.dev/get-dart
```

### Generated code being checked

Add generated files to excludes:

```toml
[dart]
excludes = [
    "*.g.dart",
    "*.freezed.dart",
    "*.mocks.dart",
    ".dart_tool/**"
]
```

### Flutter-specific lints

For Flutter projects, use `package:flutter_lints`:

```yaml
# analysis_options.yaml
include: package:flutter_lints/flutter.yaml
```

### Build outputs

Add build directories to excludes:

```toml
[dart]
excludes = ["build/**", ".dart_tool/**"]
```

## Best Practices

1. **Use analysis_options.yaml**: Standard Dart/Flutter configuration
2. **Exclude generated code**: Add `*.g.dart`, `*.freezed.dart` to excludes
3. **Use recommended lints**: Start with `package:lints/recommended.yaml`
4. **Null safety**: Ensure all code is null-safe
5. **Flutter lints**: Use `package:flutter_lints` for Flutter projects
