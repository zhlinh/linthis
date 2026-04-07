# Git Hooks

## 概述

linthis 与 Git 的 hook 系统集成，在提交（或推送）时自动运行 lint 检查和格式化。Hook 可以在两个作用域安装：

- **项目级** — 写入单个仓库的 `.git/hooks/<event>`
- **全局** — 写入 `~/.config/git/hooks/<event>`，通过 `git config --global core.hooksPath` 对机器上所有仓库生效

全局 hook 采用 **策略 B**：本地项目 hook 优先。如果本地 `.git/hooks/<event>` 已调用 `linthis`，全局 hook 完全委托给它。如果本地 hook 存在但不调用 `linthis`，全局 hook 先运行 `linthis`，再链式调用本地 hook。如果没有本地 hook，全局 hook 直接运行 `linthis`。此设计保证对其他 hook 工具零干扰。

所有 hook 类型——`git`、`prek`、`git-with-agent`、`prek-with-agent`——均支持在两个作用域安装。

---

## 快速开始

### 项目级 hook

```bash
# 默认：git pre-commit hook
linthis hook install

# git pre-push hook
linthis hook install --event pre-push

# commit message 格式检查 hook
linthis hook install --event commit-msg

# prek hook（适用于使用 prek 的项目）
linthis hook install --type prek
```

### 全局 hook

```bash
# 全局 git pre-commit hook（对所有仓库生效）
linthis hook install --global

# 全局 git pre-push hook
linthis hook install --global --event pre-push

# 全局 hook，非交互式
linthis hook install --global -y
```

运行 `linthis hook install --global` 后，该命令会：

1. 将 hook 脚本写入 `~/.config/git/hooks/pre-commit`
2. 执行 `git config --global core.hooksPath ~/.config/git/hooks`

机器上的每个仓库都会立即使用该 hook，无需对现有仓库重新运行 `git init`。

---

## Hook 类型

| 类型 | 运行器 | 触发方式 | 说明 |
|------|--------|---------|------|
| `git` | Git 原生 | `.git/hooks/<event>` | 默认类型；无需额外工具 |
| `prek` | [prek](https://github.com/prek-dev/prek) | prek 运行器 | 需要安装 prek；配置文件可提交到仓库 |
| `git-with-agent` | Git 原生 | `.git/hooks/<event>` | 与 `git` 相同，lint 失败时额外触发 AI 智能修复 |
| `prek-with-agent` | prek | prek 运行器 | 与 `prek` 相同，lint 失败时额外触发 AI 智能修复 |

---

## 全局 Hook

### 安装

```bash
# 安装全局 pre-commit hook（git 类型）
linthis hook install --global

# 安装全局 pre-push hook
linthis hook install --global --event pre-push

# 安装带 agent 修复的全局 hook
linthis hook install --global --type git-with-agent --provider claude

# 非交互式（跳过确认提示）
linthis hook install --global -y
```

### 工作原理

`--global` 执行两个操作：

1. **写入** `~/.config/git/hooks/<event>` — hook 脚本文件
2. **设置** `git config --global core.hooksPath ~/.config/git/hooks`

Git 的 `core.hooksPath` 使 Git 对所有仓库都从该目录查找 hook，立即生效，无需逐仓库配置。

### 目录结构

```
~/.config/git/hooks/
├── pre-commit     # 由 linthis hook install --global 安装
├── pre-push       # 由 linthis hook install --global --event pre-push 安装
└── ...
```

### 策略 B 详解

全局 hook 不会盲目运行，而是先检查当前仓库的本地 `.git/hooks/<event>`：

| 本地 hook 状态 | 全局 hook 行为 |
|----------------|----------------|
| 无本地 hook | 直接运行 `linthis` |
| 本地 hook 存在，**未**调用 `linthis` | 先运行 `linthis`，再委托给本地 hook |
| 本地 hook 存在，**已**调用 `linthis` | 完全委托（`exec "$LOCAL_HOOK" "$@"`）——`linthis` 不会重复运行 |

检测使用 `grep -qE '^[^#]*linthis'`——匹配任何包含 `linthis` 的非注释行，注释行的修改不影响检测结果。

### 生成的全局 hook 脚本示例（commit-msg，git 类型）

```bash
#!/bin/sh
# linthis-hook

LINTHIS_CMD="linthis cmsg"

# Locate the local project hook (git-dir aware)
GIT_DIR="$(git rev-parse --git-dir 2>/dev/null)"
LOCAL_HOOK=""
if [ -n "$GIT_DIR" ]; then
  LOCAL_HOOK="$GIT_DIR/hooks/commit-msg"
fi

if [ -f "$LOCAL_HOOK" ] && [ -x "$LOCAL_HOOK" ]; then
  if grep -qE '^[^#]*linthis' "$LOCAL_HOOK" 2>/dev/null; then
    # Local hook already calls linthis — delegate entirely
    exec "$LOCAL_HOOK" "$@"
  else
    # Local hook exists but has no linthis — run linthis first, then delegate
    $LINTHIS_CMD "$@"
    LINTHIS_EXIT=$?
    "$LOCAL_HOOK" "$@"
    LOCAL_EXIT=$?
    [ $LINTHIS_EXIT -ne 0 ] && exit $LINTHIS_EXIT
    exit $LOCAL_EXIT
  fi
else
  # No local hook — run linthis directly
  $LINTHIS_CMD "$@"
  LINTHIS_EXIT=$?
  exit $LINTHIS_EXIT
fi
```

注意：`$@` 将 git 的 `$1`（消息文件路径）安全传递，即使路径包含空格也能正确处理。

### 生成的全局 hook 脚本示例（pre-commit，git 类型）

```bash
#!/bin/sh
# linthis-hook

LINTHIS_CMD="linthis -s -c -f --hook-event=pre-commit"

# Locate the local project hook (git-dir aware)
GIT_DIR="$(git rev-parse --git-dir 2>/dev/null)"
LOCAL_HOOK=""
if [ -n "$GIT_DIR" ]; then
  LOCAL_HOOK="$GIT_DIR/hooks/pre-commit"
fi

if [ -f "$LOCAL_HOOK" ] && [ -x "$LOCAL_HOOK" ]; then
  if grep -qE '^[^#]*linthis' "$LOCAL_HOOK" 2>/dev/null; then
    # Local hook already calls linthis — delegate entirely
    exec "$LOCAL_HOOK" "$@"
  else
    # Local hook exists but has no linthis — run linthis first, then delegate
    $LINTHIS_CMD
    LINTHIS_EXIT=$?
    "$LOCAL_HOOK" "$@"
    LOCAL_EXIT=$?
    [ $LINTHIS_EXIT -ne 0 ] && exit $LINTHIS_EXIT
    exit $LOCAL_EXIT
  fi
else
  # No local hook — run linthis directly
  $LINTHIS_CMD
  LINTHIS_EXIT=$?
  exit $LINTHIS_EXIT
fi
```

---

## 三层 Hook 解析机制

`linthis hook install` 运行时，按以下三层优先级（由高到低）解析 hook 脚本：

| 层级 | 来源 | 使用方式 |
|------|------|---------|
| **第 1 层** | 固定路径自动发现 | 在项目根目录的 `hooks/git/<event>` 放置脚本 |
| **第 2 层** | TOML 来源映射 | 在 `.linthis/config.toml` 中设置 `[hook.git]` 条目 |
| **第 3 层** | 内置生成器 | 默认——内置生成的脚本 |

### 第 1 层：固定路径自动发现

在项目根目录的约定路径创建可执行文件：

```
hooks/git/pre-commit
hooks/git/pre-push
hooks/git/commit-msg
```

若该文件存在，linthis 直接使用，无需生成脚本，无需额外配置。

### 第 2 层：TOML 来源映射

在 `.linthis/config.toml` 中通过 `source` 条目覆盖 hook 来源。插件通过 `linthis plugin add` 添加时通常会自动注入这些条目。

```toml
[hook.git]
pre-commit = { source = { plugin = "my-plugin", file = "hooks/git/pre-commit" } }
```

支持五种来源变体：

```toml
# 本地文件（相对于项目根目录）
pre-commit = { source = { file = "hooks/git/pre-commit" } }

# 已安装插件中的文件
pre-commit = { source = { plugin = "my-plugin", file = "hooks/git/pre-commit" } }

# 来自命名市场的插件文件
pre-commit = { source = { marketplace = "corp", plugin = "linthis-official", file = "hooks/git/pre-commit" } }

# 直接 URL 下载
pre-commit = { source = { url = "https://example.com/hooks/pre-commit" } }

# 克隆 git 仓库
pre-commit = { source = { git = "https://github.com/org/hooks.git", ref = "main", path = "pre-commit" } }
```

同样的覆盖结构适用于所有 hook 类型（`[hook.git-with-agent]`、`[hook.prek]`、`[hook.prek-with-agent]` 等）。

### 插件捆绑 Hook

插件可以在插件根目录的 `linthis-hook.toml` 中捆绑 hook 覆盖配置。当用户运行 `linthis plugin add <alias> <url>` 时，linthis 自动：

1. 将 `plugin = "self"` 替换为 `plugin = "<alias>"`（用户指定的别名）
2. 将 `[hook.*]` 条目以非覆盖方式合并到用户的 `.linthis/config.toml` 中

这意味着添加团队插件即可让所有成员自动获得团队定制的 pre-commit 脚本。

---

## *-with-agent Hook 类型

`git-with-agent` 和 `prek-with-agent` 类型添加了 AI agent 修复兜底机制。当 `linthis` 以非零状态码退出（lint 失败）时，hook 会以无头模式调用指定的 agent CLI 尝试自动修复，然后重新运行 `linthis` 验证结果。

### 支持的 provider

| `--provider` 值 | Agent CLI | 无头模式命令 |
|----------------|-----------|------------|
| `claude` | Claude Code CLI | `claude -p '<prompt>'` |
| `codex` | OpenAI Codex CLI | `codex exec '<prompt>'` |
| `gemini` | Google Gemini CLI | `gemini -p '<prompt>'` |
| `cursor` | Cursor agent | `cursor-agent chat '<prompt>'` |
| `droid` | Droid | `droid exec --auto low '<prompt>'` |
| `auggie` | Auggie | `auggie --print '<prompt>'` |

`--provider` 支持 `provider/model` 语法（如 `claude/opus`），等同于 `--provider claude --provider-args "--model opus"`。使用 `--provider-args` 可向 AI agent CLI 传递额外参数。

### 示例

```bash
# 项目级：git hook，使用 Claude 修复兜底
linthis hook install --type git-with-agent --provider claude

# 项目级：prek hook，使用 Gemini 修复兜底
linthis hook install --type prek-with-agent --provider gemini

# 使用 provider/model 语法（向 agent CLI 传递 --model）
linthis hook install --type git-with-agent --provider claude/opus

# 使用显式 provider-args
linthis hook install --type git-with-agent --provider claude --provider-args "--model opus"

# 全局：git hook，使用 Claude 修复兜底
linthis hook install --global --type git-with-agent --provider claude
```

---

## hook status

查看所有已安装 hook 的当前状态：

```bash
linthis hook status
```

输出示例：

```
Git Hook Status
Repository: /path/to/repo

Project Hooks (.git/hooks/):
✓ /path/.git/hooks/pre-commit [project]
    pre-commit (runs before commit)
    ✓ linthis

Global Hooks (~/.config/git/hooks/):
  core.hooksPath = /Users/username/.config/git/hooks
  ✓ /Users/username/.config/git/hooks/pre-commit [global]
      ℹ Strategy: local hook takes priority
```

状态输出包含：

- 已安装的项目级 hook 及其是否包含 `linthis` 调用
- 已安装的全局 hook
- 当前生效的 `core.hooksPath` 设置
- 正在使用的委托策略

---

## 全局 vs 项目级对比

| 功能 | 全局（`--global`） | 项目级 |
|------|---------------------|--------|
| 作用域 | 机器上的所有仓库 | 仅当前仓库 |
| 位置 | `~/.config/git/hooks/` | `.git/hooks/` |
| 修改的 Git 配置 | `core.hooksPath`（全局） | 无 |
| 对现有仓库立即生效 | 是 | 是 |
| 可提交到仓库 | 否 | 否（`.git/` 不被追踪） |
| 团队共享 | 否 | 需要 prek 或 pre-commit 类型 |
| Hook 共存 | 策略 B（自动委托） | 手动链式调用 |
| 支持的类型 | 所有类型 | 所有类型 |

---

## 卸载

### 删除指定的全局 hook

```bash
# 删除全局 pre-commit hook
linthis hook uninstall --global

# 删除全局 pre-push hook
linthis hook uninstall --global --event pre-push

# 非交互式
linthis hook uninstall --global -y
```

### 删除所有全局 hook

```bash
linthis hook uninstall --global --all

# 非交互式
linthis hook uninstall --global --all -y
```

`--all` 会删除 `~/.config/git/hooks/` 中的所有 hook 脚本，如果没有其他 hook 剩余，还会取消 `core.hooksPath` 设置。

### 删除项目级 hook

```bash
# 删除项目 pre-commit hook
linthis hook uninstall

# 删除项目 pre-push hook
linthis hook uninstall --event pre-push
```

---

## 命令参考

```bash
# 项目级安装
linthis hook install                                               # git pre-commit
linthis hook install --event pre-push                             # git pre-push
linthis hook install --type prek                                   # prek
linthis hook install --type git-with-agent --provider claude       # git + agent 修复
linthis hook install --type git-with-agent --provider claude/opus  # git + agent 修复（指定 model）
linthis hook install --type prek-with-agent --provider gemini      # prek + agent 修复

# 全局安装
linthis hook install --global                                      # 全局 git pre-commit
linthis hook install --global --event pre-push                     # 全局 git pre-push
linthis hook install --global --type git-with-agent --provider claude  # 全局 + agent 修复
linthis hook install --global -y                                   # 非交互式

# 卸载
linthis hook uninstall                                             # 删除项目 pre-commit
linthis hook uninstall --global                                    # 删除全局 pre-commit
linthis hook uninstall --global --all                              # 删除所有全局 hook
linthis hook uninstall --global -y                                 # 非交互式

# 状态
linthis hook status
```

---

## 常见问题

### Q1：全局 hook 和项目级 hook 可以共存吗？

可以。这正是策略 B 的主要使用场景。如果项目有调用 `linthis` 的 `.git/hooks/pre-commit`，全局 hook 会检测到并完全委托——`linthis` 只运行一次，不会重复。如果项目 hook 不调用 `linthis`，全局 hook 会先运行 `linthis`，再调用项目 hook。

### Q2：策略 B 如何检测本地 hook 是否调用 linthis？

运行 `grep -qE '^[^#]*linthis' "$LOCAL_HOOK"`。该模式匹配任何包含字符串 `linthis` 的非注释行（`^[^#]*`）。以 `#` 开头的注释行会被忽略。因此修改注释（例如添加 `# previously used linthis`）不会影响检测结果——只有可执行行才算数。

### Q3：如何针对特定仓库禁用全局 hook？

安装一个调用 `linthis` 的项目级 hook。全局 hook 会检测到并委托，项目 hook 成为唯一入口，你可以完全控制该仓库中 `linthis` 的调用方式。

如果你希望某个仓库完全不运行 `linthis`，最简洁的方式是卸载全局 hook，改用项目级 hook 管理。

### Q4：全局 hook 会影响不使用 linthis 的仓库吗？

hook 会尝试运行 `linthis -s -c -f --hook-event=pre-commit`。如果仓库没有 linthis 配置文件（`.linthis/config.toml`、`.linthis.toml` 或 `linthis.toml`），`linthis` 会立即退出且不报错，提交正常进行。

### Q5：我使用了 `--type git-with-agent`，但 agent CLI 未安装会怎样？

hook 首先运行 `linthis`。如果 `linthis` 成功退出，agent 永远不会被调用。如果 `linthis` 失败且 agent CLI 二进制文件不存在，hook 会打印警告并以原始 `linthis` 退出码退出，提交依然被阻断。

### Q6：`--type prek` 或 `--type pre-commit` 可以与 `--global` 一起使用吗？

可以。所有 hook 类型都支持 `--global`。写入 `~/.config/git/hooks/<event>` 的 hook 脚本会调用相应的运行器（`prek` 或 `pre-commit`）而非直接调用 `linthis`。策略 B 委托逻辑同样适用。

### Q7：如何查看当前生效的 `core.hooksPath`？

```bash
git config --global --get core.hooksPath
# 输出：/Users/username/.config/git/hooks
```

如果没有输出，说明未设置全局 `core.hooksPath`，Git 按常规使用各仓库的 `.git/hooks/` 目录。

---

## 参考资料

- [AI 智能修复](./ai-fix.md) — AI provider 详情
- [AI 编程助手集成](./agent-hooks.md) — 基于规则的 agent 集成
- [CLI 参考](../reference/cli.md) — 完整命令参考
- [Git 文档 — core.hooksPath](https://git-scm.com/docs/git-config#Documentation/git-config.txt-corehooksPath)
