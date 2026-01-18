# JavaScript Language Guide

linthis uses **eslint** for linting and **prettier** for formatting JavaScript code.

## Supported File Extensions

- `.js`
- `.jsx`
- `.mjs`
- `.cjs`

## Required Tools

### Linter: eslint

```bash
# Install globally
npm install -g eslint

# Or project-local
npm install --save-dev eslint

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

[javascript]
max_complexity = 15
excludes = ["dist/**", "node_modules/**", "*.min.js"]
```

### Disable Specific Rules

```toml
[javascript.rules]
disable = [
    "no-console",
    "no-unused-vars",
    "no-undef"
]
```

## Custom Rules

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

## CLI Usage

```bash
# Check JavaScript files only
linthis -c --lang javascript

# Format JavaScript files only
linthis -f --lang javascript
```

## ESLint Configuration

Create `.eslintrc.json`:

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

## Common Issues

### ESLint not found

```
Warning: No javascript linter available for javascript files
  Install: npm install -g eslint
```

### Minified files being checked

Add to excludes:

```toml
[javascript]
excludes = ["*.min.js", "dist/**", "build/**"]
```

## Best Practices

1. **Use ES6+**: Enable modern JavaScript features in ESLint config
2. **Consistent with TypeScript**: If mixing JS/TS, use similar ESLint configs
3. **JSX support**: For React projects, add `"plugin:react/recommended"` to extends
