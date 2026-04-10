# Fix Commit Mode

Controls how linthis handles auto-format and agent fix changes during git hooks.

## Configuration

```toml
[hook.pre_commit]
fix_commit_mode = "squash"    # squash | dirty | fixup

[hook.pre_push]
fix_commit_mode = "dirty"     # squash | dirty | fixup
```

Or via CLI: `linthis hook install --fix-commit-mode <mode>`

Query: `linthis config get hook.pre_commit.fix_commit_mode`

## Modes

| Mode | Description |
|------|-------------|
| **squash** | Fix + create fixup commit + squash into original commit. Stash snapshot preserved for review. |
| **dirty** | Fix + leave changes in working tree + block commit/push. User reviews before staging. |
| **fixup** | Let original commit through. Post-commit hook creates a separate fixup commit. |

---

## Behavior Matrix

### git type

| Event | Mode | New commits | Commit message | Working tree | Recover |
|-------|------|-------------|----------------|-------------|---------|
| pre-commit | squash | 0 | original unchanged | clean (re-staged) | `git stash pop` or `linthis undo` |
| pre-commit | dirty | 0, blocked | — | dirty (format unstaged) | `linthis undo` |
| pre-commit | fixup | 1 (post-commit) | `fix(linthis): auto-fix lint issues` | clean | `git reset HEAD~1` |
| pre-push | * | 0 | — | unchanged (check only) | — |

### git-with-agent type

| Event | Mode | Change type | New commits | Commit message | Working tree | Recover |
|-------|------|-------------|-------------|----------------|-------------|---------|
| pre-commit | squash | format | 0 (fixup+squash) | original unchanged | clean | `git stash pop` or `git reset --hard HEAD@{1}` |
| pre-commit | squash | agent lint fix | 0 (fixup+squash) | original unchanged | clean | same as above |
| pre-commit | dirty | format | 0, blocked | — | dirty | `linthis undo` |
| pre-commit | dirty | agent lint fix | 0, blocked | — | dirty | `linthis undo` |
| pre-commit | fixup | format + lint fix | 1 (post-commit) | `fix(linthis): auto-fix lint issues` | clean | `git reset HEAD~1` |
| pre-push | squash | lint fix | 0 (fixup+squash) | original unchanged | clean, push continues | `git reset --hard HEAD@{1}` |
| pre-push | squash | review fix | 0 (fixup+squash) | original unchanged | clean, push continues | `git reset --hard HEAD@{1}` |
| pre-push | dirty | lint fix | 0, blocked | — | unchanged | — |
| pre-push | dirty | review fix | 0, blocked | — | dirty | `linthis undo` |
| pre-push | fixup | lint fix | 1, blocked | `fix(linthis): auto-fix lint issues` | clean | `git reset HEAD~1` |
| pre-push | fixup | review fix | 1, blocked | `fix(linthis): auto-fix review issues` | clean | `git reset HEAD~1` |

### agent type (skill-based)

| Event | Mode | New commits | Commit message | Working tree | Recover |
|-------|------|-------------|----------------|-------------|---------|
| pre-commit | squash | 0 | original (agent `git add` + approve) | clean | `linthis undo` |
| pre-commit | dirty | 0, AskUser | — | dirty (agent fixes unstaged) | `linthis undo` |
| pre-commit | fixup | 0 | — (agent check only, post-commit handles) | unchanged | — |
| pre-push | squash | 0 | original (agent amend) | clean | `git reflog` + `git reset --hard` |
| pre-push | dirty | 0, blocked | — | dirty | `linthis undo` |
| pre-push | fixup | 1, blocked | `fix(linthis): auto-fix review issues` | clean | `git reset HEAD~1` |

---

## Dirty Mode Prompt

When dirty mode blocks a commit/push:

```
[linthis] Files formatted but not staged (dirty mode).
  Review:  git diff
  Accept:  git add -u && git commit
  Revert:  linthis undo
```

## Recovery Priority

| Method | When to use | Safety |
|--------|-------------|--------|
| `linthis undo` | Any mode — restores only linthis-modified files | Safest |
| `linthis backup diff` | Review what linthis changed before deciding | Read-only |
| `git stash pop` | squash mode — restore pre-format stash snapshot | Safe |
| `git reset HEAD~1` | fixup mode — remove the fixup commit | Safe |
| `git reset --hard HEAD@{1}` | squash mode — restore from reflog | Destructive to working tree |

## Hook Footer Display

The current mode is shown in the hook output footer:

```
Global: ~/.config/git/hooks/pre-commit (--type git-with-agent, --fix-commit-mode squash)
Local:  .git/hooks/pre-commit (--type git-with-agent, --fix-commit-mode squash)
```
