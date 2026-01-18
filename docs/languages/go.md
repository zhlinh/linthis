# Go Language Guide

linthis uses **golangci-lint** for linting and **gofmt** for formatting Go code.

## Supported File Extensions

- `.go`

## Required Tools

### Linter: golangci-lint

```bash
# macOS
brew install golangci-lint

# Linux
go install github.com/golangci/golangci-lint/cmd/golangci-lint@latest

# Windows
go install github.com/golangci/golangci-lint/cmd/golangci-lint@latest

# Verify installation
golangci-lint --version
```

### Formatter: gofmt

gofmt is included with the Go installation.

```bash
# Verify installation
gofmt -h
```

## Configuration

### Basic Example

```toml
# .linthis/config.toml

[go]
max_complexity = 15
excludes = ["vendor/**", "*_test.go"]
```

### Disable Specific Rules

```toml
[go.rules]
disable = [
    "errcheck",
    "gosimple"
]
```

## Custom Rules

```toml
[[rules.custom]]
code = "go/no-panic"
pattern = "\\bpanic\\s*\\("
message = "Avoid panic in production code"
severity = "warning"
suggestion = "Return an error instead"
languages = ["go"]

[[rules.custom]]
code = "go/no-todo"
pattern = "// TODO"
message = "TODO comment found"
severity = "info"
languages = ["go"]
```

## CLI Usage

```bash
# Check Go files only
linthis -c --lang go

# Format Go files only
linthis -f --lang go
```

## Golangci-lint Configuration

Create `.golangci.yml`:

```yaml
linters:
  enable:
    - errcheck
    - gosimple
    - govet
    - ineffassign
    - staticcheck
    - unused

linters-settings:
  errcheck:
    check-type-assertions: true
  govet:
    check-shadowing: true

issues:
  exclude-rules:
    - path: _test\.go
      linters:
        - errcheck
```

## Common Issues

### Golangci-lint not found

```
Warning: No go linter available for go files
  Install: brew install golangci-lint
```

### Slow first run

Golangci-lint caches results. First run may be slow, subsequent runs are faster.

### Vendor directory being checked

Add to excludes:

```toml
[go]
excludes = ["vendor/**"]
```

## Best Practices

1. **Use .golangci.yml**: Consistent configuration across your project
2. **Enable multiple linters**: golangci-lint aggregates many linters
3. **Complexity limits**: Go code is typically less complex, set `max_complexity = 15`
4. **Test exclusions**: Consider different rules for test files
