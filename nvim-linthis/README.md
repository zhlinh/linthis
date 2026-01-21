# nvim-linthis

Neovim plugin for [linthis](https://github.com/zhlinh/linthis) - a multi-language linter and formatter.

## Features

- LSP integration with linthis
- Support for 18+ programming languages
- Format on save (optional)
- Lint on save with diagnostics
- User commands for manual operations

## Requirements

- Neovim >= 0.10.0
- [linthis](https://github.com/zhlinh/linthis) installed

```bash
# Install linthis
cargo install linthis

# Or download from releases
# https://github.com/zhlinh/linthis/releases
```

## Installation

### [lazy.nvim](https://github.com/folke/lazy.nvim)

```lua
{
  "linthis/nvim-linthis",
  opts = {
    format_on_save = false,
    lint_on_save = true,
  },
}
```

### [packer.nvim](https://github.com/wbthomason/packer.nvim)

```lua
use {
  "linthis/nvim-linthis",
  config = function()
    require("linthis").setup({
      format_on_save = false,
      lint_on_save = true,
    })
  end,
}
```

### Manual

Clone to your Neovim packages directory:

```bash
git clone https://github.com/linthis/nvim-linthis \
  ~/.local/share/nvim/site/pack/plugins/start/nvim-linthis
```

## Configuration

```lua
require("linthis").setup({
  -- Path to linthis executable
  cmd = { "linthis", "lsp" },

  -- Filetypes to attach to
  filetypes = {
    "rust", "python", "typescript", "javascript",
    "typescriptreact", "javascriptreact", "go", "java",
    "c", "cpp", "objc", "swift", "kotlin", "lua",
    "dart", "sh", "bash", "zsh", "ruby", "php", "scala", "cs",
  },

  -- Root directory patterns
  root_markers = {
    ".linthis.toml", ".git", "Cargo.toml",
    "pyproject.toml", "package.json", "go.mod",
    "pom.xml", "build.gradle", "Makefile",
  },

  -- Auto-start LSP when opening supported files
  autostart = true,

  -- Format on save
  format_on_save = false,

  -- Lint on save
  lint_on_save = true,

  -- Show notifications
  notifications = true,

  -- Log level: "debug", "info", "warn", "error"
  log_level = "warn",
})
```

## Commands

| Command | Description |
|---------|-------------|
| `:LinthisFormat` | Format current buffer |
| `:LinthisLint` | Lint current buffer |
| `:LinthisRestart` | Restart LSP server |
| `:LinthisInfo` | Show LSP info |

## Lua API

```lua
local linthis = require("linthis")

-- Format current buffer
linthis.format()

-- Format with options
linthis.format({
  bufnr = 0,        -- buffer number
  async = false,    -- async formatting
  timeout_ms = 5000,
  silent = true,    -- suppress notifications
})

-- Lint current buffer
linthis.lint()

-- Restart LSP
linthis.restart()

-- Show info
linthis.info()
```

## Keymaps

Example keymaps (add to your config):

```lua
vim.keymap.set("n", "<leader>lf", "<cmd>LinthisFormat<cr>", { desc = "Format with linthis" })
vim.keymap.set("n", "<leader>ll", "<cmd>LinthisLint<cr>", { desc = "Lint with linthis" })
vim.keymap.set("n", "<leader>lr", "<cmd>LinthisRestart<cr>", { desc = "Restart linthis LSP" })
```

## Supported Languages

| Language | Linter | Formatter |
|----------|--------|-----------|
| Rust | clippy | rustfmt |
| Python | ruff | ruff/black |
| TypeScript/JavaScript | eslint | prettier |
| Go | golangci-lint | gofmt |
| Java | checkstyle | google-java-format |
| C/C++ | clang-tidy | clang-format |
| Swift | swiftlint | swift-format |
| Kotlin | detekt | ktlint |
| Lua | luacheck | stylua |
| Dart | dart analyze | dart format |
| Shell | shellcheck | shfmt |
| Ruby | rubocop | rubocop |
| PHP | phpcs | php-cs-fixer |
| Scala | scalafix | scalafmt |
| C# | dotnet format | dotnet format |

## License

MIT
