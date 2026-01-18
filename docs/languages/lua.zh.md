# Lua 语言指南

linthis 使用 **luacheck** 进行代码检查，使用 **stylua** 进行代码格式化。

## 支持的文件扩展名

- `.lua`

## 必需工具

### 代码检查：luacheck

```bash
# 通过 LuaRocks
luarocks install luacheck

# macOS 通过 Homebrew
brew install luacheck

# 验证安装
luacheck --version
```

### 格式化：stylua

```bash
# 通过 Cargo
cargo install stylua

# macOS 通过 Homebrew
brew install stylua

# 验证安装
stylua --version
```

## 配置

### 基本示例

```toml
# .linthis/config.toml

[lua]
max_complexity = 15
excludes = ["vendor/**", "*.min.lua"]
```

### 禁用特定规则

```toml
[lua.rules]
disable = [
    "W211",   # 未使用的变量
    "W212"    # 未使用的参数
]
```

## 自定义规则

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

## CLI 用法

```bash
# 仅检查 Lua 文件
linthis -c --lang lua

# 仅格式化 Lua 文件
linthis -f --lang lua
```

## Luacheck 配置

创建 `.luacheckrc`：

```lua
std = "lua51+luajit"
codes = true

globals = {
    "vim",           -- Neovim 全局变量
    "awesome",       -- AwesomeWM
}

ignore = {
    "212",  -- 未使用的参数
    "213",  -- 未使用的循环变量
}

max_line_length = 120
```

## StyLua 配置

创建 `.stylua.toml`：

```toml
column_width = 120
line_endings = "Unix"
indent_type = "Spaces"
indent_width = 4
quote_style = "AutoPreferDouble"
call_parentheses = "Always"
```

## 常见问题

### Luacheck 未找到

```
Warning: No lua linter available for lua files
  Install: luarocks install luacheck
```

### Neovim/游戏引擎全局变量

在 `.luacheckrc` 中配置全局变量：

```lua
-- 对于 Neovim
globals = { "vim" }

-- 对于 Love2D
globals = { "love" }

-- 对于 Corona SDK
globals = { "display", "Runtime", "system" }
```

### 第三方代码

添加到排除项：

```toml
[lua]
excludes = ["vendor/**", "lib/**"]
```

## 最佳实践

1. **使用 local**：始终对变量使用 `local`
2. **配置全局变量**：在 `.luacheckrc` 中声明框架全局变量
3. **一致的风格**：使用 StyLua 保持一致的格式化
4. **模块模式**：使用正确的模块模式编写清晰代码
