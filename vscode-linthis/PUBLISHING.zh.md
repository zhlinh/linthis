# VS Code 插件发布指南

## 发布前准备

### 1. 安装 vsce (VS Code Extension Manager)

```bash
npm install -g @vscode/vsce
```

### 2. 创建 Azure DevOps 账号和个人访问令牌 (PAT)

1. 访问 [Azure DevOps](https://dev.azure.com)
2. 注册/登录账号
3. 点击右上角用户图标 → **Personal access tokens**（个人访问令牌）
4. 点击 **+ New Token**（新建令牌）
5. 配置令牌：
   - Name（名称）: `vscode-marketplace`
   - Organization（组织）: **All accessible organizations**（所有可访问组织）
   - Expiration（过期时间）: 自定义（建议 90 天或更长）
   - Scopes（权限范围）: 选择 **Custom defined**（自定义）
     - **Marketplace**: 勾选 **Manage**（管理）
6. 点击 **Create**（创建），**立即复制令牌**（只显示一次！）

### 3. 创建发布者 (Publisher)

```bash
# 使用你的 PAT 登录
vsce login <发布者名称>

# 例如：
vsce login linthis
```

输入刚才复制的 PAT。

**或者**通过 [Visual Studio Marketplace Publisher Management](https://marketplace.visualstudio.com/manage) 页面创建发布者。

### 4. 更新 package.json 中的 publisher 字段

确保 `package.json` 中的 `publisher` 字段与你创建的发布者名称一致：

```json
{
  "publisher": "linthis"
}
```

## 发布前检查清单

### 1. 检查必需字段

确保 `package.json` 包含以下字段：

- ✅ `name` - 插件名称（小写，无空格）
- ✅ `displayName` - 显示名称
- ✅ `description` - 描述
- ✅ `version` - 版本号（遵循 semver）
- ✅ `publisher` - 发布者名称
- ✅ `engines.vscode` - VS Code 最低版本要求
- ✅ `categories` - 分类
- ✅ `keywords` - 关键词
- ✅ `repository` - 仓库 URL
- ✅ `license` - 许可证

### 2. 添加 README.md

创建 `README.md` 文件，包含：
- 插件介绍
- 功能特性
- 安装方法
- 使用说明
- 配置选项
- 截图/GIF 演示

### 3. 添加 CHANGELOG.md

创建 `CHANGELOG.md` 记录版本变更：

```markdown
# 更新日志

## [0.0.1] - 2025-01-19

### 新增
- 首次发布
- LSP 集成，实时代码检查
- 保存时格式化
- 保存时检查
- 支持 14+ 种语言
```

### 4. 添加 LICENSE

确保有 LICENSE 文件（当前是 MIT）。

### 5. 添加图标（可选但推荐）

添加 128x128 的 PNG 图标：

```json
{
  "icon": "icon.png"
}
```

### 6. 构建和测试

```bash
# 安装依赖
npm install

# 构建
npm run build

# 测试
npm test

# 本地测试插件
code --install-extension ./linthis-0.0.1.vsix
```

## 发布步骤

### 方法一：使用 vsce 命令行发布

#### 1. 打包插件

```bash
# 在 vscode-linthis 目录下
vsce package

# 生成 linthis-0.0.1.vsix 文件
```

#### 2. 发布到 Marketplace

```bash
# 首次发布
vsce publish

# 或者指定版本号自动更新
vsce publish patch  # 0.0.1 -> 0.0.2（补丁版本）
vsce publish minor  # 0.0.1 -> 0.1.0（次版本）
vsce publish major  # 0.0.1 -> 1.0.0（主版本）

# 或者发布已打包的 .vsix 文件
vsce publish -p <vsix文件路径>
```

#### 3. 验证发布

发布后等待几分钟，然后访问：
- Marketplace: `https://marketplace.visualstudio.com/items?itemName=<发布者>.<插件名>`
- 例如: `https://marketplace.visualstudio.com/items?itemName=linthis.linthis`

### 方法二：通过 Web 界面上传

1. 访问 [Marketplace Publisher Management](https://marketplace.visualstudio.com/manage)
2. 登录 Azure DevOps 账号
3. 选择你的发布者
4. 点击 **+ New extension** → **Visual Studio Code**
5. 上传 `.vsix` 文件
6. 填写额外信息并发布

## 更新已发布的插件

### 1. 更新版本号

编辑 `package.json`：

```json
{
  "version": "0.0.2"
}
```

### 2. 更新 CHANGELOG.md

记录本次更新的内容。

### 3. 重新构建和发布

```bash
npm run build
vsce publish
```

## 常见问题

### 1. 发布失败：缺少 README.md

**解决方法**：确保根目录有 `README.md` 文件。

### 2. 发布失败：缺少 LICENSE

**解决方法**：添加 `LICENSE` 文件，或在 `package.json` 中添加：

```json
{
  "license": "SEE LICENSE IN LICENSE.txt"
}
```

### 3. 发布失败：Personal Access Token 无效

**解决方法**：
```bash
vsce logout
vsce login <发布者名称>
# 输入新的 PAT
```

### 4. 包太大

**解决方法**：添加 `.vscodeignore` 文件排除不必要的文件：

```
.vscode/**
.github/**
.gitignore
.eslintrc.json
tsconfig.json
webpack.config.js
src/**
out/**
node_modules/**
*.vsix
```

### 5. 需要更改发布者

**解决方法**：
1. 在 Azure DevOps 创建新发布者
2. 更新 `package.json` 的 `publisher` 字段
3. 重新发布

## 发布后管理

### 查看统计

访问 [Marketplace Publisher Management](https://marketplace.visualstudio.com/manage) 查看：
- 下载量
- 评分
- 评论
- 安装趋势

### 回应用户反馈

- 监控 GitHub Issues
- 回复 Marketplace 评论
- 及时修复 Bug

### 版本管理

遵循 [语义化版本](https://semver.org/lang/zh-CN/)：
- **Patch 补丁版本** (0.0.x): Bug 修复
- **Minor 次版本** (0.x.0): 新功能，向后兼容
- **Major 主版本** (x.0.0): 破坏性变更

## 完整发布脚本

创建 `scripts/publish.sh`：

```bash
#!/bin/bash

# 检查是否有未提交的更改
if [[ -n $(git status -s) ]]; then
  echo "错误：工作目录不干净，有未提交的更改"
  exit 1
fi

# 构建
echo "正在构建..."
npm run build

# 测试
echo "正在测试..."
npm test

# 打包
echo "正在打包..."
vsce package

# 发布
echo "正在发布..."
vsce publish

echo "发布完成！"
```

使用：
```bash
chmod +x scripts/publish.sh
./scripts/publish.sh
```

## 快速发布命令

```bash
# 一行命令：构建、打包、发布
npm run build && vsce publish patch

# 或者先打包测试，确认无误后再发布
npm run build && vsce package
# 测试 .vsix 文件
code --install-extension ./linthis-0.0.1.vsix
# 确认无误后发布
vsce publish
```

## 参考链接

- [VS Code 发布插件文档](https://code.visualstudio.com/api/working-with-extensions/publishing-extension)
- [vsce 文档](https://github.com/microsoft/vscode-vsce)
- [插件清单参考](https://code.visualstudio.com/api/references/extension-manifest)
- [Marketplace 发布管理](https://marketplace.visualstudio.com/manage)
