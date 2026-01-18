# 监视模式

监视模式允许 linthis 持续监控您的文件，并在文件更改时自动运行检查。

## 基本用法

```bash
# 监视当前目录
linthis watch

# 监视特定目录
linthis watch -i src/ -i lib/

# 监视特定语言
linthis watch --lang python,rust
```

## 选项

| 选项 | 描述 |
|-----|------|
| `-i, --include` | 要监视的目录 |
| `-e, --exclude` | 要排除的模式 |
| `-l, --lang` | 要检查的语言 |
| `-c, --check-only` | 仅检查，不格式化 |
| `-f, --format-only` | 仅格式化，不检查 |
| `--debounce` | 防抖时间（毫秒，默认：500） |

## 示例

### 仅检查模式监视

```bash
linthis watch --check-only
```

### 自定义防抖时间监视

```bash
linthis watch --debounce 1000
```

### 监视特定文件类型

```bash
linthis watch --lang python
```

## 工作原理

1. linthis 监控指定目录的文件更改
2. 当文件被修改、添加或删除时，触发 lint/format 运行
3. 更改会进行防抖处理以避免快速更改时多次运行
4. 为提高效率，仅处理更改的文件

## 键盘快捷键

在监视模式下：

| 键 | 操作 |
|---|------|
| `q` | 退出监视模式 |
| `r` | 强制对所有文件重新运行 |
| `c` | 清屏 |

## 与编辑器集成

监视模式与编辑器集成配合良好：

- **VSCode**：使用终端面板
- **Neovim/Vim**：使用分割终端
- **Emacs**：使用 shell-mode

## 性能考虑

- 监视模式使用高效的文件系统监视器（Linux 上的 inotify，macOS 上的 FSEvents）
- 仅重新检查更改的文件，而非整个项目
- 防抖机制防止快速编辑时过度运行
