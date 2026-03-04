# 视频教程

展示 linthis 功能的短视频演示。每集 15-20 秒。

## 第 1 集：快速开始

几秒钟内上手 linthis — 安装、运行首次检查、查看结果。

```bash
cargo install linthis
cd my-project
linthis
```

<video controls width="100%">
  <source src="/assets/videos/QuickStart-zh.mp4" type="video/mp4">
  您的浏览器不支持视频标签。
</video>

---

## 第 2 集：多语言支持

一条命令自动检测和检查 18+ 种编程语言。

```bash
linthis --lang python,rust,typescript
```

<video controls width="100%">
  <source src="/assets/videos/MultiLanguage-zh.mp4" type="video/mp4">
  您的浏览器不支持视频标签。
</video>

---

## 第 3 集：插件系统

通过插件系统在团队间共享和复用检查配置。

```bash
linthis plugin add my-org-standards
linthis plugin init
linthis
```

<video controls width="100%">
  <source src="/assets/videos/TencentPlugin-zh.mp4" type="video/mp4">
  您的浏览器不支持视频标签。
</video>

---

## 第 4 集：AI 智能修复

让 AI 自动修复代码问题 — 修复前后对比。

```bash
linthis --fix --ai --provider claude
```

<video controls width="100%">
  <source src="/assets/videos/AiFix-zh.mp4" type="video/mp4">
  您的浏览器不支持视频标签。
</video>

---

## 第 5 集：Git Hooks

配置一次，每次提交自动检查 — pre-commit 自动集成。

```bash
linthis init -g
git commit -m "feat: 新功能"
# linthis 自动运行
```

<video controls width="100%">
  <source src="/assets/videos/GitHooks-zh.mp4" type="video/mp4">
  您的浏览器不支持视频标签。
</video>

---

## 第 6 集：编辑器集成

支持 VS Code、JetBrains、Neovim 和 Claude Code — 边写边检查。

<video controls width="100%">
  <source src="/assets/videos/EditorSkills-zh.mp4" type="video/mp4">
  您的浏览器不支持视频标签。
</video>

---

## 第 7 集：AI Agent Hook

将 linthis 集成到 AI 编程助手中，实现自动化代码质量检查。

```bash
linthis hook install --ai --provider claude --accept-all
```

<video controls width="100%">
  <source src="/assets/videos/AgentHook-zh.mp4" type="video/mp4">
  您的浏览器不支持视频标签。
</video>

---

## 从源码构建视频

视频源代码位于 `linthis-video/`，使用 [Remotion](https://remotion.dev) 构建。

```bash
cd linthis-video
npm install

# 浏览器预览
npx remotion studio

# 渲染所有视频
./render.sh

# 渲染指定集数/语言
./render.sh --episode 1 --lang zh
```
