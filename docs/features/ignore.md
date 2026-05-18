# Ignore Rules — `.linthisignore`

The `.linthisignore` file lets you quickly suppress lint issues without editing `.linthis/config.toml`. It lives at the project root and supports two types of entries: **file glob patterns** and **rule codes**.

## File Format

```gitignore
# Comment lines and blank lines are ignored

# File/path glob patterns (gitignore syntax)
vendor/**
*.generated.go
build/

# Rule disable entries — prefix with "rule:"
rule:E501
rule:clippy::too_many_arguments
rule:complexity/*
```

**Parsing rules:**

| Line | Effect |
|------|--------|
| Empty or starts with `#` | Ignored |
| Starts with `rule:` | Disables that rule code |
| Anything else | File glob pattern — matching files are excluded from linting |

## Using the CLI

```bash
# Add a file glob pattern
linthis ignore add "vendor/**"
linthis ignore add "*.generated.go"

# Add a rule disable entry (include the "rule:" prefix)
linthis ignore add "rule:E501"
linthis ignore add "rule:clippy::too_many_arguments"
linthis ignore add "rule:linthis-complexity"

# Remove an entry (exact match)
linthis ignore remove "vendor/**"
linthis ignore remove "rule:E501"

# List all current entries
linthis ignore list
```

`add` is idempotent — adding an entry that already exists prints a notice and does nothing.

## Adding Ignores from Interactive Fix Mode

When using `linthis fix` interactively, press **`w`** on any issue to write `rule:<code>` to `.linthisignore`. The rule is permanently suppressed for all future runs — no config file editing needed.

```
[W1] [rust][clippy] src/main.rs:42
  Complexity exceeds threshold

Actions: [e]dit  [i]gnore  [a]ccept  [w]rite-ignore  [s]kip  [p]rev  [g]oto  [q]uit
> w
✓ Written to .linthisignore: rule:linthis-complexity
```

## How It Works

`.linthisignore` is applied at **two points** in the pipeline:

### 1. Path Patterns — Pre-lint File Exclusion

Path glob patterns are loaded by the file scanner before linting begins. Files matching any pattern are excluded entirely — identical in effect to listing them in `excludes` in `config.toml`.

### 2. Rule Codes — Post-lint Filtering

Rule codes (the `rule:` prefix entries) are merged into `config.rules.disable` before the rule filter runs. Any issue whose code matches a disabled rule is removed from the output. This applies to lint, complexity, and security findings.

## Relationship to `config.toml`

`.linthisignore` is complementary to `.linthis/config.toml`.

| Use case | Where to configure |
|----------|--------------------|
| Quick per-branch or per-developer suppression | `.linthisignore` |
| Auto-added by `linthis fix` interactive mode | `.linthisignore` |
| Team-wide permanent configuration | `.linthis/config.toml` |

Equivalent `config.toml` configuration:

```toml
excludes = ["vendor/**", "*.generated.go"]

[rules]
disable = ["E501", "clippy::too_many_arguments"]
```

## Example `.linthisignore`

```gitignore
# Ignore generated and vendor files
vendor/**
*.pb.go
build/
dist/

# Disable specific lint rules
rule:E501
rule:clippy::too_many_arguments

# Disable all complexity checks
rule:linthis-complexity

# Disable a rule category with a wildcard prefix
rule:clippy::*
```

## Priority and Override

`.linthisignore` entries are applied on top of `config.toml` — they extend the `excludes` and `rules.disable` lists rather than replacing them. `config.toml` takes precedence for scalar fields like `max_complexity`.

## See Also

- [Configuration](../getting-started/configuration.md) — full `config.toml` reference
- [CLI Reference: ignore](../reference/cli.md#ignore) — command flags
- [AI-Powered Fix](ai-fix.md) — interactive fix mode
