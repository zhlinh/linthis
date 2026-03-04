# AI 智能修复

linthis 集成了 AI 提供商，可以自动建议并应用 lint 问题的修复。此功能将传统代码检查与 AI 智能相结合，提供上下文感知的代码修正。

## 快速开始

```bash
# 交互式 AI 修复模式
linthis --fix --ai

# 指定 AI 提供商
linthis --fix --ai --provider claude

# 自动接受所有修复（用于 CI/自动化）
linthis --fix --ai --provider claude-cli --accept-all
```

## 支持的提供商

| 提供商 | 描述 | 认证方式 |
|-------|------|---------|
| `claude` | Anthropic Claude API（默认） | `ANTHROPIC_API_KEY` 或 `ANTHROPIC_AUTH_TOKEN` |
| `claude-cli` | Claude CLI（`claude -p` 命令） | Claude CLI 登录 |
| `codebuddy` | CodeBuddy API | `CODEBUDDY_API_KEY` |
| `codebuddy-cli` | CodeBuddy CLI | CodeBuddy CLI 登录 |
| `openai` | OpenAI API | `OPENAI_API_KEY` |
| `local` | 本地 LLM（Ollama 等） | 无需认证（本地端点） |
| `mock` | 模拟提供商（用于测试） | 无需认证 |

## 提供商优先级

AI 提供商按以下顺序解析：

1. **命令行参数** (`--provider <name>`) - 最高优先级
2. **环境变量** (`LINTHIS_AI_PROVIDER`)
3. **配置文件** (`.linthis/config.toml` 中的 `[ai]` 部分)
4. **默认值** - `claude`

## 使用模式

### 交互模式

审查并选择性地应用 AI 建议的修复：

```bash
linthis -i src/ --fix --ai
```

在交互模式下，对于每个问题你可以：
- **接受 (y)** - 应用建议的修复
- **拒绝 (n)** - 跳过此建议
- **编辑 (e)** - 应用前修改建议
- **查看差异 (d)** - 查看建议的更改
- **退出 (q)** - 退出修复模式

### 自动模式

自动接受所有 AI 修复，无需提示：

```bash
linthis --fix --ai --accept-all
```

⚠️ **警告**：这将自动修改文件。请谨慎使用，并确保有版本控制。

### 指定提供商

```bash
# 使用 Claude API
linthis --fix --ai --provider claude

# 使用 Claude CLI（运行 `claude -p` 命令）
linthis --fix --ai --provider claude-cli

# 使用 CodeBuddy
linthis --fix --ai --provider codebuddy

# 使用本地 LLM
linthis --fix --ai --provider local
```

## 配置

### 环境变量

```bash
# 设置默认提供商
export LINTHIS_AI_PROVIDER=claude-cli

# 不同提供商的 API 密钥
export ANTHROPIC_API_KEY=sk-ant-xxx
export CODEBUDDY_API_KEY=xxx
export OPENAI_API_KEY=sk-xxx

# 自定义端点
export ANTHROPIC_BASE_URL=https://api.anthropic.com
export LINTHIS_AI_ENDPOINT=http://localhost:11434  # 用于本地 LLM

# 自定义模型
export LINTHIS_AI_MODEL=claude-sonnet-4-20250514
```

### 配置文件

添加到 `.linthis/config.toml`：

```toml
[ai]
provider = "claude"      # 默认提供商
model = "claude-sonnet-4-20250514"  # 可选：覆盖默认模型
```

## Git Hook 集成

在 pre-commit hook 中使用 AI 修复自动修复问题：

```bash
# 在 .git/hooks/pre-commit 或 .prek/pre-commit 中
linthis -s --fix --ai --accept-all
```

更安全的 CI 用法（使用 Claude CLI）：

```bash
linthis -s --fix --ai --provider claude-cli --accept-all
```

## 示例

### 修复 Python 文件

```bash
# 使用 AI 检查并修复 Python 文件
linthis -i "*.py" --fix --ai

# 仅修复暂存的 Python 文件
linthis -s -l python --fix --ai --provider claude-cli
```

### 修复多种语言

```bash
# 修复 Python 和 TypeScript 文件
linthis -l python,typescript --fix --ai
```

### CI/CD 集成

```yaml
# GitHub Actions 示例
- name: Lint and Fix
  run: |
    linthis --fix --ai --provider claude --accept-all
    git diff --exit-code || (git add -A && git commit -m "style: auto-fix lint issues")
```

## 提供商详情

### Claude (API)

直接使用 Anthropic Claude API。需要 API 密钥。

```bash
export ANTHROPIC_API_KEY=sk-ant-xxx
linthis --fix --ai --provider claude
```

### Claude CLI

使用 `claude` CLI 工具的 `-p`（打印）模式。CLI 处理认证。

```bash
# 确保 claude CLI 已安装并登录
claude --version

# 使用 CLI 提供商
linthis --fix --ai --provider claude-cli
```

### 本地 LLM

连接到本地 LLM 服务器（如 Ollama）：

```bash
# 使用编程模型启动 Ollama
ollama run codellama:7b

# 配置端点
export LINTHIS_AI_ENDPOINT=http://localhost:11434

# 使用本地提供商
linthis --fix --ai --provider local
```

## 故障排除

### "API key not found"（未找到 API 密钥）

为你的提供商设置相应的环境变量：

```bash
# 对于 Claude API
export ANTHROPIC_API_KEY=your-key-here

# 对于 OpenAI
export OPENAI_API_KEY=your-key-here
```

### "Provider not available"（提供商不可用）

对于 CLI 提供商，确保 CLI 工具已安装：

```bash
# 检查 Claude CLI
claude --version

# 检查 CodeBuddy CLI
codebuddy --version
```

### 速率限制

如果遇到速率限制，考虑：
- 使用 `--provider local` 进行无限制的本地处理
- 分批处理文件
- 在请求之间添加延迟

## 观看演示

观看 [AI 修复视频教程](../getting-started/videos.md#4-ai)，20 秒了解 AI 修复前后对比。

## 参见

- [CLI 参考](../reference/cli.zh.md) - 完整命令参考
- [配置](../getting-started/configuration.zh.md) - 配置选项
- [Git Hooks](./git-hooks.zh.md) - Hook 集成
