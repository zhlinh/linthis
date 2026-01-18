// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Default linter/formatter configuration templates.
//!
//! This module provides default configuration file contents for various
//! linters and formatters supported by linthis.

/// Default config file contents for each linter/formatter
pub fn get_default_configs() -> Vec<(&'static str, &'static str)> {
    vec![
        // Python - ruff (standalone config)
        (
            "ruff.toml",
            r#"# Linthis default ruff config
# Ruff is an extremely fast Python linter and formatter, written in Rust

line-length = 120
target-version = "py38"

[lint]
# Enable recommended rules
select = ["E", "F", "W", "I", "UP", "B", "C4", "SIM"]
ignore = ["E203", "W503"]

# Allow unused variables when underscore-prefixed
dummy-variable-rgx = "^(_+|(_+[a-zA-Z0-9_]*[a-zA-Z0-9]+?))$"

[lint.per-file-ignores]
"__init__.py" = ["F401"]

[format]
# Use double quotes for strings
quote-style = "double"
# Indent with spaces
indent-style = "space"
"#,
        ),
        // Python - ruff (in pyproject.toml for projects that prefer it)
        (
            "pyproject.toml",
            r#"[tool.ruff]
line-length = 120
target-version = "py38"

[tool.ruff.lint]
# Enable recommended rules
select = ["E", "F", "W", "I", "UP", "B", "C4", "SIM"]
ignore = ["E203", "W503"]

[tool.ruff.lint.per-file-ignores]
"__init__.py" = ["F401"]

[tool.ruff.format]
quote-style = "double"
indent-style = "space"
"#,
        ),
        // C/C++ - clang-format
        (
            ".clang-format",
            r#"# Lintis default clang-format config
BasedOnStyle: Google
IndentWidth: 4
ColumnLimit: 120
AllowShortFunctionsOnASingleLine: None
AllowShortIfStatementsOnASingleLine: false
AllowShortLoopsOnASingleLine: false
BreakBeforeBraces: Attach
PointerAlignment: Left
SpaceAfterCStyleCast: false
"#,
        ),
        // C/C++ - cpplint
        (
            "CPPLINT.cfg",
            r#"# Lintis default cpplint config
set noparent
linelength=120
"#,
        ),
        // TypeScript/JavaScript - prettier
        (
            ".prettierrc",
            r#"{
  "semi": true,
  "singleQuote": true,
  "tabWidth": 2,
  "printWidth": 120,
  "trailingComma": "es5",
  "bracketSpacing": true,
  "arrowParens": "avoid"
}
"#,
        ),
        // TypeScript/JavaScript - eslint
        (
            ".eslintrc.json",
            r#"{
  "env": {
    "browser": true,
    "es2021": true,
    "node": true
  },
  "extends": ["eslint:recommended"],
  "parserOptions": {
    "ecmaVersion": "latest",
    "sourceType": "module"
  },
  "rules": {
    "no-unused-vars": "warn",
    "no-console": "off",
    "semi": ["error", "always"],
    "quotes": ["error", "single"]
  }
}
"#,
        ),
        // Rust - rustfmt
        (
            "rustfmt.toml",
            r#"# Lintis default rustfmt config
max_width = 120
tab_spaces = 4
edition = "2021"
use_small_heuristics = "Default"
"#,
        ),
    ]
}
