# Self-Update

## Overview

linthis supports automatic self-update functionality, inspired by oh-my-zsh's auto-update mechanism. This feature automatically checks and updates linthis itself when running, ensuring you always use the latest version.

## Features

- **Configurable check interval**: Customize how often to check for updates (default: 7 days)
- **Multiple update modes**:
  - `auto`: Update automatically without confirmation
  - `prompt`: Ask user before updating (default)
  - `disabled`: Disable auto-update
- **Smart update detection**: Only prompts when new version is available; silently updates timestamp when no update exists
- **Smart time tracking**: Uses Unix timestamps to avoid timezone issues
- **PyPI version detection**: Checks latest version via pip
- **Graceful user interaction**: Clear progress indicators and error handling

## Configuration

### Configuration File

Add to `.linthis/config.toml` or `~/.linthis/config.toml`:

```toml
# Self-update settings
[self_auto_update]
enabled = true           # Enable auto-update checking
mode = "prompt"          # Update mode: "auto", "prompt", "disabled"
interval_days = 7        # Check interval (days)
```

### Configuration Options

#### `enabled`
- **Type**: Boolean
- **Default**: `true`
- **Description**: Whether to enable auto-update checking

#### `mode`
- **Type**: String
- **Default**: `"prompt"`
- **Options**:
  - `"auto"`: Update automatically without user confirmation
  - `"prompt"`: Ask user before updating
  - `"disabled"`: Disable auto-update

#### `interval_days`
- **Type**: Integer
- **Default**: `7`
- **Description**: Number of days between update checks

## How It Works

### Time Tracking

Auto-update uses `~/.linthis/.self_update_last_check` to store the timestamp (Unix epoch seconds) of the last check.

### Trigger

Each time linthis runs, the system:
1. Loads `self_update` settings from config
2. Checks `~/.linthis/.self_update_last_check` file
3. Calculates time since last check
4. If interval exceeded, triggers update check flow

### Update Flow

Based on configured `mode`:

- **auto mode**:
  1. Check PyPI for latest version via `pip index versions linthis`
  2. **If no new version**: Silently update timestamp, no prompt
  3. **If new version available**: Automatically run `pip install --upgrade linthis`
  4. Display update progress
  5. Update timestamp

- **prompt mode**:
  1. Check PyPI for latest version
  2. **If no new version**: Silently update timestamp, no prompt
  3. **If new version available**: Prompt `A new version of linthis is available: 0.0.4 → 0.0.5. Update now? [Y/n]:`
  4. Wait for user input
  5. If confirmed, execute update
  6. If declined, skip and update timestamp

- **disabled mode**:
  - Skip all checks

**Important**: Only prompts or auto-updates when a new version is detected. No unnecessary interruptions when already up-to-date.

## Examples

### Example 1: Default (Prompt Mode)

```toml
[self_auto_update]
enabled = true
mode = "prompt"
interval_days = 7
```

**When interval exceeded and new version available**:
```bash
$ linthis
A new version of linthis is available: 0.0.4 → 0.0.5. Update now? [Y/n]: y
↓ Upgrading linthis via pip...
✓ linthis upgraded successfully
```

**When interval exceeded but no new version**:
```bash
$ linthis
# Silently updates check timestamp, no prompt
# Continues with normal linting flow
```

### Example 2: Auto Mode

```toml
[self_auto_update]
enabled = true
mode = "auto"
interval_days = 3
```

Auto-checks every 3 days and updates without confirmation:
```bash
$ linthis
↓ Upgrading linthis via pip...
✓ linthis upgraded successfully
```

### Example 3: Disable Auto-Update

```toml
[self_auto_update]
enabled = false
```

Or:

```toml
[self_auto_update]
mode = "disabled"
```

### Example 4: Manual Update

You can always manually update:

```bash
pip install --upgrade linthis
```

## Comparison with oh-my-zsh

| Feature | oh-my-zsh | linthis |
|---------|-----------|---------|
| Default interval | 13 days | 7 days |
| Update modes | auto, reminder, disabled | auto, prompt, disabled |
| Time tracking | `~/.zsh-update` | `~/.linthis/.self_update_last_check` |
| Update method | git pull | pip install --upgrade |

## Configuration Priority

Configuration is loaded with the following priority (highest to lowest):
1. Project config (`.linthis/config.toml`)
2. Global config (`~/.linthis/config.toml`)
3. Built-in defaults

## Troubleshooting

### Auto-update not working

**Checklist**:
1. Confirm `self_update.enabled = true`
2. Confirm `self_update.mode` is not `"disabled"`
3. Check permissions on `~/.linthis/.self_update_last_check`
4. Check for error messages in output
5. Confirm `pip` is available: `pip --version`

### Prompts too frequent

**Solution**: Increase `interval_days`:
```toml
[self_auto_update]
interval_days = 14  # Change to 14 days
```

### Want to completely disable

**Solution**:
```toml
[self_auto_update]
enabled = false
```

### pip permission issues

If you encounter permission issues, try user mode install:
```bash
pip install --user --upgrade linthis
```

Or use `sudo` (not recommended):
```bash
sudo pip install --upgrade linthis
```

## Relationship with Plugin Auto-Sync

linthis supports both:
1. **Self-Update** (this feature): Update linthis itself
2. **Auto-Sync**: Sync plugins automatically

Both are configured and run independently:
```toml
# Update linthis itself
[self_auto_update]
enabled = true
mode = "prompt"
interval_days = 7

# Sync plugins
[plugin_auto_sync]
enabled = true
mode = "prompt"
interval_days = 7
```

Execution order:
1. Check for linthis self-update first
2. Then check for plugin sync

## References

- [pip index versions documentation](https://pip.pypa.io/en/stable/cli/pip_index/)
- [oh-my-zsh auto-update mechanism](https://maxchadwick.xyz/blog/a-look-at-auto-updating-in-oh-my-zsh)
- [Semantic Versioning](https://semver.org/)
