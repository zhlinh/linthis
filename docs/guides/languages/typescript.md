# TypeScript Language Guide

linthis uses **eslint** for linting and **prettier** for formatting TypeScript code.

## Supported File Extensions

- `.ts`
- `.tsx`
- `.mts`
- `.cts`

## Required Tools

### Linter: eslint

```bash
# Install globally
npm install -g eslint

# Or project-local
npm install --save-dev eslint

# For TypeScript support
npm install --save-dev @typescript-eslint/parser @typescript-eslint/eslint-plugin

# Verify installation
eslint --version
```

### Formatter: prettier

```bash
# Install globally
npm install -g prettier

# Or project-local
npm install --save-dev prettier

# Verify installation
prettier --version
```

## Configuration

### Basic Example

```toml
# .linthis/config.toml

[typescript]
max_complexity = 15
excludes = ["dist/**", "node_modules/**", "*.d.ts"]
```

### Disable Specific Rules

```toml
[typescript.rules]
disable = [
    "@typescript-eslint/no-explicit-any",
    "@typescript-eslint/no-unused-vars",
    "no-console"
]
```

### Change Severity

```toml
[typescript.rules.severity]
"@typescript-eslint/no-unsafe-assignment" = "error"
"@typescript-eslint/explicit-function-return-type" = "warning"
```

## Custom Rules

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

## CLI Usage

```bash
# Check TypeScript files only
linthis -c --lang typescript

# Format TypeScript files only
linthis -f --lang typescript

# Check specific file
linthis -c src/index.ts
```

## ESLint Configuration

linthis uses your project's ESLint configuration. Create `.eslintrc.json`:

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

Or use flat config (`eslint.config.js`):

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

## Prettier Configuration

Create `.prettierrc`:

```json
{
  "semi": true,
  "singleQuote": true,
  "tabWidth": 2,
  "trailingComma": "es5",
  "printWidth": 100
}
```

## Common Issues

### ESLint not finding TypeScript config

Ensure you have the TypeScript ESLint packages installed:

```bash
npm install --save-dev @typescript-eslint/parser @typescript-eslint/eslint-plugin
```

### Prettier and ESLint conflicts

Use `eslint-config-prettier` to disable formatting rules in ESLint:

```bash
npm install --save-dev eslint-config-prettier
```

Then add to `.eslintrc.json`:

```json
{
  "extends": [
    "plugin:@typescript-eslint/recommended",
    "prettier"
  ]
}
```

### Type declaration files being checked

Add to excludes:

```toml
[typescript]
excludes = ["*.d.ts", "**/*.d.ts"]
```

## Best Practices

1. **Use strict TypeScript**: Enable `strict: true` in `tsconfig.json`
2. **ESLint + Prettier**: Use both together with `eslint-config-prettier`
3. **Type-aware linting**: Enable type-aware rules for better checking
4. **Consistent formatting**: Let prettier handle all formatting, disable ESLint formatting rules
