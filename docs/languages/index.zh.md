# 支持的语言

linthis 支持 17 种编程语言，具有自动语言检测功能。

## 语言支持矩阵

| 语言 | 检查工具 | 格式化工具 | 扩展名 |
|-----|---------|-----------|-------|
| [Rust](rust.md) | clippy | rustfmt | `.rs` |
| [Python](python.md) | ruff, pylint, flake8 | ruff, black | `.py`, `.pyi` |
| [TypeScript](typescript.md) | eslint | prettier | `.ts`, `.tsx` |
| [JavaScript](javascript.md) | eslint | prettier | `.js`, `.jsx`, `.mjs`, `.cjs` |
| [Go](go.md) | golangci-lint | gofmt | `.go` |
| [Java](java.md) | checkstyle | google-java-format | `.java` |
| [C++](cpp.md) | cpplint, cppcheck | clang-format | `.cpp`, `.cc`, `.cxx`, `.h`, `.hpp` |
| [Swift](swift.md) | swiftlint | swift-format | `.swift` |
| [Kotlin](kotlin.md) | detekt | ktlint | `.kt`, `.kts` |
| [Objective-C](objc.md) | clang-tidy | clang-format | `.m`, `.mm` |
| [Lua](lua.md) | luacheck | stylua | `.lua` |
| [Dart](dart.md) | dart analyze | dart format | `.dart` |
| [Shell](shell.md) | shellcheck | shfmt | `.sh`, `.bash`, `.zsh`, `.ksh` |
| [Ruby](ruby.md) | rubocop | rubocop | `.rb`, `.rake`, `.gemspec` |
| [PHP](php.md) | phpcs | php-cs-fixer | `.php`, `.phtml` |
| [Scala](scala.md) | scalafix | scalafmt | `.scala`, `.sc` |
| [C#](csharp.md) | dotnet format | dotnet format | `.cs`, `.csx` |

## 语言检测

linthis 根据文件扩展名自动检测语言。您也可以显式指定语言：

```bash
# 自动检测（默认）
linthis

# 指定单个语言
linthis --lang python

# 指定多种语言
linthis --lang python,rust,javascript
```

## 工具安装

每种语言都需要安装其底层工具。点击上面的语言名称查看具体安装说明。

### 快速安装命令

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

## 检查工具可用性

使用 `doctor` 命令检查哪些工具可用：

```bash
linthis doctor
```

这将显示每种语言所需工具的状态。
