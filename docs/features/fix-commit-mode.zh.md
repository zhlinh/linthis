# Fix Commit Mode（修复提交模式）

控制 linthis 在 git hook 中如何处理自动格式化和 agent 修复的代码变更。

## 配置

```toml
[hook.pre_commit]
fix_commit_mode = "squash"    # squash | dirty | fixup

[hook.pre_push]
fix_commit_mode = "dirty"     # squash | dirty | fixup
```

CLI 设置：`linthis hook install --fix-commit-mode <mode>`

查询当前值：`linthis config get hook.pre_commit.fix_commit_mode`

## 三种模式

| 模式 | 说明 |
|------|------|
| **squash** | 修复 → 创建 fixup commit → 压入原始 commit。保留 stash 快照供审查。 |
| **dirty** | 修复 → 修改留在工作区 → 阻止提交/推送。用户审查后手动 stage。 |
| **fixup** | 原始 commit 直接通过。post-commit hook 创建单独的 fixup commit。 |

---

## 行为矩阵

### git 类型

| 事件 | 模式 | 新增 commit | Commit message | 工作区 | 恢复方式 |
|------|------|------------|----------------|--------|---------|
| pre-commit | squash | 0 | 原始 message 不变 | 干净（已 re-stage） | `git stash pop` 或 `linthis undo` |
| pre-commit | dirty | 0，阻止 | — | dirty（格式化未 stage） | `linthis undo` |
| pre-commit | fixup | 1（post-commit） | `style(linthis): auto-format` | 干净 | `git reset HEAD~1` |
| pre-push | * | 0 | — | 不变（仅检查） | — |

### git-with-agent 类型

| 事件 | 模式 | 变更类型 | 新增 commit | Commit message | 工作区 | 恢复方式 |
|------|------|---------|------------|----------------|--------|---------|
| pre-commit | squash | 格式化 | 0（fixup→squash） | 原始 message 不变 | 干净 | `git stash pop` 或 `git reset --hard HEAD@{1}` |
| pre-commit | squash | agent lint 修复 | 0（fixup→squash） | 原始 message 不变 | 干净 | 同上 |
| pre-commit | dirty | 格式化 | 0，阻止 | — | dirty | `linthis undo` |
| pre-commit | dirty | agent lint 修复 | 0，阻止 | — | dirty | `linthis undo` |
| pre-commit | fixup | 格式化 + lint 修复 | 1（post-commit） | `style(linthis): auto-format` | 干净 | `git reset HEAD~1` |
| pre-push | squash | lint 修复 | 0（fixup→squash） | 原始 message 不变 | 干净，继续 push | `git reset --hard HEAD@{1}` |
| pre-push | squash | review 修复 | 0（fixup→squash） | 原始 message 不变 | 干净，继续 push | `git reset --hard HEAD@{1}` |
| pre-push | dirty | lint 修复 | 0，阻止 | — | 不变 | — |
| pre-push | dirty | review 修复 | 0，阻止 | — | dirty | `linthis undo` |
| pre-push | fixup | lint 修复 | 1，阻止 | `style(linthis): auto-format` | 干净 | `git reset HEAD~1` |
| pre-push | fixup | review 修复 | 1，阻止 | `fix(linthis): auto-fix review issues` | 干净 | `git reset HEAD~1` |

### agent 类型（skill 控制）

| 事件 | 模式 | 新增 commit | Commit message | 工作区 | 恢复方式 |
|------|------|------------|----------------|--------|---------|
| pre-commit | squash | 0 | 原始 message（agent `git add` + approve） | 干净 | `linthis undo` |
| pre-commit | dirty | 0，AskUser 询问 | — | dirty（agent 修改未 stage） | `linthis undo` |
| pre-commit | fixup | 0 | —（agent 仅检查，post-commit 处理） | 不变 | — |
| pre-push | squash | 0 | 原始 message（agent amend） | 干净 | `git reflog` + `git reset --hard` |
| pre-push | dirty | 0，阻止 | — | dirty | `linthis undo` |
| pre-push | fixup | 1，阻止 | `fix(linthis): auto-fix review issues` | 干净 | `git reset HEAD~1` |

---

## Dirty 模式提示信息

当 dirty 模式阻止提交/推送时显示：

```
[linthis] Files formatted but not staged (dirty mode).
  Review:  git diff
  Accept:  git add -u && git commit
  Revert:  linthis undo
```

## 恢复方式优先级

| 方法 | 使用场景 | 安全性 |
|------|---------|--------|
| `linthis undo` | 任何模式 — 仅恢复 linthis 修改过的文件 | 最安全 |
| `linthis backup diff` | 先查看 linthis 改了什么再决定 | 只读 |
| `git stash pop` | squash 模式 — 恢复格式化前的 stash 快照 | 安全 |
| `git reset HEAD~1` | fixup 模式 — 移除 fixup commit | 安全 |
| `git reset --hard HEAD@{1}` | squash 模式 — 从 reflog 恢复 | 会丢弃工作区修改 |

## Hook 输出显示

当前模式显示在 hook 输出底部：

```
Global: ~/.config/git/hooks/pre-commit (--type git-with-agent, --fix-commit-mode squash)
Local:  .git/hooks/pre-commit (--type git-with-agent, --fix-commit-mode squash)
```
