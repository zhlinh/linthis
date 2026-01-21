# Neovim Plugin Publishing Guide

## Overview

Unlike VSCode and JetBrains plugins, Neovim plugins are distributed directly through **GitHub repositories**. Users install plugins using plugin managers that clone from GitHub.

## Prerequisites

### 1. GitHub Repository

Ensure your code is pushed to GitHub:

```bash
git push origin master
```

### 2. Plugin Structure

Ensure correct Neovim plugin structure:

```
nvim-linthis/
├── lua/
│   └── linthis/
│       └── init.lua      # Main plugin code
├── plugin/
│   └── linthis.lua       # Auto-loaded on startup
├── doc/
│   └── linthis.txt       # Help documentation
├── README.md             # Installation & usage guide
└── LICENSE               # License file
```

## Publishing Steps

### Step 1: Push to GitHub

```bash
git add -A
git commit -m "feat: initial release"
git push origin master
```

That's it! Your plugin is now "published" and installable.

### Step 2: Update README.md

Ensure README.md contains installation instructions for various plugin managers:

#### lazy.nvim (Recommended)

```lua
-- For monorepo (plugin in subdirectory)
{
  "zhlinh/linthis",
  subdir = "nvim-linthis",
  config = function()
    require("linthis").setup()
  end,
}

-- For standalone repository
{
  "zhlinh/nvim-linthis",
  config = function()
    require("linthis").setup()
  end,
}
```

#### packer.nvim

```lua
-- For monorepo
use {
  "zhlinh/linthis",
  rtp = "nvim-linthis",
  config = function()
    require("linthis").setup()
  end,
}

-- For standalone repository
use {
  "zhlinh/nvim-linthis",
  config = function()
    require("linthis").setup()
  end,
}
```

#### vim-plug

```vim
" For monorepo
Plug 'zhlinh/linthis', { 'rtp': 'nvim-linthis' }

" For standalone repository
Plug 'zhlinh/nvim-linthis'
```

### Step 3: Generate Help Tags (Optional)

If you have documentation in `doc/`:

```vim
:helptags doc/
```

Or users can run after installation:

```vim
:helptags ALL
```

## Optional: Submit to Plugin Directories

### 1. awesome-neovim

The most popular Neovim plugin list.

1. Visit [awesome-neovim](https://github.com/rockerBOO/awesome-neovim)
2. Fork the repository
3. Add your plugin to the appropriate category in README.md
4. Submit a Pull Request

Example entry:
```markdown
- [zhlinh/linthis](https://github.com/zhlinh/linthis) - Multi-language linter and formatter powered by LSP.
```

### 2. neovimcraft.com

Automatically indexes Neovim plugins from GitHub.

1. Ensure your repository has the `neovim` or `neovim-plugin` topic
2. The site will automatically discover and list your plugin

### 3. dotfyle.com

Another popular Neovim plugin directory.

1. Visit [dotfyle.com](https://dotfyle.com)
2. Sign in with GitHub
3. Submit your plugin

## Updating the Plugin

### 1. Make Changes

```bash
# Edit files
vim lua/linthis/init.lua
```

### 2. Update Version (Optional)

If you maintain a version number in your code:

```lua
-- lua/linthis/init.lua
M.version = "0.0.2"
```

### 3. Update Changelog

Add changes to README.md or CHANGELOG.md.

### 4. Push Changes

```bash
git add -A
git commit -m "feat: add new feature"
git push origin master
```

Users will get updates when they run `:Lazy update` or equivalent.

## Creating Releases (Optional)

GitHub releases help users track versions:

### Via Command Line

```bash
# Create a tag
git tag -a v0.0.1 -m "Initial release"
git push origin v0.0.1

# Or use GitHub CLI
gh release create v0.0.1 --title "v0.0.1" --notes "Initial release"
```

### Via GitHub Web

1. Go to your repository
2. Click **Releases** → **Create a new release**
3. Create a new tag (e.g., `v0.0.1`)
4. Add release title and notes
5. Click **Publish release**

## Comparison with Other Platforms

| Platform | Publishing Method | Review Process |
|----------|-------------------|----------------|
| VSCode | Upload to VS Marketplace | Yes |
| JetBrains | Upload to JetBrains Marketplace | Yes (first time) |
| **Neovim** | **Push to GitHub** | **No** |

## Best Practices

### 1. Clear Documentation

- Detailed README.md
- Configuration examples
- Screenshots/GIFs if applicable

### 2. Semantic Versioning

Follow [semver](https://semver.org/):
- **Patch** (0.0.x): Bug fixes
- **Minor** (0.x.0): New features, backward compatible
- **Major** (x.0.0): Breaking changes

### 3. Respond to Issues

- Monitor GitHub Issues
- Fix bugs promptly
- Accept contributions

### 4. Test Before Release

```bash
# Run tests if available
./test.sh

# Test in a clean Neovim environment
nvim --clean -u test_init.lua
```

## Quick Reference

```bash
# Initial publish
git push origin master

# Create release
git tag -a v0.0.1 -m "Initial release"
git push origin v0.0.1

# Update plugin
git add -A && git commit -m "update" && git push

# View GitHub releases
gh release list
```

## Reference Links

- [Neovim Plugin Development](https://neovim.io/doc/user/develop.html)
- [awesome-neovim](https://github.com/rockerBOO/awesome-neovim)
- [lazy.nvim Documentation](https://github.com/folke/lazy.nvim)
- [neovimcraft.com](https://neovimcraft.com)
