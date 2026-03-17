# Git Hooks

## Overview

linthis integrates with Git's hook system to run lint checks and formatting automatically at commit time (or push time). Hooks can be installed at two scopes:

- **Project-level** — written to `.git/hooks/<event>` in a single repository
- **Global** — written to `~/.config/git/hooks/<event>` and activated for every repository on the machine via `git config --global core.hooksPath`

Global hooks use **Strategy B**: a local project hook takes priority. If the local `.git/hooks/<event>` already calls `linthis`, the global hook delegates to it entirely. If a local hook exists but does not call `linthis`, the global hook runs `linthis` first and then chains to the local hook. If there is no local hook at all, the global hook runs `linthis` directly. This design guarantees zero interference with other hook tools.

All six hook types — `git`, `prek`, `pre-commit`, `git-with-agent`, `prek-with-agent`, `pre-commit-with-agent` — are supported at both scopes.

---

## Quick Start

### Project-level hooks

```bash
# Default: git pre-commit hook
linthis hook install

# Git pre-push hook
linthis hook install --event pre-push

# Commit message format hook
linthis hook install --event commit-msg

# prek hook (for projects using prek)
linthis hook install --type prek

# pre-commit hook (for projects using pre-commit framework)
linthis hook install --type pre-commit
```

### Global hooks

```bash
# Global git pre-commit hook (applied to every repository)
linthis hook install --global

# Global git pre-push hook
linthis hook install --global --event pre-push

# Global hook, non-interactive
linthis hook install --global -y
```

After running `linthis hook install --global`, the command:

1. Writes a hook script to `~/.config/git/hooks/pre-commit`
2. Runs `git config --global core.hooksPath ~/.config/git/hooks`

Every repository on the machine will now run that hook. No `git init` re-run is required for existing repositories.

---

## Hook Types

| Type | Runner | Trigger | Notes |
|------|--------|---------|-------|
| `git` | Git native | `.git/hooks/<event>` | Default type; no extra tooling required |
| `prek` | [prek](https://github.com/prek-dev/prek) | prek's runner | Requires prek installed; config committed to repo |
| `pre-commit` | [pre-commit framework](https://pre-commit.com) | pre-commit runner | Requires pre-commit installed; config committed |
| `git-with-agent` | Git native | `.git/hooks/<event>` | Same as `git`, plus AI agent fix fallback on lint failure |
| `prek-with-agent` | prek | prek's runner | Same as `prek`, plus AI agent fix fallback |
| `pre-commit-with-agent` | pre-commit framework | pre-commit runner | Same as `pre-commit`, plus AI agent fix fallback |

---

## Global Hooks

### Installing

```bash
# Install global pre-commit hook (git type)
linthis hook install --global

# Install global pre-push hook
linthis hook install --global --event pre-push

# Install global hook with agent fix fallback
linthis hook install --global --type git-with-agent --provider claude

# Non-interactive (skip confirmation prompts)
linthis hook install --global -y
```

### How it works

`--global` performs two actions:

1. **Writes** `~/.config/git/hooks/<event>` — the hook script
2. **Sets** `git config --global core.hooksPath ~/.config/git/hooks`

Git's `core.hooksPath` makes Git look in that directory for all hooks, for every repository, immediately — no per-repo setup needed.

### Directory layout

```
~/.config/git/hooks/
├── pre-commit     # installed by linthis hook install --global
├── pre-push       # installed by linthis hook install --global --event pre-push
└── ...
```

### Strategy B explained

The global hook does not run blindly. Before running `linthis`, it inspects the local `.git/hooks/<event>` of the current repository:

| Local hook state | Global hook behaviour |
|------------------|-----------------------|
| No local hook | Runs `linthis` directly |
| Local hook exists, **does not** call `linthis` | Runs `linthis` first, then delegates to the local hook |
| Local hook exists, **calls `linthis`** | Delegates entirely (`exec "$LOCAL_HOOK" "$@"`) — linthis is not double-run |

Detection uses `grep -qE '^[^#]*linthis'` — it matches any non-comment line containing `linthis`, so renaming comments does not affect the result.

### Generated global hook script (commit-msg, git type)

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

Note: `$@` passes git's `$1` (the message file path) safely, even for paths with spaces.

### Generated global hook script (pre-commit, git type)

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

## Three-Tier Hook Resolution

When `linthis hook install` runs, it resolves the hook script through three tiers (highest → lowest priority):

| Tier | Source | How to use |
|------|--------|------------|
| **Tier 1** | Fixed-path auto-discovery | Place a script at `hooks/git/<event>` in your project root |
| **Tier 2** | TOML source mapping | Set `[hook.git]` entries in `.linthis/config.toml` |
| **Tier 3** | Built-in generator | Default — the built-in generated script |

### Tier 1: Fixed-Path Auto-Discovery

Create an executable file at the conventional path relative to your project root:

```
hooks/git/pre-commit
hooks/git/pre-push
hooks/git/commit-msg
```

If this file exists, linthis uses it directly without generating its own script. No config needed.

### Tier 2: TOML Source Mapping

Override the hook source in `.linthis/config.toml` using a `source` entry. Plugins typically inject these entries automatically when added via `linthis plugin add`.

```toml
[hook.git]
pre-commit = { source = { plugin = "my-plugin", file = "hooks/git/pre-commit" } }
```

Five source variants are supported:

```toml
# Local file (relative to project root)
pre-commit = { source = { file = "hooks/git/pre-commit" } }

# File inside an installed plugin
pre-commit = { source = { plugin = "my-plugin", file = "hooks/git/pre-commit" } }

# File from a marketplace plugin
pre-commit = { source = { marketplace = "corp", plugin = "linthis-official", file = "hooks/git/pre-commit" } }

# Direct URL download
pre-commit = { source = { url = "https://example.com/hooks/pre-commit" } }

# Clone a git repo
pre-commit = { source = { git = "https://github.com/org/hooks.git", ref = "main", path = "pre-commit" } }
```

The same override structure applies to all hook types (`[hook.git-with-agent]`, `[hook.prek]`, `[hook.prek-with-agent]`, etc.).

### Plugin-Bundled Hooks

Plugins can bundle hook overrides inside a `linthis-hook.toml` at the plugin root. When a user runs `linthis plugin add <alias> <url>`, linthis automatically:

1. Replaces `plugin = "self"` with `plugin = "<alias>"` in the bundled config
2. Non-overwritingly merges `[hook.*]` entries into the user's `.linthis/config.toml`

This means adding a team plugin is all it takes for everyone to get the team's custom pre-commit scripts automatically.

---

## *-with-agent Hook Types

The `git-with-agent`, `prek-with-agent`, and `pre-commit-with-agent` types add an AI agent fix fallback. When `linthis` exits with a non-zero status (lint failure), the hook invokes the chosen agent CLI in headless mode to attempt an automatic fix, then re-runs `linthis` to verify the result.

### Supported providers

| `--provider` value | Agent CLI | Headless command |
|--------------------|-----------|-----------------|
| `claude` | Claude Code CLI | `claude -p '<prompt>'` |
| `codex` | OpenAI Codex CLI | `codex exec '<prompt>'` |
| `gemini` | Google Gemini CLI | `gemini -p '<prompt>'` |
| `cursor` | Cursor agent | `cursor-agent chat '<prompt>'` |
| `droid` | Droid | `droid exec --auto low '<prompt>'` |
| `auggie` | Auggie | `auggie --print '<prompt>'` |

### Examples

```bash
# Project-level: git hook with Claude fix fallback
linthis hook install --type git-with-agent --provider claude

# Project-level: prek hook with Gemini fix fallback
linthis hook install --type prek-with-agent --provider gemini

# Project-level: pre-commit hook with Codex fix fallback
linthis hook install --type pre-commit-with-agent --provider codex

# Global: git hook with Claude fix fallback
linthis hook install --global --type git-with-agent --provider claude

# Global: pre-commit hook with Gemini fix fallback
linthis hook install --global --type pre-commit-with-agent --provider gemini
```

---

## hook status

Check the current state of all installed hooks:

```bash
linthis hook status
```

Example output:

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
      ℹ Strategy B: local hook takes priority
```

The status output shows:

- Which project-level hooks are installed and whether they contain a `linthis` call
- Which global hooks are installed
- The active `core.hooksPath` setting
- The delegation strategy in use

---

## Global vs Project Comparison

| Feature | Global (`--global`) | Project-level |
|---------|---------------------|---------------|
| Scope | Every repository on the machine | Current repository only |
| Location | `~/.config/git/hooks/` | `.git/hooks/` |
| Git config changed | `core.hooksPath` (global) | None |
| Works for existing repos | Yes, immediately | Yes, immediately |
| Committable to repo | No | No (`.git/` is not tracked) |
| Team sharing | No | Requires prek or pre-commit type |
| Hook coexistence | Strategy B (auto-delegation) | Manual chaining |
| Supported types | All six types | All six types |

---

## Uninstall

### Remove a specific global hook

```bash
# Remove global pre-commit hook
linthis hook uninstall --global

# Remove global pre-push hook
linthis hook uninstall --global --event pre-push

# Non-interactive
linthis hook uninstall --global -y
```

### Remove all global hooks

```bash
linthis hook uninstall --global --all

# Non-interactive
linthis hook uninstall --global --all -y
```

`--all` removes all hook scripts from `~/.config/git/hooks/` and unsets `core.hooksPath` if no other hooks remain.

### Remove a project-level hook

```bash
# Remove the project pre-commit hook
linthis hook uninstall

# Remove the project pre-push hook
linthis hook uninstall --event pre-push
```

---

## Command Reference

```bash
# Project-level install
linthis hook install                                               # git pre-commit
linthis hook install --event pre-push                             # git pre-push
linthis hook install --type prek                                   # prek
linthis hook install --type pre-commit                             # pre-commit framework
linthis hook install --type git-with-agent --provider claude       # git + agent fix
linthis hook install --type prek-with-agent --provider gemini      # prek + agent fix
linthis hook install --type pre-commit-with-agent --provider codex # pre-commit + agent fix

# Global install
linthis hook install --global                                      # global git pre-commit
linthis hook install --global --event pre-push                     # global git pre-push
linthis hook install --global --type git-with-agent --provider claude  # global + agent fix
linthis hook install --global -y                                   # non-interactive

# Uninstall
linthis hook uninstall                                             # remove project pre-commit
linthis hook uninstall --global                                    # remove global pre-commit
linthis hook uninstall --global --all                              # remove all global hooks
linthis hook uninstall --global -y                                 # non-interactive

# Status
linthis hook status
```

---

## FAQ

### Q1: Can a global hook and a project-level hook coexist?

Yes. This is Strategy B's primary use case. If the project has a `.git/hooks/pre-commit` that calls `linthis`, the global hook detects it and delegates entirely — `linthis` runs once, not twice. If the project hook does not call `linthis`, the global hook prepends `linthis` before calling the project hook.

### Q2: How does Strategy B detect whether the local hook calls linthis?

It runs `grep -qE '^[^#]*linthis' "$LOCAL_HOOK"`. The pattern matches any non-comment line (`^[^#]*`) that contains the string `linthis`. Comment lines starting with `#` are ignored. This means renaming a comment or adding a note like `# previously used linthis` does not affect detection — only executable lines matter.

### Q3: How do I disable the global hook for a specific repository?

Install a project-level hook that calls `linthis`. The global hook will detect it and delegate, so the project hook is the sole entry point. You then have full control over how `linthis` is invoked in that repository.

Alternatively, install any project-level hook that does not call `linthis`. The global hook will still run `linthis` before it — to suppress that, remove the global hook for that event or use `--event` to choose a different event scope.

If you want `linthis` to be completely silent in one repository, create a no-op project hook:

```bash
printf '#!/bin/sh\n# intentionally no linthis\nexit 0\n' > .git/hooks/pre-commit
chmod +x .git/hooks/pre-commit
```

The global hook will see that this hook does not call `linthis`, so it will run `linthis` first. To suppress `linthis` entirely in that repo, the cleanest approach is to uninstall the global hook and rely on project-level hooks only.

### Q4: Will the global hook affect repositories that do not use linthis?

The hook will attempt to run `linthis -s -c -f --hook-event=pre-commit`. If the repository has no linthis configuration (`.linthis/config.toml`, `.linthis.toml`, or `linthis.toml`), `linthis` exits immediately with no errors. The commit proceeds normally.

### Q5: What happens if the agent CLI is not installed but I used `--type git-with-agent`?

The hook first runs `linthis`. If `linthis` exits cleanly, the agent is never invoked. If `linthis` fails and the agent CLI binary is missing, the hook prints a warning and exits with the original `linthis` exit code so the commit is still blocked.

### Q6: Can I use `--type prek` or `--type pre-commit` with `--global`?

Yes. All six hook types are supported with `--global`. The hook script written to `~/.config/git/hooks/<event>` will invoke the appropriate runner (`prek` or `pre-commit`) rather than `linthis` directly. The same Strategy B delegation logic applies.

### Q7: How do I check which `core.hooksPath` is active?

```bash
git config --global --get core.hooksPath
# Output: /Users/username/.config/git/hooks
```

If this returns nothing, no global `core.hooksPath` is set and Git is using `.git/hooks/` per-repository as usual.

---

## See Also

- [AI-Powered Fix](./ai-fix.md) — AI provider details
- [AI Coding Agent Integration](./agent-hooks.md) — Rules-based agent integration
- [CLI Reference](../reference/cli.md) — Complete command reference
- [Git documentation — core.hooksPath](https://git-scm.com/docs/git-config#Documentation/git-config.txt-corehooksPath)
