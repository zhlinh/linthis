# AI 编程助手集成

## 概述

linthis 可以与 AI 编程助手（Claude Code、Codex、Gemini、Cursor、Droid、Auggie、CodeBuddy）集成，在 AI 辅助开发过程中自动执行代码质量检查。

安装后，AI 助手会在修改代码后自动运行 `linthis` 检查，并在提交前修复问题——无需手动干预。

## 支持的 AI 助手

| AI 助手 | 规则文件 | 检测方式 | 安装策略 |
|--------|---------|---------|---------|
| Claude Code | `.claude/skills/lt-lint/SKILL.md`, `.claude/skills/lt-cmsg/SKILL.md`, `.claude/skills/lt-review/SKILL.md` + `.claude/settings.json` | `.claude/` 目录 | 每事件技能文件 + Stop Hook |
| Codex | `AGENTS.md` | `AGENTS.md` 或 `.codex/` | 每事件段落 |
| Gemini | `.gemini/linthis-lint.md`, `.gemini/linthis-cmsg.md`, `.gemini/linthis-review.md` | `.gemini/` 目录 | 每事件独立文件 |
| Cursor | `.cursor/rules/linthis-lint.mdc`, `.cursor/rules/linthis-cmsg.mdc`, `.cursor/rules/linthis-review.mdc` | `.cursor/` 目录 | 每事件独立文件 |
| Droid | `.droid/rules/linthis-lint.md`, `.droid/rules/linthis-cmsg.md`, `.droid/rules/linthis-review.md` | `.droid/` 目录 | 每事件独立文件 |
| Auggie | `.augment/rules/linthis-lint.md`, `.augment/rules/linthis-cmsg.md`, `.augment/rules/linthis-review.md` | `.augment/` 目录 | 每事件独立文件 |
| CodeBuddy | `.codebuddy/skills/lt-lint/SKILL.md`, `.codebuddy/skills/lt-cmsg/SKILL.md`, `.codebuddy/skills/lt-review/SKILL.md` + `.codebuddy/settings.json` | `.codebuddy/` 目录 | 每事件技能文件 + Stop Hook |

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
| Claude Code | `.claude/skills/lt-{lint,cmsg,review}/SKILL.md` | `~/.claude/skills/lt-{lint,cmsg,review}/SKILL.md` |
| Codex | `AGENTS.md` | `~/.codex/AGENTS.md` |
| Gemini | `.gemini/linthis-{lint,cmsg,review}.md` | `~/.gemini/linthis-{lint,cmsg,review}.md` |
| Cursor | `.cursor/rules/linthis-{lint,cmsg,review}.mdc` | `~/.cursor/rules/linthis-{lint,cmsg,review}.mdc` |
| Droid | `.droid/rules/linthis-{lint,cmsg,review}.md` | `~/.droid/rules/linthis-{lint,cmsg,review}.md` |
| Auggie | `.augment/rules/linthis-{lint,cmsg,review}.md` | `~/.augment/rules/linthis-{lint,cmsg,review}.md` |
| CodeBuddy | `.codebuddy/skills/lt-{lint,cmsg,review}/SKILL.md` | `~/.codebuddy/skills/lt-{lint,cmsg,review}/SKILL.md` |

## 安装内容详解

### Claude Code

技能文件按 hook 事件分别安装，共创建三个技能文件和一个 Stop Hook 设置：

1. **`.claude/skills/lt-lint/SKILL.md`** — `pre-commit` 事件：暂存文件 lint 检查
2. **`.claude/skills/lt-cmsg/SKILL.md`** — `commit-msg` 事件：提交信息格式验证
3. **`.claude/skills/lt-review/SKILL.md`** — `pre-push` 事件：推送前代码审查
4. **`.claude/settings.json`** — Stop Hook，在 AI 助手结束前触发 linthis 检查

### Codex

在 `AGENTS.md` 中按 hook 事件追加每事件段落（如果文件不存在则创建）。

### Gemini

按 hook 事件创建独立规则文件：

```
.gemini/linthis-lint.md
.gemini/linthis-cmsg.md
.gemini/linthis-review.md
```

### Cursor

按 hook 事件创建带 YAML frontmatter 的独立规则文件：

```
.cursor/rules/linthis-lint.mdc
.cursor/rules/linthis-cmsg.mdc
.cursor/rules/linthis-review.mdc
```

`alwaysApply: true` 前置信息确保规则在所有对话中生效。

### Droid

按 hook 事件创建独立规则文件：

```
.droid/rules/linthis-lint.md
.droid/rules/linthis-cmsg.md
.droid/rules/linthis-review.md
```

### Auggie

按 hook 事件创建独立规则文件：

```
.augment/rules/linthis-lint.md
.augment/rules/linthis-cmsg.md
.augment/rules/linthis-review.md
```

### CodeBuddy

技能文件按 hook 事件分别安装，共创建三个技能文件和一个 Stop Hook 设置：

1. **`.codebuddy/skills/lt-lint/SKILL.md`** — `pre-commit` 事件：暂存文件 lint 检查
2. **`.codebuddy/skills/lt-cmsg/SKILL.md`** — `commit-msg` 事件：提交信息格式验证
3. **`.codebuddy/skills/lt-review/SKILL.md`** — `pre-push` 事件：推送前代码审查
4. **`.codebuddy/settings.json`** — Stop Hook，在 AI 助手结束前触发 linthis 检查

## 工作原理

安装的规则指导 AI 助手执行以下操作：

1. **修改代码后** — 运行 `linthis -i <file1> -i <file2> -c` 检查所有修改的文件
2. **手动修复问题** — 阅读 lint 错误并直接修改代码（不使用 `--fix` 或 AI 自动修复）
3. **提交前** — 运行 `linthis -s -c` 检查暂存文件
4. **重新检查** — 修复后重新运行 linthis，直到通过

这确保 AI 助手生成符合代码规范的代码，具有正确的上下文感知能力，而非依赖自动修复工具。

## 三层 Agent Hook 解析机制

`linthis hook install --type agent` 运行时，按以下三层优先级（由高到低）解析各 agent 插件包和 Stop Hook：

| 层级 | 来源 | 使用方式 |
|------|------|---------|
| **第 1 层** | 固定路径自动发现 | 在项目根目录的 `hooks/agent/plugins/<id>/` 或 `hooks/agent/hook/stop/<provider>/` 放置文件 |
| **第 2 层** | TOML 来源映射 | 在 `.linthis/config.toml` 中设置 `[hooks.agent-plugins]` / `[hooks.agent-hook.stop]` 条目 |
| **第 3 层** | 内置生成器 | 默认——linthis 内置生成的规则内容 |

### Agent 插件包目录结构

Agent 插件包是包含以下扁平化目录布局的文件夹，各子目录均为可选：

```
<bundle-dir>/
├── skills/<skill_name>/SKILL.md    — 技能指令文件（如 skills/lt-lint/SKILL.md）
├── commands/                        — 斜杠命令文件（可选）
├── memories/TOPLEVEL.md             — 注入 CLAUDE.md 等文件的记忆段落（可选）
└── hooks/hooks.json                 — Stop Hook 设置（可选）
```

### 第 2 层：Agent Hook 的 TOML 来源映射

在 `.linthis/config.toml` 中覆盖 agent 插件包和 Stop Hook：

```toml
[hooks.agent-plugins]
"lt.lint"   = { source = { plugin = "my-plugin", file = "hooks/agent/plugins/lt/lint" } }
"lt.cmsg"   = { source = { plugin = "my-plugin", file = "hooks/agent/plugins/lt/cmsg" } }
"lt.review" = { source = { plugin = "my-plugin", file = "hooks/agent/plugins/lt/review" } }

[hooks.agent-hook.stop]
"claude.settings" = { source = { plugin = "my-plugin", file = "hooks/agent/hook/stop/claude/settings.json" } }
```

git hook 可用的五种 `HookSource` 变体同样适用于此处（参见[配置参考](../reference/configuration.md#hooksource--source-specification)）。

### 插件捆绑 Agent Hook

插件可以在插件根目录的 `linthis-config.toml` 中捆绑 agent hook 覆盖配置。当用户运行 `linthis plugin add <alias> <url>` 时，这些条目会自动合并到用户的 `.linthis/config.toml` 中。之后运行 `linthis hook install --type agent --provider claude` 将自动使用插件的定制技能/命令/记忆包和 Stop Hook 设置。

### 可配置技能目录名

已有自定义技能目录名的团队，可以在 `.linthis/config.toml` 中将 linthis 的 hook 事件映射到自定义技能目录名：

```toml
[hooks.agent-skill-names]
pre-commit = "my-team-lint"     # 默认: "lt-lint"
commit-msg = "my-team-cmsg"     # 默认: "lt-cmsg"
pre-push = "my-team-review"     # 默认: "lt-review"
```

值为 `.claude/skills/`、`.codebuddy/skills/` 下使用的目录名，同时也是扁平文件提供者的基础文件名（Gemini: `{name}.md`，Cursor: `{name}.mdc` 等）。

未配置时，使用默认值 `lt-lint`、`lt-cmsg`、`lt-review`（向后兼容）。

---

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
| `claude` | `claude` | `claude -p --dangerously-skip-permissions '...'` |
| `codex` | `codex` | `codex exec --ask-for-approval never '...'` |
| `gemini` | `gemini` | `gemini -p --approval-mode=auto_edit '...'` |
| `cursor` | `cursor-agent` | `cursor-agent chat --force '...'` |
| `droid` | `droid` | `droid exec --auto high '...'` |
| `auggie` | `auggie` | `auggie --print '...'` |
| `codebuddy` | `codebuddy` | `codebuddy -p --dangerously-skip-permissions '...'` |

### 生成的 Hook 脚本示例

以下是使用 `--type git-with-agent --provider claude` 时写入 `.git/hooks/pre-commit` 的脚本：

```bash
#!/bin/sh

LINTHIS_CMD="linthis -s -c -f --hook-event=pre-commit"

$LINTHIS_CMD
LINTHIS_EXIT=$?

if [ $LINTHIS_EXIT -ne 0 ]; then
  echo "[linthis] Lint errors detected. Invoking Claude Code to fix..."
  claude -p --dangerously-skip-permissions 'Staged files have linthis lint errors. Run '\''linthis -s -c'\'' to inspect them. Fix all issues by editing the files directly (do NOT use linthis --fix). Verify with '\''linthis -s -c'\'' until it passes cleanly.'
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
- `AGENTS.md` 中的 linthis 段落（追加式文件）
- 每事件技能文件（`.claude/skills/lt-lint/SKILL.md`、`.codebuddy/skills/lt-lint/SKILL.md` 等）
- 每事件独立规则文件（`.cursor/rules/linthis-lint.mdc`、`.gemini/linthis-lint.md` 等）
- Claude Code Stop Hook（`.claude/settings.json`）
- CodeBuddy Stop Hook（`.codebuddy/settings.json`）
- linthis 创建的空目录

## Hook 同步

重新同步所有已安装的 hook 和 agent 技能文件，确保它们是最新的：

```bash
# 同步本地项目 hook
linthis hook sync

# 同步全局 hook
linthis hook sync -g
```

此命令会重新生成 thin wrapper 脚本，并刷新所有已记录安装的 agent 技能文件。

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
