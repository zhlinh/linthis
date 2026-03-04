# AI Coding Agent Integration

## Overview

linthis can integrate with AI coding agents (Claude Code, Cursor, Windsurf, GitHub Copilot, Cline, CodeBuddy) to automatically enforce code quality rules during AI-assisted development.

When installed, the agent will run `linthis` checks after modifying code and fix any issues before committing — all without manual intervention.

## Supported Agents

| Agent | Rules File | Detection | Strategy |
|-------|-----------|-----------|----------|
| Claude Code | `CLAUDE.md` + `.claude/settings.local.json` | `.claude/` dir | Append section + Stop Hook |
| Cursor | `.cursor/rules/linthis.mdc` | `.cursor/` dir | Dedicated file |
| Windsurf | `.windsurf/rules/linthis.md` | `.windsurf/` dir | Dedicated file |
| GitHub Copilot | `.github/copilot-instructions.md` | `.github/` dir | Append section |
| Cline | `.clinerules/linthis.md` | `.clinerules/` dir | Dedicated file |
| CodeBuddy | `.codebuddy/rules/linthis.md` | `.codebuddy/` dir | Dedicated file |

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

## What Gets Installed

### Claude Code

Two files are created:

1. **`CLAUDE.md`** — A `## Linthis Agent Rules` section is appended (or the file is created if it doesn't exist)
2. **`.claude/settings.local.json`** — A Stop Hook that triggers linthis checks before the agent finishes

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

## Check Status

View which agents are installed:

```bash
linthis hook status
```

Output:
```
Hook Status:
  Agent Hooks:
    ✓ Claude Code  (installed)
    ✓ Cursor       (installed)
    ✗ Windsurf
    ✗ GitHub Copilot
    ✗ Cline
    ✗ CodeBuddy
```

## Uninstall

Remove all agent integrations:

```bash
linthis hook uninstall --all -y
```

This removes:
- Linthis sections from `CLAUDE.md` and `.github/copilot-instructions.md`
- Dedicated rule files (`.cursor/rules/linthis.mdc`, etc.)
- Claude Code Stop Hook (`.claude/settings.local.json`)
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

The Stop Hook (`.claude/settings.local.json`) adds an automatic check before Claude Code finishes a task. It prompts the agent to run linthis on any modified files, ensuring no lint issues slip through.

### Q6: How is this different from AI auto-fix in git hooks?

These are different features:

| Command | Purpose |
|---------|---------|
| `linthis hook install --type agent --provider claude` | Install agent rules (code quality enforcement during AI coding) |
| `linthis hook install --args "-c -f --fix --ai --provider claude --accept-all"` | Install git hook with AI auto-fix (fixes lint issues during git commit) |

The `--provider` flag in `--type agent` context specifies the agent platform, while in `--args` context it specifies the AI fix provider.
