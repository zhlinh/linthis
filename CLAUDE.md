# Linthis Development Instructions

## Build & Install

Every time code is modified, rebuild and reinstall:

```bash
cargo build --release && cargo install --path . --force
```

This is **mandatory** after any code change to ensure the installed binary is up to date.

## Linthis Agent Rules

### After modifying code files

After editing code files, run linthis to check for issues:

```bash
linthis -i <file1> -i <file2> -c
```

- Use separate `-i` flags for each modified file
- Use `-c` (check-only) — do NOT use `--fix` or `linthis fix`
- If lint issues are found, **fix them yourself by editing the code directly**, then re-run linthis to confirm

### Before committing

Always run linthis on staged files before any `git commit`:

```bash
linthis -s -c
```

If issues are found, fix them by editing the code, re-stage, and re-check until clean.

### Key principle

Never rely on `linthis --fix` or `linthis fix` for automated fixing. Always read the lint errors, understand them, and apply fixes manually through code edits. This ensures higher quality fixes with proper context awareness.
