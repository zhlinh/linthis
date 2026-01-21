# Neovim 插件发布指南

## 概述

与 VSCode 和 JetBrains 插件不同，Neovim 插件直接通过 **GitHub 仓库** 分发。用户使用插件管理器从 GitHub 克隆安装。

## 前提条件

### 1. GitHub 仓库

确保代码已推送到 GitHub：

```bash
git push origin master
```

### 2. 插件结构

确保正确的 Neovim 插件结构：

```
nvim-linthis/
├── lua/
│   └── linthis/
│       └── init.lua      # 主插件代码
├── plugin/
│   └── linthis.lua       # 启动时自动加载
├── doc/
│   └── linthis.txt       # 帮助文档
├── README.md             # 安装和使用指南
└── LICENSE               # 许可证文件
```

## 发布步骤

### 步骤 1：推送到 GitHub

```bash
git add -A
git commit -m "feat: initial release"
git push origin master
```

就这样！你的插件现在已经"发布"并可以安装了。

### 步骤 2：更新 README.md

确保 README.md 包含各种插件管理器的安装说明：

#### lazy.nvim（推荐）

```lua
-- 对于 monorepo（插件在子目录中）
{
  "zhlinh/linthis",
  subdir = "nvim-linthis",
  config = function()
    require("linthis").setup()
  end,
}

-- 对于独立仓库
{
  "zhlinh/nvim-linthis",
  config = function()
    require("linthis").setup()
  end,
}
```

#### packer.nvim

```lua
-- 对于 monorepo
use {
  "zhlinh/linthis",
  rtp = "nvim-linthis",
  config = function()
    require("linthis").setup()
  end,
}

-- 对于独立仓库
use {
  "zhlinh/nvim-linthis",
  config = function()
    require("linthis").setup()
  end,
}
```

#### vim-plug

```vim
" 对于 monorepo
Plug 'zhlinh/linthis', { 'rtp': 'nvim-linthis' }

" 对于独立仓库
Plug 'zhlinh/nvim-linthis'
```

### 步骤 3：生成帮助标签（可选）

如果你在 `doc/` 目录有文档：

```vim
:helptags doc/
```

或者用户安装后可以运行：

```vim
:helptags ALL
```

## 可选：提交到插件目录

### 1. awesome-neovim

最受欢迎的 Neovim 插件列表。

1. 访问 [awesome-neovim](https://github.com/rockerBOO/awesome-neovim)
2. Fork 仓库
3. 在 README.md 的适当分类中添加你的插件
4. 提交 Pull Request

示例条目：
```markdown
- [zhlinh/linthis](https://github.com/zhlinh/linthis) - 基于 LSP 的多语言代码检查和格式化工具。
```

### 2. neovimcraft.com

自动索引 GitHub 上的 Neovim 插件。

1. 确保你的仓库有 `neovim` 或 `neovim-plugin` 标签
2. 网站会自动发现并列出你的插件

### 3. dotfyle.com

另一个流行的 Neovim 插件目录。

1. 访问 [dotfyle.com](https://dotfyle.com)
2. 使用 GitHub 登录
3. 提交你的插件

## 更新插件

### 1. 修改代码

```bash
# 编辑文件
vim lua/linthis/init.lua
```

### 2. 更新版本（可选）

如果你在代码中维护版本号：

```lua
-- lua/linthis/init.lua
M.version = "0.0.2"
```

### 3. 更新更新日志

在 README.md 或 CHANGELOG.md 中添加更改。

### 4. 推送更改

```bash
git add -A
git commit -m "feat: add new feature"
git push origin master
```

用户运行 `:Lazy update` 或类似命令时会获取更新。

## 创建发布版本（可选）

GitHub releases 帮助用户跟踪版本：

### 通过命令行

```bash
# 创建标签
git tag -a v0.0.1 -m "Initial release"
git push origin v0.0.1

# 或使用 GitHub CLI
gh release create v0.0.1 --title "v0.0.1" --notes "首次发布"
```

### 通过 GitHub 网页

1. 进入你的仓库
2. 点击 **Releases** → **Create a new release**
3. 创建新标签（如 `v0.0.1`）
4. 添加发布标题和说明
5. 点击 **Publish release**

## 与其他平台对比

| 平台 | 发布方式 | 审核流程 |
|------|----------|----------|
| VSCode | 上传到 VS Marketplace | 需要 |
| JetBrains | 上传到 JetBrains Marketplace | 需要（首次） |
| **Neovim** | **推送到 GitHub** | **不需要** |

## 最佳实践

### 1. 清晰的文档

- 详细的 README.md
- 配置示例
- 截图/GIF（如适用）

### 2. 语义化版本

遵循 [语义化版本](https://semver.org/lang/zh-CN/)：
- **Patch 补丁版本** (0.0.x): Bug 修复
- **Minor 次版本** (0.x.0): 新功能，向后兼容
- **Major 主版本** (x.0.0): 破坏性变更

### 3. 响应 Issues

- 监控 GitHub Issues
- 及时修复 Bug
- 接受贡献

### 4. 发布前测试

```bash
# 运行测试（如有）
./test.sh

# 在干净的 Neovim 环境中测试
nvim --clean -u test_init.lua
```

## 快速参考

```bash
# 首次发布
git push origin master

# 创建发布版本
git tag -a v0.0.1 -m "Initial release"
git push origin v0.0.1

# 更新插件
git add -A && git commit -m "update" && git push

# 查看 GitHub releases
gh release list
```

## 参考链接

- [Neovim 插件开发](https://neovim.io/doc/user/develop.html)
- [awesome-neovim](https://github.com/rockerBOO/awesome-neovim)
- [lazy.nvim 文档](https://github.com/folke/lazy.nvim)
- [neovimcraft.com](https://neovimcraft.com)
