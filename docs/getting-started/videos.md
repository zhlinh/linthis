# Video Tutorials

Short video demos showing linthis features in action. Each episode is 15-20 seconds.

## Episode 1: Quick Start

Get up and running with linthis in seconds — install, run your first check, and see results.

```bash
pip install linthis
linthis plugin add -g sample https://github.com/zhlinh/linthis-plugin-template
linthis hook install
linthis hook install --type agent
linthis -i src/
git add src/main.py
linthis -s
```

<video controls width="100%">
  <source src="/assets/videos/QuickStart-en.mp4" type="video/mp4">
  Your browser does not support the video tag.
</video>

---

## Episode 2: Multi-Language Support

See linthis auto-detect and lint 18+ programming languages in a single command.

```bash
linthis --lang python,rust,typescript
```

<video controls width="100%">
  <source src="/assets/videos/MultiLanguage-en.mp4" type="video/mp4">
  Your browser does not support the video tag.
</video>

---

## Episode 3: Plugin System

Share and reuse lint configurations across teams with the plugin system.

```bash
linthis plugin add -g sample https://github.com/zhlinh/linthis-plugin-template
linthis
```

<video controls width="100%">
  <source src="/assets/videos/PluginSystem-en.mp4" type="video/mp4">
  Your browser does not support the video tag.
</video>

---

## Episode 4: AI-Powered Fix

Let AI automatically fix lint issues — before and after, side by side.

```bash
linthis fix --ai --provider claude-cli
```

<video controls width="100%">
  <source src="/assets/videos/AiFix-en.mp4" type="video/mp4">
  Your browser does not support the video tag.
</video>

---

## Episode 5: Git Hooks

Configure once, lint on every commit — automatic pre-commit integration.

```bash
linthis hook install
linthis hook status
git commit -m "feat: add new feature"
```

<video controls width="100%">
  <source src="/assets/videos/GitHooks-en.mp4" type="video/mp4">
  Your browser does not support the video tag.
</video>

---

## Episode 6: AI Agent Hook

Integrate linthis into AI coding agents for automated code quality.

```bash
linthis hook install --type agent --provider claude
```

<video controls width="100%">
  <source src="/assets/videos/AgentHook-en.mp4" type="video/mp4">
  Your browser does not support the video tag.
</video>

---

## Episode 7: Editor Integration

Works with VS Code, JetBrains, Neovim, and Claude Code — lint as you type.

<video controls width="100%">
  <source src="/assets/videos/EditorSkills-en.mp4" type="video/mp4">
  Your browser does not support the video tag.
</video>

---

## Building Videos from Source

The video source code lives in `linthis-video/` and is built with [Remotion](https://remotion.dev).

```bash
cd linthis-video
npm install

# Preview in browser
npx remotion studio

# Render all videos
./render.sh

# Render specific episode/language
./render.sh --episode 1 --lang en
```
