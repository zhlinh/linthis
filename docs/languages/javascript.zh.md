# JavaScript 语言指南

linthis 使用 **eslint** 进行代码检查，使用 **prettier** 进行代码格式化。

## 支持的文件扩展名

- `.js`
- `.jsx`
- `.mjs`
- `.cjs`

## 必需工具

### 代码检查：eslint

```bash
# 全局安装
npm install -g eslint

# 或项目本地安装
npm install --save-dev eslint

# 验证安装
eslint --version
```

### 格式化：prettier

```bash
# 全局安装
npm install -g prettier

# 或项目本地安装
npm install --save-dev prettier

# 验证安装
prettier --version
```

## 配置

### 基本示例

```toml
# .linthis/config.toml

[javascript]
max_complexity = 15
excludes = ["dist/**", "node_modules/**", "*.min.js"]
```

### 禁用特定规则

```toml
[javascript.rules]
disable = [
    "no-console",
    "no-unused-vars",
    "no-undef"
]
```

## 自定义规则

```toml
[[rules.custom]]
code = "js/no-var"
pattern = "\\bvar\\s+"
message = "Use 'let' or 'const' instead of 'var'"
severity = "warning"
languages = ["javascript"]

[[rules.custom]]
code = "js/no-alert"
pattern = "\\balert\\s*\\("
message = "Avoid using alert()"
severity = "warning"
languages = ["javascript"]
```

## CLI 用法

```bash
# 仅检查 JavaScript 文件
linthis -c --lang javascript

# 仅格式化 JavaScript 文件
linthis -f --lang javascript
```

## ESLint 配置

创建 `.eslintrc.json`：

```json
{
  "env": {
    "browser": true,
    "es2021": true,
    "node": true
  },
  "extends": "eslint:recommended",
  "parserOptions": {
    "ecmaVersion": "latest",
    "sourceType": "module"
  },
  "rules": {
    "no-unused-vars": "warn",
    "no-console": "off"
  }
}
```

## 常见问题

### ESLint 未找到

```
Warning: No javascript linter available for javascript files
  Install: npm install -g eslint
```

### 压缩文件被检查

添加到排除项：

```toml
[javascript]
excludes = ["*.min.js", "dist/**", "build/**"]
```

## 最佳实践

1. **使用 ES6+**：在 ESLint 配置中启用现代 JavaScript 特性
2. **与 TypeScript 一致**：如果混合 JS/TS，使用相似的 ESLint 配置
3. **JSX 支持**：React 项目添加 `"plugin:react/recommended"` 到 extends
