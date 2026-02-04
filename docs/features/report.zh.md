# 报告与分析

`linthis report` 命令提供了生成报告、查看结果、分析趋势和检查代码一致性的工具。

## 子命令概览

| 命令 | 描述 |
|------|------|
| `show` | 以人类可读格式显示保存的检查结果 |
| `html` | 生成包含图表和统计信息的 HTML 报告 |
| `stats` | 显示检查结果的统计信息 |
| `trends` | 分析代码质量趋势 |
| `consistency` | 分析团队代码风格一致性 |

## 查看结果

将保存的检查结果（JSON）以与 `linthis -c` 相同的人类可读格式显示。适用于查看 hook 输出中被截断的完整错误详情或回顾之前的检查结果。

### 基本用法

```bash
# 显示最后一次检查结果
linthis report show

# 显示指定结果文件
linthis report show .linthis/result/result-20240101-120000.json

# 限制显示数量
linthis report show -n 10
```

### 选项

| 选项 | 描述 |
|------|------|
| `-n, --limit` | 限制显示的问题数量（0 = 无限制） |
| `--compact` | 紧凑格式：仅显示 file:line message，不显示代码上下文 |
| `--errors-only` | 仅显示错误（跳过警告和信息） |
| `--warnings-only` | 仅显示警告（跳过错误和信息） |
| `-f, --format` | 输出格式：`human`（默认）、`json` |

### 示例

```bash
# 紧凑格式（无代码上下文）
linthis report show --compact

# 仅显示错误
linthis report show --errors-only

# 限制显示 5 个问题
linthis report show -n 5

# JSON 输出
linthis report show --format json
```

### 典型工作流程

在 git hook 输出中看到截断的消息后：

```bash
# Hook 显示截断的输出如：
# ╭──────────────────────────────────────╮
# │  E dt_head.h:11 inclusion of deprec... │
# ╰──────────────────────────────────────╯

# 使用以下命令查看完整详情：
linthis report show
```

## HTML 报告

生成包含图表、统计信息和详细问题列表的独立 HTML 文件。

### 基本用法

```bash
# 从最后一次结果生成 HTML 报告
linthis report html

# 包含趋势分析
linthis report html --with-trends

# 指定输出文件
linthis report html -o my-report.html
```

### 选项

| 选项 | 描述 |
|------|------|
| `-o, --output` | 输出文件路径（默认：`.linthis/reports/report-{timestamp}.html`） |
| `--with-trends` | 在报告中包含历史趋势分析 |
| `-n, --trend-count` | 包含的历史运行次数（默认：10） |

## 统计信息

按严重程度、语言、工具和规则显示问题数量。还显示问题最多的文件和摘要指标。

### 基本用法

```bash
# 显示最后一次结果的统计信息
linthis report stats

# JSON 格式
linthis report stats --format json
```

### 选项

| 选项 | 描述 |
|------|------|
| `-f, --format` | 输出格式：`human`（默认）、`json` |

## 趋势分析

检查历史检查结果以识别代码质量趋势。显示问题是在改善、稳定还是恶化。

### 基本用法

```bash
# 分析最后 10 次运行
linthis report trends

# 分析最后 20 次运行
linthis report trends -n 20

# JSON 格式
linthis report trends --format json
```

### 选项

| 选项 | 描述 |
|------|------|
| `-n, --count` | 要分析的历史运行次数（默认：10） |
| `-f, --format` | 输出格式：`human`（默认）、`json` |

## 一致性分析

识别重复模式、异常文件和系统性问题，以帮助提高代码库的代码一致性。

### 基本用法

```bash
# 分析代码一致性
linthis report consistency

# JSON 格式
linthis report consistency --format json
```

### 选项

| 选项 | 描述 |
|------|------|
| `-f, --format` | 输出格式：`human`（默认）、`json` |

## 结果存储

默认情况下，linthis 将检查结果保存到 `.linthis/result/` 目录：

- 结果以带时间戳的 JSON 文件保存：`result-YYYYMMDD-HHMMSS.json`
- 使用 `--no-save-result` 禁用自动保存
- 使用 `--output-file` 指定自定义输出路径
- 使用 `--keep-results` 限制保留的结果文件数量（默认：10）

## Hook 输出宽度

Git hook 输出框的宽度现在会自适应终端宽度：

- 自动检测终端宽度（50-120 字符）
- 可通过配置文件中的 `hooks.output_width` 配置

```toml
# .linthis/config.toml
[hooks]
output_width = 100  # 0 = 自动检测（默认）
```
