# Linthis Plugin Auto-Sync 功能说明

## 概述

linthis 现在支持自动同步插件功能，灵感来自 oh-my-zsh 的自动更新机制。该功能可以在运行 linthis 时自动检查并同步插件更新，确保您始终使用最新的插件配置。

## 特性

- ✅ **可配置的同步间隔**：自定义检查更新的频率（默认 7 天）
- ✅ **多种同步模式**：
  - `auto`：自动同步，无需确认
  - `prompt`：同步前询问用户（默认）
  - `disabled`：禁用自动同步
- ✅ **智能时间追踪**：使用 Unix 时间戳避免时区问题
- ✅ **优雅的用户交互**：清晰的进度提示和错误处理
- ✅ **智能更新检测**：只在有实际更新时提示用户

## 配置方式

### 1. 在配置文件中设置

在 `.linthis/config.toml` 或 `~/.linthis/config.toml` 中添加：

```toml
# Plugin settings
[plugin]
sources = [
    { name = "myplugin", url = "https://github.com/your-org/myplugin.git", ref = "main" }
]

# Plugin auto-sync settings
[plugin_auto_sync]
enabled = true           # 启用自动同步
mode = "prompt"          # 同步模式: "auto", "prompt", "disabled"
interval_days = 7        # 同步间隔（天）
```

### 2. 配置选项说明

#### `enabled`
- **类型**：布尔值
- **默认值**：`true`
- **说明**：是否启用自动同步功能

#### `mode`
- **类型**：字符串
- **默认值**：`"prompt"`
- **可选值**：
  - `"auto"`：自动同步，不需要用户确认
  - `"prompt"`：同步前询问用户是否继续
  - `"disabled"`：禁用自动同步
- **说明**：同步模式

#### `interval_days`
- **类型**：整数
- **默认值**：`7`
- **说明**：检查更新的间隔天数

## 工作原理

### 时间追踪

自动同步功能使用 `~/.linthis/.plugin_sync_last_check` 文件记录上次同步的时间戳（Unix epoch 秒数）。

### 触发时机

每次运行 linthis 主命令时，系统会：
1. 加载配置文件中的 `plugin_auto_sync` 设置
2. 检查 `~/.linthis/.plugin_sync_last_check` 文件
3. 计算距离上次同步的时间
4. 如果超过配置的间隔，触发同步流程

### 同步流程

根据配置的 `mode`：

- **auto 模式**：
  1. 检查所有插件是否有更新
  2. 如果有更新，自动开始同步
  3. 显示同步进度
  4. 更新时间戳

- **prompt 模式**：
  1. 检查所有插件是否有更新
  2. 如果有更新，提示用户：`Updates available for plugins. Update now? [Y/n]:`
  3. 等待用户输入
  4. 如果用户确认，执行同步
  5. 如果用户拒绝或没有更新，跳过并更新时间戳（避免重复提示）

- **disabled 模式**：
  - 跳过所有检查

## 使用示例

### 示例 1：默认配置（提示模式）

```toml
[plugin_auto_sync]
enabled = true
mode = "prompt"
interval_days = 7
```

当距离上次同步超过 7 天且有更新时：
```bash
$ linthis
Updates available for plugins. Update now? [Y/n]: y
↓ Syncing project plugins...
  ↓ myplugin... ✓ @ a1b2c3d
✓ Synced 1 plugin(s), 1 updated
```

当没有更新时，不会显示任何提示，只是静默更新时间戳。

### 示例 2：自动模式（无需确认）

```toml
[plugin_auto_sync]
enabled = true
mode = "auto"
interval_days = 3
```

每 3 天自动同步，无需用户确认：
```bash
$ linthis
↓ Syncing project plugins...
  ↓ myplugin... ✓ @ a1b2c3d
↓ Syncing global plugins...
  ↓ official... ✓ @ e4f5g6h (up to date)
✓ Synced 2 plugin(s), 1 updated
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

即使配置了自动同步，您仍然可以随时手动同步：

```bash
# 同步项目插件
linthis plugin sync

# 同步全局插件
linthis plugin sync -g
```

## 与 oh-my-zsh 的对比

| 特性 | oh-my-zsh | linthis |
|-----|-----------|---------|
| 默认同步间隔 | 13 天 | 7 天 |
| 同步模式 | auto, prompt, disabled | auto, prompt, disabled |
| 时间追踪 | `~/.zsh-update` | `~/.linthis/.plugin_sync_last_check` |
| 提示信息 | shell 提示 | CLI 交互 |
| 手动同步 | `omz update` | `linthis plugin sync` |
| 智能更新检测 | 否 | 是（只在有更新时提示） |

## 配置优先级

配置加载遵循以下优先级（从高到低）：
1. 项目配置（`.linthis/config.toml`）
2. 全局配置（`~/.linthis/config.toml`）
3. 内置默认值

## 故障排查

### 问题：自动同步不工作

**检查清单**：
1. 确认 `plugin_auto_sync.enabled = true`
2. 确认 `plugin_auto_sync.mode` 不是 `"disabled"`
3. 检查 `~/.linthis/.plugin_sync_last_check` 文件的权限
4. 查看是否有错误信息输出

### 问题：提示太频繁

**解决方案**：增加 `interval_days` 的值：
```toml
[plugin_auto_sync]
interval_days = 14  # 改为 14 天
```

### 问题：想要完全禁用

**解决方案**：
```toml
[plugin_auto_sync]
enabled = false
```

或者删除整个 `[plugin_auto_sync]` 配置段（将使用默认值）。

## 技术细节

### 实现位置
- **模块**：`linthis/src/plugin/auto_sync.rs`
- **配置**：`linthis/src/config/mod.rs`
- **集成**：`linthis/src/main.rs`

### 核心组件
- `AutoSyncConfig`：配置结构体
- `AutoSyncManager`：同步管理器，处理时间追踪和用户交互
- `perform_auto_sync()`：主执行函数
- `check_plugins_for_updates()`：检查插件更新
- `sync_plugins()`：插件同步辅助函数

### 测试覆盖
- ✅ 配置验证
- ✅ 时间戳读写
- ✅ 同步间隔计算
- ✅ 模式检查
- ✅ 默认值测试

运行测试：
```bash
cargo test plugin::auto_sync
```

## 与自动更新的关系

linthis 同时支持：
1. **Plugin Auto-Sync**（本功能）：自动同步插件
2. **Self Auto-Update**：更新 linthis 自身

两者独立配置，独立运行：
```toml
# 同步插件
[plugin_auto_sync]
enabled = true
mode = "prompt"
interval_days = 7

# 更新 linthis 自身
[self_auto_update]
enabled = true
mode = "prompt"
interval_days = 7
```

执行顺序：
1. 先检查 linthis 自身更新
2. 再检查插件同步


## 参考

- [oh-my-zsh 自动更新机制](https://maxchadwick.xyz/blog/a-look-at-auto-updating-in-oh-my-zsh)
- [oh-my-zsh 设置文档](https://github.com/ohmyzsh/ohmyzsh/wiki/Settings)
