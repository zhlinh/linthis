# Plugin Auto-Sync

## Overview

linthis supports automatic plugin synchronization, inspired by oh-my-zsh's auto-update mechanism. This feature automatically checks and syncs plugin updates when running linthis, ensuring you always use the latest plugin configurations.

## Features

- **Configurable sync interval**: Customize how often to check for updates (default: 7 days)
- **Multiple sync modes**:
  - `auto`: Sync automatically without confirmation
  - `prompt`: Ask user before syncing (default)
  - `disabled`: Disable auto-sync
- **Smart time tracking**: Uses Unix timestamps to avoid timezone issues
- **Smart update detection**: Only prompts when actual updates are available
- **Graceful user interaction**: Clear progress indicators and error handling

## Configuration

### Configuration File

Add to `.linthis/config.toml` or `~/.linthis/config.toml`:

```toml
# Plugin settings
[plugin]
sources = [
    { name = "myplugin", url = "https://github.com/your-org/myplugin.git", ref = "main" }
]

# Plugin auto-sync settings
[plugin_auto_sync]
enabled = true           # Enable auto-sync
mode = "prompt"          # Sync mode: "auto", "prompt", "disabled"
interval_days = 7        # Sync interval (days)
```

### Configuration Options

#### `enabled`
- **Type**: Boolean
- **Default**: `true`
- **Description**: Whether to enable auto-sync

#### `mode`
- **Type**: String
- **Default**: `"prompt"`
- **Options**:
  - `"auto"`: Sync automatically without user confirmation
  - `"prompt"`: Ask user before syncing
  - `"disabled"`: Disable auto-sync

#### `interval_days`
- **Type**: Integer
- **Default**: `7`
- **Description**: Number of days between sync checks

## How It Works

### Time Tracking

Auto-sync uses `~/.linthis/.plugin_sync_last_check` to store the timestamp (Unix epoch seconds) of the last sync.

### Trigger

Each time linthis runs, the system:
1. Loads `plugin_auto_sync` settings from config
2. Checks `~/.linthis/.plugin_sync_last_check` file
3. Calculates time since last sync
4. If interval exceeded, triggers sync flow

### Sync Flow

Based on configured `mode`:

- **auto mode**:
  1. Check all plugins for updates
  2. If updates available, automatically start sync
  3. Display sync progress
  4. Update timestamp

- **prompt mode**:
  1. Check all plugins for updates
  2. If updates available, prompt: `Updates available for plugins. Update now? [Y/n]:`
  3. Wait for user input
  4. If confirmed, execute sync
  5. If declined or no updates, skip and update timestamp

- **disabled mode**:
  - Skip all checks

## Examples

### Example 1: Default (Prompt Mode)

```toml
[plugin_auto_sync]
enabled = true
mode = "prompt"
interval_days = 7
```

When interval exceeded and updates available:
```bash
$ linthis
Updates available for plugins. Update now? [Y/n]: y
↓ Syncing project plugins...
  ↓ myplugin... ✓ @ a1b2c3d
✓ Synced 1 plugin(s), 1 updated
```

### Example 2: Auto Mode

```toml
[plugin_auto_sync]
enabled = true
mode = "auto"
interval_days = 3
```

Auto-syncs every 3 days without confirmation:
```bash
$ linthis
↓ Syncing project plugins...
  ↓ myplugin... ✓ @ a1b2c3d
✓ Synced 1 plugin(s), 1 updated
```

### Example 3: Disable Auto-Sync

```toml
[plugin_auto_sync]
enabled = false
```

Or:

```toml
[plugin_auto_sync]
mode = "disabled"
```

### Example 4: Manual Sync

You can always manually sync regardless of auto-sync settings:

```bash
# Sync project plugins
linthis plugin sync

# Sync global plugins
linthis plugin sync -g
```

## Comparison with oh-my-zsh

| Feature | oh-my-zsh | linthis |
|---------|-----------|---------|
| Default interval | 13 days | 7 days |
| Sync modes | auto, prompt, disabled | auto, prompt, disabled |
| Time tracking | `~/.zsh-update` | `~/.linthis/.plugin_sync_last_check` |
| Manual sync | `omz update` | `linthis plugin sync` |
| Smart detection | No | Yes (only prompts when updates exist) |

## Configuration Priority

Configuration is loaded with the following priority (highest to lowest):
1. Project config (`.linthis/config.toml`)
2. Global config (`~/.linthis/config.toml`)
3. Built-in defaults

## Troubleshooting

### Auto-sync not working

**Checklist**:
1. Confirm `plugin_auto_sync.enabled = true`
2. Confirm `plugin_auto_sync.mode` is not `"disabled"`
3. Check permissions on `~/.linthis/.plugin_sync_last_check`
4. Check for error messages in output

### Prompts too frequent

**Solution**: Increase `interval_days`:
```toml
[plugin_auto_sync]
interval_days = 14  # Change to 14 days
```

### Want to completely disable

**Solution**:
```toml
[plugin_auto_sync]
enabled = false
```

## Relationship with Self-Update

linthis supports both:
1. **Plugin Auto-Sync** (this feature): Sync plugins
2. **Self Auto-Update**: Update linthis itself

Both are configured and run independently:
```toml
# Sync plugins
[plugin_auto_sync]
enabled = true
mode = "prompt"
interval_days = 7

# Update linthis itself
[self_auto_update]
enabled = true
mode = "prompt"
interval_days = 7
```

Execution order:
1. Check for linthis self-update first
2. Then check for plugin sync

## References

- [oh-my-zsh auto-update mechanism](https://maxchadwick.xyz/blog/a-look-at-auto-updating-in-oh-my-zsh)
- [oh-my-zsh settings documentation](https://github.com/ohmyzsh/ohmyzsh/wiki/Settings)
