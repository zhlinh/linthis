# Objective-C Language Guide

linthis uses **cpplint** and **clang-tidy** for linting, and **clang-format** for formatting Objective-C code.

## Supported File Extensions

- `.m`
- `.mm`
- `.h` (auto-detected based on content)

## Required Tools

Same as C++ - see [C++ Language Guide](cpp.md).

### Linter: cpplint

```bash
pip install cpplint
```

### Formatter: clang-format

```bash
# macOS
brew install clang-format

# Or install LLVM
brew install llvm
```

## Configuration

### Basic Example

```toml
# .linthis/config.toml

[oc]
linelength = 120
excludes = ["Pods/**", "Carthage/**"]
cpplint_filter = "-build/header_guard,-runtime/int"
```

Alternative key name:

```toml
[objectivec]
linelength = 120
```

### Clang-tidy Ignored Checks

```toml
[oc]
clang_tidy_ignored_checks = [
    "clang-analyzer-osx.cocoa.RetainCount",
    "clang-analyzer-osx.cocoa.SelfInit"
]
```

## Custom Rules

```toml
[[rules.custom]]
code = "oc/no-nslog"
pattern = "NSLog\\s*\\("
message = "Remove NSLog before release"
severity = "warning"
languages = ["oc"]

[[rules.custom]]
code = "oc/arc-bridge"
pattern = "__bridge"
message = "Review ARC bridge usage"
severity = "info"
languages = ["oc"]
```

## CLI Usage

```bash
# Check Objective-C files only
linthis -c --lang oc

# Format Objective-C files only
linthis -f --lang oc
```

## Clang-Format Configuration

Create `.clang-format`:

```yaml
BasedOnStyle: Google
Language: ObjC
ObjCSpaceAfterProperty: true
ObjCSpaceBeforeProtocolList: true
ColumnLimit: 120
```

## Header File Detection

linthis automatically detects `.h` files as Objective-C when they contain:
- `#import` or `@import` statements
- `@interface`, `@implementation`, `@protocol`
- NS types (NSString, NSArray, etc.)
- Objective-C method syntax (`+ (`, `- (`)

## Common Issues

### .h files detected as C++

If `.h` files are incorrectly detected, ensure they contain Objective-C patterns or explicitly specify:

```bash
linthis -c --lang oc src/
```

### Pods/Carthage being checked

Add to excludes:

```toml
[oc]
excludes = ["Pods/**", "Carthage/**", "Vendor/**"]
```

## Best Practices

1. **Use .clang-format**: Consistent formatting for ObjC-specific style
2. **Exclude dependencies**: Add Pods/Carthage to excludes
3. **ARC considerations**: Review `__bridge` usage warnings
4. **NSLog cleanup**: Remove debug logging before release
