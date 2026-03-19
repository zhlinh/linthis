# CLI 参考

linthis 所有命令和选项的完整参考。

## 主命令

```bash
linthis [OPTIONS] [COMMAND]
```

### 全局选项

| 短选项 | 长选项 | 描述 | 示例 |
|-------|-------|------|------|
| `-i` | `--include` | 要检查的文件/目录 | `-i src -i lib` |
| `-e` | `--exclude` | 要排除的模式 | `-e "*.test.js"` |
| `-c` | `--check-only` | 仅检查，不格式化 | `-c` |
| `-f` | `--format-only` | 仅格式化，不检查 | `-f` |
| `-s` | `--staged` | 仅检查 Git 暂存文件 | `-s` |
| `-m` | `--modified` | 检查所有本地修改文件（暂存 + 未暂存） | `-m` |
| `-l` | `--lang` | 语言（逗号分隔） | `-l python,rust` |
| `-o` | `--output` | 输出格式 | `-o json` |
| `-v` | `--verbose` | 详细输出 | `-v` |
| `-q` | `--quiet` | 安静模式（仅错误） | `-q` |
| | `--config` | 配置文件路径 | `--config custom.toml` |
| | `--preset` | 格式化预设 | `--preset google` |
| | `--no-default-excludes` | 禁用默认排除项 | |
| | `--no-gitignore` | 禁用 .gitignore 规则 | |
| | `--no-plugin` | 跳过加载插件 | |

### 输出格式

- `human` - 人类可读（默认）
- `json` - JSON 格式
- `github-actions` - GitHub Actions 注释

---

## init

初始化配置文件。

```bash
linthis init [OPTIONS]
```

| 选项 | 描述 |
|-----|------|
| `-g, --global` | 创建全局配置 |
| `--with-hook` | 同时安装 git hook |
| `--force` | 强制覆盖现有文件 |

**示例：**

```bash
linthis init                    # 创建 .linthis.toml
linthis init -g                 # 创建 ~/.linthis/config.toml
linthis init --with-hook        # 初始化配置并安装 hook
```

---

## hook

管理 Git hooks。

### hook install

```bash
linthis hook install [OPTIONS]
```

| 选项 | 描述 |
|-----|------|
| `--type` | Hook 类型：`git`（默认）、`git-with-agent`、`agent`、`prek`、`prek-with-agent` |
| `--event` | Hook 事件：`pre-commit`（默认）、`pre-push`、`commit-msg` |
| `--args` | Hook 脚本中 linthis 命令的额外参数（默认：`-c -f`，检查 + 格式化） |
| `-g, --global` | 全局安装：agent 类型 → 用户主目录；其他类型 → `~/.config/git/hooks/` + `core.hooksPath` |
| `--provider` | AI 提供商：`claude`、`codex`、`gemini`、`cursor`、`droid`、`auggie`、`codebuddy`。支持 `provider/model` 语法（如 `claude/opus`）。用于 `--type agent`：安装规则/设置文件。用于 `*-with-agent`：使用无头 CLI 自动修复。 |
| `--provider-args` | 传递给 AI agent CLI 的额外参数（如 `"--model opus"`）。与 `provider/model` 语法中的 model 合并（若同时指定）。 |
| `--force` | 强制覆盖现有 hook |
| `-y, --yes` | 非交互模式 |

**示例：**

```bash
# 项目级 hook
linthis hook install                                           # 默认 git hook（检查 + 格式化）
linthis hook install --event pre-push                         # Pre-push hook
linthis hook install --event commit-msg                       # Commit message 格式检查 hook
linthis hook install --args "-c"                              # 仅检查模式
linthis hook install --type git-with-agent --provider claude  # git hook + AI 自动修复失败
linthis hook install --type agent --provider claude           # Agent 集成

# 全局 hook（适用于此机器上的所有仓库）
linthis hook install --global                                 # 全局 git pre-commit
linthis hook install --global --event commit-msg              # 全局 commit-msg hook
linthis hook install --global --type git-with-agent --provider claude  # 全局 + AI 自动修复
linthis hook install --type agent --provider claude --global  # AI agent 规则（用户主目录）
```

### hook uninstall

```bash
linthis hook uninstall [OPTIONS]
```

| 选项 | 描述 |
|-----|------|
| `--event` | 要卸载的 hook 事件 |
| `-g, --global` | 卸载全局 hook |
| `--all` | 卸载所有 hook |
| `-y, --yes` | 非交互模式 |

**示例：**

```bash
linthis hook uninstall                  # 卸载项目 pre-commit hook
linthis hook uninstall --global         # 卸载全局 pre-commit hook
linthis hook uninstall --global --all   # 卸载所有全局 hook
```

### hook status

```bash
linthis hook status
```

### hook check

```bash
linthis hook check
```

---

## plugin

管理插件。

### plugin add

```bash
linthis plugin add <ALIAS> <URL> [OPTIONS]
```

| 选项 | 描述 |
|-----|------|
| `-g, --global` | 添加到全局配置 |
| `--ref` | Git 引用（分支/标签/提交） |

**示例：**

```bash
linthis plugin add myconfig https://github.com/user/config.git
linthis plugin add -g company https://github.com/company/standards.git
linthis plugin add myconfig https://github.com/user/config.git --ref v1.0.0
```

### plugin remove

```bash
linthis plugin remove <ALIAS> [OPTIONS]
```

| 选项 | 描述 |
|-----|------|
| `-g, --global` | 从全局配置移除 |

### plugin list

```bash
linthis plugin list [OPTIONS]
```

| 选项 | 描述 |
|-----|------|
| `-g, --global` | 列出全局插件 |
| `-v, --verbose` | 显示详细信息 |

### plugin sync

```bash
linthis plugin sync [OPTIONS]
```

| 选项 | 描述 |
|-----|------|
| `--global` | 同步全局插件 |

### plugin init

```bash
linthis plugin init <NAME>
```

### plugin validate

```bash
linthis plugin validate <PATH>
```

### plugin clean

```bash
linthis plugin clean [OPTIONS]
```

| 选项 | 描述 |
|-----|------|
| `--all` | 清理所有缓存 |

---

## config

管理配置。

### config add

```bash
linthis config add <FIELD> <VALUE> [OPTIONS]
```

**支持的字段：** `includes`、`excludes`、`languages`

| 选项 | 描述 |
|-----|------|
| `-g, --global` | 添加到全局配置 |

### config remove

```bash
linthis config remove <FIELD> <VALUE> [OPTIONS]
```

### config clear

```bash
linthis config clear <FIELD> [OPTIONS]
```

### config set

```bash
linthis config set <FIELD> <VALUE> [OPTIONS]
```

**支持的字段：** `max_complexity`、`preset`、`verbose`

### config unset

```bash
linthis config unset <FIELD> [OPTIONS]
```

### config get

```bash
linthis config get <FIELD> [OPTIONS]
```

### config list

```bash
linthis config list [OPTIONS]
```

| 选项 | 描述 |
|-----|------|
| `-g, --global` | 列出全局配置 |
| `-v, --verbose` | 显示所有字段 |

### config migrate

```bash
linthis config migrate [OPTIONS]
```

| 选项 | 描述 |
|-----|------|
| `--from` | 迁移特定工具 |
| `--dry-run` | 预览更改 |
| `--backup` | 创建备份 |
| `-v, --verbose` | 详细输出 |

---

## fix

交互式修复模式，支持可选的 AI 辅助。

```bash
linthis --fix [OPTIONS]
linthis fix [OPTIONS]
```

### 修复选项

| 选项 | 描述 | 示例 |
|-----|------|------|
| `--fix` | 检查/格式化后进入修复模式 | `--fix` |
| `--ai` | 使用 AI 进行修复建议（需要 `--fix`） | `--fix --ai` |
| `--provider` | AI 提供商（需要 `--ai`） | `--provider claude` |
| `-y` | 自动接受所有修复（需要 `--fix`） | `--fix -y` |

### AI 提供商

| 提供商 | 描述 |
|-------|------|
| `claude` | Anthropic Claude API（默认） |
| `claude-cli` | Claude CLI（`claude -p` 命令） |
| `codebuddy` | CodeBuddy API |
| `codebuddy-cli` | CodeBuddy CLI |
| `openai` | OpenAI API |
| `local` | 本地 LLM（Ollama 等） |
| `mock` | 模拟提供商（用于测试） |

### 提供商优先级

1. 命令行参数 (`--provider`)
2. 环境变量 (`LINTHIS_AI_PROVIDER`)
3. 配置文件 (`[ai]` 部分)
4. 默认值：`claude`

**示例：**

```bash
# 交互式修复模式（手动审查）
linthis -i src/ --fix

# AI 辅助修复，交互式审查
linthis -i src/ --fix --ai

# 使用特定提供商的 AI 修复
linthis --fix --ai --provider claude
linthis --fix --ai --provider claude-cli

# 自动接受所有 AI 修复（用于 CI/自动化）
linthis --auto-fix
linthis --auto-fix --provider claude-cli

# 仅修复暂存文件并使用 AI
linthis -s --auto-fix --provider claude-cli

# 修复特定语言
linthis -l python --fix --ai --provider claude
```

详见 [AI 智能修复](../features/ai-fix.zh.md)。

---

## cmsg

验证 commit message 格式（Conventional Commits）。

```bash
linthis cmsg <MSG_OR_FILE> [OPTIONS]
```

| 参数 | 描述 |
|-----|------|
| `MSG_OR_FILE` | commit message 文件路径（如 `.git/COMMIT_EDITMSG`），或直接传入 commit message 字符串 |

| 选项 | 描述 |
|-----|------|
| `--auto-fix` | 当验证失败时，使用 AI 自动重写 commit message |
| `--provider` | AI 自动修复所用提供商（需要 `--auto-fix`） |

**示例：**

```bash
# 直接验证 commit message 字符串
linthis cmsg "feat: add login page"
linthis cmsg "fix(auth): handle token expiry"

# 通过文件路径验证（由 git commit-msg hook 调用）
linthis cmsg .git/COMMIT_EDITMSG

# 验证失败时 AI 自动重写（并写回文件）
linthis cmsg .git/COMMIT_EDITMSG --auto-fix
linthis cmsg .git/COMMIT_EDITMSG --auto-fix --provider claude-cli

# 安装 commit-msg hook（自动调用 linthis cmsg）
linthis hook install --event commit-msg
linthis hook install --global --event commit-msg
```

默认模式要求遵循 [Conventional Commits](https://www.conventionalcommits.org/) 格式：

```
type(scope)?: description
```

允许的类型：`feat`、`fix`、`docs`、`style`、`refactor`、`perf`、`test`、`build`、`ci`、`chore`、`revert`

**配置（`.linthis.toml`）：**

```toml
[cmsg]
commit_msg_pattern = '^(feat|fix|docs|style|refactor|perf|test|build|ci|chore|revert)(\(.+\))?: .{1,72}'
require_ticket = false
# ticket_pattern = '^(PROJ-\d+|feat|fix|...)'
```

---

## format

格式化文件，自动备份并支持撤销。

```bash
linthis format [OPTIONS]
```

| 选项 | 描述 |
|-----|------|
| `-s, --staged` | 仅格式化 Git 暂存文件 |
| `-m, --modified` | 仅格式化本地修改文件（暂存 + 未暂存） |
| `-i, --include` | 要格式化的文件/目录 |
| `--undo` | 从上次备份恢复文件 |
| `--list-backups` | 列出可用的备份 |
| `--source` | 撤销时使用的备份来源（默认：最新） |

**示例：**

```bash
linthis format                        # 格式化所有文件
linthis format -s                     # 格式化暂存文件
linthis format -m                     # 格式化修改过的文件
linthis format -i src/main.rs         # 格式化指定文件
linthis format --undo                 # 撤销上次格式化
linthis format --list-backups         # 列出可用备份
```

每次格式化操作前会自动创建备份。如果格式化结果不符合预期，可通过 `--undo` 恢复。

---

## review

AI 代码审查，支持自动创建 PR/MR。

```bash
linthis review [OPTIONS]
```

| 选项 | 描述 |
|-----|------|
| `-b, --background` | 在后台运行 review（非阻塞） |
| `--auto-fix` | Review + 自动修复 + 创建 PR/MR |
| `-r, --reviewer` | 指定 PR/MR 的审查人（可重复） |
| `--provider` | AI 提供商 |
| `--base` | 对比的基础分支/提交 |
| `--head` | 要 review 的 HEAD 引用（默认：`HEAD`） |
| `--no-pr` | 仅生成报告，不创建 PR/MR |
| `--notify` | 通知渠道（可重复） |
| `--status` | 查看后台 review 状态 |
| `--dry-run` | 预览自动修复操作，不推送或创建 PR |
| `--clean` | 清理旧的 review 产物 |
| `-o, --output` | 输出格式：`markdown`（默认）或 `json` |

**示例：**

```bash
linthis review                        # 对当前分支与远端进行 review
linthis review --auto-fix             # Review + 自动修复 + 创建 PR
linthis review -r alice -r bob        # 指定审查人
linthis review --base main            # 与 main 分支对比
linthis review --background           # 后台运行（非阻塞）
linthis review --status               # 查看后台 review 状态
linthis review --no-pr                # 仅生成 Markdown 报告
linthis review --dry-run              # 预览操作，不推送
```

**支持的平台：**

| 平台 | 识别方式 | CLI 工具 |
|-----|---------|---------|
| GitHub | `github.com` remote | `gh` |
| GitLab | `gitlab.com` / 自托管 | `glab` |

**配置（`.linthis.toml`）：**

```toml
[review]
enabled = true
auto_fix = false
provider = "claude-cli"
retention_days = 30

[review.reviewers]
default = ["alice", "bob"]
```

---

## watch

监视模式，持续检查。

```bash
linthis watch [OPTIONS]
```

详见[监视模式](../features/watch-mode.md)。

---

## doctor

检查工具可用性。

```bash
linthis doctor [OPTIONS]
```

| 选项 | 描述 |
|-----|------|
| `-l, --lang` | 检查特定语言 |

---

## 退出码

| 代码 | 含义 |
|-----|------|
| 0 | 成功（无问题或所有问题已修复） |
| 1 | 发现 lint/format 问题 |
| 2 | 配置错误 |
| 3 | 工具不可用 |
