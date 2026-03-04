# AI-Powered Fix

linthis integrates with AI providers to automatically suggest and apply fixes for lint issues. This feature combines traditional linting with AI intelligence to provide context-aware code corrections.

## Quick Start

```bash
# Interactive AI fix mode
linthis --fix --ai

# Specify AI provider
linthis --fix --ai --provider claude

# Auto-accept all fixes (for CI/automation)
linthis --fix --ai --provider claude-cli --accept-all
```

## Supported Providers

| Provider | Description | Authentication |
|----------|-------------|----------------|
| `claude` | Anthropic Claude API (default) | `ANTHROPIC_API_KEY` or `ANTHROPIC_AUTH_TOKEN` |
| `claude-cli` | Claude CLI (`claude -p` command) | Claude CLI login |
| `codebuddy` | CodeBuddy API | `CODEBUDDY_API_KEY` |
| `codebuddy-cli` | CodeBuddy CLI | CodeBuddy CLI login |
| `openai` | OpenAI API | `OPENAI_API_KEY` |
| `local` | Local LLM (Ollama, etc.) | None (local endpoint) |
| `mock` | Mock provider for testing | None |

## Provider Priority

The AI provider is resolved in the following order:

1. **Command line** (`--provider <name>`) - highest priority
2. **Environment variable** (`LINTHIS_AI_PROVIDER`)
3. **Config file** (`[ai]` section in `.linthis/config.toml`)
4. **Default** - `claude`

## Usage Modes

### Interactive Mode

Review and selectively apply AI-suggested fixes:

```bash
linthis -i src/ --fix --ai
```

In interactive mode, for each issue you can:
- **Accept (y)** - Apply the suggested fix
- **Reject (n)** - Skip this suggestion
- **Edit (e)** - Modify the suggestion before applying
- **View diff (d)** - See the proposed changes
- **Quit (q)** - Exit fix mode

### Automatic Mode

Automatically accept all AI fixes without prompting:

```bash
linthis --fix --ai --accept-all
```

⚠️ **Warning**: This will modify files automatically. Use with caution and ensure you have version control.

### With Specific Provider

```bash
# Use Claude API
linthis --fix --ai --provider claude

# Use Claude CLI (runs `claude -p` command)
linthis --fix --ai --provider claude-cli

# Use CodeBuddy
linthis --fix --ai --provider codebuddy

# Use local LLM
linthis --fix --ai --provider local
```

## Configuration

### Environment Variables

```bash
# Set default provider
export LINTHIS_AI_PROVIDER=claude-cli

# API keys for different providers
export ANTHROPIC_API_KEY=sk-ant-xxx
export CODEBUDDY_API_KEY=xxx
export OPENAI_API_KEY=sk-xxx

# Custom endpoints
export ANTHROPIC_BASE_URL=https://api.anthropic.com
export LINTHIS_AI_ENDPOINT=http://localhost:11434  # for local LLM

# Custom model
export LINTHIS_AI_MODEL=claude-sonnet-4-20250514
```

### Config File

Add to `.linthis/config.toml`:

```toml
[ai]
provider = "claude"      # default provider
model = "claude-sonnet-4-20250514"  # optional: override default model
```

## Git Hook Integration

Use AI fix in pre-commit hooks to automatically fix issues:

```bash
# In .git/hooks/pre-commit or .prek/pre-commit
linthis -s --fix --ai --accept-all
```

For safer CI usage with Claude CLI:

```bash
linthis -s --fix --ai --provider claude-cli --accept-all
```

## Examples

### Fix Python Files

```bash
# Check and fix Python files with AI
linthis -i "*.py" --fix --ai

# Fix only staged Python files
linthis -s -l python --fix --ai --provider claude-cli
```

### Fix Multiple Languages

```bash
# Fix Python and TypeScript files
linthis -l python,typescript --fix --ai
```

### CI/CD Integration

```yaml
# GitHub Actions example
- name: Lint and Fix
  run: |
    linthis --fix --ai --provider claude --accept-all
    git diff --exit-code || (git add -A && git commit -m "style: auto-fix lint issues")
```

## Provider Details

### Claude (API)

Uses Anthropic Claude API directly. Requires API key.

```bash
export ANTHROPIC_API_KEY=sk-ant-xxx
linthis --fix --ai --provider claude
```

### Claude CLI

Uses the `claude` CLI tool with `-p` (print) mode. The CLI handles authentication.

```bash
# Ensure claude CLI is installed and logged in
claude --version

# Use CLI provider
linthis --fix --ai --provider claude-cli
```

### Local LLM

Connect to local LLM servers like Ollama:

```bash
# Start Ollama with a coding model
ollama run codellama:7b

# Configure endpoint
export LINTHIS_AI_ENDPOINT=http://localhost:11434

# Use local provider
linthis --fix --ai --provider local
```

## Troubleshooting

### "API key not found"

Set the appropriate environment variable for your provider:

```bash
# For Claude API
export ANTHROPIC_API_KEY=your-key-here

# For OpenAI
export OPENAI_API_KEY=your-key-here
```

### "Provider not available"

For CLI providers, ensure the CLI tool is installed:

```bash
# Check Claude CLI
claude --version

# Check CodeBuddy CLI
codebuddy --version
```

### Rate Limits

If hitting rate limits, consider:
- Using `--provider local` for unlimited local processing
- Processing files in smaller batches
- Adding delays between requests

## See It in Action

Watch the [AI Fix video tutorial](../getting-started/videos.md#episode-4-ai-powered-fix) for a 20-second demo showing before/after AI-powered fixes.

## See Also

- [CLI Reference](../reference/cli.md) - Complete command reference
- [Configuration](../getting-started/configuration.md) - Configuration options
- [Git Hooks](./git-hooks.md) - Hook integration
