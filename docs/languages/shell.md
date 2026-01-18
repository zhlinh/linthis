# Shell/Bash

linthis supports Shell/Bash scripts using ShellCheck for checking and shfmt for formatting.

## Supported Extensions

- `.sh`
- `.bash`
- `.zsh`
- `.ksh`

## Tools

| Tool | Type | Description |
|------|------|-------------|
| [ShellCheck](https://www.shellcheck.net/) | Checker | Static analysis tool for shell scripts |
| [shfmt](https://github.com/mvdan/sh) | Formatter | Shell script formatter |

## Installation

### macOS

```bash
brew install shellcheck shfmt
```

### Ubuntu/Debian

```bash
apt install shellcheck
# For shfmt, download from GitHub releases or use go install
go install mvdan.cc/sh/v3/cmd/shfmt@latest
```

### Windows

```bash
# Using scoop
scoop install shellcheck shfmt

# Using chocolatey
choco install shellcheck
```

## Configuration

### ShellCheck

Create `.shellcheckrc` in your project root:

```ini
# Disable specific rules
disable=SC2034,SC2086

# Set shell dialect
shell=bash

# Enable additional checks
enable=all
```

### shfmt

shfmt uses flags or EditorConfig. Common options:

```bash
# Indent with 4 spaces
shfmt -i 4

# Use tabs
shfmt -i 0

# Binary operators at start of line
shfmt -bn
```

## Usage

```bash
# Check shell scripts
linthis --lang shell --check-only

# Format shell scripts
linthis --lang shell --format-only

# Check and format
linthis --lang shell
```

## Common Issues

### SC2086: Double quote to prevent globbing

```bash
# Bad
echo $var

# Good
echo "$var"
```

### SC2034: Variable appears unused

Add a comment to suppress:

```bash
# shellcheck disable=SC2034
unused_var="value"
```

## Severity Mapping

| ShellCheck Level | linthis Severity |
|-----------------|------------------|
| error | Error |
| warning | Warning |
| info | Info |
| style | Info |
