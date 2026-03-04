# AI 编程助手集成

## 概述

linthis 可以与 AI 编程助手（Claude Code、Cursor、Windsurf、GitHub Copilot、Cline、CodeBuddy）集成，在 AI 辅助开发过程中自动执行代码质量检查。

安装后，AI 助手会在修改代码后自动运行 `linthis` 检查，并在提交前修复问题——无需手动干预。

## 支持的 AI 助手

| AI 助手 | 规则文件 | 检测方式 | 安装策略 |
|--------|---------|---------|---------|
| Claude Code | `CLAUDE.md` + `.claude/settings.local.json` | `.claude/` 目录 | 追加段落 + Stop Hook |
| Cursor | `.cursor/rules/linthis.mdc` | `.cursor/` 目录 | 独立文件 |
| Windsurf | `.windsurf/rules/linthis.md` | `.windsurf/` 目录 | 独立文件 |
| GitHub Copilot | `.github/copilot-instructions.md` | `.github/` 目录 | 追加段落 |
| Cline | `.clinerules/linthis.md` | `.clinerules/` 目录 | 独立文件 |
| CodeBuddy | `.codebuddy/rules/linthis.md` | `.codebuddy/` 目录 | 独立文件 |

## 快速开始

### 安装指定 AI 助手

```bash
# 安装 Claude Code
linthis hook install --type agent --provider claude

# 安装 Cursor
linthis hook install --type agent --provider cursor

# 安装 Windsurf
linthis hook install --type agent --provider windsurf

# 安装 GitHub Copilot
linthis hook install --type agent --provider copilot

# 安装 Cline
linthis hook install --type agent --provider cline

# 安装 CodeBuddy
linthis hook install --type agent --provider codebuddy
```

### 自动检测并全部安装

为所有检测到的 AI 助手安装（如果未检测到则安装全部）：

```bash
linthis hook install --type agent -y
```

### 交互式菜单

不带 `-y` 或 `--provider` 运行，进入交互式选择：

```bash
linthis hook install --type agent
```

输出：
```
🤖 AI Coding Agent Integration

Select agent(s) to integrate with linthis:

  1. Claude Code  (installed)
  2. Cursor        (detected)
  3. Windsurf
  4. GitHub Copilot (detected)
  5. Cline
  6. CodeBuddy

  7. All detected agents
  8. All agents
  9. Cancel

Choose (comma-separated for multiple, e.g. 1,2):
```

## 安装内容详解

### Claude Code

创建两个文件：

1. **`CLAUDE.md`** — 追加 `## Linthis Agent Rules` 段落（如果文件不存在则创建）
2. **`.claude/settings.local.json`** — Stop Hook，在 AI 助手结束前触发 linthis 检查

### Cursor

创建带 YAML frontmatter 的独立规则文件：

```
.cursor/rules/linthis.mdc
```

`alwaysApply: true` 前置信息确保规则在所有对话中生效。

### Windsurf

创建独立规则文件：

```
.windsurf/rules/linthis.md
```

### GitHub Copilot

在以下文件中追加 `## Linthis Agent Rules` 段落：

```
.github/copilot-instructions.md
```

如果文件不存在，会使用默认标题创建。

### Cline

创建独立规则文件：

```
.clinerules/linthis.md
```

### CodeBuddy

创建独立规则文件：

```
.codebuddy/rules/linthis.md
```

## 工作原理

安装的规则指导 AI 助手执行以下操作：

1. **修改代码后** — 运行 `linthis -i <file1> -i <file2> -c` 检查所有修改的文件
2. **手动修复问题** — 阅读 lint 错误并直接修改代码（不使用 `--fix` 或 AI 自动修复）
3. **提交前** — 运行 `linthis -s -c` 检查暂存文件
4. **重新检查** — 修复后重新运行 linthis，直到通过

这确保 AI 助手生成符合代码规范的代码，具有正确的上下文感知能力，而非依赖自动修复工具。

## 查看状态

查看已安装的 AI 助手：

```bash
linthis hook status
```

输出：
```
Hook Status:
  Agent Hooks:
    ✓ Claude Code  (installed)
    ✓ Cursor       (installed)
    ✗ Windsurf
    ✗ GitHub Copilot
    ✗ Cline
    ✗ CodeBuddy
```

## 卸载

移除所有 AI 助手集成：

```bash
linthis hook uninstall --all -y
```

这会移除：
- `CLAUDE.md` 和 `.github/copilot-instructions.md` 中的 linthis 段落
- 独立规则文件（`.cursor/rules/linthis.mdc` 等）
- Claude Code Stop Hook（`.claude/settings.local.json`）
- linthis 创建的空目录

## 常见问题

### Q1：会覆盖我现有的 CLAUDE.md 或 copilot-instructions.md 吗？

**不会。** 对于追加式文件（CLAUDE.md、copilot-instructions.md），linthis 只会添加一个 `## Linthis Agent Rules` 段落，现有内容完全保留。如果段落已存在，不会重复添加。

### Q2：可以自定义规则吗？

可以。安装后直接编辑规则文件即可。对于独立文件，你拥有完全控制权；对于追加式文件，修改 `## Linthis Agent Rules` 段落即可。

### Q3：可以同时使用多个 AI 助手吗？

可以。你可以同时为多个 AI 助手安装规则，每个助手有自己独立的规则文件，互不干扰：

```bash
linthis hook install --type agent --provider claude
linthis hook install --type agent --provider cursor
```

### Q4：检测机制是怎样的？

linthis 检查项目根目录下的特定目录：

- `.claude/` → Claude Code
- `.cursor/` → Cursor
- `.windsurf/` → Windsurf
- `.github/` → GitHub Copilot
- `.clinerules/` → Cline
- `.codebuddy/` → CodeBuddy

使用 `-y`（自动安装）时，只为检测到的 AI 助手配置。如果未检测到任何助手，则全部安装。

### Q5：什么是 Claude Code Stop Hook？

Stop Hook（`.claude/settings.local.json`）在 Claude Code 完成任务前添加自动检查，提示 AI 助手对所有修改过的文件运行 linthis，确保不会遗漏任何 lint 问题。

### Q6：这和 git hook 的 AI 自动修复有什么区别？

这是两个不同的功能：

| 命令 | 用途 |
|-----|------|
| `linthis hook install --type agent --provider claude` | 安装 AI 助手规则（AI 编码过程中的代码质量检查） |
| `linthis hook install --args "-c -f --fix --ai --provider claude --accept-all"` | 安装带 AI 自动修复的 git hook（git commit 时自动修复 lint 问题） |

`--type agent` 中的 `--provider` 指定 AI 助手平台，而 `--args` 中的 `--provider` 指定 AI 修复提供者。
