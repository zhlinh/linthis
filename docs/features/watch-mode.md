# Watch Mode

Watch mode allows linthis to continuously monitor your files and automatically run checks when files change.

## Basic Usage

```bash
# Watch current directory
linthis watch

# Watch specific directories
linthis watch -i src/ -i lib/

# Watch with specific languages
linthis watch --lang python,rust
```

## Options

| Option | Description |
|--------|-------------|
| `-i, --include` | Directories to watch |
| `-e, --exclude` | Patterns to exclude |
| `-l, --lang` | Languages to check |
| `-c, --check-only` | Only check, don't format |
| `-f, --format-only` | Only format, don't check |
| `--debounce` | Debounce time in milliseconds (default: 500) |

## Examples

### Watch with Check Only

```bash
linthis watch --check-only
```

### Watch with Custom Debounce

```bash
linthis watch --debounce 1000
```

### Watch Specific File Types

```bash
linthis watch --lang python
```

## How It Works

1. linthis monitors the specified directories for file changes
2. When a file is modified, added, or deleted, it triggers a lint/format run
3. Changes are debounced to avoid running multiple times for rapid changes
4. Only changed files are processed for efficiency

## Keyboard Shortcuts

While in watch mode:

| Key | Action |
|-----|--------|
| `q` | Quit watch mode |
| `r` | Force re-run on all files |
| `c` | Clear screen |

## Integration with Editors

Watch mode works well alongside editor integrations:

- **VSCode**: Use the terminal panel
- **Neovim/Vim**: Use a split terminal
- **Emacs**: Use shell-mode

## Performance Considerations

- Watch mode uses efficient file system watchers (inotify on Linux, FSEvents on macOS)
- Only changed files are re-checked, not the entire project
- Debouncing prevents excessive runs during rapid edits
