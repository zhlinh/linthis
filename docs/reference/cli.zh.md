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
| `--type` | Hook 类型：`git`（默认）、`agent`、`prek`、`pre-commit` |
| `--event` | Hook 事件：`pre-commit`（默认）、`pre-push`、`commit-msg` |
| `--args` | Hook 脚本中 linthis 命令的额外参数（默认：`-c -f`） |
| `--provider` | Agent 提供者（仅 `--type agent`）：`claude`、`cursor`、`windsurf`、`copilot`、`cline`、`codebuddy` |
| `--force` | 强制覆盖现有 hook |
| `-y, --yes` | 非交互模式 |

**示例：**

```bash
linthis hook install                                  # 默认 git hook（检查 + 格式化）
linthis hook install --event pre-push                 # Pre-push hook
linthis hook install --args "-c"                      # 仅检查模式
linthis hook install --type agent --provider claude   # Agent 集成
```

### hook uninstall

```bash
linthis hook uninstall [OPTIONS]
```

| 选项 | 描述 |
|-----|------|
| `--event` | 要卸载的 hook 事件 |
| `-y, --yes` | 非交互模式 |

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
linthis --fix --ai -y
linthis --fix --ai --provider claude-cli -y

# 仅修复暂存文件并使用 AI
linthis -s --fix --ai --provider claude-cli -y

# 修复特定语言
linthis -l python --fix --ai --provider claude
```

详见 [AI 智能修复](../features/ai-fix.zh.md)。

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
