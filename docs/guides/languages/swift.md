# Swift Language Guide

linthis uses **swiftlint** for linting and **swift-format** for formatting Swift code.

## Supported File Extensions

- `.swift`

## Required Tools

### Linter: swiftlint

```bash
# macOS (recommended)
brew install swiftlint

# Or via Mint
mint install realm/SwiftLint

# Verify installation
swiftlint version
```

### Formatter: swift-format

```bash
# macOS
brew install swift-format

# Or build from source
git clone https://github.com/apple/swift-format.git
cd swift-format
swift build -c release

# Verify installation
swift-format --version
```

## Configuration

### Basic Example

```toml
# .linthis/config.toml

[swift]
max_complexity = 15
excludes = ["Pods/**", "Carthage/**", ".build/**"]
```

### Disable Specific Rules

```toml
[swift.rules]
disable = [
    "line_length",
    "trailing_whitespace"
]
```

## Custom Rules

```toml
[[rules.custom]]
code = "swift/no-print"
pattern = "\\bprint\\s*\\("
message = "Use os_log or Logger instead of print()"
severity = "warning"
languages = ["swift"]

[[rules.custom]]
code = "swift/force-unwrap"
pattern = "\\!(?![=])"
message = "Avoid force unwrapping"
severity = "warning"
suggestion = "Use optional binding or nil coalescing"
languages = ["swift"]
```

## CLI Usage

```bash
# Check Swift files only
linthis -c --lang swift

# Format Swift files only
linthis -f --lang swift
```

## SwiftLint Configuration

Create `.swiftlint.yml`:

```yaml
disabled_rules:
  - trailing_whitespace
  - line_length

opt_in_rules:
  - empty_count
  - closure_spacing

line_length:
  warning: 120
  error: 150

excluded:
  - Pods
  - Carthage
  - .build
```

## Swift-Format Configuration

Create `.swift-format`:

```json
{
  "version": 1,
  "lineLength": 120,
  "indentation": {
    "spaces": 4
  },
  "lineBreakBeforeControlFlowKeywords": false,
  "lineBreakBeforeEachArgument": true
}
```

## Common Issues

### SwiftLint not found

```
Warning: No swift linter available for swift files
  Install: brew install swiftlint
```

**Note**: SwiftLint is primarily macOS-only.

### SPM packages being checked

Add to excludes:

```toml
[swift]
excludes = [".build/**", "Package.resolved"]
```

### Xcode-managed files

Add generated files to excludes:

```toml
[swift]
excludes = ["*.generated.swift", "R.generated.swift"]
```

## Best Practices

1. **Use .swiftlint.yml**: Configure rules per project
2. **Force unwrap detection**: Warn on `!` usage
3. **SPM exclusions**: Exclude `.build` directory
4. **Line length**: 120 characters is common for Swift projects
