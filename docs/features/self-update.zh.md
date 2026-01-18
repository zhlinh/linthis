# 自更新

## 概述

linthis 支持自动自更新功能，灵感来源于 oh-my-zsh 的自动更新机制。该功能会在运行时自动检查并更新 linthis 本身，确保您始终使用最新版本。

## 功能特性

- **可配置的检查间隔**：自定义更新检查频率（默认：7 天）
- **多种更新模式**：
  - `auto`：无需确认自动更新
  - `prompt`：更新前询问用户（默认）
  - `disabled`：禁用自动更新
- **智能更新检测**：仅在有新版本时才提示；无更新时静默更新时间戳
- **智能时间追踪**：使用 Unix 时间戳避免时区问题
- **PyPI 版本检测**：通过 pip 检查最新版本
- **友好的用户交互**：清晰的进度指示器和错误处理

## 配置

### 配置文件

在 `.linthis/config.toml` 或 `~/.linthis/config.toml` 中添加：

```toml
# 自更新设置
[self_auto_update]
enabled = true           # 启用自动更新检查
mode = "prompt"          # 更新模式："auto"、"prompt"、"disabled"
interval_days = 7        # 检查间隔（天）
```

### 配置选项

#### `enabled`
- **类型**：布尔值
- **默认值**：`true`
- **描述**：是否启用自动更新检查

#### `mode`
- **类型**：字符串
- **默认值**：`"prompt"`
- **选项**：
  - `"auto"`：无需用户确认自动更新
  - `"prompt"`：更新前询问用户
  - `"disabled"`：禁用自动更新

#### `interval_days`
- **类型**：整数
- **默认值**：`7`
- **描述**：更新检查间隔天数

## 工作原理

### 时间追踪

自动更新使用 `~/.linthis/.self_update_last_check` 文件存储上次检查的时间戳（Unix 纪元秒）。

### 触发条件

每次运行 linthis 时，系统会：
1. 从配置加载 `self_update` 设置
2. 检查 `~/.linthis/.self_update_last_check` 文件
3. 计算距上次检查的时间
4. 如果超过间隔时间，触发更新检查流程

### 更新流程

根据配置的 `mode`：

- **auto 模式**：
  1. 通过 `pip index versions linthis` 检查 PyPI 上的最新版本
  2. **如果无新版本**：静默更新时间戳，无提示
  3. **如果有新版本**：自动运行 `pip install --upgrade linthis`
  4. 显示更新进度
  5. 更新时间戳

- **prompt 模式**：
  1. 检查 PyPI 上的最新版本
  2. **如果无新版本**：静默更新时间戳，无提示
  3. **如果有新版本**：提示 `A new version of linthis is available: 0.0.4 → 0.0.5. Update now? [Y/n]:`
  4. 等待用户输入
  5. 如果确认，执行更新
  6. 如果拒绝，跳过并更新时间戳

- **disabled 模式**：
  - 跳过所有检查

**重要说明**：仅在检测到新版本时才提示或自动更新。已是最新版本时不会有不必要的中断。

## 示例

### 示例 1：默认（Prompt 模式）

```toml
[self_auto_update]
enabled = true
mode = "prompt"
interval_days = 7
```

**当超过间隔时间且有新版本可用时**：
```bash
$ linthis
A new version of linthis is available: 0.0.4 → 0.0.5. Update now? [Y/n]: y
↓ Upgrading linthis via pip...
✓ linthis upgraded successfully
```

**当超过间隔时间但无新版本时**：
```bash
$ linthis
# 静默更新检查时间戳，无提示
# 继续正常的检查流程
```

### 示例 2：Auto 模式

```toml
[self_auto_update]
enabled = true
mode = "auto"
interval_days = 3
```

每 3 天自动检查并无需确认更新：
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

您可以随时手动更新：

```bash
pip install --upgrade linthis
```

## 与 oh-my-zsh 对比

| 功能 | oh-my-zsh | linthis |
|-----|-----------|---------|
| 默认间隔 | 13 天 | 7 天 |
| 更新模式 | auto、reminder、disabled | auto、prompt、disabled |
| 时间追踪 | `~/.zsh-update` | `~/.linthis/.self_update_last_check` |
| 更新方式 | git pull | pip install --upgrade |

## 配置优先级

配置按以下优先级加载（从高到低）：
1. 项目配置 (`.linthis/config.toml`)
2. 全局配置 (`~/.linthis/config.toml`)
3. 内置默认值

## 故障排除

### 自动更新不工作

**检查清单**：
1. 确认 `self_update.enabled = true`
2. 确认 `self_update.mode` 不是 `"disabled"`
3. 检查 `~/.linthis/.self_update_last_check` 的权限
4. 检查输出中的错误信息
5. 确认 `pip` 可用：`pip --version`

### 提示太频繁

**解决方案**：增加 `interval_days`：
```toml
[self_auto_update]
interval_days = 14  # 改为 14 天
```

### 想要完全禁用

**解决方案**：
```toml
[self_auto_update]
enabled = false
```

### pip 权限问题

如果遇到权限问题，尝试用户模式安装：
```bash
pip install --user --upgrade linthis
```

或者使用 `sudo`（不推荐）：
```bash
sudo pip install --upgrade linthis
```

## 与插件自动同步的关系

linthis 支持两种自动更新：
1. **自更新**（本功能）：更新 linthis 本身
2. **自动同步**：同步插件

两者独立配置和运行：
```toml
# 更新 linthis 本身
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
1. 首先检查 linthis 自更新
2. 然后检查插件同步

## 参考资料

- [pip index versions 文档](https://pip.pypa.io/en/stable/cli/pip_index/)
- [oh-my-zsh 自动更新机制](https://maxchadwick.xyz/blog/a-look-at-auto-updating-in-oh-my-zsh)
- [语义化版本](https://semver.org/)
