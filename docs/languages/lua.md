# Lua Language Guide

linthis uses **luacheck** for linting and **stylua** for formatting Lua code.

## Supported File Extensions

- `.lua`

## Required Tools

### Linter: luacheck

```bash
# Via LuaRocks
luarocks install luacheck

# macOS via Homebrew
brew install luacheck

# Verify installation
luacheck --version
```

### Formatter: stylua

```bash
# Via Cargo
cargo install stylua

# macOS via Homebrew
brew install stylua

# Verify installation
stylua --version
```

## Configuration

### Basic Example

```toml
# .linthis/config.toml

[lua]
max_complexity = 15
excludes = ["vendor/**", "*.min.lua"]
```

### Disable Specific Rules

```toml
[lua.rules]
disable = [
    "W211",   # Unused variable
    "W212"    # Unused argument
]
```

## Custom Rules

```toml
[[rules.custom]]
code = "lua/no-global"
pattern = "^[a-zA-Z_][a-zA-Z0-9_]*\\s*="
message = "Avoid implicit globals - use 'local'"
severity = "warning"
languages = ["lua"]

[[rules.custom]]
code = "lua/no-print"
pattern = "\\bprint\\s*\\("
message = "Use logging instead of print()"
severity = "info"
languages = ["lua"]
```

## CLI Usage

```bash
# Check Lua files only
linthis -c --lang lua

# Format Lua files only
linthis -f --lang lua
```

## Luacheck Configuration

Create `.luacheckrc`:

```lua
std = "lua51+luajit"
codes = true

globals = {
    "vim",           -- Neovim globals
    "awesome",       -- AwesomeWM
}

ignore = {
    "212",  -- Unused argument
    "213",  -- Unused loop variable
}

max_line_length = 120
```

## StyLua Configuration

Create `.stylua.toml`:

```toml
column_width = 120
line_endings = "Unix"
indent_type = "Spaces"
indent_width = 4
quote_style = "AutoPreferDouble"
call_parentheses = "Always"
```

## Common Issues

### Luacheck not found

```
Warning: No lua linter available for lua files
  Install: luarocks install luacheck
```

### Neovim/game engine globals

Configure globals in `.luacheckrc`:

```lua
-- For Neovim
globals = { "vim" }

-- For Love2D
globals = { "love" }

-- For Corona SDK
globals = { "display", "Runtime", "system" }
```

### Vendor/third-party code

Add to excludes:

```toml
[lua]
excludes = ["vendor/**", "lib/**"]
```

## Best Practices

1. **Use local**: Always use `local` for variables
2. **Configure globals**: Declare framework globals in `.luacheckrc`
3. **Consistent style**: Use StyLua for consistent formatting
4. **Module pattern**: Use proper module patterns for clean code
