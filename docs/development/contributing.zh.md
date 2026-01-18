# 贡献指南

感谢您有兴趣为 linthis 做贡献！

## 开始之前

### 前提条件

- Rust 1.75+（稳定版）
- Cargo

### 克隆和构建

```bash
git clone https://github.com/zhlinh/linthis.git
cd linthis
cargo build
```

### 运行测试

```bash
cargo test
```

### 运行 Clippy

```bash
cargo clippy
```

## 开发流程

1. Fork 仓库
2. 创建功能分支：`git checkout -b feature/my-feature`
3. 进行更改
4. 运行测试：`cargo test`
5. 运行 clippy：`cargo clippy`
6. 提交更改：`git commit -m "feat: add my feature"`
7. 推送到您的 fork：`git push origin feature/my-feature`
8. 开启 Pull Request

## 代码风格

- 遵循 Rust 标准约定
- 使用 `rustfmt` 进行格式化
- 使用 `clippy` 进行代码检查
- 为新功能编写测试
- 为公共 API 编写文档

## 提交信息

我们遵循[约定式提交](https://www.conventionalcommits.org/)：

- `feat:` - 新功能
- `fix:` - Bug 修复
- `docs:` - 文档更改
- `refactor:` - 代码重构
- `test:` - 测试更改
- `chore:` - 构建/工具更改

示例：

```
feat(python): add support for mypy
fix(cli): handle empty file list correctly
docs: update installation instructions
```

## 添加语言支持

要添加对新语言的支持：

1. 在 `src/checkers/<language>.rs` 中创建检查器
2. 在 `src/formatters/<language>.rs` 中创建格式化器
3. 在 `src/lib.rs` 中为 `Language` 枚举添加语言变体
4. 更新扩展名映射
5. 添加安装提示
6. 编写测试
7. 在 `docs/languages/<language>.md` 中创建文档

参考现有实现作为示例。

## 项目结构

```
src/
├── main.rs          # CLI 入口点
├── lib.rs           # 主库
├── checkers/        # 语言检查器
├── formatters/      # 语言格式化器
├── config/          # 配置处理
├── plugin/          # 插件系统
├── cli/             # CLI 命令
└── utils/           # 工具函数
```

## 测试

### 单元测试

```bash
cargo test
```

### 集成测试

```bash
cargo test --test integration
```

### 特定测试

```bash
cargo test test_name
```

## 文档

- 在 `docs/` 目录更新文档
- 使用 MkDocs 进行本地预览：`mkdocs serve`
- 保持 README.md 与重大更改同步

## 有问题？

- 为 bug 或功能请求开启 issue
- 为问题发起讨论
- 创建新 issue 前检查现有 issue

## 许可证

通过贡献，您同意您的贡献将在 MIT 许可证下获得许可。
