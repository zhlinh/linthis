# Linthis Self-Update 功能说明

## 概述

linthis 现在支持自动更新功能，灵感来自 oh-my-zsh 的自动更新机制。该功能可以在运行 linthis 时自动检查并更新 linthis 自身，确保您始终使用最新版本。

## 特性

- ✅ **可配置的检查间隔**：自定义检查更新的频率（默认 7 天）
- ✅ **多种更新模式**：
  - `auto`：自动更新，无需确认
  - `prompt`：更新前询问用户（默认）
  - `disabled`：禁用自动更新
- ✅ **智能更新检测**：只在有实际更新时提示用户，无更新时静默更新时间戳
- ✅ **智能时间追踪**：使用 Unix 时间戳避免时区问题
- ✅ **PyPI 版本检测**：通过 pip 检查最新版本
- ✅ **优雅的用户交互**：清晰的进度提示和错误处理

## 配置方式

### 1. 在配置文件中设置

在 `.linthis/config.toml` 或 `~/.linthis/config.toml` 中添加：

```toml
# Self-update settings
[self_auto_update]
enabled = true           # 启用自动更新检查
mode = "prompt"          # 更新模式: "auto", "prompt", "disabled"
interval_days = 7        # 检查间隔（天）
```

### 2. 配置选项说明

#### `enabled`
- **类型**：布尔值
- **默认值**：`true`
- **说明**：是否启用自动更新检查功能

#### `mode`
- **类型**：字符串
- **默认值**：`"prompt"`
- **可选值**：
  - `"auto"`：自动更新，不需要用户确认
  - `"prompt"`：更新前询问用户是否继续
  - `"disabled"`：禁用自动更新
- **说明**：更新模式

#### `interval_days`
- **类型**：整数
- **默认值**：`7`
- **说明**：检查更新的间隔天数

## 工作原理

### 时间追踪

自动更新功能使用 `~/.linthis/.self_update_last_check` 文件记录上次检查的时间戳（Unix epoch 秒数）。

### 触发时机

每次运行 linthis 主命令时，系统会：
1. 加载配置文件中的 `self_update` 设置
2. 检查 `~/.linthis/.self_update_last_check` 文件
3. 计算距离上次检查的时间
4. 如果超过配置的间隔，触发更新检查流程

### 更新流程

根据配置的 `mode`：

- **auto 模式**：
  1. 通过 `pip index versions linthis` 检查 PyPI 上的最新版本
  2. **如果没有新版本**：静默更新时间戳，不显示任何提示
  3. **如果有新版本**：自动执行 `pip install --upgrade linthis`
  4. 显示更新进度
  5. 更新时间戳

- **prompt 模式**：
  1. 检查 PyPI 上的最新版本
  2. **如果没有新版本**：静默更新时间戳，不显示任何提示
  3. **如果有新版本**：提示用户 `A new version of linthis is available: 0.0.4 → 0.0.5. Update now? [Y/n]:`
  4. 等待用户输入
  5. 如果用户确认，执行更新
  6. 如果用户拒绝，跳过并更新时间戳（避免重复提示）

- **disabled 模式**：
  - 跳过所有检查

**重要提示**：只有在检测到新版本时才会提示用户或自动更新，没有更新时会静默更新检查时间戳，避免不必要的干扰。

### 版本检测

使用 `pip index versions linthis` 命令从 PyPI 获取最新版本信息，并与当前版本（来自 `CARGO_PKG_VERSION`）进行比较。

## 使用示例

### 示例 1：默认配置（提示模式）

```toml
[self_auto_update]
enabled = true
mode = "prompt"
interval_days = 7
```

**当距离上次检查超过 7 天且有新版本时**：
```bash
$ linthis
A new version of linthis is available: 0.0.4 → 0.0.5. Update now? [Y/n]: y
↓ Upgrading linthis via pip...
✓ linthis upgraded successfully
```

**当距离上次检查超过 7 天但没有新版本时**：
```bash
$ linthis
# 静默更新检查时间戳，不显示任何提示
# 直接开始正常的 linting 流程
```

### 示例 2：自动模式（无需确认）

```toml
[self_auto_update]
enabled = true
mode = "auto"
interval_days = 3
```

每 3 天自动检查并更新，无需用户确认：
```bash
$ linthis
↓ Upgrading linthis via pip...
✓ linthis upgraded successfully
```

### 示例 3：禁用自动更新

```toml
[self_auto_update]
enabled = false
```

或者：

```toml
[self_auto_update]
mode = "disabled"
```

### 示例 4：手动更新

即使配置了自动更新，您仍然可以随时手动更新：

```bash
pip install --upgrade linthis
```

## 与 oh-my-zsh 的对比

| 特性 | oh-my-zsh | linthis |
|-----|-----------|---------|
| 默认检查间隔 | 13 天 | 7 天 |
| 更新模式 | auto, reminder, disabled | auto, prompt, disabled |
| 时间追踪 | `~/.zsh-update` | `~/.linthis/.self_update_last_check` |
| 提示信息 | shell 提示 | CLI 交互 |
| 更新方式 | git pull | pip install --upgrade |

## 配置优先级

配置加载遵循以下优先级（从高到低）：
1. 项目配置（`.linthis/config.toml`）
2. 全局配置（`~/.linthis/config.toml`）
3. 内置默认值

## 故障排查

### 问题：自动更新不工作

**检查清单**：
1. 确认 `self_update.enabled = true`
2. 确认 `self_update.mode` 不是 `"disabled"`
3. 检查 `~/.linthis/.self_update_last_check` 文件的权限
4. 查看是否有错误信息输出
5. 确认 `pip` 命令可用：`pip --version`

### 问题：提示太频繁

**解决方案**：增加 `interval_days` 的值：
```toml
[self_auto_update]
interval_days = 14  # 改为 14 天
```

### 问题：想要完全禁用

**解决方案**：
```toml
[self_auto_update]
enabled = false
```

或者删除整个 `[self_auto_update]` 配置段（将使用默认值）。

### 问题：pip 权限问题

如果遇到权限问题，可能需要使用用户模式安装：
```bash
pip install --user --upgrade linthis
```

或者使用 `sudo`（不推荐）：
```bash
sudo pip install --upgrade linthis
```

## 技术细节

### 实现位置
- **模块**：`linthis/src/self_update.rs`
- **配置**：`linthis/src/config/mod.rs`
- **集成**：`linthis/src/main.rs`

### 核心组件
- `SelfUpdateConfig`：配置结构体
- `SelfUpdateManager`：更新管理器，处理版本检测、时间追踪和用户交互
- `perform_self_update()`：主执行函数
- `get_current_version()`：获取当前版本（从 CARGO_PKG_VERSION）
- `get_latest_version()`：从 PyPI 获取最新版本
- `has_update()`：检查是否有可用更新
- `upgrade()`：执行 pip 升级

### 版本比较

使用简单的语义化版本比较：
- 将版本号拆分为 major.minor.patch
- 逐段比较数值
- 示例：`0.0.4 < 0.0.5`、`0.1.0 > 0.0.9`

### 测试覆盖
- ✅ 配置验证
- ✅ 时间戳读写
- ✅ 检查间隔计算
- ✅ 模式检查
- ✅ 版本比较
- ✅ 默认值测试

运行测试：
```bash
cargo test self_update
```

## 与插件自动同步的关系

linthis 同时支持：
1. **Self-Update**（本功能）：更新 linthis 自身
2. **Auto-Sync**：自动同步插件

两者独立配置，独立运行：
```toml
# 更新 linthis 自身
[self_auto_update]
enabled = true
mode = "prompt"
interval_days = 7

# 同步插件
[plugin_auto_sync]
enabled = true
mode = "prompt"
interval_days = 7
```

执行顺序：
1. 先检查 linthis 自身更新
2. 再检查插件同步

## 参考

- [pip index versions 文档](https://pip.pypa.io/en/stable/cli/pip_index/)
- [oh-my-zsh 自动更新机制](https://maxchadwick.xyz/blog/a-look-at-auto-updating-in-oh-my-zsh)
- [语义化版本规范](https://semver.org/)
