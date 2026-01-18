# Creating Plugins

This guide explains how to create and distribute linthis plugins.

## What is a Plugin?

A linthis plugin is a Git repository containing:
- A `linthis-plugin.toml` manifest file
- One or more configuration files (TOML, YAML, or JSON)
- Optional custom rules and presets

Plugins allow you to share lint configurations across projects or teams.

## Plugin Structure

```
my-linthis-plugin/
├── linthis-plugin.toml      # Required: Plugin manifest
├── config.toml              # Main configuration
├── rules/                   # Optional: Additional rule configs
│   ├── strict.toml
│   └── relaxed.toml
└── README.md                # Optional: Documentation
```

## Plugin Manifest

The `linthis-plugin.toml` manifest describes your plugin:

```toml
[plugin]
# Required fields
name = "my-plugin"
version = "1.0.0"

# Optional fields
description = "Custom lint rules for my organization"
author = "Your Name <email@example.com>"
repository = "https://github.com/username/my-linthis-plugin"
license = "MIT"

# Minimum linthis version required (optional)
min_linthis_version = "0.1.0"

[config]
# Path to the main configuration file
path = "config.toml"

# Optional: Additional config files that can be referenced
# [config.presets]
# strict = "rules/strict.toml"
# relaxed = "rules/relaxed.toml"
```

## Configuration File

The main configuration file follows the standard linthis config format:

```toml
# config.toml

# Exclude patterns (will be merged with project config)
excludes = ["vendor/**", "third_party/**"]

# Default max complexity for this plugin's style guide
max_complexity = 15

# Rule modifications
[rules]
disable = ["E501"]  # Disable line length checks

[rules.severity]
"W0612" = "error"   # Treat unused variables as errors

# Custom rules
[[rules.custom]]
code = "org/no-fixme"
pattern = "FIXME|XXX"
message = "FIXME comments must be resolved before merge"
severity = "error"
suggestion = "Create a tracking issue or resolve the FIXME"

[[rules.custom]]
code = "org/copyright-header"
pattern = "^(?!// Copyright)"
message = "Missing copyright header"
severity = "warning"
extensions = ["rs", "go", "java"]

# Language-specific settings
[rust]
max_complexity = 12

[python]
excludes = ["*_test.py", "test_*.py"]
```

## Creating Your First Plugin

### Step 1: Create Repository

```bash
mkdir my-linthis-plugin
cd my-linthis-plugin
git init
```

### Step 2: Create Manifest

Create `linthis-plugin.toml`:

```toml
[plugin]
name = "my-plugin"
version = "1.0.0"
description = "My custom lint configuration"

[config]
path = "config.toml"
```

### Step 3: Create Configuration

Create `config.toml`:

```toml
# My organization's lint rules

excludes = ["generated/**"]
max_complexity = 20

[[rules.custom]]
code = "custom/no-console-log"
pattern = "console\\.log"
message = "Remove console.log before committing"
severity = "warning"
languages = ["typescript", "javascript"]
```

### Step 4: Test Locally

Before publishing, test your plugin locally:

```bash
# In your project's .linthis/config.toml
[plugins]
sources = [
    { name = "local-test", url = "file:///path/to/my-linthis-plugin" }
]
```

Then run:

```bash
linthis plugin init
linthis -c
```

### Step 5: Publish

Push to GitHub or any Git host:

```bash
git add .
git commit -m "Initial plugin release"
git tag v1.0.0
git push origin main --tags
```

## Using Your Plugin

Add to your project's `.linthis/config.toml`:

```toml
[plugins]
sources = [
    { name = "my-plugin", url = "https://github.com/username/my-linthis-plugin.git" }
]
```

Or pin to a specific version:

```toml
[plugins]
sources = [
    { name = "my-plugin", url = "https://github.com/username/my-linthis-plugin.git", ref = "v1.0.0" }
]
```

## Plugin Commands

```bash
# Initialize/download plugins
linthis plugin init

# List installed plugins
linthis plugin list

# Update all plugins to latest
linthis plugin update

# Update specific plugin
linthis plugin update my-plugin

# Clean plugin cache
linthis plugin clean
```

## Configuration Merging

Plugin configurations are merged with your project configuration. The precedence is:

1. **CLI arguments** (highest)
2. **Project config** (`.linthis/config.toml`)
3. **Plugin configs** (in order listed)
4. **User config** (`~/.linthis/config.toml`)
5. **Built-in defaults** (lowest)

Array fields (`excludes`, `includes`, `rules.disable`, `rules.custom`) are **extended** (added to), while scalar fields are **overridden**.

## Best Practices

### 1. Version Your Plugin

Use semantic versioning and Git tags:

```bash
git tag v1.0.0   # Initial release
git tag v1.1.0   # New features (backward compatible)
git tag v2.0.0   # Breaking changes
```

### 2. Document Your Rules

Add comments explaining each custom rule:

```toml
# Prevent debug code from being committed
[[rules.custom]]
code = "org/no-debug"
pattern = "debugger|console\\.debug"
message = "Debug code should not be committed"
```

### 3. Set Minimum Version

If your plugin uses features from a specific linthis version:

```toml
[plugin]
min_linthis_version = "0.2.0"
```

### 4. Provide Presets

For flexibility, offer multiple configuration levels:

```
my-plugin/
├── config.toml           # Default (recommended) settings
├── presets/
│   ├── strict.toml       # Stricter rules for CI
│   └── relaxed.toml      # More lenient for prototyping
```

### 5. Test Thoroughly

Before releasing, test your plugin:
- On multiple languages your rules target
- With existing project configurations
- After linthis updates

## Example Plugins

### Organization Style Guide

```toml
# linthis-plugin.toml
[plugin]
name = "acme-style"
version = "2.0.0"
description = "ACME Corp coding standards"

[config]
path = "config.toml"
```

```toml
# config.toml
max_complexity = 15
preset = "google"

[rules]
disable = ["E501"]  # We use 120-char lines

[[rules.custom]]
code = "acme/ticket-ref"
pattern = "TODO(?!.*\\[ACME-\\d+\\])"
message = "TODO comments must reference a JIRA ticket [ACME-XXX]"
severity = "warning"

[rust]
max_complexity = 12

[python]
max_complexity = 10
```

### Security Rules Plugin

```toml
# linthis-plugin.toml
[plugin]
name = "security-rules"
version = "1.0.0"
description = "Security-focused lint rules"

[config]
path = "security.toml"
```

```toml
# security.toml
[[rules.custom]]
code = "sec/no-eval"
pattern = "\\beval\\s*\\("
message = "Avoid eval() - potential code injection vulnerability"
severity = "error"
languages = ["javascript", "typescript", "python"]

[[rules.custom]]
code = "sec/no-hardcoded-secret"
pattern = "(password|secret|api_key|token)\\s*=\\s*['\"][^'\"]+['\"]"
message = "Potential hardcoded secret detected"
severity = "error"

[[rules.custom]]
code = "sec/no-http"
pattern = "http://"
message = "Use HTTPS instead of HTTP"
severity = "warning"
```

## Troubleshooting

### Plugin not loading

1. Check `linthis plugin list` to see installed plugins
2. Run `linthis plugin init` to re-fetch
3. Verify the Git URL is accessible
4. Check for manifest errors in `linthis-plugin.toml`

### Configuration not applied

1. Check merge order (later plugins override earlier)
2. Verify the config file path in manifest
3. Run with `--verbose` to see config loading

### Version conflicts

If you see version compatibility errors:
1. Update linthis to the required version
2. Or use an older plugin version with `ref = "v1.0.0"`
