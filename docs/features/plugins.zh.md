# 创建插件

本指南介绍如何创建和发布 linthis 插件。

## 什么是插件？

linthis 插件是一个包含以下内容的 Git 仓库：
- `linthis-plugin.toml` 清单文件
- 一个或多个配置文件（TOML、YAML 或 JSON）
- 可选的自定义规则和预设

插件允许您在项目或团队之间共享 lint 配置。

## 插件结构

```
my-linthis-plugin/
├── linthis-plugin.toml      # 必需：插件清单
├── config.toml              # 主配置
├── rules/                   # 可选：额外规则配置
│   ├── strict.toml
│   └── relaxed.toml
└── README.md                # 可选：文档
```

## 插件清单

`linthis-plugin.toml` 清单描述您的插件：

```toml
[plugin]
# 必填字段
name = "my-plugin"
version = "1.0.0"

# 可选字段
description = "Custom lint rules for my organization"
author = "Your Name <email@example.com>"
repository = "https://github.com/username/my-linthis-plugin"
license = "MIT"

# 最低 linthis 版本要求（可选）
min_linthis_version = "0.1.0"

[config]
# 主配置文件路径
path = "config.toml"

# 可选：可引用的额外配置文件
# [config.presets]
# strict = "rules/strict.toml"
# relaxed = "rules/relaxed.toml"
```

## 配置文件

主配置文件遵循标准 linthis 配置格式：

```toml
# config.toml

# 排除模式（将与项目配置合并）
excludes = ["vendor/**", "third_party/**"]

# 此插件风格指南的默认最大复杂度
max_complexity = 15

# 规则修改
[rules]
disable = ["E501"]  # 禁用行长度检查

[rules.severity]
"W0612" = "error"   # 将未使用变量视为错误

# 自定义规则
[[rules.custom]]
code = "org/no-fixme"
pattern = "FIXME|XXX"
message = "FIXME comments must be resolved before merge"
severity = "error"
suggestion = "Create a tracking issue or resolve the FIXME"

[[rules.custom]]
code = "org/copyright-header"
pattern = "^(?!// Copyright)"
message = "Missing copyright header"
severity = "warning"
extensions = ["rs", "go", "java"]

# 语言特定设置
[rust]
max_complexity = 12

[python]
excludes = ["*_test.py", "test_*.py"]
```

## 创建您的第一个插件

### 步骤 1：创建仓库

```bash
mkdir my-linthis-plugin
cd my-linthis-plugin
git init
```

### 步骤 2：创建清单

创建 `linthis-plugin.toml`：

```toml
[plugin]
name = "my-plugin"
version = "1.0.0"
description = "My custom lint configuration"

[config]
path = "config.toml"
```

### 步骤 3：创建配置

创建 `config.toml`：

```toml
# My organization's lint rules

excludes = ["generated/**"]
max_complexity = 20

[[rules.custom]]
code = "custom/no-console-log"
pattern = "console\\.log"
message = "Remove console.log before committing"
severity = "warning"
languages = ["typescript", "javascript"]
```

### 步骤 4：本地测试

发布前，本地测试您的插件：

```bash
# 在项目的 .linthis/config.toml 中
[plugins]
sources = [
    { name = "local-test", url = "file:///path/to/my-linthis-plugin" }
]
```

然后运行：

```bash
linthis plugin init
linthis -c
```

### 步骤 5：发布

推送到 GitHub 或任何 Git 托管：

```bash
git add .
git commit -m "Initial plugin release"
git tag v1.0.0
git push origin main --tags
```

## 使用您的插件

添加到项目的 `.linthis/config.toml`：

```toml
[plugins]
sources = [
    { name = "my-plugin", url = "https://github.com/username/my-linthis-plugin.git" }
]
```

或固定到特定版本：

```toml
[plugins]
sources = [
    { name = "my-plugin", url = "https://github.com/username/my-linthis-plugin.git", ref = "v1.0.0" }
]
```

## 插件命令

```bash
# 初始化/下载插件
linthis plugin init

# 列出已安装插件
linthis plugin list

# 更新所有插件到最新版
linthis plugin update

# 更新特定插件
linthis plugin update my-plugin

# 清理插件缓存
linthis plugin clean
```

## 配置合并

插件配置与项目配置合并。优先级为：

1. **CLI 参数**（最高）
2. **项目配置** (`.linthis/config.toml`)
3. **插件配置**（按列出顺序）
4. **用户配置** (`~/.linthis/config.toml`)
5. **内置默认值**（最低）

数组字段（`excludes`、`includes`、`rules.disable`、`rules.custom`）会被**扩展**（添加），而标量字段会被**覆盖**。

## 最佳实践

### 1. 版本化您的插件

使用语义化版本和 Git 标签：

```bash
git tag v1.0.0   # 初始发布
git tag v1.1.0   # 新功能（向后兼容）
git tag v2.0.0   # 破坏性更改
```

### 2. 记录您的规则

添加注释解释每个自定义规则：

```toml
# Prevent debug code from being committed
[[rules.custom]]
code = "org/no-debug"
pattern = "debugger|console\\.debug"
message = "Debug code should not be committed"
```

### 3. 设置最低版本

如果您的插件使用特定 linthis 版本的功能：

```toml
[plugin]
min_linthis_version = "0.2.0"
```

### 4. 提供预设

为灵活性提供多个配置级别：

```
my-plugin/
├── config.toml           # 默认（推荐）设置
├── presets/
│   ├── strict.toml       # CI 更严格的规则
│   └── relaxed.toml      # 原型开发更宽松
```

### 5. 彻底测试

发布前测试您的插件：
- 在您的规则针对的多种语言上
- 与现有项目配置一起
- linthis 更新后

## 示例插件

### 组织风格指南

```toml
# linthis-plugin.toml
[plugin]
name = "acme-style"
version = "2.0.0"
description = "ACME Corp coding standards"

[config]
path = "config.toml"
```

```toml
# config.toml
max_complexity = 15
preset = "google"

[rules]
disable = ["E501"]  # We use 120-char lines

[[rules.custom]]
code = "acme/ticket-ref"
pattern = "TODO(?!.*\\[ACME-\\d+\\])"
message = "TODO comments must reference a JIRA ticket [ACME-XXX]"
severity = "warning"

[rust]
max_complexity = 12

[python]
max_complexity = 10
```

### 安全规则插件

```toml
# linthis-plugin.toml
[plugin]
name = "security-rules"
version = "1.0.0"
description = "Security-focused lint rules"

[config]
path = "security.toml"
```

```toml
# security.toml
[[rules.custom]]
code = "sec/no-eval"
pattern = "\\beval\\s*\\("
message = "Avoid eval() - potential code injection vulnerability"
severity = "error"
languages = ["javascript", "typescript", "python"]

[[rules.custom]]
code = "sec/no-hardcoded-secret"
pattern = "(password|secret|api_key|token)\\s*=\\s*['\"][^'\"]+['\"]"
message = "Potential hardcoded secret detected"
severity = "error"

[[rules.custom]]
code = "sec/no-http"
pattern = "http://"
message = "Use HTTPS instead of HTTP"
severity = "warning"
```

## 故障排除

### 插件未加载

1. 检查 `linthis plugin list` 查看已安装插件
2. 运行 `linthis plugin init` 重新获取
3. 验证 Git URL 可访问
4. 检查 `linthis-plugin.toml` 中的清单错误

### 配置未应用

1. 检查合并顺序（后面的插件覆盖前面的）
2. 验证清单中的配置文件路径
3. 使用 `--verbose` 运行查看配置加载

### 版本冲突

如果看到版本兼容性错误：
1. 将 linthis 更新到所需版本
2. 或使用 `ref = "v1.0.0"` 使用旧版插件
