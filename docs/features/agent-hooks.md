# AI Coding Agent Integration

## Overview

linthis can integrate with AI coding agents (Claude Code, Codex, Gemini, Cursor, Droid, Auggie, CodeBuddy, OpenClaw) to automatically enforce code quality rules during AI-assisted development.

When installed, the agent will run `linthis` checks after modifying code and fix any issues before committing — all without manual intervention.

## Supported Agents

| Agent | Rules Files | Detection | Strategy |
|-------|------------|-----------|----------|
| Claude Code | `.claude/skills/lt-lint/SKILL.md`, `.claude/skills/lt-cmsg/SKILL.md`, `.claude/skills/lt-review/SKILL.md` + `.claude/settings.json` | `.claude/` dir | Per-event skills + Stop Hook |
| Codex | `AGENTS.md` | `AGENTS.md` or `.codex/` | Per-event sections |
| Gemini | `.gemini/linthis-lint.md`, `.gemini/linthis-cmsg.md`, `.gemini/linthis-review.md` | `.gemini/` dir | Per-event files |
| Cursor | `.cursor/rules/linthis-lint.mdc`, `.cursor/rules/linthis-cmsg.mdc`, `.cursor/rules/linthis-review.mdc` | `.cursor/` dir | Per-event files |
| Droid | `.droid/rules/linthis-lint.md`, `.droid/rules/linthis-cmsg.md`, `.droid/rules/linthis-review.md` | `.droid/` dir | Per-event files |
| Auggie | `.augment/rules/linthis-lint.md`, `.augment/rules/linthis-cmsg.md`, `.augment/rules/linthis-review.md` | `.augment/` dir | Per-event files |
| CodeBuddy | `.codebuddy/skills/lt-lint/SKILL.md`, `.codebuddy/skills/lt-cmsg/SKILL.md`, `.codebuddy/skills/lt-review/SKILL.md` + `.codebuddy/settings.json` | `.codebuddy/` dir | Per-event skills + Stop Hook |
| OpenClaw | `.openclaw/skills/lt-lint/SKILL.md`, `.openclaw/skills/lt-cmsg/SKILL.md`, `.openclaw/skills/lt-review/SKILL.md` | `.openclaw/` dir | Per-event skills |

Each provider receives three per-event files corresponding to the hook events:

| Hook Event | Skill Name | Purpose |
|------------|-----------|---------|
| `pre-commit` | `lt-lint` | Run linthis checks on staged files |
| `commit-msg` | `lt-cmsg` | Validate commit message format |
| `pre-push` | `lt-review` | Review outgoing commits |

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
| Claude Code | `.claude/skills/lt-{lint,cmsg,review}/SKILL.md` + `.claude/settings.json` | `~/.claude/skills/lt-{lint,cmsg,review}/SKILL.md` + `~/.claude/settings.json` |
| Codex | `AGENTS.md` | `~/.codex/AGENTS.md` |
| Gemini | `.gemini/linthis-{lint,cmsg,review}.md` | `~/.gemini/linthis-{lint,cmsg,review}.md` |
| Cursor | `.cursor/rules/linthis-{lint,cmsg,review}.mdc` | `~/.cursor/rules/linthis-{lint,cmsg,review}.mdc` |
| Droid | `.droid/rules/linthis-{lint,cmsg,review}.md` | `~/.droid/rules/linthis-{lint,cmsg,review}.md` |
| Auggie | `.augment/rules/linthis-{lint,cmsg,review}.md` | `~/.augment/rules/linthis-{lint,cmsg,review}.md` |
| CodeBuddy | `.codebuddy/skills/lt-{lint,cmsg,review}/SKILL.md` + `.codebuddy/settings.json` | `~/.codebuddy/skills/lt-{lint,cmsg,review}/SKILL.md` + `~/.codebuddy/settings.json` |

## What Gets Installed

### Claude Code

Three skill files and a Stop Hook are created:

1. **`.claude/skills/lt-lint/SKILL.md`** — Skill for pre-commit linting (run linthis checks on staged files)
2. **`.claude/skills/lt-cmsg/SKILL.md`** — Skill for commit-msg validation (validate commit message format)
3. **`.claude/skills/lt-review/SKILL.md`** — Skill for pre-push review (review outgoing commits)
4. **`.claude/settings.json`** — A Stop Hook that triggers linthis checks before the agent finishes

Slash commands are also installed under `.claude/commands/linthis/`.

### Codex

Three per-event sections are appended to:

```
AGENTS.md
```

Each section covers one hook event (pre-commit, commit-msg, pre-push). If the file doesn't exist, it is created with a default header.

### Gemini

Three dedicated per-event rules files:

```
.gemini/linthis-lint.md
.gemini/linthis-cmsg.md
.gemini/linthis-review.md
```

### Cursor

Three dedicated per-event rules files with YAML frontmatter:

```
.cursor/rules/linthis-lint.mdc
.cursor/rules/linthis-cmsg.mdc
.cursor/rules/linthis-review.mdc
```

The `alwaysApply: true` frontmatter ensures the rules are active for all conversations.

### Droid

Three dedicated per-event rules files:

```
.droid/rules/linthis-lint.md
.droid/rules/linthis-cmsg.md
.droid/rules/linthis-review.md
```

### Auggie

Three dedicated per-event rules files:

```
.augment/rules/linthis-lint.md
.augment/rules/linthis-cmsg.md
.augment/rules/linthis-review.md
```

### CodeBuddy

Three skill files and a Stop Hook are created:

1. **`.codebuddy/skills/lt-lint/SKILL.md`** — Skill for pre-commit linting
2. **`.codebuddy/skills/lt-cmsg/SKILL.md`** — Skill for commit-msg validation
3. **`.codebuddy/skills/lt-review/SKILL.md`** — Skill for pre-push review
4. **`.codebuddy/settings.json`** — A Stop Hook that triggers linthis checks before the agent finishes

## How It Works

The installed rules instruct the AI agent to:

1. **After modifying code** — Run `linthis -i <file1> -i <file2>` on all changed files
2. **Fix issues manually** — Read lint errors and apply fixes directly (no `--fix` or AI auto-fix)
3. **Before committing** — Run `linthis -s` on staged files
4. **Re-check** — Re-run linthis after fixes until clean

This ensures the agent produces lint-clean code with proper context awareness, rather than relying on automated fixers.

## Three-Tier Agent Hook Resolution

When `linthis hook install --type agent` runs, it resolves each agent plugin bundle and stop hook through three tiers (highest → lowest priority):

| Tier | Source | How to use |
|------|--------|------------|
| **Tier 1** | Fixed-path auto-discovery | Place files at `hooks/agent/plugins/<id>/` or `hooks/agent/hook/stop/<provider>/` in your project root |
| **Tier 2** | TOML source mapping | Set `[hook.agent.plugins._default]` / `[hook.agent.stop]` entries in `.linthis/config.toml` |
| **Tier 3** | Built-in generator | Default — the built-in rules content generated by linthis |

### Agent Plugin Bundle Structure

An agent plugin bundle is a directory with the following layout. Any sub-directory is optional:

```
<bundle-dir>/
├── skills/<skill_name>/SKILL.md    — skill instruction (e.g. skills/lt-lint/SKILL.md)
├── commands/                        — slash command files (optional)
├── memories/TOPLEVEL.md             — memory section injected into CLAUDE.md etc. (optional)
└── hooks/hooks.json                 — stop hook settings (optional)
```

Example for Claude Code:
```
hooks/agent/plugins/lt/lint/
├── skills/lt-lint/SKILL.md    — instructions Claude follows for linting
├── commands/lt-lint.md        — defines a /lt-lint slash command
├── memories/TOPLEVEL.md       — memory section added to CLAUDE.md
└── hooks/hooks.json           — stop hook settings
```

### Tier 2: TOML Source Mapping for Agent Hooks

Override agent plugin bundles and stop hooks in `.linthis/config.toml`:

```toml
[hook.agent.plugins._default]
"lt.lint"   = { source = { plugin = "my-plugin", file = "hooks/agent/plugins/lt/lint" } }
"lt.cmsg"   = { source = { plugin = "my-plugin", file = "hooks/agent/plugins/lt/cmsg" } }
"lt.review" = { source = { plugin = "my-plugin", file = "hooks/agent/plugins/lt/review" } }

[hook.agent.stop]
"claude.settings" = { source = { plugin = "my-plugin", file = "hooks/agent/hook/stop/claude/settings.json" } }
```

The same five `HookSource` variants available for git hooks also apply here (see [Configuration Reference](../reference/configuration.md#hooksource--source-specification)).

### Plugin-Bundled Agent Hooks

Plugins can bundle their agent hook overrides inside a `linthis-hook.toml` at the plugin root. When a user runs `linthis plugin add <alias> <url>`, linthis automatically merges these entries into the user's `.linthis/config.toml`. The next `linthis hook install --type agent --provider claude` will then use the plugin's custom skill/command/memory bundle and stop hook settings.

### Configurable Skill Directory Names

Teams that already have existing skills with different names can map linthis hook events to custom skill directory names via `.linthis/config.toml`:

```toml
[hook.agent.skill-names]
pre-commit = "my-team-lint"     # default: "lt-lint"
commit-msg = "my-team-cmsg"     # default: "lt-cmsg"
pre-push = "my-team-review"     # default: "lt-review"
```

The values are the directory names used under `.claude/skills/`, `.codebuddy/skills/`, and also the base names for flat-file providers (Gemini: `{name}.md`, Cursor: `{name}.mdc`, etc.).

Without this config, the defaults `lt-lint`, `lt-cmsg`, and `lt-review` are used (backward compatible).

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
| `openclaw` | `openclaw` | `openclaw agent --message '...'` |

### Generated Hook Script (Thin Wrapper)

linthis installs a **thin wrapper** script into `.git/hooks/`. The wrapper delegates all logic to `linthis hook run`, which dynamically generates and executes the full hook script from the current linthis binary. This means hook logic is always up-to-date after a `cargo install` or binary upgrade — no need to re-install hooks.

Example `.git/hooks/pre-commit` for `--type git-with-agent --provider claude`:

```bash
#!/bin/sh
exec linthis hook run --event pre-commit --type git-with-agent --provider claude "$@"
```

At runtime, `linthis hook run` generates the full script internally — including the linthis check, agent CLI availability detection, auto-fix invocation, re-staging, and retry logic. The wrapper never contains this logic directly.

To update hook behavior after upgrading linthis, use:

```bash
linthis hook sync       # re-sync local project hooks
linthis hook sync -g    # re-sync global hooks
```

### How It Differs from --type agent

| Feature | `--type agent` | `--type *-with-agent` |
|---------|---------------|----------------------|
| Hook type | Agent rules file | Git hook (pre-commit) |
| Trigger | AI agent finishes a task | `git commit` |
| `--provider` values | `claude`, `codex`, `gemini`, `cursor`, `droid`, `auggie`, `codebuddy` | `claude`, `codex`, `gemini`, `cursor`, `droid`, `auggie`, `codebuddy` |
| What it installs | Per-event skill/rule files (+ Stop Hook for claude/codebuddy) | Shell script in `.git/hooks/` |

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
- Linthis sections from `AGENTS.md` (append-style files)
- Per-event skill files (`.claude/skills/lt-*/SKILL.md`, `.codebuddy/skills/lt-*/SKILL.md`)
- Per-event rule files (`.cursor/rules/linthis-*.mdc`, `.gemini/linthis-*.md`, `.droid/rules/linthis-*.md`, `.augment/rules/linthis-*.md`)
- Claude Code Stop Hook (`.claude/settings.json`)
- CodeBuddy Stop Hook (`.codebuddy/settings.json`)
- Slash commands (`.claude/commands/linthis/`)
- Empty directories created by linthis

## Hook Sync

Re-sync all installed hooks and agent skills to ensure they're up to date:

```bash
# Sync local project hooks
linthis hook sync

# Sync global hooks
linthis hook sync -g
```

This regenerates thin wrapper scripts and refreshes agent skill files for all recorded installations.

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
| Agent rules (global) | `linthis hook install --type agent --provider claude --global` | Same as above, but installed to `~/.claude/skills/` — applies to all projects |
| Git hook with agent fallback | `linthis hook install --type git-with-agent --provider claude` | Installs a git pre-commit hook; if linthis fails, the AI CLI is invoked to fix issues before re-checking |

The `--provider` flag accepts the same set of values for both types (`claude`, `codex`, `gemini`, `cursor`, `droid`, `auggie`, `codebuddy`), but the implementation differs:
- In `--type agent`: installs **rules/settings files** so the AI enforces linting during coding sessions
- In `--type *-with-agent`: invokes the provider's **headless CLI** to auto-fix issues on git hook failure
