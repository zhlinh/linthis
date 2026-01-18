# C++ 语言指南

linthis 使用 **cpplint** 和 **clang-tidy** 进行代码检查，使用 **clang-format** 进行代码格式化。

## 支持的文件扩展名

- `.c`、`.cc`、`.cpp`、`.cxx`
- `.h`、`.hpp`、`.hxx`

## 必需工具

### 代码检查：cpplint

```bash
# 通过 pip 安装
pip install cpplint

# 验证安装
cpplint --version
```

### 代码检查：clang-tidy（可选，额外检查）

```bash
# macOS
brew install llvm

# Ubuntu/Debian
sudo apt install clang-tidy

# Windows
choco install llvm

# 验证安装
clang-tidy --version
```

### 格式化：clang-format

```bash
# macOS
brew install clang-format

# Ubuntu/Debian
sudo apt install clang-format

# Windows
choco install llvm

# 验证安装
clang-format --version
```

## 配置

### 基本示例

```toml
# .linthis/config.toml

[cpp]
max_complexity = 25
linelength = 120
excludes = ["build/**", "third_party/**"]
```

### Cpplint 过滤器

禁用特定 cpplint 检查：

```toml
[cpp]
cpplint_filter = "-build/c++11,-whitespace/tab,-build/header_guard"
```

### Clang-tidy 检查

忽略特定 clang-tidy 检查：

```toml
[cpp]
clang_tidy_ignored_checks = [
    "modernize-use-trailing-return-type",
    "readability-magic-numbers"
]
```

### 禁用特定规则

```toml
[cpp.rules]
disable = [
    "build/include_order",
    "whitespace/braces"
]
```

## 自定义规则

```toml
[[rules.custom]]
code = "cpp/no-raw-pointer-new"
pattern = "new\\s+\\w+"
message = "Consider using smart pointers instead of raw new"
severity = "warning"
suggestion = "Use std::make_unique or std::make_shared"
languages = ["cpp"]

[[rules.custom]]
code = "cpp/no-goto"
pattern = "\\bgoto\\b"
message = "Avoid using goto"
severity = "error"
languages = ["cpp"]
```

## CLI 用法

```bash
# 仅检查 C++ 文件
linthis -c --lang cpp

# 仅格式化 C++ 文件
linthis -f --lang cpp
```

## Clang-Format 配置

创建 `.clang-format`：

```yaml
BasedOnStyle: Google
IndentWidth: 4
ColumnLimit: 120
AllowShortFunctionsOnASingleLine: Inline
BreakBeforeBraces: Attach
```

## Cpplint 配置

在项目根目录创建 `CPPLINT.cfg`：

```
linelength=120
filter=-build/c++11,-whitespace/tab
```

## 常见问题

### .h 文件被检测为错误语言

linthis 对 `.h` 文件使用智能检测。如果检测错误，可以强制指定语言：

```bash
linthis -c --lang cpp src/
```

### Cpplint 未找到

```
Warning: No cpp linter available for cpp files
  Install: pip install cpplint
```

### 第三方代码被检查

添加到排除项：

```toml
[cpp]
excludes = ["third_party/**", "external/**", "vendor/**"]
```

## 最佳实践

1. **使用 .clang-format**：在项目中保持一致的格式化
2. **行长度**：120 字符对现代显示器来说较为常见
3. **头文件保护**：如果 cpplint 报错，考虑使用 `#pragma once`
4. **智能指针**：启用原始指针使用的警告
