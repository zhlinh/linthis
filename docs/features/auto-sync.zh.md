# 插件自动同步

## 概述

linthis 支持插件自动同步功能，灵感来源于 oh-my-zsh 的自动更新机制。该功能会在运行 linthis 时自动检查并同步插件更新，确保您始终使用最新的插件配置。

## 功能特性

- **可配置的同步间隔**：自定义更新检查频率（默认：7 天）
- **多种同步模式**：
  - `auto`：无需确认自动同步
  - `prompt`：同步前询问用户（默认）
  - `disabled`：禁用自动同步
- **智能时间追踪**：使用 Unix 时间戳避免时区问题
- **智能更新检测**：仅在有实际更新时才提示
- **友好的用户交互**：清晰的进度指示器和错误处理

## 配置

### 配置文件

在 `.linthis/config.toml` 或 `~/.linthis/config.toml` 中添加：

```toml
# 插件设置
[plugin]
sources = [
    { name = "myplugin", url = "https://github.com/your-org/myplugin.git", ref = "main" }
]

# 插件自动同步设置
[plugin_auto_sync]
enabled = true           # 启用自动同步
mode = "prompt"          # 同步模式："auto"、"prompt"、"disabled"
interval_days = 7        # 同步间隔（天）
```

### 配置选项

#### `enabled`
- **类型**：布尔值
- **默认值**：`true`
- **描述**：是否启用自动同步

#### `mode`
- **类型**：字符串
- **默认值**：`"prompt"`
- **选项**：
  - `"auto"`：无需用户确认自动同步
  - `"prompt"`：同步前询问用户
  - `"disabled"`：禁用自动同步

#### `interval_days`
- **类型**：整数
- **默认值**：`7`
- **描述**：同步检查间隔天数

## 工作原理

### 时间追踪

自动同步使用 `~/.linthis/.plugin_sync_last_check` 文件存储上次同步的时间戳（Unix 纪元秒）。

### 触发条件

每次运行 linthis 时，系统会：
1. 从配置加载 `plugin_auto_sync` 设置
2. 检查 `~/.linthis/.plugin_sync_last_check` 文件
3. 计算距上次同步的时间
4. 如果超过间隔时间，触发同步流程

### 同步流程

根据配置的 `mode`：

- **auto 模式**：
  1. 检查所有插件是否有更新
  2. 如果有更新，自动开始同步
  3. 显示同步进度
  4. 更新时间戳

- **prompt 模式**：
  1. 检查所有插件是否有更新
  2. 如果有更新，提示：`Updates available for plugins. Update now? [Y/n]:`
  3. 等待用户输入
  4. 如果确认，执行同步
  5. 如果拒绝或无更新，跳过并更新时间戳

- **disabled 模式**：
  - 跳过所有检查

## 示例

### 示例 1：默认（Prompt 模式）

```toml
[plugin_auto_sync]
enabled = true
mode = "prompt"
interval_days = 7
```

当超过间隔时间且有更新可用时：
```bash
$ linthis
Updates available for plugins. Update now? [Y/n]: y
↓ Syncing project plugins...
  ↓ myplugin... ✓ @ a1b2c3d
✓ Synced 1 plugin(s), 1 updated
```

### 示例 2：Auto 模式

```toml
[plugin_auto_sync]
enabled = true
mode = "auto"
interval_days = 3
```

每 3 天无需确认自动同步：
```bash
$ linthis
↓ Syncing project plugins...
  ↓ myplugin... ✓ @ a1b2c3d
✓ Synced 1 plugin(s), 1 updated
```

### 示例 3：禁用自动同步

```toml
[plugin_auto_sync]
enabled = false
```

或者：

```toml
[plugin_auto_sync]
mode = "disabled"
```

### 示例 4：手动同步

无论自动同步设置如何，您都可以手动同步：

```bash
# 同步项目插件
linthis plugin sync

# 同步全局插件
linthis plugin sync -g
```

## 与 oh-my-zsh 对比

| 功能 | oh-my-zsh | linthis |
|-----|-----------|---------|
| 默认间隔 | 13 天 | 7 天 |
| 同步模式 | auto、prompt、disabled | auto、prompt、disabled |
| 时间追踪 | `~/.zsh-update` | `~/.linthis/.plugin_sync_last_check` |
| 手动同步 | `omz update` | `linthis plugin sync` |
| 智能检测 | 无 | 是（仅在有更新时提示） |

## 配置优先级

配置按以下优先级加载（从高到低）：
1. 项目配置 (`.linthis/config.toml`)
2. 全局配置 (`~/.linthis/config.toml`)
3. 内置默认值

## 故障排除

### 自动同步不工作

**检查清单**：
1. 确认 `plugin_auto_sync.enabled = true`
2. 确认 `plugin_auto_sync.mode` 不是 `"disabled"`
3. 检查 `~/.linthis/.plugin_sync_last_check` 的权限
4. 检查输出中的错误信息

### 提示太频繁

**解决方案**：增加 `interval_days`：
```toml
[plugin_auto_sync]
interval_days = 14  # 改为 14 天
```

### 想要完全禁用

**解决方案**：
```toml
[plugin_auto_sync]
enabled = false
```

## 与自更新的关系

linthis 支持两种自动更新：
1. **插件自动同步**（本功能）：同步插件
2. **自更新**：更新 linthis 本身

两者独立配置和运行：
```toml
# 同步插件
[plugin_auto_sync]
enabled = true
mode = "prompt"
interval_days = 7

# 更新 linthis 本身
[self_auto_update]
enabled = true
mode = "prompt"
interval_days = 7
```

执行顺序：
1. 首先检查 linthis 自更新
2. 然后检查插件同步

## 参考资料

- [oh-my-zsh 自动更新机制](https://maxchadwick.xyz/blog/a-look-at-auto-updating-in-oh-my-zsh)
- [oh-my-zsh 设置文档](https://github.com/ohmyzsh/ohmyzsh/wiki/Settings)
