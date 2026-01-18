# 配置

linthis 可以通过配置文件、CLI 参数或两者结合进行配置。

## 配置文件

### 项目配置

在项目根目录创建 `.linthis.toml`：

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
2. **项目配置**：`.linthis.toml`
3. **全局配置**：`~/.linthis/config.toml`
4. **插件配置**：sources 数组中的插件（后面的覆盖前面的）
5. **内置默认值**

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

## 下一步

- [插件系统](../features/plugins.md) - 共享配置
- [CLI 参考](../reference/cli.md) - 所有命令选项
