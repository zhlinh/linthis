# AI Coding Agent Integration

## Overview

linthis can integrate with AI coding agents (Claude Code, Codex, Gemini, Cursor, Droid, Auggie, CodeBuddy) to automatically enforce code quality rules during AI-assisted development.

When installed, the agent will run `linthis` checks after modifying code and fix any issues before committing — all without manual intervention.

## Supported Agents

| Agent | Rules File | Detection | Strategy |
|-------|-----------|-----------|----------|
| Claude Code | `CLAUDE.md` + `.claude/settings.json` | `.claude/` dir | Append section + Stop Hook |
| Codex | `AGENTS.md` | `AGENTS.md` or `.codex/` | Append section |
| Gemini | `.gemini/instructions.md` | `.gemini/` dir | Dedicated file |
| Cursor | `.cursor/rules/linthis.mdc` | `.cursor/` dir | Dedicated file |
| Droid | `.droid/rules/linthis.md` | `.droid/` dir | Dedicated file |
| Auggie | `.augment/rules/linthis.md` | `.augment/` dir | Dedicated file |
| CodeBuddy | `.codebuddy/rules/linthis.md` + `.codebuddy/settings.json` | `.codebuddy/` dir | Dedicated file + Stop Hook |

## Quick Start

### Install for a Specific Agent

```bash
# Install for Claude Code
linthis hook install --type agent --provider claude

# Install for Codex
linthis hook install --type agent --provider codex

# Install for Gemini
linthis hook install --type agent --provider gemini

# Install for Cursor
linthis hook install --type agent --provider cursor

# Install for Droid
linthis hook install --type agent --provider droid

# Install for Auggie
linthis hook install --type agent --provider auggie

# Install for CodeBuddy
linthis hook install --type agent --provider codebuddy
```

### Auto-Detect and Install All

Install for all detected agents (or all if none detected):

```bash
linthis hook install --type agent -y
```

### Interactive Menu

Run without `-y` or `--provider` to choose interactively:

```bash
linthis hook install --type agent
```

Output:
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

### Global Installation

Install agent rules into your home directory so they apply to every project (not just the current one):

```bash
# Install agent rules globally for a specific provider
linthis hook install --type agent --provider claude --global

# Install globally for all detected providers
linthis hook install --type agent -g
```

When `--global` is set, rules are written to user-level locations instead of the project root:

| Agent | Project-level | Global (`--global`) |
|-------|--------------|---------------------|
| Claude Code | `CLAUDE.md` | `~/.claude/CLAUDE.md` |
| Codex | `AGENTS.md` | `~/.codex/AGENTS.md` |
| Gemini | `.gemini/instructions.md` | `~/.gemini/instructions.md` |
| Cursor | `.cursor/rules/linthis.mdc` | `~/.cursor/rules/linthis.mdc` |
| Droid | `.droid/rules/linthis.md` | `~/.droid/rules/linthis.md` |
| Auggie | `.augment/rules/linthis.md` | `~/.augment/rules/linthis.md` |
| CodeBuddy | `.codebuddy/rules/linthis.md` | `~/.codebuddy/rules/linthis.md` |

## What Gets Installed

### Claude Code

Two files are created:

1. **`CLAUDE.md`** — A `## Linthis Agent Rules` section is appended (or the file is created if it doesn't exist)
2. **`.claude/settings.json`** — A Stop Hook that triggers linthis checks before the agent finishes

### Codex

A `## Linthis Agent Rules` section is appended to:

```
AGENTS.md
```

If the file doesn't exist, it is created with a default header.

### Gemini

A dedicated rules file:

```
.gemini/instructions.md
```

### Cursor

A dedicated rules file with YAML frontmatter:

```
.cursor/rules/linthis.mdc
```

The `alwaysApply: true` frontmatter ensures the rules are active for all conversations.

### Droid

A dedicated rules file:

```
.droid/rules/linthis.md
```

### Auggie

A dedicated rules file:

```
.augment/rules/linthis.md
```

### CodeBuddy

Two files are created:

1. **`.codebuddy/rules/linthis.md`** — A dedicated rules file
2. **`.codebuddy/settings.json`** — A Stop Hook that triggers linthis checks before the agent finishes

## How It Works

The installed rules instruct the AI agent to:

1. **After modifying code** — Run `linthis -i <file1> -i <file2> -c` on all changed files
2. **Fix issues manually** — Read lint errors and apply fixes directly (no `--fix` or AI auto-fix)
3. **Before committing** — Run `linthis -s -c` on staged files
4. **Re-check** — Re-run linthis after fixes until clean

This ensures the agent produces lint-clean code with proper context awareness, rather than relying on automated fixers.

## Three-Tier Agent Hook Resolution

When `linthis hook install --type agent` runs, it resolves each agent plugin bundle and stop hook through three tiers (highest → lowest priority):

| Tier | Source | How to use |
|------|--------|------------|
| **Tier 1** | Fixed-path auto-discovery | Place files at `hooks/agent/plugins/<id>/` or `hooks/agent/hook/stop/<provider>/` in your project root |
| **Tier 2** | TOML source mapping | Set `[hooks.agent-plugins]` / `[hooks.agent-hook.stop]` entries in `.linthis/config.toml` |
| **Tier 3** | Built-in generator | Default — the built-in rules content generated by linthis |

### Agent Plugin Bundle Structure

An agent plugin bundle is a directory with the following layout. Any sub-directory is optional:

```
<bundle-dir>/
├── skill/<provider>/          — skill instruction file (e.g., claude/lint.md)
├── command/<provider>/        — slash command definition file (optional)
└── memory/<provider>/         — memory section injected into CLAUDE.md etc. (optional)
```

Example for Claude Code:
```
hooks/agent/plugins/lt/lint/
├── skill/claude/lint.md       — instructions Claude follows for linting
├── command/claude/lt-lint.md  — defines a /lt-lint slash command
└── memory/claude/lint.md      — memory section added to ~/.claude/projects/.../MEMORY.md
```

### Tier 2: TOML Source Mapping for Agent Hooks

Override agent plugin bundles and stop hooks in `.linthis/config.toml`:

```toml
[hooks.agent-plugins]
"lt.lint"   = { source = { plugin = "my-plugin", file = "hooks/agent/plugins/lt/lint" } }
"lt.cmsg"   = { source = { plugin = "my-plugin", file = "hooks/agent/plugins/lt/cmsg" } }
"lt.review" = { source = { plugin = "my-plugin", file = "hooks/agent/plugins/lt/review" } }

[hooks.agent-hook.stop]
"claude.settings" = { source = { plugin = "my-plugin", file = "hooks/agent/hook/stop/claude/settings.json" } }
```

The same five `HookSource` variants available for git hooks also apply here (see [Configuration Reference](../reference/configuration.md#hooksource--source-specification)).

### Plugin-Bundled Agent Hooks

Plugins can bundle their agent hook overrides inside a `linthis-config.toml` at the plugin root. When a user runs `linthis plugin add <alias> <url>`, linthis automatically merges these entries into the user's `.linthis/config.toml`. The next `linthis hook install --type agent --provider claude` will then use the plugin's custom skill/command/memory bundle and stop hook settings.

---

## Git Hook with Agent Fix (--type *-with-agent)

These are **git hook types** (distinct from `--type agent`) that add an AI agent auto-fix fallback when a linthis check fails during a commit. If linthis exits non-zero, the hook invokes the specified AI CLI tool to fix the issues, then re-runs linthis to verify.

### Install

```bash
# Install a pre-commit git hook with Claude Code auto-fix fallback
linthis hook install --type git-with-agent --provider claude

# Other AI CLI providers
linthis hook install --type git-with-agent --provider codex
linthis hook install --type prek-with-agent --provider gemini
linthis hook install --type pre-commit-with-agent --provider cursor
linthis hook install --type git-with-agent --provider droid
linthis hook install --type git-with-agent --provider auggie

# Global installation (writes to ~/.config/git/hooks/)
linthis hook install --type git-with-agent --provider claude --global
```

### Supported Providers

| Provider | CLI Binary | Headless Command |
|----------|-----------|-----------------|
| `claude` | `claude` | `claude -p --dangerously-skip-permissions '...'` |
| `codex` | `codex` | `codex exec --ask-for-approval never '...'` |
| `gemini` | `gemini` | `gemini -p --approval-mode=auto_edit '...'` |
| `cursor` | `cursor-agent` | `cursor-agent chat --force '...'` |
| `droid` | `droid` | `droid exec --auto high '...'` |
| `auggie` | `auggie` | `auggie --print '...'` |
| `codebuddy` | `codebuddy` | `codebuddy -p --dangerously-skip-permissions '...'` |

### Generated Hook Script

The following is an example of the script written to `.git/hooks/pre-commit` when using `--type git-with-agent --provider claude`:

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

### How It Differs from --type agent

| Feature | `--type agent` | `--type *-with-agent` |
|---------|---------------|----------------------|
| Hook type | Agent rules file | Git hook (pre-commit) |
| Trigger | AI agent finishes a task | `git commit` |
| `--provider` values | `claude`, `codex`, `gemini`, `cursor`, `droid`, `auggie`, `codebuddy` | `claude`, `codex`, `gemini`, `cursor`, `droid`, `auggie`, `codebuddy` |
| What it installs | Rules file (+ Stop Hook for claude/codebuddy) | Shell script in `.git/hooks/` |

## Check Status

View which hooks and agents are installed:

```bash
linthis hook status
```

Output:
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

## Uninstall

Remove all agent integrations:

```bash
linthis hook uninstall --all -y
```

Remove only agent rules for a specific provider:

```bash
linthis hook uninstall --type agent --provider claude -y
```

Remove globally installed agent rules:

```bash
linthis hook uninstall --type agent --global -y
```

The uninstall command removes:
- Linthis sections from `CLAUDE.md` and `AGENTS.md` (append-style files)
- Dedicated rule files (`.cursor/rules/linthis.mdc`, `.gemini/instructions.md`, etc.)
- Claude Code Stop Hook (`.claude/settings.json`)
- CodeBuddy Stop Hook (`.codebuddy/settings.json`)
- Empty directories created by linthis

## FAQ

### Q1: Will this overwrite my existing CLAUDE.md or AGENTS.md?

**No.** For append-style files (`CLAUDE.md`, `AGENTS.md`), linthis only adds a `## Linthis Agent Rules` section. Your existing content is preserved. If the section already exists, it won't be duplicated.

### Q2: Can I customize the rules?

Yes. After installation, edit the rules file directly. For dedicated files, you have full control. For append-style files, modify the `## Linthis Agent Rules` section.

### Q3: What happens if I use multiple agents?

You can install rules for multiple agents simultaneously. Each agent gets its own rules file, so they don't interfere with each other:

```bash
linthis hook install --type agent --provider claude
linthis hook install --type agent --provider cursor
```

### Q4: How does detection work?

linthis checks for agent-specific directories/files in your project root:

- `.claude/` → Claude Code
- `AGENTS.md` or `.codex/` → Codex
- `.gemini/` → Gemini
- `.cursor/` → Cursor
- `.droid/` → Droid
- `.augment/` → Auggie
- `.codebuddy/` → CodeBuddy

When using `-y` (auto-install), only detected agents are configured. If no agents are detected, all are installed.

### Q5: What is the Claude Code Stop Hook?

The Stop Hook (`.claude/settings.json`) adds an automatic check before Claude Code finishes a task. It prompts the agent to run linthis on any modified files, ensuring no lint issues slip through.

### Q6: What are the different approaches to AI-assisted linting?

There are three distinct approaches:

| Approach | Command | How it works |
|----------|---------|-------------|
| Agent rules (project) | `linthis hook install --type agent --provider claude` | Installs rules into the agent's config file so the AI enforces linting during coding sessions |
| Agent rules (global) | `linthis hook install --type agent --provider claude --global` | Same as above, but installed to `~/.claude/CLAUDE.md` — applies to all projects |
| Git hook with agent fallback | `linthis hook install --type git-with-agent --provider claude` | Installs a git pre-commit hook; if linthis fails, the AI CLI is invoked to fix issues before re-checking |

The `--provider` flag accepts the same set of values for both types (`claude`, `codex`, `gemini`, `cursor`, `droid`, `auggie`, `codebuddy`), but the implementation differs:
- In `--type agent`: installs **rules/settings files** so the AI enforces linting during coding sessions
- In `--type *-with-agent`: invokes the provider's **headless CLI** to auto-fix issues on git hook failure
