# 配置

linthis 可以通过配置文件、CLI 参数或两者结合进行配置。

## 配置文件

### 项目配置

使用 `linthis init` 创建配置文件：

```bash
linthis init
```

这会在项目根目录创建 `.linthis/config.toml`：

```toml
# 指定要检查的语言（省略则自动检测）
languages = ["rust", "python", "javascript"]

# 排除文件和目录
excludes = [
    "target/**",
    "node_modules/**",
    "*.generated.rs",
    "dist/**"
]

# 最大圈复杂度
max_complexity = 20

# 格式化预设
preset = "google"  # 选项：google、airbnb、standard

# 配置插件
[plugins]
sources = [
    { name = "official" },
    { name = "myplugin", url = "https://github.com/user/plugin.git", ref = "main" }
]

# 语言特定配置
# [rust]
# max_complexity = 15

# [python]
# excludes = ["*_test.py"]
```

### 全局配置

全局配置位于 `~/.linthis/config.toml`，格式与项目配置相同。

### 配置优先级

配置合并优先级（从高到低）：

1. **CLI 参数**：`--option value`
2. **项目配置**：`.linthis/config.toml`
3. **全局配置**：`~/.linthis/config.toml`
4. **内置默认值**

对于工具特定的配置文件（ruff.toml、.eslintrc.js 等），优先级如下：

1. **本地手动配置**（最高）- 项目中的 ruff.toml、pyproject.toml、.eslintrc.js
2. **CLI 插件配置** - 来自 `--use-plugin` 选项
3. **项目插件配置** - 来自 `.linthis/config.toml` 的 plugins 部分
4. **全局插件配置** - 来自 `~/.linthis/config.toml` 的 plugins
5. **工具默认值**（最低）

## 配置管理命令

### 数组字段操作

支持的数组字段：`includes`、`excludes`、`languages`

```bash
# 添加值
linthis config add includes "src/**"
linthis config add excludes "*.log"
linthis config add languages "rust"

# 添加到全局配置
linthis config add -g includes "lib/**"

# 移除值
linthis config remove excludes "*.log"

# 清空字段
linthis config clear languages
```

### 标量字段操作

支持的标量字段：`max_complexity`、`preset`、`verbose`

```bash
# 设置值
linthis config set max_complexity 15
linthis config set preset google

# 设置全局配置
linthis config set -g max_complexity 20

# 取消设置
linthis config unset max_complexity
```

### 查询操作

```bash
# 获取单个字段
linthis config get includes
linthis config get max_complexity

# 列出所有配置
linthis config list
linthis config list -g  # 全局配置
linthis config list -v  # 详细模式（显示空字段）
```

## 配置迁移

linthis 可以迁移现有的 linter/formatter 配置：

```bash
# 自动检测并迁移所有配置
linthis config migrate

# 迁移特定工具
linthis config migrate --from eslint
linthis config migrate --from prettier
linthis config migrate --from black

# 预览更改
linthis config migrate --dry-run

# 创建备份
linthis config migrate --backup
```

### 支持的工具

| 工具 | 检测文件 |
|-----|---------|
| ESLint | `.eslintrc.js`、`.eslintrc.json`、`.eslintrc.yml`、`eslint.config.js` |
| Prettier | `.prettierrc`、`.prettierrc.json`、`.prettierrc.yml`、`prettier.config.js` |
| Black | `pyproject.toml[tool.black]` |
| isort | `pyproject.toml[tool.isort]` |

## 环境变量

### `LINTHIS_SKIP` — 跳过指定 hook

`git commit --no-verify` 会一次跳过所有 hook。`LINTHIS_SKIP` 可以只跳过某个，其余照常运行。值以逗号分隔，大小写不敏感。

| 值 | 跳过的 hook |
|----|------------|
| `check` 或 `pc` | pre-commit + post-commit（两者配对，一起跳） |
| `pre-commit` | 只跳 pre-commit |
| `post-commit` | 只跳 post-commit |
| `cmsg`、`cm` 或 `commit-msg` | commit-msg |
| `pp` 或 `pre-push` | pre-push |
| `all` | 全部 hook |

```bash
# 只跳过 commit-msg 正则校验，lint 仍然跑
LINTHIS_SKIP=cm git commit -m "WIP: 临时消息"

# 跳过 pre-commit 的 lint/security/complexity 以及 post-commit，但保留 commit-msg
LINTHIS_SKIP=check git commit -m "feat: 快速保存"

# 同时跳过多个
LINTHIS_SKIP=cm,pp git push
```

未知 token 会报错且不跳过 —— 避免"以为跳了但其实没跳"。

### `LINTHIS_SKIP_CHECKS` — 跳过具体的 check

用于在 hook（pre-commit / pre-push）内部跳过具体的 check。支持全名，也支持**不少于 3 个字符**的前缀（大小写不敏感）。

| 值 | 效果 |
|----|-----|
| `lin` 或 `lint` | 跳过 lint 检查 |
| `sec` 或 `security` | 跳过 security (SAST) 检查 |
| `com` 或 `complexity` | 跳过 complexity 检查 |

```bash
# 跳过较慢的 complexity 检查
LINTHIS_SKIP_CHECKS=com git commit -m "fix: bug"

# 只跑 security（跳过 lint 和 complexity）
LINTHIS_SKIP_CHECKS=lin,com git commit -m "fix: bug"
```

少于 3 字符或无法匹配的 token 会打印警告并被忽略，其余依旧生效。

两个变量互相正交，可以同时用：

```bash
LINTHIS_SKIP=cm LINTHIS_SKIP_CHECKS=com git commit -m "WIP"
```

### `LINTHIS_AGENT_MAX_AUTO_FIX` — 限制大规模 auto-fix

`git-with-agent` 类型的 hook 在 lint 失败时会调 AI agent（Claude / Codex / Gemini ...）自动修复。但当错误有成百上千个时，调用会阻塞几分钟并且看不到进度。

`LINTHIS_AGENT_MAX_AUTO_FIX` 给 `errors + warnings` 设置上限 —— 超过阈值则跳过 auto-fix，commit 快速失败并提示用交互式修复（agent 输出会实时流式打印）：

```bash
# 默认：errors+warnings > 100 时跳过 auto-fix
git commit -m "..."

# 临时提高阈值
LINTHIS_AGENT_MAX_AUTO_FIX=500 git commit -m "..."

# 完全关闭限制（不推荐 —— commit 可能长时间卡住）
LINTHIS_AGENT_MAX_AUTO_FIX=0 git commit -m "..."
```

当 hook 判定调 agent 时，agent 的输出会直接流式打到终端 —— 不再是只有 spinner 的"假死"。随时 Ctrl-C 取消。

对 Claude Code 和 CodeBuddy，`linthis` 会用 `--output-format stream-json` 启动它们，然后把事件流通过管道喂给 `linthis agent-stream`，逐行把每个 assistant 消息、工具调用（`Edit /path/to/file`、`Bash cargo check` 等）渲染成人类可读的输出。其他 provider（`codex exec`、`cursor-agent chat`、`droid exec`）本身就是流式的。

## 下一步

- [插件系统](../features/plugins.md) - 共享配置
- [CLI 参考](../reference/cli.md) - 所有命令选项
