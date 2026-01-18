# Go 语言指南

linthis 使用 **golangci-lint** 进行代码检查，使用 **gofmt** 进行代码格式化。

## 支持的文件扩展名

- `.go`

## 必需工具

### 代码检查：golangci-lint

```bash
# macOS
brew install golangci-lint

# Linux
go install github.com/golangci/golangci-lint/cmd/golangci-lint@latest

# Windows
go install github.com/golangci/golangci-lint/cmd/golangci-lint@latest

# 验证安装
golangci-lint --version
```

### 格式化：gofmt

gofmt 包含在 Go 安装中。

```bash
# 验证安装
gofmt -h
```

## 配置

### 基本示例

```toml
# .linthis/config.toml

[go]
max_complexity = 15
excludes = ["vendor/**", "*_test.go"]
```

### 禁用特定规则

```toml
[go.rules]
disable = [
    "errcheck",
    "gosimple"
]
```

## 自定义规则

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

## CLI 用法

```bash
# 仅检查 Go 文件
linthis -c --lang go

# 仅格式化 Go 文件
linthis -f --lang go
```

## Golangci-lint 配置

创建 `.golangci.yml`：

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

## 常见问题

### Golangci-lint 未找到

```
Warning: No go linter available for go files
  Install: brew install golangci-lint
```

### 首次运行缓慢

Golangci-lint 会缓存结果。首次运行可能较慢，后续运行更快。

### Vendor 目录被检查

添加到排除项：

```toml
[go]
excludes = ["vendor/**"]
```

## 最佳实践

1. **使用 .golangci.yml**：在项目中保持一致的配置
2. **启用多个 linter**：golangci-lint 聚合了许多 linter
3. **复杂度限制**：Go 代码通常较简单，设置 `max_complexity = 15`
4. **测试排除**：考虑对测试文件使用不同的规则
