# TypeScript 语言指南

linthis 使用 **eslint** 进行代码检查，使用 **prettier** 进行代码格式化。

## 支持的文件扩展名

- `.ts`
- `.tsx`
- `.mts`
- `.cts`

## 必需工具

### 代码检查：eslint

```bash
# 全局安装
npm install -g eslint

# 或项目本地安装
npm install --save-dev eslint

# TypeScript 支持
npm install --save-dev @typescript-eslint/parser @typescript-eslint/eslint-plugin

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

[typescript]
max_complexity = 15
excludes = ["dist/**", "node_modules/**", "*.d.ts"]
```

### 禁用特定规则

```toml
[typescript.rules]
disable = [
    "@typescript-eslint/no-explicit-any",
    "@typescript-eslint/no-unused-vars",
    "no-console"
]
```

### 更改严重性

```toml
[typescript.rules.severity]
"@typescript-eslint/no-unsafe-assignment" = "error"
"@typescript-eslint/explicit-function-return-type" = "warning"
```

## 自定义规则

```toml
[[rules.custom]]
code = "ts/no-console-log"
pattern = "console\\.log"
message = "Remove console.log before committing"
severity = "warning"
suggestion = "Use a proper logging library"
languages = ["typescript"]

[[rules.custom]]
code = "ts/no-any-cast"
pattern = "as any"
message = "Avoid 'as any' type assertions"
severity = "warning"
languages = ["typescript"]
```

## CLI 用法

```bash
# 仅检查 TypeScript 文件
linthis -c --lang typescript

# 仅格式化 TypeScript 文件
linthis -f --lang typescript

# 检查特定文件
linthis -c src/index.ts
```

## ESLint 配置

linthis 使用您项目的 ESLint 配置。创建 `.eslintrc.json`：

```json
{
  "parser": "@typescript-eslint/parser",
  "plugins": ["@typescript-eslint"],
  "extends": [
    "eslint:recommended",
    "plugin:@typescript-eslint/recommended"
  ],
  "rules": {
    "@typescript-eslint/no-unused-vars": "warn",
    "@typescript-eslint/explicit-function-return-type": "off"
  }
}
```

或使用 flat config（`eslint.config.js`）：

```javascript
import tseslint from '@typescript-eslint/eslint-plugin';
import tsparser from '@typescript-eslint/parser';

export default [
  {
    files: ['**/*.ts', '**/*.tsx'],
    languageOptions: {
      parser: tsparser,
    },
    plugins: {
      '@typescript-eslint': tseslint,
    },
    rules: {
      ...tseslint.configs.recommended.rules,
    },
  },
];
```

## Prettier 配置

创建 `.prettierrc`：

```json
{
  "semi": true,
  "singleQuote": true,
  "tabWidth": 2,
  "trailingComma": "es5",
  "printWidth": 100
}
```

## 常见问题

### ESLint 找不到 TypeScript 配置

确保安装了 TypeScript ESLint 包：

```bash
npm install --save-dev @typescript-eslint/parser @typescript-eslint/eslint-plugin
```

### Prettier 和 ESLint 冲突

使用 `eslint-config-prettier` 禁用 ESLint 中的格式化规则：

```bash
npm install --save-dev eslint-config-prettier
```

然后添加到 `.eslintrc.json`：

```json
{
  "extends": [
    "plugin:@typescript-eslint/recommended",
    "prettier"
  ]
}
```

### 类型声明文件被检查

添加到排除项：

```toml
[typescript]
excludes = ["*.d.ts", "**/*.d.ts"]
```

## 最佳实践

1. **使用严格 TypeScript**：在 `tsconfig.json` 中启用 `strict: true`
2. **ESLint + Prettier**：结合使用，配合 `eslint-config-prettier`
3. **类型感知检查**：启用类型感知规则以获得更好的检查
4. **一致的格式化**：让 prettier 处理所有格式化，禁用 ESLint 格式化规则
