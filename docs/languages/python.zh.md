# Python 语言指南

linthis 使用 **ruff** 进行 Python 代码检查和格式化。

## 支持的文件扩展名

- `.py`
- `.pyw`

## 必需工具

### 代码检查和格式化：ruff

```bash
# 通过 pip 安装
pip install ruff

# 或通过 pipx（隔离环境）
pipx install ruff

# 或通过 homebrew（macOS）
brew install ruff

# 验证安装
ruff --version
```

## 配置

### 基本示例

```toml
# .linthis/config.toml

[python]
max_complexity = 10
excludes = ["*_test.py", "test_*.py", "venv/**"]
```

### 禁用特定规则

```toml
[python.rules]
disable = [
    "E501",    # 行太长
    "W503",    # 二元运算符前换行
    "F401"     # 未使用的导入
]
```

### 更改严重性

```toml
[python.rules.severity]
"F841" = "error"     # 未使用的变量视为错误
"E999" = "error"     # 语法错误视为错误
```

## 自定义规则

```toml
[[rules.custom]]
code = "python/no-print"
pattern = "\\bprint\\s*\\("
message = "Use logging instead of print()"
severity = "warning"
suggestion = "import logging; logging.info(...)"
languages = ["python"]

[[rules.custom]]
code = "python/no-assert"
pattern = "\\bassert\\b"
message = "Avoid assert in production code"
severity = "info"
languages = ["python"]
```

## CLI 用法

```bash
# 仅检查 Python 文件
linthis -c --lang python

# 仅格式化 Python 文件
linthis -f --lang python

# 检查特定文件
linthis -c src/main.py
```

## Ruff 配置

linthis 会使用您的 `ruff.toml` 或 `pyproject.toml` 配置：

```toml
# ruff.toml
line-length = 100
target-version = "py311"

[lint]
select = ["E", "F", "W", "I", "N", "UP"]
ignore = ["E501"]

[lint.per-file-ignores]
"__init__.py" = ["F401"]
"tests/*" = ["S101"]
```

或在 `pyproject.toml` 中：

```toml
[tool.ruff]
line-length = 100

[tool.ruff.lint]
select = ["E", "F", "W"]
```

## 常见问题

### Ruff 未找到

```
Warning: No python linter available for python files
  Install: pip install ruff
```

**解决方案**：运行 `pip install ruff`

### 虚拟环境文件被检查

**解决方案**：将虚拟环境添加到排除项：

```toml
[python]
excludes = ["venv/**", ".venv/**", "env/**"]
```

### 类型提示未检查

Ruff 不进行类型检查。如需类型检查，请单独使用 mypy 或在 CI 中配置。

## 最佳实践

1. **使用 pyproject.toml**：将 ruff 配置放在 `pyproject.toml` 中以获得更好的工具集成
2. **区分测试文件**：考虑对测试文件使用不同的严重性
3. **复杂度限制**：Python 设置 `max_complexity = 10`（由于 Python 的表达能力，比其他语言低）
4. **导入排序**：Ruff 可以排序导入 - 在 ruff 配置中启用 `select = ["I"]`
