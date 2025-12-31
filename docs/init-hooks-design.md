# linthis init 支持 pre-commit hooks 集成设计

## 需求

在执行 `linthis init` 时，可以同时创建 pre-commit hooks 配置文件，简化项目初始化流程。

## 设计方案对比

### 方案 1: 添加 --hook 标志（推荐）

```bash
# 只创建配置文件（当前行为，向后兼容）
linthis init
linthis init -g

# 创建配置文件 + hooks 配置
linthis init --hook prek
linthis init --hook pre-commit
linthis init --hook git
linthis init -g --hook prek
```

**优点**：
- 简单直观
- 支持多种 hook 管理器
- 向后兼容（不指定 --hook 时行为不变）
- 灵活，可以明确选择工具

**缺点**：
- 需要记住三种工具名称

**CLI 定义**：
```rust
Init {
    /// Create global configuration (~/.linthis/config.toml)
    #[arg(short, long)]
    global: bool,

    /// Initialize pre-commit hooks configuration
    #[arg(long, value_name = "TOOL")]
    hook: Option<HookTool>,
}

#[derive(Clone, Debug, ValueEnum)]
enum HookTool {
    /// Create .pre-commit-config.yaml for prek (recommended, faster)
    Prek,
    /// Create .pre-commit-config.yaml for pre-commit
    PreCommit,
    /// Create .git/hooks/pre-commit script
    Git,
}
```

---

### 方案 2: 添加 --with-hooks 布尔标志

```bash
# 创建配置文件 + hooks（默认 prek）
linthis init --with-hooks
linthis init --hooks  # 简写

# 不创建 hooks（默认）
linthis init
```

**优点**：
- 最简单，推荐默认工具（prek）
- 降低选择成本

**缺点**：
- 不够灵活，无法选择工具
- 如果用户想用 pre-commit 或 git hook 就需要额外参数

**扩展方案 2A**：
```bash
linthis init --with-hooks       # 默认 prek
linthis init --with-hooks=prek
linthis init --with-hooks=pre-commit
linthis init --with-hooks=git
```

---

### 方案 3: 添加 hooks 子命令

```bash
# 只创建配置文件
linthis init

# 只创建 hooks
linthis init hooks
linthis init hooks --tool prek
linthis init hooks --tool pre-commit

# 不支持一条命令同时创建
```

**优点**：
- 职责分离，语义清晰
- 可以单独初始化 hooks

**缺点**：
- 命令层级深
- 不够便捷，需要两条命令

---

### 方案 4: 交互式提示

```bash
linthis init
# Output:
# ✓ Created .linthis.toml
#
# Would you like to set up pre-commit hooks? (y/n)
# > y
#
# Choose a hook manager:
#   1. prek (recommended, faster)
#   2. pre-commit (standard)
#   3. git hook (simple)
# > 1
#
# ✓ Created .pre-commit-config.yaml
```

**优点**：
- 用户友好，引导式
- 降低学习成本

**缺点**：
- 不适合脚本化/CI 环境
- 需要处理非交互模式

**改进**：支持 `--non-interactive` 或通过环境变量禁用

---

## 推荐方案：方案 1 + 交互式提示

### 命令行接口

```bash
# 非交互模式（明确指定）
linthis init --hook prek          # 创建 .linthis.toml + .pre-commit-config.yaml
linthis init --hook pre-commit    # 创建 .linthis.toml + .pre-commit-config.yaml
linthis init --hook git           # 创建 .linthis.toml + .git/hooks/pre-commit
linthis init                      # 只创建 .linthis.toml（向后兼容）

# 全局配置
linthis init -g --hook prek       # 创建全局配置，hooks 仅在项目级别

# 交互模式（推荐新用户）
linthis init --interactive        # 或 -i
# 提示用户选择是否创建 hooks 及工具类型
```

### 实现细节

#### 1. CLI 参数定义

```rust
/// Initialize configuration file
Init {
    /// Create global configuration (~/.linthis/config.toml)
    #[arg(short, long)]
    global: bool,

    /// Initialize pre-commit hooks (prek, pre-commit, or git)
    #[arg(long, value_name = "TOOL")]
    hook: Option<HookTool>,

    /// Interactive mode - prompt for hooks setup
    #[arg(short, long)]
    interactive: bool,
}

#[derive(Clone, Debug, ValueEnum)]
#[value(rename_all = "kebab-case")]
enum HookTool {
    /// Prek (Rust-based, faster)
    Prek,
    /// Pre-commit (Python-based, standard)
    PreCommit,
    /// Traditional git hook
    Git,
}
```

#### 2. 创建的文件

**prek / pre-commit**:
```yaml
# .pre-commit-config.yaml
repos:
  - repo: local
    hooks:
      - id: linthis
        name: linthis
        entry: linthis --staged --check-only
        language: system
        pass_filenames: false
```

**git**:
```bash
#!/bin/sh
# .git/hooks/pre-commit
linthis --staged --check-only
```

#### 3. 执行流程

```
linthis init [--global] [--hook TOOL] [--interactive]
  |
  ├─> 创建配置文件 (.linthis.toml 或 ~/.linthis/config.toml)
  |
  ├─> if --hook specified:
  |     └─> 创建对应的 hooks 配置文件
  |
  ├─> else if --interactive:
  |     ├─> 询问: "Set up pre-commit hooks? (y/n)"
  |     └─> if yes:
  |           ├─> 显示选项 (prek/pre-commit/git)
  |           └─> 创建对应的 hooks 配置文件
  |
  └─> 输出成功信息
```

#### 4. 输出示例

```bash
$ linthis init --hook prek
✓ Created .linthis.toml
✓ Created .pre-commit-config.yaml (prek/pre-commit compatible)

Next steps:
  1. Install prek: cargo install prek
  2. Set up hooks: prek install

Or use pre-commit:
  1. Install pre-commit: pip install pre-commit
  2. Set up hooks: pre-commit install
```

```bash
$ linthis init --hook git
✓ Created .linthis.toml
✓ Created .git/hooks/pre-commit

Next steps:
  Make sure the hook is executable:
    chmod +x .git/hooks/pre-commit
```

#### 5. 特殊情况处理

- **全局配置 + hooks**: hooks 只在项目级别有意义，如果 `--global --hook` 同时指定，给出警告并忽略 hooks
  ```bash
  $ linthis init -g --hook prek
  Warning: Hooks can only be configured at project level, ignoring --hook flag
  ✓ Created /Users/user/.linthis/config.toml
  ```

- **文件已存在**:
  ```bash
  $ linthis init --hook prek
  ✓ Created .linthis.toml
  Warning: .pre-commit-config.yaml already exists, skipping
  ```

- **不在 git 仓库中使用 --hook git**:
  ```bash
  $ linthis init --hook git
  ✓ Created .linthis.toml
  Error: Not in a git repository, cannot create .git/hooks/pre-commit
  ```

---

## 实现计划

### 阶段 1: 基础功能（MVP）

1. ✅ 定义 CLI 参数（`--hook` 标志）
2. ✅ 实现 `HookTool` 枚举
3. ✅ 实现创建 `.pre-commit-config.yaml` 的逻辑
4. ✅ 实现创建 `.git/hooks/pre-commit` 的逻辑
5. ✅ 添加相应的输出和提示信息
6. ✅ 测试各种场景

### 阶段 2: 增强功能

1. ⬜ 添加 `--interactive` 模式
2. ⬜ 改进错误处理和用户提示
3. ⬜ 添加单元测试

### 阶段 3: 文档

1. ⬜ 更新 README.md
2. ⬜ 更新命令行帮助文本
3. ⬜ 添加示例和最佳实践

---

## 用户故事

### 故事 1: Rust 开发者（推荐路径）

```bash
# 初始化项目配置和 prek hooks
linthis init --hook prek

# 安装 prek
cargo install prek

# 设置 hooks
prek install

# 完成！
```

### 故事 2: Python 开发者

```bash
# 初始化项目配置和 pre-commit hooks
linthis init --hook pre-commit

# 安装 pre-commit
pip install pre-commit

# 设置 hooks
pre-commit install
```

### 故事 3: 简单项目

```bash
# 只用传统 git hook，不需要额外工具
linthis init --hook git

# 完成！hook 已设置
```

### 故事 4: 新手用户

```bash
# 交互式引导
linthis init --interactive

# 系统会询问并引导设置
```

---

## 优势

1. **便捷**: 一条命令完成配置文件和 hooks 的初始化
2. **灵活**: 支持三种主流 hook 管理方式
3. **向后兼容**: 不指定 --hook 时行为不变
4. **推荐最佳实践**: 默认推荐 prek（性能最好）
5. **新手友好**: 提供交互式模式
6. **CI 友好**: 支持非交互模式

---

## 待讨论问题

1. 是否需要支持同时安装多个 hook 管理器？（可能不需要）
2. 是否需要 `linthis init --hook auto` 自动检测已安装的工具？
3. 交互模式是否应该是默认行为？（可能会影响脚本化使用）
4. 是否需要 `linthis hooks` 子命令用于后续管理？
