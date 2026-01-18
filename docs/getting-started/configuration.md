# Configuration

linthis can be configured through configuration files, CLI parameters, or both.

## Configuration Files

### Project Configuration

Create `.linthis.toml` in your project root:

```toml
# Specify languages to check (omit for auto-detection)
languages = ["rust", "python", "javascript"]

# Exclude files and directories
excludes = [
    "target/**",
    "node_modules/**",
    "*.generated.rs",
    "dist/**"
]

# Maximum cyclomatic complexity
max_complexity = 20

# Format preset
preset = "google"  # Options: google, airbnb, standard

# Configure plugins
[plugins]
sources = [
    { name = "official" },
    { name = "myplugin", url = "https://github.com/user/plugin.git", ref = "main" }
]

# Language-specific configuration
# [rust]
# max_complexity = 15

# [python]
# excludes = ["*_test.py"]
```

### Global Configuration

Global configuration is located at `~/.linthis/config.toml`, with the same format as project config.

### Configuration Priority

Configuration merge priority (from high to low):

1. **CLI Parameters**: `--option value`
2. **Project Config**: `.linthis.toml`
3. **Global Config**: `~/.linthis/config.toml`
4. **Plugin Config**: Plugins in sources array (later ones override earlier ones)
5. **Built-in Defaults**

## Configuration Management Commands

### Array Field Operations

Supported array fields: `includes`, `excludes`, `languages`

```bash
# Add values
linthis config add includes "src/**"
linthis config add excludes "*.log"
linthis config add languages "rust"

# Add to global config
linthis config add -g includes "lib/**"

# Remove values
linthis config remove excludes "*.log"

# Clear field
linthis config clear languages
```

### Scalar Field Operations

Supported scalar fields: `max_complexity`, `preset`, `verbose`

```bash
# Set value
linthis config set max_complexity 15
linthis config set preset google

# Set in global config
linthis config set -g max_complexity 20

# Unset value
linthis config unset max_complexity
```

### Query Operations

```bash
# Get single field
linthis config get includes
linthis config get max_complexity

# List all configuration
linthis config list
linthis config list -g  # global config
linthis config list -v  # verbose (show empty fields)
```

## Configuration Migration

linthis can migrate existing linter/formatter configurations:

```bash
# Auto-detect and migrate all configs
linthis config migrate

# Migrate specific tool
linthis config migrate --from eslint
linthis config migrate --from prettier
linthis config migrate --from black

# Preview changes
linthis config migrate --dry-run

# Create backup
linthis config migrate --backup
```

### Supported Tools

| Tool | Detected Files |
|------|---------------|
| ESLint | `.eslintrc.js`, `.eslintrc.json`, `.eslintrc.yml`, `eslint.config.js` |
| Prettier | `.prettierrc`, `.prettierrc.json`, `.prettierrc.yml`, `prettier.config.js` |
| Black | `pyproject.toml[tool.black]` |
| isort | `pyproject.toml[tool.isort]` |

## Next Steps

- [Plugin System](../features/plugins.md) - Share configurations
- [CLI Reference](../reference/cli.md) - All command options
