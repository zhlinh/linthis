# 全局 Git Hook 模板

## 概述

linthis 支持创建全局 Git hook 模板，让所有新创建的 Git 仓库自动包含 linthis pre-commit hook，实现"一次配置，终身受益"。

## 快速开始

### 1. 创建全局 hook 模板

```bash
# 创建全局配置 + Git hook 模板
linthis init -g --hook-type git

# 或者简写（-g 默认创建 git hook 模板）
linthis init -g
```

输出：
```
✓ Created /Users/username/.linthis/config.toml
✓ Created /Users/username/.linthis/.git-template/hooks/pre-commit
✓ Configured git global template: init.templateDir
  All new repositories will include this hook

Next steps:
  • New repositories will automatically include the linthis hook
  • For existing repositories, run: git init
  • Or manually copy the hook to .git/hooks/pre-commit
```

### 2. 自动生效

创建新仓库时，hook 会自动包含：

```bash
# 创建新仓库
mkdir my-project
cd my-project
git init

# hook 已经自动创建
ls .git/hooks/pre-commit  # ✓ 存在
```

### 3. 应用到现有仓库

对于已存在的仓库，运行 `git init` 重新应用模板：

```bash
cd existing-project
git init  # 会复制模板中的 hooks
```

## 详细说明

### 目录结构

全局 hook 模板存放在：
```
~/.linthis/
├── config.toml              # 全局配置
└── .git-template/           # Git 模板目录
    └── hooks/
        └── pre-commit       # pre-commit hook 模板
```

### Git 配置

linthis 会自动配置 Git 全局设置：

```bash
# 查看配置
git config --global --get init.templateDir
# 输出: /Users/username/.linthis/.git-template
```

这个配置让 `git init` 和 `git clone` 自动应用模板。

### Hook 内容

默认创建的 hook 内容：

```bash
#!/bin/sh
# linthis pre-commit hook (global template)
linthis -s -c -f -w
```

参数说明：
- `-s`: 仅检查暂存文件
- `-c`: 运行检查
- `-f`: 运行格式化
- `-w`: 警告视为错误（严格模式）

## 高级用法

### 自定义 Hook 行为

#### 仅检查模式

```bash
linthis init -g --hook-type git --hook-check-only
```

生成的 hook：
```bash
#!/bin/sh
# linthis pre-commit hook (global template)
linthis -s -c -w
```

#### 仅格式化模式

```bash
linthis init -g --hook-type git --hook-format-only
```

生成的 hook：
```bash
#!/bin/sh
# linthis pre-commit hook (global template)
linthis -s -f -w
```

### 强制覆盖

如果模板已存在，使用 `--force` 覆盖：

```bash
linthis init -g --hook-type git --force
```

### 禁用 Hook 创建

只创建全局配置，不创建 hook 模板：

```bash
linthis init -g --no-hook
```

## 与项目级 Hook 的区别

| 特性 | 全局模板 (`-g`) | 项目级 Hook |
|------|----------------|-------------|
| 作用范围 | 所有新仓库 | 当前项目 |
| 配置位置 | `~/.linthis/.git-template/` | `.git/hooks/` |
| 可提交到仓库 | ❌ 否 | ❌ 否（.git 不被跟踪） |
| 团队共享 | ❌ 否 | 需要 prek/pre-commit |
| 适用场景 | 个人开发环境 | 单个项目 |

## 团队协作建议

### 个人开发者

使用全局模板：
```bash
linthis init -g
```

### 团队项目

使用 prek 或 pre-commit（配置可提交）：

```bash
# 在项目目录
linthis init --hook-type prek
# 或
linthis init --hook-type pre-commit
```

这样配置文件可以提交到仓库，团队成员共享。

## 常见问题

### Q1: 如何卸载全局 hook 模板？

```bash
# 删除模板目录
rm -rf ~/.linthis/.git-template

# 取消 git 配置
git config --global --unset init.templateDir
```

### Q2: 现有仓库不想使用这个 hook 怎么办？

**方法 1**（推荐）：不创建 linthis 配置文件

Hook 会自动检测，如果项目没有 linthis 配置文件，就不会运行 linthis。

**方法 2**：删除 hook

```bash
# 删除项目中的 hook
cd my-project
rm .git/hooks/pre-commit
```

### Q3: 可以同时使用全局模板和项目级 prek 吗？

可以，但不推荐。建议：
- 个人项目：使用全局模板
- 团队项目：使用项目级 prek/pre-commit

### Q4: hook 不执行怎么办？

检查权限：
```bash
ls -l ~/.linthis/.git-template/hooks/pre-commit
# 应该显示 -rwxr-xr-x (可执行)

# 如果不可执行，手动设置
chmod +x ~/.linthis/.git-template/hooks/pre-commit
```

### Q5: 为什么 `-g --hook-type prek` 会警告？

全局模板只支持 git hook 类型，因为：
- prek/pre-commit 需要在项目目录运行 `prek install`
- 它们的配置文件（.pre-commit-config.yaml）是项目级的

如果需要 prek/pre-commit，请在项目目录使用：
```bash
linthis init --hook-type prek
```

### Q6: 如何与其他 hook 工具（husky、pre-commit）共存？

**方案 1**：使用 `.git/hooks/pre-commit.local`

全局 hook 会自动链式调用 `.local` 文件：

```bash
# 将其他工具的命令放到 .local 文件
cat > .git/hooks/pre-commit.local << 'EOF'
#!/bin/sh
# 运行其他检查
npm run lint
pytest
EOF
chmod +x .git/hooks/pre-commit.local
```

执行顺序：
1. linthis（如果有配置）
2. .local 中的命令

**方案 2**：禁用全局 hook，使用工具自己的 hook

```bash
# 项目中不创建 linthis 配置
# 全局 hook 会自动跳过，不影响其他工具
```

### Q7: 会不会影响不使用 linthis 的项目？

**不会！** Hook 使用智能检测：

- 只有存在 linthis 配置文件时才运行
- 没有配置文件的项目完全不受影响
- 测试验证：创建新项目不添加 linthis 配置，hook 不会执行任何 linthis 命令

## 智能执行机制

全局 hook 模板使用**智能条件执行**，不会干扰其他项目：

### 工作流程

1. **创建模板**：linthis 在 `~/.linthis/.git-template/hooks/` 创建智能 pre-commit
2. **配置 Git**：设置 `git config --global init.templateDir`
3. **自动应用**：`git init` 时 Git 会复制模板目录的内容到 `.git/`
4. **Hook 执行**：提交时 Git 自动运行 `.git/hooks/pre-commit`

### 智能检测逻辑

Hook 会按以下顺序执行：

```bash
1. 检查项目是否有 linthis 配置文件：
   - .linthis/config.toml
   - .linthis.toml
   - linthis.toml

2. 如果有配置 → 运行 linthis
   如果没有配置 → 跳过 linthis（不影响项目）

3. 检查是否有项目特定 hook：
   - .git/hooks/pre-commit.local

4. 如果存在 → 链式调用执行
```

### Hook 源码

生成的智能 hook 内容：

```bash
#!/bin/sh
# linthis pre-commit hook (global template)
# This hook is installed globally and will only run if the project uses linthis

# Check if this project uses linthis
if [ -f ".linthis/config.toml" ] || [ -f ".linthis.toml" ] || [ -f "linthis.toml" ]; then
    # Run linthis for this project
    linthis -s -c -f -w || exit 1
fi

# Chain to project-specific hook if it exists
# This allows projects to have their own hooks alongside linthis
if [ -f ".git/hooks/pre-commit.local" ]; then
    .git/hooks/pre-commit.local || exit 1
fi
```

### 场景示例

#### 场景 1：使用 linthis 的项目

```bash
my-rust-project/
├── .linthis/
│   └── config.toml     # ✓ 有配置文件
└── .git/
    └── hooks/
        └── pre-commit  # 会运行 linthis
```

**结果**：提交时自动运行 linthis 检查和格式化

#### 场景 2：不使用 linthis 的项目

```bash
other-project/
└── .git/
    └── hooks/
        └── pre-commit  # ✗ 没有 linthis 配置
```

**结果**：hook 跳过 linthis，不影响项目

#### 场景 3：有额外 hook 需求的项目

```bash
complex-project/
├── .linthis/
│   └── config.toml       # ✓ 有配置文件
└── .git/
    └── hooks/
        ├── pre-commit        # 运行 linthis
        └── pre-commit.local  # 然后运行这个
```

**结果**：先运行 linthis，再运行项目特定的检查

#### 场景 4：使用其他 hook 工具的项目

如果项目使用 husky、pre-commit 等工具：

```bash
# 方案 1：移除全局 hook，使用工具自己的 hook
rm .git/hooks/pre-commit
# 然后 husky/pre-commit 会创建自己的 hook

# 方案 2：将工具的命令放到 pre-commit.local
mv .git/hooks/pre-commit .git/hooks/pre-commit.backup
# 创建 pre-commit.local 调用其他工具
```

## 参考

- [Git 文档 - init.templateDir](https://git-scm.com/docs/git-init#_template_directory)
- [linthis Hook 集成设计](./init-hooks-design.md)
