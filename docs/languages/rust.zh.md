# Rust 语言指南

linthis 使用 **clippy** 进行代码检查，使用 **rustfmt** 进行代码格式化。

## 支持的文件扩展名

- `.rs`

## 必需工具

### 代码检查：clippy

```bash
# 通过 rustup 安装（推荐）
rustup component add clippy

# 验证安装
cargo clippy --version
```

### 格式化：rustfmt

```bash
# 通过 rustup 安装（推荐）
rustup component add rustfmt

# 验证安装
rustfmt --version
```

## 配置

### 基本示例

```toml
# .linthis/config.toml

[rust]
max_complexity = 15
excludes = ["target/**"]
```

### 禁用特定 Clippy 检查

```toml
[rust.rules]
disable = [
    "clippy::needless_return",
    "clippy::too_many_arguments",
    "clippy::type_complexity"
]
```

### 更改严重性

```toml
[rust.rules.severity]
"clippy::unwrap_used" = "error"    # 将 unwrap 视为错误
"clippy::todo" = "warning"         # 保持 TODO 为警告
```

## 自定义规则

```toml
[[rules.custom]]
code = "rust/no-println"
pattern = "println!"
message = "Use log macros instead of println!"
severity = "warning"
suggestion = "Use log::info!, log::debug!, etc."
languages = ["rust"]
```

## CLI 用法

```bash
# 仅检查 Rust 文件
linthis -c --lang rust

# 仅格式化 Rust 文件
linthis -f --lang rust

# 检查特定文件
linthis -c src/main.rs
```

## Clippy 配置

linthis 会使用您的 `.clippy.toml` 或 `clippy.toml` 配置文件：

```toml
# clippy.toml
cognitive-complexity-threshold = 15
too-many-arguments-threshold = 8
```

## 常见问题

### Clippy 未找到

```
Warning: No rust linter available for rust files
  Install: rustup component add clippy
```

**解决方案**：运行 `rustup component add clippy`

### 首次运行缓慢

Clippy 首次运行时会编译您的项目。后续运行使用增量编译。

### 与项目 clippy 配置冲突

linthis 使用与 `cargo clippy` 相同的配置。您项目的 `clippy.toml` 设置会被尊重。

## 最佳实践

1. **使用工作空间级配置**：将 `.linthis/config.toml` 放在工作空间根目录
2. **CI 集成**：在 CI 中运行 `linthis -c --lang rust` 以保持一致的检查
3. **复杂度限制**：设置 `max_complexity = 15` 以保持代码可维护
4. **Unwrap 处理**：考虑在生产代码中将 `clippy::unwrap_used` 视为错误
