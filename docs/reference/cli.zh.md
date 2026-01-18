# CLI 参考

linthis 所有命令和选项的完整参考。

## 主命令

```bash
linthis [OPTIONS] [COMMAND]
```

### 全局选项

| 短选项 | 长选项 | 描述 | 示例 |
|-------|-------|------|------|
| `-i` | `--include` | 要检查的文件/目录 | `-i src -i lib` |
| `-e` | `--exclude` | 要排除的模式 | `-e "*.test.js"` |
| `-c` | `--check-only` | 仅检查，不格式化 | `-c` |
| `-f` | `--format-only` | 仅格式化，不检查 | `-f` |
| `-s` | `--staged` | 仅检查 Git 暂存文件 | `-s` |
| `-l` | `--lang` | 语言（逗号分隔） | `-l python,rust` |
| `-o` | `--output` | 输出格式 | `-o json` |
| `-v` | `--verbose` | 详细输出 | `-v` |
| `-q` | `--quiet` | 安静模式（仅错误） | `-q` |
| | `--config` | 配置文件路径 | `--config custom.toml` |
| | `--preset` | 格式化预设 | `--preset google` |
| | `--no-default-excludes` | 禁用默认排除项 | |
| | `--no-gitignore` | 禁用 .gitignore 规则 | |
| | `--no-plugin` | 跳过加载插件 | |

### 输出格式

- `human` - 人类可读（默认）
- `json` - JSON 格式
- `github-actions` - GitHub Actions 注释

---

## init

初始化配置文件。

```bash
linthis init [OPTIONS]
```

| 选项 | 描述 |
|-----|------|
| `-g, --global` | 创建全局配置 |
| `--with-hook` | 同时安装 git hook |
| `--force` | 强制覆盖现有文件 |

**示例：**

```bash
linthis init                    # 创建 .linthis.toml
linthis init -g                 # 创建 ~/.linthis/config.toml
linthis init --with-hook        # 初始化配置并安装 hook
```

---

## hook

管理 Git hooks。

### hook install

```bash
linthis hook install [OPTIONS]
```

| 选项 | 描述 |
|-----|------|
| `--type` | Hook 类型：`prek`、`pre-commit`、`git` |
| `--event` | Hook 事件：`pre-commit`、`pre-push`、`commit-msg` |
| `-c, --check-only` | Hook 仅运行检查 |
| `-f, --format-only` | Hook 仅运行格式化 |
| `--force` | 强制覆盖现有 hook |
| `-y, --yes` | 非交互模式 |

**示例：**

```bash
linthis hook install --type git
linthis hook install --type git --event pre-push
linthis hook install --type prek --check-only
```

### hook uninstall

```bash
linthis hook uninstall [OPTIONS]
```

| 选项 | 描述 |
|-----|------|
| `--event` | 要卸载的 hook 事件 |
| `-y, --yes` | 非交互模式 |

### hook status

```bash
linthis hook status
```

### hook check

```bash
linthis hook check
```

---

## plugin

管理插件。

### plugin add

```bash
linthis plugin add <ALIAS> <URL> [OPTIONS]
```

| 选项 | 描述 |
|-----|------|
| `-g, --global` | 添加到全局配置 |
| `--ref` | Git 引用（分支/标签/提交） |

**示例：**

```bash
linthis plugin add myconfig https://github.com/user/config.git
linthis plugin add -g company https://github.com/company/standards.git
linthis plugin add myconfig https://github.com/user/config.git --ref v1.0.0
```

### plugin remove

```bash
linthis plugin remove <ALIAS> [OPTIONS]
```

| 选项 | 描述 |
|-----|------|
| `-g, --global` | 从全局配置移除 |

### plugin list

```bash
linthis plugin list [OPTIONS]
```

| 选项 | 描述 |
|-----|------|
| `-g, --global` | 列出全局插件 |
| `-v, --verbose` | 显示详细信息 |

### plugin sync

```bash
linthis plugin sync [OPTIONS]
```

| 选项 | 描述 |
|-----|------|
| `--global` | 同步全局插件 |

### plugin init

```bash
linthis plugin init <NAME>
```

### plugin validate

```bash
linthis plugin validate <PATH>
```

### plugin clean

```bash
linthis plugin clean [OPTIONS]
```

| 选项 | 描述 |
|-----|------|
| `--all` | 清理所有缓存 |

---

## config

管理配置。

### config add

```bash
linthis config add <FIELD> <VALUE> [OPTIONS]
```

**支持的字段：** `includes`、`excludes`、`languages`

| 选项 | 描述 |
|-----|------|
| `-g, --global` | 添加到全局配置 |

### config remove

```bash
linthis config remove <FIELD> <VALUE> [OPTIONS]
```

### config clear

```bash
linthis config clear <FIELD> [OPTIONS]
```

### config set

```bash
linthis config set <FIELD> <VALUE> [OPTIONS]
```

**支持的字段：** `max_complexity`、`preset`、`verbose`

### config unset

```bash
linthis config unset <FIELD> [OPTIONS]
```

### config get

```bash
linthis config get <FIELD> [OPTIONS]
```

### config list

```bash
linthis config list [OPTIONS]
```

| 选项 | 描述 |
|-----|------|
| `-g, --global` | 列出全局配置 |
| `-v, --verbose` | 显示所有字段 |

### config migrate

```bash
linthis config migrate [OPTIONS]
```

| 选项 | 描述 |
|-----|------|
| `--from` | 迁移特定工具 |
| `--dry-run` | 预览更改 |
| `--backup` | 创建备份 |
| `-v, --verbose` | 详细输出 |

---

## watch

监视模式，持续检查。

```bash
linthis watch [OPTIONS]
```

详见[监视模式](../features/watch-mode.md)。

---

## doctor

检查工具可用性。

```bash
linthis doctor [OPTIONS]
```

| 选项 | 描述 |
|-----|------|
| `-l, --lang` | 检查特定语言 |

---

## 退出码

| 代码 | 含义 |
|-----|------|
| 0 | 成功（无问题或所有问题已修复） |
| 1 | 发现 lint/format 问题 |
| 2 | 配置错误 |
| 3 | 工具不可用 |
