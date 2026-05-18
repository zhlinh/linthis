# 忽略规则 — `.linthisignore`

`.linthisignore` 文件让你无需编辑 `.linthis/config.toml` 就能快速抑制 lint 问题。它位于项目根目录，支持两种条目：**文件路径 glob 模式** 和 **规则代码**。

## 文件格式

```gitignore
# 注释行和空行会被忽略

# 文件/路径 glob 模式（gitignore 语法）
vendor/**
*.generated.go
build/

# 规则禁用条目 — 以 "rule:" 为前缀
rule:E501
rule:clippy::too_many_arguments
rule:complexity/*
```

**解析规则：**

| 行内容 | 效果 |
|--------|------|
| 空行或以 `#` 开头 | 忽略 |
| 以 `rule:` 开头 | 禁用对应规则代码 |
| 其他内容 | 文件 glob 模式 — 匹配的文件从 lint 中排除 |

## 使用 CLI

```bash
# 添加文件 glob 模式
linthis ignore add "vendor/**"
linthis ignore add "*.generated.go"

# 添加规则禁用条目（需要包含 "rule:" 前缀）
linthis ignore add "rule:E501"
linthis ignore add "rule:clippy::too_many_arguments"
linthis ignore add "rule:linthis-complexity"

# 删除条目（精确匹配）
linthis ignore remove "vendor/**"
linthis ignore remove "rule:E501"

# 列出所有当前条目
linthis ignore list
```

`add` 是幂等的 — 添加已存在的条目会打印提示并什么都不做。

## 在交互式修复模式中添加忽略

使用 `linthis fix` 交互模式时，对任意问题按 **`w`** 键，即可将 `rule:<code>` 写入 `.linthisignore`，该规则将在所有后续运行中永久抑制——无需手动编辑任何配置文件。

```
[W1] [rust][clippy] src/main.rs:42
  复杂度超过阈值

操作: [e]编辑  [i]忽略  [a]接受  [w]写入忽略  [s]跳过  [p]上一个  [g]跳转  [q]退出
> w
✓ Written to .linthisignore: rule:linthis-complexity
```

## 工作原理

`.linthisignore` 在流水线中 **两个节点** 生效：

### 1. 路径模式 — lint 前文件排除

路径 glob 模式在文件扫描器启动 lint 之前加载。匹配任意模式的文件会被完全排除——与在 `config.toml` 的 `excludes` 中列出效果相同。

### 2. 规则代码 — lint 后过滤

规则代码（`rule:` 前缀条目）在规则过滤器运行之前合并到 `config.rules.disable` 中。代码匹配禁用规则的任何问题都会从输出中移除。这同时适用于 lint、复杂度和安全检测结果。

## 与 `config.toml` 的关系

`.linthisignore` 与 `.linthis/config.toml` 互补。

| 使用场景 | 配置位置 |
|----------|----------|
| 快速的分支级或开发者级抑制 | `.linthisignore` |
| 由 `linthis fix` 交互模式自动添加 | `.linthisignore` |
| 团队级永久配置 | `.linthis/config.toml` |

等效的 `config.toml` 配置：

```toml
excludes = ["vendor/**", "*.generated.go"]

[rules]
disable = ["E501", "clippy::too_many_arguments"]
```

## `.linthisignore` 示例

```gitignore
# 忽略生成文件和第三方代码
vendor/**
*.pb.go
build/
dist/

# 禁用特定 lint 规则
rule:E501
rule:clippy::too_many_arguments

# 禁用所有复杂度检查
rule:linthis-complexity

# 用通配符前缀禁用整个规则类别
rule:clippy::*
```

## 优先级与覆盖

`.linthisignore` 条目叠加在 `config.toml` 之上——它们扩展 `excludes` 和 `rules.disable` 列表，而非替换。标量字段（如 `max_complexity`）仍以 `config.toml` 为准。

## 参见

- [配置](../getting-started/configuration.zh.md) — 完整的 `config.toml` 参考
- [CLI 参考：ignore](../reference/cli.zh.md#ignore) — 命令标志说明
- [AI 智能修复](ai-fix.zh.md) — 交互式修复模式
