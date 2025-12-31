# linthis config CLI 设计方案

## 1. 设计目标

添加 `linthis config` 子命令，用于管理配置文件（项目配置或全局配置），支持对 `includes`、`excludes`、`languages` 等字段的增删查改。

## 2. 命令设计（推荐方案）

### 2.1 命令风格

采用 **Git 风格**，与现有的 `plugin` 命令保持一致：

```bash
linthis config <action> <field> <value> [--global]
```

### 2.2 支持的操作

#### 数组字段操作（includes, excludes, languages）

```bash
# 添加元素到数组
linthis config add includes "src/**"
linthis config add excludes "*.log"
linthis config add languages rust

# 从数组中移除元素
linthis config remove includes "src/**"
linthis config remove excludes "*.log"
linthis config remove languages rust

# 清空整个数组
linthis config clear includes
linthis config clear excludes
linthis config clear languages
```

#### 标量字段操作（max_complexity, preset, verbose）

```bash
# 设置标量值
linthis config set max_complexity 20
linthis config set preset google
linthis config set verbose true

# 删除标量值（恢复默认）
linthis config unset max_complexity
linthis config unset preset
```

#### 查询操作

```bash
# 获取单个字段的值
linthis config get includes
linthis config get max_complexity

# 列出所有配置
linthis config list
linthis config list --verbose  # 显示详细信息（包括来源）
```

#### 全局配置支持

所有命令都支持 `-g/--global` 标志：

```bash
# 操作全局配置 (~/.linthis/config.toml)
linthis config add includes "src/**" --global
linthis config add --global includes "src/**"
linthis config add includes "src/**" -g

# 默认操作项目配置 (.linthis.toml)
linthis config add includes "src/**"
```

## 3. 命令结构（Clap 定义）

```rust
#[derive(clap::Subcommand, Debug)]
enum Commands {
    Plugin { ... },

    /// Configuration management commands
    Config {
        #[command(subcommand)]
        action: ConfigCommands,
    },
}

#[derive(clap::Subcommand, Debug)]
enum ConfigCommands {
    /// Add value to an array field (includes, excludes, languages)
    Add {
        /// Field name (includes, excludes, languages)
        field: ConfigField,
        /// Value to add
        value: String,
        /// Modify global configuration
        #[arg(short, long)]
        global: bool,
    },

    /// Remove value from an array field
    Remove {
        /// Field name (includes, excludes, languages)
        field: ConfigField,
        /// Value to remove
        value: String,
        /// Modify global configuration
        #[arg(short, long)]
        global: bool,
    },

    /// Clear all values from an array field
    Clear {
        /// Field name (includes, excludes, languages)
        field: ConfigField,
        /// Modify global configuration
        #[arg(short, long)]
        global: bool,
    },

    /// Set a scalar field value (max_complexity, preset, verbose)
    Set {
        /// Field name
        field: String,
        /// Field value
        value: String,
        /// Modify global configuration
        #[arg(short, long)]
        global: bool,
    },

    /// Unset a scalar field (restore to default)
    Unset {
        /// Field name
        field: String,
        /// Modify global configuration
        #[arg(short, long)]
        global: bool,
    },

    /// Get the value of a field
    Get {
        /// Field name
        field: String,
        /// Get from global configuration
        #[arg(short, long)]
        global: bool,
    },

    /// List all configuration values
    List {
        /// Show detailed information (including source)
        #[arg(short, long)]
        verbose: bool,
        /// List global configuration
        #[arg(short, long)]
        global: bool,
    },
}

#[derive(clap::ValueEnum, Clone, Debug)]
enum ConfigField {
    Includes,
    Excludes,
    Languages,
}
```

## 4. 实现细节

### 4.1 配置文件路径

- **项目配置**: `.linthis.toml`（当前目录或向上查找）
- **全局配置**: `~/.linthis/config.toml`

### 4.2 文件操作

使用 `toml_edit` 库保持文件格式：

```rust
use toml_edit::{DocumentMut, Array, value};

// 添加到数组
fn add_to_array(config_path: &Path, field: &str, value: &str) -> Result<()> {
    let content = fs::read_to_string(config_path)?;
    let mut doc = content.parse::<DocumentMut>()?;

    // 获取或创建数组
    let arr = doc.entry(field)
        .or_insert(toml_edit::array())
        .as_array_mut()?;

    // 去重检查
    if !arr.iter().any(|v| v.as_str() == Some(value)) {
        arr.push(value);
    }

    fs::write(config_path, doc.to_string())?;
    Ok(())
}

// 从数组移除
fn remove_from_array(config_path: &Path, field: &str, value: &str) -> Result<()> {
    let content = fs::read_to_string(config_path)?;
    let mut doc = content.parse::<DocumentMut>()?;

    if let Some(arr) = doc.get_mut(field).and_then(|v| v.as_array_mut()) {
        arr.retain(|v| v.as_str() != Some(value));
    }

    fs::write(config_path, doc.to_string())?;
    Ok(())
}

// 设置标量值
fn set_scalar(config_path: &Path, field: &str, value: &str) -> Result<()> {
    let content = fs::read_to_string(config_path)?;
    let mut doc = content.parse::<DocumentMut>()?;

    // 根据字段类型转换值
    let parsed_value = match field {
        "max_complexity" => value.parse::<i64>()?.into(),
        "verbose" => value.parse::<bool>()?.into(),
        _ => value.into(),
    };

    doc[field] = parsed_value;

    fs::write(config_path, doc.to_string())?;
    Ok(())
}
```

### 4.3 配置文件自动创建

如果配置文件不存在，自动创建：

```rust
fn ensure_config_file(global: bool) -> Result<PathBuf> {
    let config_path = if global {
        let home = dirs::home_dir().ok_or("Cannot find home directory")?;
        let config_dir = home.join(".linthis");
        fs::create_dir_all(&config_dir)?;
        config_dir.join("config.toml")
    } else {
        PathBuf::from(".linthis.toml")
    };

    if !config_path.exists() {
        // 创建带默认注释的空配置
        let default_content = Config::generate_default_toml();
        fs::write(&config_path, default_content)?;
    }

    Ok(config_path)
}
```

### 4.4 字段验证

```rust
fn validate_field(field: &str, value: &str) -> Result<()> {
    match field {
        "max_complexity" => {
            value.parse::<u32>()
                .map_err(|_| "max_complexity must be a positive integer")?;
        }
        "preset" => {
            if !["google", "standard", "airbnb"].contains(&value) {
                return Err("preset must be: google, standard, or airbnb".into());
            }
        }
        "verbose" => {
            value.parse::<bool>()
                .map_err(|_| "verbose must be true or false")?;
        }
        _ => {}
    }
    Ok(())
}
```

## 5. 使用示例

### 5.1 基础使用

```bash
# 配置项目
linthis config add includes "src/**"
linthis config add includes "lib/**"
linthis config add excludes "target/**"
linthis config add excludes "*.tmp"
linthis config add languages rust
linthis config add languages python

linthis config set max_complexity 20
linthis config set preset google

# 查看配置
linthis config get includes
# 输出: ["src/**", "lib/**"]

linthis config list
# 输出:
# includes = ["src/**", "lib/**"]
# excludes = ["target/**", "*.tmp"]
# languages = ["rust", "python"]
# max_complexity = 20
# preset = "google"

# 移除配置
linthis config remove includes "lib/**"
linthis config unset preset

# 清空数组
linthis config clear languages
```

### 5.2 全局配置

```bash
# 配置全局默认
linthis config add excludes "*.log" --global
linthis config add excludes "node_modules/**" -g
linthis config set max_complexity 15 --global

# 查看全局配置
linthis config list --global

# 查看全局配置某个字段
linthis config get max_complexity --global
```

### 5.3 查看详细信息

```bash
# 显示配置来源（项目 vs 全局）
linthis config list --verbose

# 输出:
# Configuration sources (higher precedence overrides lower):
# 1. Project config (.linthis.toml)
# 2. Global config (~/.linthis/config.toml)
# 3. Built-in defaults
#
# [Project (.linthis.toml)]
# includes = ["src/**", "lib/**"]
# excludes = ["target/**"]
#
# [Global (~/.linthis/config.toml)]
# excludes = ["*.log", "node_modules/**"]
# max_complexity = 15
#
# [Effective (merged)]
# includes = ["src/**", "lib/**"]
# excludes = ["target/**", "*.log", "node_modules/**"]
# max_complexity = 15
```

## 6. 错误处理

```bash
# 字段不存在
$ linthis config add invalid_field "value"
Error: Unknown field 'invalid_field'
Available fields: includes, excludes, languages

# 无效的值
$ linthis config set max_complexity abc
Error: max_complexity must be a positive integer

# 操作标量字段用数组操作
$ linthis config add max_complexity 20
Error: 'max_complexity' is not an array field
Use: linthis config set max_complexity 20

# 操作数组字段用标量操作
$ linthis config set includes "src/**"
Error: 'includes' is an array field
Use: linthis config add includes "src/**"
```

## 7. 实现文件结构

```
src/
├── main.rs                    # 添加 ConfigCommands 枚举
├── config/
│   ├── mod.rs                # Config 结构体（已有）
│   └── cli.rs                # 新增：config 命令处理逻辑
│       ├── fn handle_config_add()
│       ├── fn handle_config_remove()
│       ├── fn handle_config_clear()
│       ├── fn handle_config_set()
│       ├── fn handle_config_unset()
│       ├── fn handle_config_get()
│       └── fn handle_config_list()
```

## 8. 测试用例

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_config_add_includes() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join(".linthis.toml");

        add_to_array(&config_path, "includes", "src/**").unwrap();
        add_to_array(&config_path, "includes", "lib/**").unwrap();

        let config = Config::load(&config_path).unwrap();
        assert_eq!(config.includes, vec!["src/**", "lib/**"]);
    }

    #[test]
    fn test_config_add_dedup() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join(".linthis.toml");

        add_to_array(&config_path, "excludes", "*.log").unwrap();
        add_to_array(&config_path, "excludes", "*.log").unwrap();

        let config = Config::load(&config_path).unwrap();
        assert_eq!(config.excludes, vec!["*.log"]);
    }

    #[test]
    fn test_config_set_max_complexity() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join(".linthis.toml");

        set_scalar(&config_path, "max_complexity", "25").unwrap();

        let config = Config::load(&config_path).unwrap();
        assert_eq!(config.max_complexity, Some(25));
    }
}
```

## 9. 向后兼容性

- 旧字段名（`exclude`, `plugin`）通过 serde alias 仍然支持
- `config` 命令只操作新字段名（`includes`, `excludes`, `plugins`）
- 读取配置时，新旧字段名都能正确解析

## 10. 待讨论的点

1. **是否支持嵌套字段**？
   - 例如：`linthis config set language_overrides.rust.max_complexity 15`
   - 建议：第一版不支持，保持简单

2. **是否支持 plugins 字段**？
   - 已有 `linthis plugin add/remove` 命令
   - 建议：`config` 命令不管理 `plugins`，避免重复

3. **list 命令的输出格式**？
   - 建议：默认 TOML 格式，`--verbose` 显示来源
   - 可选：`--output json` 输出 JSON 格式

4. **是否支持 init 子命令**？
   - `linthis config init` 创建默认配置文件
   - 建议：可以添加，但已有 `linthis --init`，可能重复

## 11. 实施优先级

### 第一阶段（核心功能）
- [ ] 数组字段：add, remove, clear
- [ ] 标量字段：set, unset
- [ ] 查询：get, list
- [ ] 全局配置支持：`-g/--global`

### 第二阶段（增强功能）
- [ ] 详细输出：`--verbose` 显示配置来源
- [ ] 输出格式：`--output json`
- [ ] 交互式编辑：`linthis config edit`（打开编辑器）

### 第三阶段（高级功能）
- [ ] 嵌套字段支持
- [ ] 配置模板：`linthis config template <preset>`
- [ ] 配置验证：`linthis config validate`

## 12. 与其他命令的关系

| 命令 | 功能 | 关系 |
|------|------|------|
| `linthis --init` | 创建默认 `.linthis.toml` | `config init` 可能重复 |
| `linthis plugin add` | 添加插件到配置 | 管理 `plugins` 字段 |
| `linthis config add` | 添加配置项 | 管理其他字段 |
| `linthis --config <file>` | 指定配置文件 | 读取配置，不修改 |

建议：保持职责分离，`config` 命令不管理 `plugins` 字段。
