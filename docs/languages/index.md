# Supported Languages

linthis supports 18 programming languages with automatic language detection.

## Language Support Matrix

| Language | Checker Tool | Formatter Tool | Extensions |
|----------|-------------|----------------|------------|
| [Rust](rust.md) | clippy | rustfmt | `.rs` |
| [Python](python.md) | ruff, pylint, flake8 | ruff, black | `.py`, `.pyi` |
| [TypeScript](typescript.md) | eslint | prettier | `.ts`, `.tsx` |
| [JavaScript](javascript.md) | eslint | prettier | `.js`, `.jsx`, `.mjs`, `.cjs` |
| [Go](go.md) | golangci-lint | gofmt | `.go` |
| [Java](java.md) | checkstyle | clang-format | `.java` |
| [C](c.md) | clang-tidy, cppcheck | clang-format | `.c`, `.h` |
| [C++](cpp.md) | clang-tidy, cpplint, cppcheck | clang-format | `.cpp`, `.cc`, `.cxx`, `.hpp`, `.hh`, `.hxx` |
| [Objective-C](objc.md) | clang-tidy | clang-format | `.m`, `.mm` |
| [Swift](swift.md) | swiftlint | swift-format | `.swift` |
| [Kotlin](kotlin.md) | ktlint, detekt | ktlint | `.kt`, `.kts` |
| [Lua](lua.md) | luacheck | stylua | `.lua` |
| [Dart](dart.md) | dart analyze | dart format | `.dart` |
| [Shell](shell.md) | shellcheck | shfmt | `.sh`, `.bash`, `.zsh`, `.ksh` |
| [Ruby](ruby.md) | rubocop | rubocop | `.rb`, `.rake`, `.gemspec` |
| [PHP](php.md) | phpcs | php-cs-fixer | `.php`, `.phtml` |
| [Scala](scala.md) | scalafix | scalafmt | `.scala`, `.sc` |
| [C#](csharp.md) | dotnet format | dotnet format | `.cs`, `.csx` |

## Language Detection

linthis automatically detects languages based on file extensions. You can also explicitly specify languages:

```bash
# Auto-detect (default)
linthis

# Specify single language
linthis --lang python

# Specify multiple languages
linthis --lang python,rust,javascript
```

## Tool Installation

Each language requires its underlying tools to be installed. Click on the language name above to see specific installation instructions.

### Quick Install Commands

```bash
# Rust
rustup component add clippy rustfmt

# Python
pip install ruff

# JavaScript/TypeScript
npm install -g eslint prettier

# Go
go install github.com/golangci/golangci-lint/cmd/golangci-lint@latest

# Shell
brew install shellcheck shfmt  # macOS
apt install shellcheck         # Ubuntu/Debian
```

## See It in Action

Watch the [Multi-Language video tutorial](../getting-started/videos.md#episode-2-multi-language-support) for a 15-second demo.

## Checking Tool Availability

Use the `doctor` command to check which tools are available:

```bash
linthis doctor
```

This will show the status of all required tools for each language.
