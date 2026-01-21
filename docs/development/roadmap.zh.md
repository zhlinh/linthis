# Linthis 路线图

> 版本: v0.0.11 | 更新时间: 2025-01-18

## 项目现状总览

| 模块 | 完成度 | 状态 |
|------|--------|------|
| 多语言检查器 | 100% | 18种语言支持 |
| 多语言格式化器 | 100% | 18种语言支持 |
| 配置系统 | 100% | 3层级配置 + 迁移工具 |
| 插件系统 | 100% | 完整实现 |
| CLI界面 | 100% | 核心功能完成 |
| 交互式模式 | 100% | TUI + 通知 |
| 测试覆盖 | 85% | 单元测试 + 集成测试 |
| 文档 | 95% | MkDocs 多语言支持 |

**已支持语言**: Rust, Python, C++, TypeScript, JavaScript, Go, Java, Objective-C, Swift, Kotlin, Lua, Dart, Shell, Ruby, PHP, Scala, C#

---

## 优先级定义

- **P0 (Critical)**: 阻塞核心功能或发布的问题
- **P1 (High)**: 重要功能或用户体验改进
- **P2 (Medium)**: 增强功能，非紧急但有价值
- **P3 (Low)**: 长期规划，nice-to-have

---

## P0 - Critical (立即处理) ✅ 已完成

### 1. 集成测试补全
- [x] 添加端到端集成测试 ✅
- [x] 各语言检查器真实环境测试 ✅
- [x] 插件系统集成测试 ✅
- [x] CI/CD 流水线完善 ✅

### 2. 错误处理增强
- [x] 统一错误类型定义 (`LintisError`, `PluginError` 等) ✅
- [x] 友好的错误提示信息 ✅
- [x] 工具缺失时的降级处理 ✅
- [x] 配置文件解析错误提示 ✅

### 3. main.rs 重构
- [x] 拆分大型 main.rs 到独立模块 (583 行，原 5373 行) ✅
- [x] 抽取命令处理到 `cli/` 模块 ✅
- [x] 分离业务逻辑和CLI逻辑 ✅

---

## P1 - High (下一版本) ✅ 已完成

### 4. 语言支持扩展
- [x] Swift 检查器/格式化器 (SwiftLint + swift-format) ✅
- [x] Kotlin 检查器/格式化器 (Detekt + ktlint) ✅
- [x] Lua 检查器/格式化器 (Luacheck + StyLua) ✅
- [x] Dart 检查器/格式化器 (dart analyze + dart format) ✅

### 5. 性能优化
- [x] 增量检查支持 (只检查变更文件) ✅
- [x] 文件级缓存机制 (`src/cache/`) ✅
- [x] 大文件检测和跳过 (`large_file_threshold`) ✅
- [x] rayon 并行处理 (`par_iter()`) ✅

### 6. Git Hooks 增强
- [x] pre-push hook 支持 ✅
- [x] commit-msg hook 支持 ✅
- [x] hook 并行执行 (使用 rayon) ✅
- [x] hook 失败原因详细报告 ✅

### 7. 配置迁移工具
- [x] 从 ESLint 配置迁移 ✅
- [x] 从 Prettier 配置迁移 ✅
- [x] 从 Black/isort 配置迁移 ✅
- [x] 配置校验和建议 (`validate.rs`) ✅

---

## P2 - Medium (后续版本) ✅ 已完成

### 8. IDE 集成
- [x] VS Code 扩展 (`vscode-linthis/`) ✅
- [x] JetBrains 插件 (`jetbrains-linthis/`) ✅
- [x] Neovim/Vim 插件 (`nvim-linthis/`) ✅
- [x] LSP 服务器支持 (`src/lsp/`) ✅

### 9. 报告与分析
- [x] HTML 报告生成 (`src/reports/html.rs`) ✅
- [x] 代码质量趋势图 (`src/reports/trends.rs`) ✅
- [x] 问题分类统计 (`src/reports/statistics.rs`) ✅
- [x] 团队代码风格一致性分析 (`src/reports/consistency.rs`) ✅

### 10. 自定义规则
- [x] 自定义正则规则支持 (`CustomRulesChecker`) ✅
- [x] 规则禁用/启用细粒度控制 (`RuleFilter`) ✅
- [x] 项目特定规则配置 ✅
- [x] 规则严重级别自定义 (`SeverityOverride`) ✅

### 11. 文档完善
- [x] API 文档生成 (cargo doc) ✅
- [x] 配置项完整文档 ✅
- [x] 各语言使用指南 ✅
- [x] 插件开发指南 (`docs/features/plugins.md`) ✅

### 12. Watch 模式
- [x] 文件变更自动检查 ✅
- [x] 增量结果更新 ✅
- [x] 终端 TUI 界面 (`src/tui/`) ✅
- [x] 通知集成 (`src/watch/notifications.rs`) ✅

---

## P3 - Low (长期规划)

### 13. 额外语言支持
- [x] Ruby (RuboCop) ✅
- [x] PHP (PHP_CodeSniffer + php-cs-fixer) ✅
- [x] C# (dotnet format) ✅
- [x] Scala (scalafix + scalafmt) ✅
- [x] Shell/Bash (ShellCheck + shfmt) ✅

### 14. 高级功能
- [ ] AI 辅助修复建议
- [ ] 代码复杂度可视化
- [ ] 依赖安全扫描
- [ ] 许可证合规检查

### 15. 企业功能
- [ ] 中央配置服务器
- [ ] 团队共享规则集
- [ ] 审计日志
- [ ] LDAP/SSO 集成

### 16. 生态系统
- [ ] GitHub App
- [ ] GitLab CI 模板
- [ ] Jenkins 插件
- [ ] Docker 镜像优化

---

## 版本规划

### v0.1.0 (里程碑)
- ~~完成 P0 所有任务~~ ✅
- ~~main.rs 重构完成~~ ✅
- ~~集成测试覆盖率 > 80%~~ ✅

### v0.2.0 (里程碑)
- ~~完成 P1 任务~~ ✅
- ~~性能优化~~ ✅
- ~~Git Hooks 增强~~ ✅

### v1.0.0 (稳定版) - 已就绪
- ~~完成 P2 核心任务~~ ✅
- VS Code 扩展发布 (进行中)
- ~~完整文档~~ ✅

---

## 贡献指南

欢迎贡献！优先考虑以下方向：
1. VS Code / JetBrains / Neovim 扩展
2. P3 高级功能
3. 测试用例补充
4. 文档改进

请在开始工作前先创建 Issue 讨论。

---

## 更新日志

- **2025-01-18**: 更新路线图 - 所有 P0、P1、P2 任务已完成
- **2025-01-18**: 更新路线图至 v0.0.11，标记已完成项目
- **2025-01-18**: 初始 Roadmap 创建，基于 v0.0.8 版本分析
