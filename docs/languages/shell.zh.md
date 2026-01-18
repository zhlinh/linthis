# Shell/Bash

linthis 使用 ShellCheck 进行检查，使用 shfmt 进行 Shell/Bash 脚本格式化。

## 支持的扩展名

- `.sh`
- `.bash`
- `.zsh`
- `.ksh`

## 工具

| 工具 | 类型 | 描述 |
|-----|-----|------|
| [ShellCheck](https://www.shellcheck.net/) | 检查器 | Shell 脚本静态分析工具 |
| [shfmt](https://github.com/mvdan/sh) | 格式化器 | Shell 脚本格式化工具 |

## 安装

### macOS

```bash
brew install shellcheck shfmt
```

### Ubuntu/Debian

```bash
apt install shellcheck
# 对于 shfmt，从 GitHub releases 下载或使用 go install
go install mvdan.cc/sh/v3/cmd/shfmt@latest
```

### Windows

```bash
# 使用 scoop
scoop install shellcheck shfmt

# 使用 chocolatey
choco install shellcheck
```

## 配置

### ShellCheck

在项目根目录创建 `.shellcheckrc`：

```ini
# 禁用特定规则
disable=SC2034,SC2086

# 设置 shell 方言
shell=bash

# 启用额外检查
enable=all
```

### shfmt

shfmt 使用命令行参数或 EditorConfig。常见选项：

```bash
# 4 空格缩进
shfmt -i 4

# 使用 tab
shfmt -i 0

# 二元运算符在行首
shfmt -bn
```

## 用法

```bash
# 检查 shell 脚本
linthis --lang shell --check-only

# 格式化 shell 脚本
linthis --lang shell --format-only

# 检查并格式化
linthis --lang shell
```

## 常见问题

### SC2086: 双引号防止通配符展开

```bash
# 错误
echo $var

# 正确
echo "$var"
```

### SC2034: 变量看起来未使用

添加注释抑制：

```bash
# shellcheck disable=SC2034
unused_var="value"
```

## 严重性映射

| ShellCheck 级别 | linthis 严重性 |
|----------------|---------------|
| error | 错误 |
| warning | 警告 |
| info | 信息 |
| style | 信息 |
