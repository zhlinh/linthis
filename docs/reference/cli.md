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
| `--type` | Hook type: `prek`, `pre-commit`, `git` |
| `--event` | Hook event: `pre-commit`, `pre-push`, `commit-msg` |
| `-c, --check-only` | Hook only runs check |
| `-f, --format-only` | Hook only runs format |
| `--force` | Force overwrite existing |
| `-y, --yes` | Non-interactive mode |

**Examples:**

```bash
linthis hook install --type git
linthis hook install --type git --event pre-push
linthis hook install --type prek --check-only
```

### hook uninstall

```bash
linthis hook uninstall [OPTIONS]
```

| Option | Description |
|--------|-------------|
| `--event` | Hook event to uninstall |
| `-y, --yes` | Non-interactive mode |

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
