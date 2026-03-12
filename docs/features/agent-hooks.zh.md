# AI 编程助手集成

## 概述

linthis 可以与 AI 编程助手（Claude Code、Codex、Gemini、Cursor、Droid、Auggie、CodeBuddy）集成，在 AI 辅助开发过程中自动执行代码质量检查。

安装后，AI 助手会在修改代码后自动运行 `linthis` 检查，并在提交前修复问题——无需手动干预。

## 支持的 AI 助手

| AI 助手 | 规则文件 | 检测方式 | 安装策略 |
|--------|---------|---------|---------|
| Claude Code | `CLAUDE.md` + `.claude/settings.json` | `.claude/` 目录 | 追加段落 + Stop Hook |
| Codex | `AGENTS.md` | `AGENTS.md` 或 `.codex/` | 追加段落 |
| Gemini | `.gemini/instructions.md` | `.gemini/` 目录 | 独立文件 |
| Cursor | `.cursor/rules/linthis.mdc` | `.cursor/` 目录 | 独立文件 |
| Droid | `.droid/rules/linthis.md` | `.droid/` 目录 | 独立文件 |
| Auggie | `.augment/rules/linthis.md` | `.augment/` 目录 | 独立文件 |
| CodeBuddy | `.codebuddy/rules/linthis.md` + `.codebuddy/settings.json` | `.codebuddy/` 目录 | 独立文件 + Stop Hook |

## 快速开始

### 安装指定 AI 助手

```bash
# 安装 Claude Code
linthis hook install --type agent --provider claude

# 安装 Codex
linthis hook install --type agent --provider codex

# 安装 Gemini
linthis hook install --type agent --provider gemini

# 安装 Cursor
linthis hook install --type agent --provider cursor

# 安装 Droid
linthis hook install --type agent --provider droid

# 安装 Auggie
linthis hook install --type agent --provider auggie

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
  2. Codex
  3. Gemini
  4. Cursor       (detected)
  5. Droid
  6. Auggie
  7. CodeBuddy

  8. All detected agents
  9. All agents
  10. Cancel

Choose (comma-separated for multiple, e.g. 1,4):
```

### 全局安装

将 AI 助手规则安装到用户主目录，使其对所有项目生效（而非仅当前项目）：

```bash
# 为指定提供者全局安装 AI 助手规则
linthis hook install --type agent --provider claude --global

# 为所有检测到的提供者全局安装
linthis hook install --type agent -g
```

使用 `--global` 时，规则文件写入用户级别路径而非项目根目录：

| AI 助手 | 项目级别 | 全局（`--global`） |
|--------|---------|------------------|
| Claude Code | `CLAUDE.md` | `~/.claude/CLAUDE.md` |
| Codex | `AGENTS.md` | `~/.codex/AGENTS.md` |
| Gemini | `.gemini/instructions.md` | `~/.gemini/instructions.md` |
| Cursor | `.cursor/rules/linthis.mdc` | `~/.cursor/rules/linthis.mdc` |
| Droid | `.droid/rules/linthis.md` | `~/.droid/rules/linthis.md` |
| Auggie | `.augment/rules/linthis.md` | `~/.augment/rules/linthis.md` |
| CodeBuddy | `.codebuddy/rules/linthis.md` | `~/.codebuddy/rules/linthis.md` |

## 安装内容详解

### Claude Code

创建两个文件：

1. **`CLAUDE.md`** — 追加 `## Linthis Agent Rules` 段落（如果文件不存在则创建）
2. **`.claude/settings.json`** — Stop Hook，在 AI 助手结束前触发 linthis 检查

### Codex

在以下文件中追加 `## Linthis Agent Rules` 段落：

```
AGENTS.md
```

如果文件不存在，会使用默认标题创建。

### Gemini

创建独立规则文件：

```
.gemini/instructions.md
```

### Cursor

创建带 YAML frontmatter 的独立规则文件：

```
.cursor/rules/linthis.mdc
```

`alwaysApply: true` 前置信息确保规则在所有对话中生效。

### Droid

创建独立规则文件：

```
.droid/rules/linthis.md
```

### Auggie

创建独立规则文件：

```
.augment/rules/linthis.md
```

### CodeBuddy

创建两个文件：

1. **`.codebuddy/rules/linthis.md`** — 独立规则文件
2. **`.codebuddy/settings.json`** — Stop Hook，在 AI 助手结束前触发 linthis 检查

## 工作原理

安装的规则指导 AI 助手执行以下操作：

1. **修改代码后** — 运行 `linthis -i <file1> -i <file2> -c` 检查所有修改的文件
2. **手动修复问题** — 阅读 lint 错误并直接修改代码（不使用 `--fix` 或 AI 自动修复）
3. **提交前** — 运行 `linthis -s -c` 检查暂存文件
4. **重新检查** — 修复后重新运行 linthis，直到通过

这确保 AI 助手生成符合代码规范的代码，具有正确的上下文感知能力，而非依赖自动修复工具。

## Git Hook 与 AI 自动修复（--type *-with-agent）

这些是 **git hook 类型**（与 `--type agent` 不同），在 git commit 时 linthis 检查失败后自动调用 AI CLI 工具进行修复，然后重新运行 linthis 验证结果。

### 安装

```bash
# 安装带 Claude Code 自动修复回退的 pre-commit git hook
linthis hook install --type git-with-agent --provider claude

# 其他 AI CLI 提供者
linthis hook install --type git-with-agent --provider codex
linthis hook install --type prek-with-agent --provider gemini
linthis hook install --type pre-commit-with-agent --provider cursor
linthis hook install --type git-with-agent --provider droid
linthis hook install --type git-with-agent --provider auggie
linthis hook install --type git-with-agent --provider codebuddy

# 全局安装（写入 ~/.config/git/hooks/）
linthis hook install --type git-with-agent --provider claude --global
```

### 支持的提供者

| 提供者 | CLI 可执行文件 | 无交互命令 |
|--------|-------------|-----------|
| `claude` | `claude` | `claude -p --permission-mode acceptEdits '...'` |
| `codex` | `codex` | `codex exec --ask-for-approval never '...'` |
| `gemini` | `gemini` | `gemini -p --approval-mode=auto_edit '...'` |
| `cursor` | `cursor-agent` | `cursor-agent chat --force '...'` |
| `droid` | `droid` | `droid exec --auto high '...'` |
| `auggie` | `auggie` | `auggie --print '...'` |
| `codebuddy` | `codebuddy` | `codebuddy -p --permission-mode acceptEdits '...'` |

### 生成的 Hook 脚本示例

以下是使用 `--type git-with-agent --provider claude` 时写入 `.git/hooks/pre-commit` 的脚本：

```bash
#!/bin/sh

LINTHIS_CMD="linthis -s -c -f --hook-event=pre-commit"

$LINTHIS_CMD
LINTHIS_EXIT=$?

if [ $LINTHIS_EXIT -ne 0 ]; then
  echo "[linthis] Lint errors detected. Invoking Claude Code to fix..."
  claude -p --permission-mode acceptEdits 'Staged files have linthis lint errors. Run '\''linthis -s -c'\'' to inspect them. Fix all issues by editing the files directly (do NOT use linthis --fix). Verify with '\''linthis -s -c'\'' until it passes cleanly.'
  $LINTHIS_CMD
  LINTHIS_EXIT=$?
fi

exit $LINTHIS_EXIT
```

### 与 --type agent 的区别

| 特性 | `--type agent` | `--type *-with-agent` |
|-----|---------------|----------------------|
| Hook 类型 | AI 助手规则文件 | git hook（pre-commit） |
| 触发时机 | AI 助手完成任务时 | 执行 `git commit` 时 |
| `--provider` 可选值 | `claude`, `codex`, `gemini`, `cursor`, `droid`, `auggie`, `codebuddy` | `claude`, `codex`, `gemini`, `cursor`, `droid`, `auggie`, `codebuddy` |
| 安装内容 | 规则文件（claude/codebuddy 另有 Stop Hook） | `.git/hooks/` 中的 Shell 脚本 |

## 查看状态

查看已安装的 hook 和 AI 助手：

```bash
linthis hook status
```

输出：
```
Git Hook Status
Repository: /path/to/repo

Project Hooks (.git/hooks/):
✓ /path/.git/hooks/pre-commit [project]
    pre-commit (runs before commit)
    ✓ linthis

Global Hooks (~/.config/git/hooks/):
  ℹ (core.hooksPath not set)
  ℹ No global linthis hooks installed

Agent Integration
✓ Claude Code (CLAUDE.md)
✗ Codex (not installed)
✗ Gemini (not installed)
✗ Cursor (not installed)
✗ Droid (not installed)
✗ Auggie (not installed)
✗ CodeBuddy (not installed)
```

## 卸载

移除所有 AI 助手集成：

```bash
linthis hook uninstall --all -y
```

移除指定提供者的 AI 助手规则：

```bash
linthis hook uninstall --type agent --provider claude -y
```

移除全局安装的 AI 助手规则：

```bash
linthis hook uninstall --type agent --global -y
```

卸载命令会移除：
- `CLAUDE.md` 和 `AGENTS.md` 中的 linthis 段落（追加式文件）
- 独立规则文件（`.cursor/rules/linthis.mdc`、`.gemini/instructions.md` 等）
- Claude Code Stop Hook（`.claude/settings.json`）
- CodeBuddy Stop Hook（`.codebuddy/settings.json`）
- linthis 创建的空目录

## 常见问题

### Q1：会覆盖我现有的 CLAUDE.md 或 AGENTS.md 吗？

**不会。** 对于追加式文件（`CLAUDE.md`、`AGENTS.md`），linthis 只会添加一个 `## Linthis Agent Rules` 段落，现有内容完全保留。如果段落已存在，不会重复添加。

### Q2：可以自定义规则吗？

可以。安装后直接编辑规则文件即可。对于独立文件，你拥有完全控制权；对于追加式文件，修改 `## Linthis Agent Rules` 段落即可。

### Q3：可以同时使用多个 AI 助手吗？

可以。你可以同时为多个 AI 助手安装规则，每个助手有自己独立的规则文件，互不干扰：

```bash
linthis hook install --type agent --provider claude
linthis hook install --type agent --provider cursor
```

### Q4：检测机制是怎样的？

linthis 检查项目根目录下的特定目录或文件：

- `.claude/` → Claude Code
- `AGENTS.md` 或 `.codex/` → Codex
- `.gemini/` → Gemini
- `.cursor/` → Cursor
- `.droid/` → Droid
- `.augment/` → Auggie
- `.codebuddy/` → CodeBuddy

使用 `-y`（自动安装）时，只为检测到的 AI 助手配置。如果未检测到任何助手，则全部安装。

### Q5：什么是 Stop Hook？

Stop Hook（`.claude/settings.json` 或 `.codebuddy/settings.json`）在 AI 助手完成任务前添加自动检查，提示 AI 助手对所有修改过的文件运行 linthis，确保不会遗漏任何 lint 问题。目前支持 Claude Code 和 CodeBuddy。

### Q6：AI 辅助 lint 检查有哪几种方式？

共有三种不同的方式：

| 方式 | 命令 | 工作原理 |
|-----|------|---------|
| AI 助手规则（项目级别） | `linthis hook install --type agent --provider claude` | 将规则安装到 AI 助手的配置文件，让 AI 在编码过程中主动执行 lint 检查 |
| AI 助手规则（全局） | `linthis hook install --type agent --provider claude --global` | 同上，但安装到 `~/.claude/CLAUDE.md`，对所有项目生效 |
| Git hook 带 AI 修复回退 | `linthis hook install --type git-with-agent --provider claude` | 安装 git pre-commit hook；如果 linthis 检查失败，自动调用 AI CLI 工具修复后重新验证 |

`--provider` 参数对两种类型使用相同的可选值（`claude`、`codex`、`gemini`、`cursor`、`droid`、`auggie`、`codebuddy`），但实现方式不同：
- `--type agent` 中：安装**规则/设置文件**，让 AI 在编码过程中主动执行 lint 检查
- `--type *-with-agent` 中：调用提供者的**无头 CLI 工具**，在 git hook 失败时自动修复问题
