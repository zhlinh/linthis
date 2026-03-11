# AI Coding Agent Integration

## Overview

linthis can integrate with AI coding agents (Claude Code, Cursor, Windsurf, GitHub Copilot, Cline, CodeBuddy) to automatically enforce code quality rules during AI-assisted development.

When installed, the agent will run `linthis` checks after modifying code and fix any issues before committing — all without manual intervention.

## Supported Agents

| Agent | Rules File | Detection | Strategy |
|-------|-----------|-----------|----------|
| Claude Code | `CLAUDE.md` + `.claude/settings.json` | `.claude/` dir | Append section + Stop Hook |
| Cursor | `.cursor/rules/linthis.mdc` | `.cursor/` dir | Dedicated file |
| Windsurf | `.windsurf/rules/linthis.md` | `.windsurf/` dir | Dedicated file |
| GitHub Copilot | `.github/copilot-instructions.md` | `.github/` dir | Append section |
| Cline | `.clinerules/linthis.md` | `.clinerules/` dir | Dedicated file |
| CodeBuddy | `.codebuddy/rules/linthis.md` + `.codebuddy/settings.json` | `.codebuddy/` dir | Dedicated file |

## Quick Start

### Install for a Specific Agent

```bash
# Install for Claude Code
linthis hook install --type agent --provider claude

# Install for Cursor
linthis hook install --type agent --provider cursor

# Install for Windsurf
linthis hook install --type agent --provider windsurf

# Install for GitHub Copilot
linthis hook install --type agent --provider copilot

# Install for Cline
linthis hook install --type agent --provider cline

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
| Cursor | `.cursor/rules/linthis.mdc` | `~/.cursor/rules/linthis.mdc` |
| Windsurf | `.windsurf/rules/linthis.md` | `~/.windsurf/rules/linthis.md` |
| GitHub Copilot | `.github/copilot-instructions.md` | *(project-only)* |
| Cline | `.clinerules/linthis.md` | `~/.clinerules/linthis.md` |
| CodeBuddy | `.codebuddy/rules/linthis.md` | `~/.codebuddy/rules/linthis.md` |

## What Gets Installed

### Claude Code

Two files are created:

1. **`CLAUDE.md`** — A `## Linthis Agent Rules` section is appended (or the file is created if it doesn't exist)
2. **`.claude/settings.json`** — A Stop Hook that triggers linthis checks before the agent finishes

### Cursor

A dedicated rules file with YAML frontmatter:

```
.cursor/rules/linthis.mdc
```

The `alwaysApply: true` frontmatter ensures the rules are active for all conversations.

### Windsurf

A dedicated rules file:

```
.windsurf/rules/linthis.md
```

### GitHub Copilot

A `## Linthis Agent Rules` section is appended to:

```
.github/copilot-instructions.md
```

If the file doesn't exist, it is created with a default header.

### Cline

A dedicated rules file:

```
.clinerules/linthis.md
```

### CodeBuddy

A dedicated rules file:

```
.codebuddy/rules/linthis.md
```

## How It Works

The installed rules instruct the AI agent to:

1. **After modifying code** — Run `linthis -i <file1> -i <file2> -c` on all changed files
2. **Fix issues manually** — Read lint errors and apply fixes directly (no `--fix` or AI auto-fix)
3. **Before committing** — Run `linthis -s -c` on staged files
4. **Re-check** — Re-run linthis after fixes until clean

This ensures the agent produces lint-clean code with proper context awareness, rather than relying on automated fixers.

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
| `claude` | `claude` | `claude -p '...'` |
| `codex` | `codex` | `codex exec '...'` |
| `gemini` | `gemini` | `gemini -p '...'` |
| `cursor` | `cursor-agent` | `cursor-agent chat '...'` |
| `droid` | `droid` | `droid exec --auto low '...'` |
| `auggie` | `auggie` | `auggie --print '...'` |

### Generated Hook Script

The following is an example of the script written to `.git/hooks/pre-commit` when using `--type git-with-agent --provider claude`:

```bash
#!/bin/sh

LINTHIS_CMD="linthis -s -c -f --hook-event=pre-commit"

$LINTHIS_CMD
LINTHIS_EXIT=$?

if [ $LINTHIS_EXIT -ne 0 ]; then
  echo "[linthis] Lint errors detected. Invoking Claude Code to fix..."
  claude -p 'Staged files have linthis lint errors. Run '\''linthis -s -c'\'' to inspect them. Fix all issues by editing the files directly (do NOT use linthis --fix). Verify with '\''linthis -s -c'\'' until it passes cleanly.'
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
| `--provider` values | `claude`, `cursor`, `windsurf`, `copilot`, `cline`, `codebuddy` | `claude`, `codex`, `gemini`, `cursor`, `droid`, `auggie` |
| What it installs | Rules file + Stop Hook | Shell script in `.git/hooks/` |

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
✗ Cursor (not installed)
✗ Windsurf (not installed)
✗ GitHub Copilot (not installed)
✗ Cline (not installed)
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
- Linthis sections from `CLAUDE.md` and `.github/copilot-instructions.md`
- Dedicated rule files (`.cursor/rules/linthis.mdc`, etc.)
- Claude Code Stop Hook (`.claude/settings.json`)
- CodeBuddy Stop Hook (`.codebuddy/settings.json`)
- Empty directories created by linthis

## FAQ

### Q1: Will this overwrite my existing CLAUDE.md or copilot-instructions.md?

**No.** For append-style files (CLAUDE.md, copilot-instructions.md), linthis only adds a `## Linthis Agent Rules` section. Your existing content is preserved. If the section already exists, it won't be duplicated.

### Q2: Can I customize the rules?

Yes. After installation, edit the rules file directly. For dedicated files, you have full control. For append-style files, modify the `## Linthis Agent Rules` section.

### Q3: What happens if I use multiple agents?

You can install rules for multiple agents simultaneously. Each agent gets its own rules file, so they don't interfere with each other:

```bash
linthis hook install --type agent --provider claude
linthis hook install --type agent --provider cursor
```

### Q4: How does detection work?

linthis checks for agent-specific directories in your project root:

- `.claude/` → Claude Code
- `.cursor/` → Cursor
- `.windsurf/` → Windsurf
- `.github/` → GitHub Copilot
- `.clinerules/` → Cline
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

The `--provider` flag means different things depending on context:
- In `--type agent`: specifies the **agent platform** to install rules for (`claude`, `cursor`, `windsurf`, `copilot`, `cline`, `codebuddy`)
- In `--type *-with-agent`: specifies the **AI CLI binary** to invoke for auto-fix (`claude`, `codex`, `gemini`, `cursor`, `droid`, `auggie`)
