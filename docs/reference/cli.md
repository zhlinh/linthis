# CLI Reference

Complete reference for all linthis commands and options.

## Main Command

```bash
linthis [OPTIONS] [COMMAND]
```

### Global Options

| Short | Long | Description | Example |
|-------|------|-------------|---------|
| `-i` | `--include` | Files/directories to check | `-i src -i lib` |
| `-e` | `--exclude` | Patterns to exclude | `-e "*.test.js"` |
| `-c` | `--check-only` | Check only, no formatting | `-c` |
| `-f` | `--format-only` | Format only, no checking | `-f` |
| `-s` | `--staged` | Check Git staged files only | `-s` |
| `-m` | `--modified` | Check all locally modified files (staged + unstaged) | `-m` |
| `-l` | `--lang` | Languages (comma-separated) | `-l python,rust` |
| `-o` | `--output` | Output format | `-o json` |
| `-v` | `--verbose` | Verbose output | `-v` |
| `-q` | `--quiet` | Quiet mode (errors only) | `-q` |
| | `--config` | Config file path | `--config custom.toml` |
| | `--preset` | Format preset | `--preset google` |
| | `--no-default-excludes` | Disable default excludes | |
| | `--no-gitignore` | Disable .gitignore rules | |
| | `--no-plugin` | Skip loading plugins | |

### Output Formats

- `human` - Human-readable (default)
- `json` - JSON format
- `github-actions` - GitHub Actions annotations

---

## init

Initialize configuration files.

```bash
linthis init [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `-g, --global` | Create global config |
| `--with-hook` | Also install git hook |
| `--force` | Force overwrite existing |

**Examples:**

```bash
linthis init                    # Create .linthis.toml
linthis init -g                 # Create ~/.linthis/config.toml
linthis init --with-hook        # Init config and install hook
```

---

## hook

Manage Git hooks.

### hook install

```bash
linthis hook install [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `--type` | Hook type: `git` (default), `git-with-agent`, `agent`, `prek`, `prek-with-agent`, `pre-commit`, `pre-commit-with-agent` |
| `--event` | Hook event: `pre-commit` (default), `pre-push`, `commit-msg` |
| `--args` | Extra arguments for the linthis command in hook script (default: none, runs check + format) |
| `-g, --global` | Install globally: agent type → user home dir; others → `~/.config/git/hooks/` + `core.hooksPath` |
| `--provider` | AI provider: `claude`, `codex`, `gemini`, `cursor`, `droid`, `auggie`, `codebuddy`. For `--type agent`: installs rules/settings files. For `*-with-agent`: uses headless CLI to auto-fix. |
| `--force` | Force overwrite existing |
| `-y, --yes` | Non-interactive mode |

**Examples:**

```bash
# Project-level hooks
linthis hook install                                           # Default git hook (check + format)
linthis hook install --event pre-push                         # Pre-push hook
linthis hook install --event commit-msg                       # Commit message format hook
linthis hook install --args "-c"                              # Check-only hook
linthis hook install --type git-with-agent --provider claude  # git hook + AI auto-fix on failure
linthis hook install --type agent --provider claude           # Agent integration

# Global hooks (apply to all repos on this machine)
linthis hook install --global                                 # Global git pre-commit
linthis hook install --global --event commit-msg              # Global commit-msg hook
linthis hook install --global --type git-with-agent --provider claude  # Global + AI auto-fix
linthis hook install --type agent --provider claude --global  # AI agent rules (user home)
```

### hook uninstall

```bash
linthis hook uninstall [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `--event` | Hook event to uninstall |
| `-g, --global` | Uninstall global hook |
| `--all` | Uninstall all hooks |
| `-y, --yes` | Non-interactive mode |

**Examples:**

```bash
linthis hook uninstall                  # Uninstall project pre-commit hook
linthis hook uninstall --global         # Uninstall global pre-commit hook
linthis hook uninstall --global --all   # Uninstall all global hooks
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

Manage plugins.

### plugin add

```bash
linthis plugin add <ALIAS> <URL> [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `-g, --global` | Add to global config |
| `--ref` | Git reference (branch/tag/commit) |

**Examples:**

```bash
linthis plugin add myconfig https://github.com/user/config.git
linthis plugin add -g company https://github.com/company/standards.git
linthis plugin add myconfig https://github.com/user/config.git --ref v1.0.0
```

### plugin remove

```bash
linthis plugin remove <ALIAS> [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `-g, --global` | Remove from global config |

### plugin list

```bash
linthis plugin list [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `-g, --global` | List global plugins |
| `-v, --verbose` | Show detailed info |

### plugin sync

```bash
linthis plugin sync [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `--global` | Sync global plugins |

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

| Option | Description |
|--------|-------------|
| `--all` | Clean all caches |

---

## config

Manage configuration.

### config add

```bash
linthis config add <FIELD> <VALUE> [OPTIONS]
```

**Supported fields:** `includes`, `excludes`, `languages`

| Option | Description |
|--------|-------------|
| `-g, --global` | Add to global config |

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

**Supported fields:** `max_complexity`, `preset`, `verbose`

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

| Option | Description |
|--------|-------------|
| `-g, --global` | List global config |
| `-v, --verbose` | Show all fields |

### config migrate

```bash
linthis config migrate [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `--from` | Migrate specific tool |
| `--dry-run` | Preview changes |
| `--backup` | Create backup |
| `-v, --verbose` | Verbose output |

---

## fix

Interactive fix mode with optional AI assistance.

```bash
linthis --fix [OPTIONS]
linthis fix [OPTIONS]
```

### Fix Options

| Option | Description | Example |
|--------|-------------|---------|
| `--fix` | Enter fix mode after check/format | `--fix` |
| `--ai` | Use AI for fix suggestions (requires `--fix`) | `--fix --ai` |
| `--provider` | AI provider (requires `--ai`) | `--provider claude` |
| `-y` | Auto-accept all fixes (requires `--fix`) | `--fix -y` |

### AI Providers

| Provider | Description |
|----------|-------------|
| `claude` | Anthropic Claude API (default) |
| `claude-cli` | Claude CLI (`claude -p` command) |
| `codebuddy` | CodeBuddy API |
| `codebuddy-cli` | CodeBuddy CLI |
| `openai` | OpenAI API |
| `local` | Local LLM (Ollama, etc.) |
| `mock` | Mock provider for testing |

### Provider Priority

1. Command line (`--provider`)
2. Environment variable (`LINTHIS_AI_PROVIDER`)
3. Config file (`[ai]` section)
4. Default: `claude`

**Examples:**

```bash
# Interactive fix mode (manual review)
linthis -i src/ --fix

# AI-powered fix with interactive review
linthis -i src/ --fix --ai

# AI fix with specific provider
linthis --fix --ai --provider claude
linthis --fix --ai --provider claude-cli

# Auto-accept all AI fixes (for CI/automation)
linthis --fix --ai -y
linthis --fix --ai --provider claude-cli -y

# Fix only staged files with AI
linthis -s --fix --ai --provider claude-cli -y

# Fix specific language
linthis -l python --fix --ai --provider claude
```

See [AI-Powered Fix](../features/ai-fix.md) for detailed documentation.

---

## cmsg

Validate commit message format (Conventional Commits).

```bash
linthis cmsg <MSG_OR_FILE>
```

| Argument | Description |
|----------|-------------|
| `MSG_OR_FILE` | Path to a commit message file (e.g. `.git/COMMIT_EDITMSG`), or the commit message string directly |

**Examples:**

```bash
# Validate a commit message string directly
linthis cmsg "feat: add login page"
linthis cmsg "fix(auth): handle token expiry"

# Validate via file path (used by git commit-msg hook)
linthis cmsg .git/COMMIT_EDITMSG

# Install the commit-msg hook (calls linthis cmsg automatically)
linthis hook install --event commit-msg
linthis hook install --global --event commit-msg
```

The default pattern enforces [Conventional Commits](https://www.conventionalcommits.org/) format:

```
type(scope)?: description
```

Allowed types: `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `build`, `ci`, `chore`, `revert`

**Config (`.linthis.toml`):**

```toml
[hooks]
commit_msg_pattern = '^(feat|fix|docs|style|refactor|perf|test|build|ci|chore|revert)(\(.+\))?: .{1,72}'
require_ticket = false
# ticket_pattern = '^(PROJ-\d+|feat|fix|...)'
```

---

## watch

Watch mode for continuous checking.

```bash
linthis watch [OPTIONS]
```

See [Watch Mode](../features/watch-mode.md) for details.

---

## doctor

Check tool availability.

```bash
linthis doctor [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `-l, --lang` | Check specific language |

---

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success (no issues or all issues fixed) |
| 1 | Lint/format issues found |
| 2 | Configuration error |
| 3 | Tool not available |
