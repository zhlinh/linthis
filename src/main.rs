// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Linthis CLI - A fast, cross-platform multi-language linter and formatter.

use clap::Parser;
use colored::Colorize;
use std::path::PathBuf;
use std::process::ExitCode;

use linthis::utils::output::{format_result, OutputFormat};
use linthis::{run, Language, RunMode, RunOptions};

#[derive(Parser, Debug)]
#[command(name = "linthis")]
#[command(
    author,
    version,
    about = "A fast, cross-platform multi-language linter and formatter"
)]
struct Cli {
    /// Files or directories to include (can be specified multiple times)
    /// Examples: -i src -i lib, --include ./plugin
    #[arg(short = 'i', long = "include")]
    paths: Vec<PathBuf>,

    /// Only run lint checks, no formatting
    #[arg(short = 'c', long)]
    check_only: bool,

    /// Only format files, no lint checking
    #[arg(short = 'f', long)]
    format_only: bool,

    /// Check only staged files (git cached)
    #[arg(short = 's', long)]
    staged: bool,

    /// Specify languages to check (comma-separated: rust,python,typescript)
    #[arg(short, long, value_delimiter = ',')]
    lang: Option<Vec<String>>,

    /// Exclude patterns (glob patterns)
    #[arg(short, long)]
    exclude: Option<Vec<String>>,

    /// Disable default exclusions (.git, node_modules, target, etc.)
    #[arg(long)]
    no_default_excludes: bool,

    /// Disable .gitignore pattern exclusions
    #[arg(long)]
    no_gitignore: bool,

    /// Path to configuration file
    #[arg(long)]
    config: Option<std::path::PathBuf>,

    /// Initialize a new .linthis/config.toml configuration file
    #[arg(long)]
    init: bool,

    /// Generate default config files for all linters/formatters
    #[arg(long)]
    init_configs: bool,

    /// Format preset (google, standard, airbnb)
    #[arg(long)]
    preset: Option<String>,

    /// Output format: human, json, github-actions
    #[arg(short, long, default_value = "human")]
    output: String,

    /// Disable auto-saving results to .linthis/result/
    #[arg(long)]
    no_save_result: bool,

    /// Save results to custom file path (instead of default .linthis/result/)
    #[arg(long, value_name = "FILE")]
    output_file: Option<PathBuf>,

    /// Maximum number of result files to keep (default: 10, 0 = unlimited)
    #[arg(long, default_value = "10")]
    keep_results: usize,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,

    /// Suppress non-error output
    #[arg(short, long)]
    quiet: bool,

    /// Run benchmark comparing ruff vs flake8+black for Python
    #[arg(long)]
    benchmark: bool,

    /// Use a config plugin (name or Git URL)
    /// Examples: -p official, --plugin https://github.com/org/config.git
    #[arg(short = 'p', long)]
    plugin: Option<String>,

    /// Force update cached plugins
    #[arg(long)]
    plugin_update: bool,

    /// Plugin subcommands (init, list, clean)
    #[command(subcommand)]
    command: Option<Commands>,
}

/// Hook management tools
#[derive(Clone, Debug, clap::ValueEnum)]
#[value(rename_all = "kebab-case")]
enum HookTool {
    /// Prek (Rust-based, faster)
    Prek,
    /// Pre-commit (Python-based, standard)
    PreCommit,
    /// Traditional git hook
    Git,
}

#[derive(clap::Subcommand, Debug)]
enum Commands {
    /// Plugin management commands
    Plugin {
        #[command(subcommand)]
        action: PluginCommands,
    },
    /// Configuration management commands
    Config {
        #[command(subcommand)]
        action: ConfigCommands,
    },
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

        /// Force overwrite existing files
        #[arg(short, long)]
        force: bool,
    },
}

#[derive(clap::Subcommand, Debug)]
enum PluginCommands {
    /// Initialize a new plugin
    Init {
        /// Plugin name
        name: String,
    },
    /// List configured or cached plugins
    List {
        /// Show detailed information
        #[arg(short, long)]
        verbose: bool,
        /// List global plugins (~/.linthis/config.toml)
        #[arg(short, long)]
        global: bool,
        /// List cached (downloaded) plugins instead of configured
        #[arg(short, long)]
        cached: bool,
    },
    /// Clean cached plugins
    Clean {
        /// Remove all cached plugins
        #[arg(long)]
        all: bool,
    },
    /// Sync (download/update) configured plugins to latest version
    Sync {
        /// Sync global plugins (~/.linthis/config.toml)
        #[arg(short, long)]
        global: bool,
    },
    /// Validate a plugin manifest
    Validate {
        /// Path to plugin directory
        path: PathBuf,
    },
    /// Add a plugin to configuration
    Add {
        /// Plugin alias (unique name for the plugin)
        alias: String,
        /// Plugin Git repository URL
        url: String,
        /// Git reference (branch, tag, or commit)
        #[arg(long = "ref")]
        git_ref: Option<String>,
        /// Add to global configuration (~/.linthis/config.toml)
        #[arg(short, long)]
        global: bool,
    },
    /// Remove a plugin from configuration (by alias)
    Remove {
        /// Plugin alias to remove
        alias: String,
        /// Remove from global configuration
        #[arg(short, long)]
        global: bool,
    },
    /// Apply (copy) plugin configs to current project
    Apply {
        /// Plugin alias to apply configs from
        alias: Option<String>,
        /// Apply configs from global plugins
        #[arg(short, long)]
        global: bool,
        /// Languages to apply configs for (e.g., cpp, oc, swift)
        #[arg(short, long)]
        language: Option<Vec<String>>,
    },
}

#[derive(clap::Subcommand, Debug)]
enum ConfigCommands {
    /// Add value to an array field (includes, excludes, languages)
    Add {
        /// Field name (includes, excludes, languages)
        field: ConfigField,
        /// Value to add
        value: String,
        /// Modify global configuration (~/.linthis/config.toml)
        #[arg(short, long)]
        global: bool,
    },
    /// Remove value from an array field
    Remove {
        /// Field name (includes, excludes, languages)
        field: ConfigField,
        /// Value to remove
        value: String,
        /// Modify global configuration
        #[arg(short, long)]
        global: bool,
    },
    /// Clear all values from an array field
    Clear {
        /// Field name (includes, excludes, languages)
        field: ConfigField,
        /// Modify global configuration
        #[arg(short, long)]
        global: bool,
    },
    /// Set a scalar field value (max_complexity, preset, verbose)
    Set {
        /// Field name (max_complexity, preset, verbose)
        field: String,
        /// Field value
        value: String,
        /// Modify global configuration
        #[arg(short, long)]
        global: bool,
    },
    /// Unset a scalar field (restore to default)
    Unset {
        /// Field name
        field: String,
        /// Modify global configuration
        #[arg(short, long)]
        global: bool,
    },
    /// Get the value of a field
    Get {
        /// Field name
        field: String,
        /// Get from global configuration
        #[arg(short, long)]
        global: bool,
    },
    /// List all configuration values
    List {
        /// Show detailed information (including source)
        #[arg(short, long)]
        verbose: bool,
        /// List global configuration
        #[arg(short, long)]
        global: bool,
    },
}

#[derive(clap::ValueEnum, Clone, Debug)]
#[allow(non_camel_case_types)]
enum ConfigField {
    #[value(name = "includes")]
    Includes,
    #[value(name = "excludes")]
    Excludes,
    #[value(name = "languages")]
    Languages,
}

impl ConfigField {
    fn as_str(&self) -> &'static str {
        match self {
            ConfigField::Includes => "includes",
            ConfigField::Excludes => "excludes",
            ConfigField::Languages => "languages",
        }
    }
}

/// Generate template config files for a new plugin
fn get_plugin_template_configs(name: &str) -> Vec<(&'static str, String)> {
    vec![
        // ==================== Rust ====================
        (
            "rust/clippy.toml",
            format!(
                r#"# Clippy Configuration for {} Plugin
# Documentation: https://doc.rust-lang.org/clippy/configuration.html
#
# This file configures the Clippy linter for Rust projects.
# Place this file in your project root or configure via Cargo.toml.

# ============================================================================
# COMPLEXITY SETTINGS
# ============================================================================

# Maximum cognitive complexity allowed for functions (default: 25)
# Lower values encourage simpler, more maintainable functions
cognitive-complexity-threshold = 20

# Maximum number of lines in a function body (default: 100)
too-many-lines-threshold = 80

# Maximum number of arguments a function can have (default: 7)
too-many-arguments-threshold = 6

# ============================================================================
# NAMING CONVENTIONS
# ============================================================================

# Minimum length for variable/function names to avoid abbreviations
min-ident-chars-threshold = 2

# ============================================================================
# DOCUMENTATION
# ============================================================================

# Require documentation for public items
# Enable this in library crates for better API documentation
# missing-docs-in-private-items = true

# ============================================================================
# SAFETY & CORRECTNESS
# ============================================================================

# Avoid breaking changes in public API
avoid-breaking-exported-api = true

# Maximum allowed size for stack-allocated arrays (default: 512000 bytes)
array-size-threshold = 512000

# ============================================================================
# STYLE PREFERENCES
# ============================================================================

# Prefer using explicit return types
# allow-private-module-inception = false

# Enforce consistent brace style
# brace-style = "SameLineWhere"
"#,
                name
            ),
        ),
        (
            "rust/rustfmt.toml",
            format!(
                r#"# Rustfmt Configuration for {} Plugin
# Documentation: https://rust-lang.github.io/rustfmt/
#
# This file configures the Rust code formatter.
# Run with: cargo fmt

# ============================================================================
# BASIC SETTINGS
# ============================================================================

# Rust edition (affects parsing and formatting rules)
edition = "2021"

# Maximum line width before wrapping
max_width = 100

# Number of spaces per indentation level
tab_spaces = 4

# Use spaces instead of tabs
hard_tabs = false

# ============================================================================
# IMPORTS
# ============================================================================

# How to group imports: Preserve, Crate, Module, Item, One
imports_granularity = "Crate"

# Reorder import statements alphabetically
reorder_imports = true

# Group imports: std, external crates, then local modules
group_imports = "StdExternalCrate"

# ============================================================================
# FORMATTING STYLE
# ============================================================================

# Use field init shorthand: {{ x: x }} -> {{ x }}
use_field_init_shorthand = true

# Use try shorthand: try!(expr) -> expr?
use_try_shorthand = true

# Format string literals with line breaks
format_strings = false

# Normalize documentation comments (/// vs //!)
normalize_doc_attributes = false

# ============================================================================
# FUNCTION SIGNATURES
# ============================================================================

# Where to put function arguments: Compressed, Tall, Vertical
fn_args_layout = "Tall"

# Where to put function params on trait/impl blocks
fn_params_layout = "Tall"

# ============================================================================
# COMMENTS
# ============================================================================

# Wrap comments at max_width
wrap_comments = false

# Format code in doc comments
format_code_in_doc_comments = true

# Normalize comments (add/remove spaces)
normalize_comments = false

# ============================================================================
# MISCELLANEOUS
# ============================================================================

# Format macro bodies
format_macro_matchers = false
format_macro_bodies = true

# Reorder module declarations
reorder_modules = true

# Use verbose output during formatting
# verbose = false
"#,
                name
            ),
        ),
        // ==================== Python ====================
        (
            "python/ruff.toml",
            format!(
                r#"# Ruff Configuration for {} Plugin
# Documentation: https://docs.astral.sh/ruff/configuration/
#
# Ruff is an extremely fast Python linter and formatter, written in Rust.
# It can replace Flake8, isort, and Black in most projects.

# ============================================================================
# BASIC SETTINGS
# ============================================================================

# Maximum line length (matches Black default)
line-length = 88

# Minimum Python version to target
target-version = "py38"

# File patterns to include/exclude
extend-exclude = [
    ".git",
    ".venv",
    "venv",
    "__pycache__",
    "*.egg-info",
    "build",
    "dist",
]

# ============================================================================
# LINT RULES
# ============================================================================

[lint]
# Select which rule sets to enable
# See: https://docs.astral.sh/ruff/rules/
select = [
    "E",      # pycodestyle errors
    "W",      # pycodestyle warnings
    "F",      # Pyflakes
    "I",      # isort
    "B",      # flake8-bugbear
    "C4",     # flake8-comprehensions
    "UP",     # pyupgrade
    "SIM",    # flake8-simplify
    "TCH",    # flake8-type-checking
    "RUF",    # Ruff-specific rules
]

# Rules to ignore
ignore = [
    "E501",   # Line too long (handled by formatter)
    "B008",   # Do not perform function calls in argument defaults
    "C901",   # Too complex (use cognitive-complexity instead)
]

# Allow autofix for all enabled rules
fixable = ["ALL"]
unfixable = []

# Allow unused variables when underscore-prefixed
dummy-variable-rgx = "^(_+|(_+[a-zA-Z0-9_]*[a-zA-Z0-9]+?))$"

# ============================================================================
# PER-FILE IGNORES
# ============================================================================

[lint.per-file-ignores]
# Allow unused imports in __init__.py
"__init__.py" = ["F401", "F403"]
# Allow assert in tests
"tests/**/*.py" = ["S101"]
"test_*.py" = ["S101"]

# ============================================================================
# ISORT SETTINGS
# ============================================================================

[lint.isort]
# Number of blank lines after imports
lines-after-imports = 2

# Known first-party modules (your project name)
# known-first-party = ["myproject"]

# Force single line imports
force-single-line = false

# ============================================================================
# FORMAT SETTINGS
# ============================================================================

[format]
# Use double quotes for strings (matches Black)
quote-style = "double"

# Indent with spaces
indent-style = "space"

# Skip magic trailing comma
skip-magic-trailing-comma = false

# Unix-style line endings
line-ending = "auto"
"#,
                name
            ),
        ),
        // ==================== TypeScript/JavaScript ====================
        (
            "typescript/.eslintrc.json",
            format!(
                r#"{{
  "$schema": "https://json.schemastore.org/eslintrc",
  "_comment": "ESLint Configuration for {} Plugin",
  "_docs": "https://eslint.org/docs/user-guide/configuring/",

  "root": true,

  "env": {{
    "browser": true,
    "es2022": true,
    "node": true
  }},

  "extends": [
    "eslint:recommended"
  ],

  "parserOptions": {{
    "ecmaVersion": "latest",
    "sourceType": "module"
  }},

  "rules": {{
    "_comment_style": "=== Code Style ===",
    "semi": ["error", "always"],
    "quotes": ["error", "single", {{ "avoidEscape": true }}],
    "indent": ["error", 2, {{ "SwitchCase": 1 }}],
    "comma-dangle": ["error", "always-multiline"],
    "max-len": ["warn", {{ "code": 100, "ignoreUrls": true, "ignoreStrings": true }}],

    "_comment_quality": "=== Code Quality ===",
    "no-unused-vars": ["warn", {{ "argsIgnorePattern": "^_" }}],
    "no-console": ["warn", {{ "allow": ["warn", "error"] }}],
    "eqeqeq": ["error", "always"],
    "curly": ["error", "all"],
    "no-var": "error",
    "prefer-const": "error",
    "prefer-arrow-callback": "error",

    "_comment_safety": "=== Safety ===",
    "no-eval": "error",
    "no-implied-eval": "error",
    "no-new-func": "error",
    "no-return-await": "error"
  }},

  "overrides": [
    {{
      "_comment": "TypeScript files",
      "files": ["*.ts", "*.tsx"],
      "parser": "@typescript-eslint/parser",
      "plugins": ["@typescript-eslint"],
      "extends": [
        "plugin:@typescript-eslint/recommended"
      ],
      "rules": {{
        "@typescript-eslint/no-unused-vars": ["warn", {{ "argsIgnorePattern": "^_" }}],
        "@typescript-eslint/explicit-function-return-type": "off",
        "@typescript-eslint/no-explicit-any": "warn"
      }}
    }},
    {{
      "_comment": "Test files",
      "files": ["*.test.ts", "*.test.js", "*.spec.ts", "*.spec.js"],
      "env": {{
        "jest": true
      }},
      "rules": {{
        "no-console": "off"
      }}
    }}
  ]
}}
"#,
                name
            ),
        ),
        (
            "typescript/.prettierrc",
            format!(
                r#"{{
  "$schema": "https://json.schemastore.org/prettierrc",
  "_comment": "Prettier Configuration for {} Plugin",
  "_docs": "https://prettier.io/docs/en/options.html",

  "semi": true,
  "singleQuote": true,
  "tabWidth": 2,
  "useTabs": false,
  "printWidth": 100,
  "trailingComma": "all",
  "bracketSpacing": true,
  "arrowParens": "avoid",
  "endOfLine": "lf",

  "overrides": [
    {{
      "files": ["*.json", "*.jsonc"],
      "options": {{
        "trailingComma": "none"
      }}
    }},
    {{
      "files": ["*.md"],
      "options": {{
        "proseWrap": "always",
        "printWidth": 80
      }}
    }}
  ]
}}
"#,
                name
            ),
        ),
        // ==================== Go ====================
        (
            "go/.golangci.yml",
            format!(
                r#"# golangci-lint Configuration for {} Plugin
# Documentation: https://golangci-lint.run/usage/configuration/
#
# Run with: golangci-lint run

# ============================================================================
# RUNTIME OPTIONS
# ============================================================================

run:
  # Timeout for analysis (e.g., 5m, 10m)
  timeout: 5m

  # Include test files in analysis
  tests: true

  # Skip directories
  skip-dirs:
    - vendor
    - third_party
    - testdata

  # Skip files by regex
  skip-files:
    - ".*_generated\\.go$"
    - ".*\\.pb\\.go$"

# ============================================================================
# OUTPUT CONFIGURATION
# ============================================================================

output:
  # Format: colored-line-number, line-number, json, tab, checkstyle
  formats:
    - format: colored-line-number

  # Print lines of code with issue
  print-issued-lines: true

  # Print linter name
  print-linter-name: true

# ============================================================================
# LINTERS CONFIGURATION
# ============================================================================

linters:
  # Disable all linters and then enable specific ones
  disable-all: true

  enable:
    # Default linters
    - errcheck       # Check for unchecked errors
    - gosimple       # Suggest code simplifications
    - govet          # Examines Go source code
    - ineffassign    # Detect ineffectual assignments
    - staticcheck    # Static analysis checks
    - unused         # Find unused code

    # Additional recommended linters
    - bodyclose      # Check HTTP response body is closed
    - dogsled        # Check for too many blank identifiers
    - dupl           # Find duplicate code
    - exhaustive     # Check exhaustiveness of enum switch statements
    - funlen         # Limit function length
    - gocognit       # Cognitive complexity checker
    - goconst        # Find repeated strings that could be constants
    - gocritic       # Opinionated linter
    - gocyclo        # Check cyclomatic complexity
    - gofmt          # Check formatting
    - goimports      # Check import formatting
    - goprintffuncname  # Check printf-like function names
    - gosec          # Security checker
    - misspell       # Find misspelled words
    - nakedret       # Find naked returns
    - noctx          # Find HTTP requests without context
    - nolintlint     # Check nolint directives
    - prealloc       # Find slice declarations that could be preallocated
    - revive         # Replacement for golint
    - stylecheck     # Style checker
    - unconvert      # Remove unnecessary type conversions
    - unparam        # Find unused function parameters
    - whitespace     # Check for unnecessary whitespace

# ============================================================================
# LINTER-SPECIFIC SETTINGS
# ============================================================================

linters-settings:
  errcheck:
    # Check for type assertions: a].(type)
    check-type-assertions: true
    # Check for blank identifiers: _ = f()
    check-blank: true

  funlen:
    # Maximum function length (lines)
    lines: 80
    # Maximum statements in function
    statements: 50

  gocognit:
    # Minimal cognitive complexity to report
    min-complexity: 20

  gocyclo:
    # Minimal cyclomatic complexity to report
    min-complexity: 15

  govet:
    # Enable all analyzers
    enable-all: true

  misspell:
    locale: US

  nakedret:
    # Maximum function length for naked returns
    max-func-lines: 30

  revive:
    rules:
      - name: exported
        disabled: false
      - name: var-naming
        disabled: false

  stylecheck:
    # https://staticcheck.io/docs/options#checks
    checks: ["all", "-ST1000", "-ST1003"]

# ============================================================================
# ISSUES CONFIGURATION
# ============================================================================

issues:
  # Show all issues (don't limit)
  max-issues-per-linter: 0
  max-same-issues: 0

  # Don't skip any checks
  exclude-use-default: false

  # Exclude some patterns
  exclude-rules:
    # Exclude some linters from running on tests
    - path: _test\.go
      linters:
        - funlen
        - dupl
        - gocyclo
"#,
                name
            ),
        ),
        // ==================== Java ====================
        (
            "java/checkstyle.xml",
            format!(
                r#"<?xml version="1.0"?>
<!DOCTYPE module PUBLIC
    "-//Checkstyle//DTD Checkstyle Configuration 1.3//EN"
    "https://checkstyle.org/dtds/configuration_1_3.dtd">

<!--
    Checkstyle Configuration for {} Plugin
    Documentation: https://checkstyle.org/checks.html

    Based on Google Java Style with some modifications.
    Run with: java -jar checkstyle.jar -c checkstyle.xml src/
-->

<module name="Checker">
    <!-- Charset for source files -->
    <property name="charset" value="UTF-8"/>

    <!-- Severity level -->
    <property name="severity" value="warning"/>

    <!-- File extensions to check -->
    <property name="fileExtensions" value="java"/>

    <!-- ================================================================ -->
    <!-- File-level Checks -->
    <!-- ================================================================ -->

    <!-- Maximum file length -->
    <module name="FileLength">
        <property name="max" value="500"/>
    </module>

    <!-- No tabs allowed -->
    <module name="FileTabCharacter"/>

    <!-- Trailing whitespace -->
    <module name="RegexpSingleline">
        <property name="format" value="\s+$"/>
        <property name="message" value="Line has trailing whitespace"/>
    </module>

    <!-- ================================================================ -->
    <!-- Tree Walker (AST-based Checks) -->
    <!-- ================================================================ -->

    <module name="TreeWalker">
        <!-- ============================================================ -->
        <!-- Naming Conventions -->
        <!-- ============================================================ -->

        <!-- Package names: lowercase letters and digits -->
        <module name="PackageName">
            <property name="format" value="^[a-z]+(\.[a-z][a-z0-9]*)*$"/>
        </module>

        <!-- Type names: UpperCamelCase -->
        <module name="TypeName"/>

        <!-- Method names: lowerCamelCase -->
        <module name="MethodName">
            <property name="format" value="^[a-z][a-zA-Z0-9]*$"/>
        </module>

        <!-- Constant names: UPPER_CASE -->
        <module name="ConstantName"/>

        <!-- Local variables: lowerCamelCase -->
        <module name="LocalVariableName">
            <property name="format" value="^[a-z][a-zA-Z0-9]*$"/>
        </module>

        <!-- ============================================================ -->
        <!-- Imports -->
        <!-- ============================================================ -->

        <!-- No star imports -->
        <module name="AvoidStarImport"/>

        <!-- No redundant imports -->
        <module name="RedundantImport"/>

        <!-- No unused imports -->
        <module name="UnusedImports"/>

        <!-- ============================================================ -->
        <!-- Size Limits -->
        <!-- ============================================================ -->

        <!-- Maximum line length -->
        <module name="LineLength">
            <property name="max" value="120"/>
            <property name="ignorePattern" value="^package.*|^import.*|a]href|href|http://|https://|ftp://"/>
        </module>

        <!-- Maximum method length -->
        <module name="MethodLength">
            <property name="max" value="80"/>
        </module>

        <!-- Maximum parameters -->
        <module name="ParameterNumber">
            <property name="max" value="7"/>
        </module>

        <!-- ============================================================ -->
        <!-- Whitespace -->
        <!-- ============================================================ -->

        <!-- Whitespace around operators -->
        <module name="WhitespaceAround">
            <property name="allowEmptyConstructors" value="true"/>
            <property name="allowEmptyMethods" value="true"/>
            <property name="allowEmptyTypes" value="true"/>
            <property name="allowEmptyLoops" value="true"/>
        </module>

        <!-- No whitespace after -->
        <module name="NoWhitespaceAfter"/>

        <!-- No whitespace before -->
        <module name="NoWhitespaceBefore"/>

        <!-- ============================================================ -->
        <!-- Code Quality -->
        <!-- ============================================================ -->

        <!-- Require braces for all blocks -->
        <module name="NeedBraces"/>

        <!-- Left curly brace placement -->
        <module name="LeftCurly"/>

        <!-- Right curly brace placement -->
        <module name="RightCurly"/>

        <!-- Empty blocks must have comment -->
        <module name="EmptyBlock">
            <property name="option" value="TEXT"/>
        </module>

        <!-- Avoid nested blocks -->
        <module name="AvoidNestedBlocks"/>

        <!-- ============================================================ -->
        <!-- Best Practices -->
        <!-- ============================================================ -->

        <!-- Avoid empty statements -->
        <module name="EmptyStatement"/>

        <!-- Require equals/hashCode together -->
        <module name="EqualsHashCode"/>

        <!-- Avoid hidden fields -->
        <module name="HiddenField">
            <property name="ignoreConstructorParameter" value="true"/>
            <property name="ignoreSetter" value="true"/>
        </module>

        <!-- Require default in switch -->
        <module name="MissingSwitchDefault"/>

        <!-- Avoid modifying loop variables -->
        <module name="ModifiedControlVariable"/>

        <!-- Simplify boolean expressions -->
        <module name="SimplifyBooleanExpression"/>

        <!-- Simplify boolean returns -->
        <module name="SimplifyBooleanReturn"/>

    </module>
</module>
"#,
                name
            ),
        ),
        // ==================== C/C++ ====================
        (
            "cpp/.clang-format",
            format!(
                r#"# Clang-Format Configuration for {} Plugin
# Documentation: https://clang.llvm.org/docs/ClangFormatStyleOptions.html
#
# Run with: clang-format -i file.cpp
# Or use editor integration (VSCode, CLion, etc.)

---
# ============================================================================
# BASE STYLE
# ============================================================================

# Start from Google style and customize
BasedOnStyle: Google

# Use C++17 standard for parsing
Standard: c++17

# ============================================================================
# INDENTATION
# ============================================================================

# Use spaces for indentation
UseTab: Never

# Number of spaces per indentation level
IndentWidth: 4

# Continuation indent (for wrapped lines)
ContinuationIndentWidth: 4

# Indent case labels in switch
IndentCaseLabels: true

# Indent goto labels
IndentGotoLabels: false

# Indent preprocessor directives
IndentPPDirectives: AfterHash

# Access modifiers (public/private) indentation
AccessModifierOffset: -4

# ============================================================================
# LINE WRAPPING
# ============================================================================

# Maximum line width
ColumnLimit: 120

# How to wrap function arguments
AlignAfterOpenBracket: Align

# Pack function arguments
BinPackArguments: false

# Pack function parameters
BinPackParameters: false

# Always break after return type for function definitions
AlwaysBreakAfterReturnType: None

# Break before braces
BreakBeforeBraces: Attach

# Break after operators
BreakBeforeBinaryOperators: None

# Break before ternary operators
BreakBeforeTernaryOperators: true

# ============================================================================
# ALIGNMENT
# ============================================================================

# Align consecutive assignments
AlignConsecutiveAssignments: false

# Align consecutive declarations
AlignConsecutiveDeclarations: false

# Align consecutive macros
AlignConsecutiveMacros: true

# Align escaped newlines (backslash in macros)
AlignEscapedNewlines: Left

# Align trailing comments
AlignTrailingComments: true

# ============================================================================
# SPACING
# ============================================================================

# Space after C-style cast
SpaceAfterCStyleCast: false

# Space after logical not
SpaceAfterLogicalNot: false

# Space before assignment operators
SpaceBeforeAssignmentOperators: true

# Space before parentheses
SpaceBeforeParens: ControlStatements

# Space in empty parentheses
SpaceInEmptyParentheses: false

# Spaces in parentheses
SpacesInParentheses: false

# Spaces in square brackets
SpacesInSquareBrackets: false

# Spaces in container literals
SpacesInContainerLiterals: false

# Spaces in angles (templates)
SpacesInAngles: false

# ============================================================================
# BRACES & BLOCKS
# ============================================================================

# Allow short blocks on a single line
AllowShortBlocksOnASingleLine: Empty

# Allow short case labels on a single line
AllowShortCaseLabelsOnASingleLine: false

# Allow short functions on a single line
AllowShortFunctionsOnASingleLine: Empty

# Allow short if statements on a single line
AllowShortIfStatementsOnASingleLine: Never

# Allow short loops on a single line
AllowShortLoopsOnASingleLine: false

# Allow short lambdas on a single line
AllowShortLambdasOnASingleLine: All

# ============================================================================
# INCLUDES
# ============================================================================

# Sort includes
SortIncludes: CaseSensitive

# Include categories (priority order)
IncludeCategories:
  # Main header (same name as .cpp file)
  - Regex: '^"[^/]*\.h"'
    Priority: 1
  # Project headers
  - Regex: '^"'
    Priority: 2
  # System headers
  - Regex: '^<'
    Priority: 3

# ============================================================================
# COMMENTS
# ============================================================================

# Reflow comments to fit within column limit
ReflowComments: true

# Space before trailing comments
SpacesBeforeTrailingComments: 2

# ============================================================================
# MISCELLANEOUS
# ============================================================================

# Pointer alignment (Left = int* p, Right = int *p)
PointerAlignment: Left

# Reference alignment (follows PointerAlignment)
ReferenceAlignment: Pointer

# Don't sort using declarations
SortUsingDeclarations: true

# Fix namespace end comments
FixNamespaceComments: true

# Maximum empty lines to keep
MaxEmptyLinesToKeep: 1
"#,
                name
            ),
        ),
        (
            "cpp/CPPLINT.cfg",
            format!(
                r#"# CPPLint Configuration for {} Plugin
# Documentation: https://github.com/cpplint/cpplint
#
# Run with: cpplint --recursive src/
#
# Place this file in your project root.
# CPPLint will automatically find and use it.

# ============================================================================
# GENERAL SETTINGS
# ============================================================================

# Don't inherit from parent directories
set noparent

# Maximum line length
linelength=120

# ============================================================================
# FILTERS
# ============================================================================

# Filter format: +/- category/subcategory
# + enables a check, - disables it
#
# Available categories:
#   build, legal, readability, runtime, whitespace
#
# See: https://github.com/cpplint/cpplint#filters

filter=-build/include_subdir
filter=-build/c++11
filter=-legal/copyright
filter=-readability/todo
filter=-runtime/references
filter=-whitespace/indent

# ============================================================================
# FILE EXTENSIONS
# ============================================================================

# Header file extensions
headers=h,hpp,hxx

# Implementation file extensions
extensions=c,cc,cpp,cxx

# ============================================================================
# EXCLUDE PATTERNS
# ============================================================================

# Exclude directories (one per line)
exclude_files=build
exclude_files=third_party
exclude_files=vendor
exclude_files=.*_test\.cpp
"#,
                name
            ),
        ),
        // ==================== Swift ====================
        (
            "swift/.swiftlint.yml",
            format!(
                r#"# SwiftLint Configuration for {} Plugin
# Documentation: https://realm.github.io/SwiftLint/
#
# This file configures SwiftLint for Swift projects.
# Run with: swiftlint lint

# ============================================================================
# DISABLED RULES
# ============================================================================
# Rules to disable (less strict)
disabled_rules:
  - force_cast           # Allow force casting with `as!`
  - force_try            # Allow force try with `try!`

# ============================================================================
# OPT-IN RULES
# ============================================================================
# Additional rules to enable
opt_in_rules:
  - empty_count          # Prefer checking isEmpty over count == 0

# ============================================================================
# LINE LENGTH
# ============================================================================
line_length:
  warning: 120
  error: 150
  ignores_function_declarations: false
  ignores_comments: false
  ignores_urls: true

# ============================================================================
# TYPE BODY LENGTH
# ============================================================================
type_body_length:
  warning: 300
  error: 400

# ============================================================================
# FUNCTION BODY LENGTH
# ============================================================================
function_body_length:
  warning: 50
  error: 100

# ============================================================================
# FILE LENGTH
# ============================================================================
file_length:
  warning: 500
  error: 1000
  ignore_comment_only_lines: true

# ============================================================================
# CYCLOMATIC COMPLEXITY
# ============================================================================
cyclomatic_complexity:
  warning: 10
  error: 20

# ============================================================================
# IDENTIFIER NAMING
# ============================================================================
identifier_name:
  min_length:
    warning: 2
    error: 1
  max_length:
    warning: 40
    error: 50
  excluded:
    - id
    - URL
    - x
    - y

# ============================================================================
# EXCLUDED FILES
# ============================================================================
excluded:
  - Pods
  - .build
  - DerivedData
  - Carthage
  - vendor
"#,
                name
            ),
        ),
        (
            "swift/.swift-format",
            r#"{
  "version": 1,
  "lineLength": 100,
  "indentation": {
    "spaces": 2
  },
  "maximumBlankLines": 1,
  "respectsExistingLineBreaks": true,
  "lineBreakBeforeControlFlowKeywords": false,
  "lineBreakBeforeEachArgument": true,
  "lineBreakBeforeEachGenericRequirement": false,
  "prioritizeKeepingFunctionOutputTogether": false,
  "indentConditionalCompilationBlocks": true,
  "lineBreakAroundMultilineExpressionChainComponents": false,
  "rules": {
    "AllPublicDeclarationsHaveDocumentation": false,
    "AlwaysUseLowerCamelCase": true,
    "AmbiguousTrailingClosureOverload": true,
    "BeginDocumentationCommentWithOneLineSummary": false,
    "DoNotUseSemicolons": true,
    "DontRepeatTypeInStaticProperties": true,
    "FileScopedDeclarationPrivacy": true,
    "FullyIndirectEnum": true,
    "GroupNumericLiterals": true,
    "IdentifiersMustBeASCII": true,
    "NeverForceUnwrap": false,
    "NeverUseForceTry": false,
    "NeverUseImplicitlyUnwrappedOptionals": false,
    "NoAccessLevelOnExtensionDeclaration": true,
    "NoBlockComments": true,
    "NoCasesWithOnlyFallthrough": true,
    "NoEmptyTrailingClosureParentheses": true,
    "NoLabelsInCasePatterns": true,
    "NoLeadingUnderscores": false,
    "NoParensAroundConditions": true,
    "NoVoidReturnOnFunctionSignature": true,
    "OneCasePerLine": true,
    "OneVariableDeclarationPerLine": true,
    "OnlyOneTrailingClosureArgument": true,
    "OrderedImports": true,
    "ReturnVoidInsteadOfEmptyTuple": true,
    "UseLetInEveryBoundCaseVariable": true,
    "UseShorthandTypeNames": true,
    "UseSingleLinePropertyGetter": true,
    "UseSynthesizedInitializer": true,
    "UseTripleSlashForDocumentationComments": true,
    "ValidateDocumentationComments": false
  }
}
"#
            .to_string(),
        ),
        // ==================== Objective-C ====================
        (
            "objectivec/.clang-format",
            format!(
                r#"# Clang-Format Configuration for {} Plugin (Objective-C)
# Documentation: https://clang.llvm.org/docs/ClangFormatStyleOptions.html
#
# This file configures clang-format for Objective-C projects.
# Run with: clang-format -i *.m *.h

# ============================================================================
# BASE STYLE
# ============================================================================

# Based on LLVM style with modifications
Language: Cpp
BasedOnStyle: LLVM

# ============================================================================
# INDENTATION
# ============================================================================

# Number of spaces for indentation
IndentWidth: 2

# Use spaces instead of tabs
UseTab: Never

# ============================================================================
# LINE WIDTH
# ============================================================================

# Maximum line length before wrapping
ColumnLimit: 100

# ============================================================================
# OBJECTIVE-C SPECIFIC
# ============================================================================

# Number of spaces to indent Objective-C blocks
ObjCBlockIndentWidth: 4

# Add space after @property keyword
ObjCSpaceAfterProperty: false

# Add space before protocol list
ObjCSpaceBeforeProtocolList: true

# Break before binary operators
BreakBeforeBinaryOperators: None

# ============================================================================
# BRACES
# ============================================================================

# Brace wrapping style
BreakBeforeBraces: Attach

# ============================================================================
# SPACING
# ============================================================================

# Add space before parentheses
SpaceBeforeParens: ControlStatements

# Space around pointer qualifiers
SpaceAroundPointerQualifiers: Default

# ============================================================================
# ALIGNMENT
# ============================================================================

# Align consecutive assignments
AlignConsecutiveAssignments: false

# Align consecutive declarations
AlignConsecutiveDeclarations: false

# Pointer alignment
PointerAlignment: Right

# ============================================================================
# INCLUDES
# ============================================================================

# Sort #include directives
SortIncludes: true

# Include categories
IncludeCategories:
  - Regex:           '^<.*\.h>'
    Priority:        1
  - Regex:           '^<.*>'
    Priority:        2
  - Regex:           '.*'
    Priority:        3
"#,
                name
            ),
        ),
        // ==================== SQL ====================
        (
            "sql/.sqlfluff",
            format!(
                r#"# SQLFluff Configuration for {} Plugin
# Documentation: https://docs.sqlfluff.com/

[sqlfluff]
# SQL dialect
dialect = sqlite

# Template language (jinja, dbt, etc.)
templater = raw

# Exclude files
exclude_rules = L034,L036

# Maximum line length
max_line_length = 100

[sqlfluff:rules]
# Tab space size
tab_space_size = 2

# Indentation
indent_unit = space

# Comma style (leading or trailing)
comma_style = trailing

# Capitalisation policy (upper, lower, capitalise)
capitalisation_policy = upper

[sqlfluff:rules:L010]
# Keywords should be upper case
capitalisation_policy = upper

[sqlfluff:rules:L014]
# Unquoted identifiers should be lower case
capitalisation_policy = lower
"#,
                name
            ),
        ),
        // ==================== C# ====================
        (
            "csharp/.editorconfig",
            format!(
                r#"# EditorConfig for {} Plugin (C#)
# Documentation: https://learn.microsoft.com/en-us/dotnet/fundamentals/code-analysis/code-style-rule-options

root = true

# All files
[*]
charset = utf-8
indent_style = space
indent_size = 2
insert_final_newline = true
trim_trailing_whitespace = true

# C# files
[*.cs]
indent_size = 4

# Code style rules
csharp_prefer_braces = true:warning
csharp_style_expression_bodied_methods = false:suggestion
csharp_style_expression_bodied_constructors = false:suggestion
csharp_style_expression_bodied_operators = false:suggestion
csharp_style_expression_bodied_properties = true:suggestion
csharp_style_expression_bodied_indexers = true:suggestion
csharp_style_expression_bodied_accessors = true:suggestion

# Pattern matching preferences
csharp_style_pattern_matching_over_is_with_cast_check = true:suggestion
csharp_style_pattern_matching_over_as_with_null_check = true:suggestion

# Null-checking preferences
csharp_style_throw_expression = true:suggestion
csharp_style_conditional_delegate_call = true:suggestion

# Modifier preferences
csharp_preferred_modifier_order = public,private,protected,internal,static,extern,new,virtual,abstract,sealed,override,readonly,unsafe,volatile,async:suggestion

# Expression-level preferences
csharp_prefer_simple_default_expression = true:suggestion

# Naming conventions
dotnet_naming_rule.interface_should_be_begins_with_i.severity = warning
dotnet_naming_rule.interface_should_be_begins_with_i.symbols = interface
dotnet_naming_rule.interface_should_be_begins_with_i.style = begins_with_i

dotnet_naming_rule.types_should_be_pascal_case.severity = warning
dotnet_naming_rule.types_should_be_pascal_case.symbols = types
dotnet_naming_rule.types_should_be_pascal_case.style = pascal_case

dotnet_naming_rule.non_field_members_should_be_pascal_case.severity = warning
dotnet_naming_rule.non_field_members_should_be_pascal_case.symbols = non_field_members
dotnet_naming_rule.non_field_members_should_be_pascal_case.style = pascal_case

# Symbol specifications
dotnet_naming_symbols.interface.applicable_kinds = interface
dotnet_naming_symbols.interface.applicable_accessibilities = public, internal, private, protected, protected_internal, private_protected
dotnet_naming_symbols.interface.required_modifiers =

dotnet_naming_symbols.types.applicable_kinds = class, struct, interface, enum
dotnet_naming_symbols.types.applicable_accessibilities = public, internal, private, protected, protected_internal, private_protected
dotnet_naming_symbols.types.required_modifiers =

dotnet_naming_symbols.non_field_members.applicable_kinds = property, event, method
dotnet_naming_symbols.non_field_members.applicable_accessibilities = public, internal, private, protected, protected_internal, private_protected
dotnet_naming_symbols.non_field_members.required_modifiers =

# Naming styles
dotnet_naming_style.pascal_case.required_prefix =
dotnet_naming_style.pascal_case.required_suffix =
dotnet_naming_style.pascal_case.word_separator =
dotnet_naming_style.pascal_case.capitalization = pascal_case

dotnet_naming_style.begins_with_i.required_prefix = I
dotnet_naming_style.begins_with_i.required_suffix =
dotnet_naming_style.begins_with_i.word_separator =
dotnet_naming_style.begins_with_i.capitalization = pascal_case
"#,
                name
            ),
        ),
        // ==================== Lua ====================
        (
            "lua/.luacheckrc",
            format!(
                r#"-- Luacheck Configuration for {} Plugin
-- Documentation: https://luacheck.readthedocs.io/

-- Lua version
std = "lua54"

-- Maximum line length
max_line_length = 100

-- Allow unused arguments starting with underscore
unused_args = false

-- Allow unused variables starting with underscore
unused = false

-- Global variables that are allowed
globals = {{
    "vim",  -- For Neovim configs
}}

-- Read-only global variables
read_globals = {{
    "awesome",  -- For AwesomeWM
    "client",
    "root",
}}

-- Ignore specific warnings
ignore = {{
    "212",  -- Unused argument
    "213",  -- Unused loop variable
}}
"#,
                name
            ),
        ),
        (
            "lua/stylua.toml",
            format!(
                r#"# StyLua Configuration for {} Plugin
# Documentation: https://github.com/JohnnyMorganz/StyLua

# Lua version
column_width = 100
line_endings = "Unix"
indent_type = "Spaces"
indent_width = 2
quote_style = "AutoPreferDouble"
call_parentheses = "Always"

[sort_requires]
enabled = false
"#,
                name
            ),
        ),
        // ==================== CSS ====================
        (
            "css/.stylelintrc.json",
            r#"{
  "extends": "stylelint-config-standard",
  "rules": {
    "indentation": 2,
    "string-quotes": "double",
    "no-duplicate-selectors": true,
    "color-hex-case": "lower",
    "color-hex-length": "short",
    "selector-combinator-space-after": "always",
    "selector-attribute-operator-space-before": "never",
    "selector-attribute-operator-space-after": "never",
    "selector-attribute-brackets-space-inside": "never",
    "declaration-block-trailing-semicolon": "always",
    "declaration-colon-space-before": "never",
    "declaration-colon-space-after": "always",
    "number-leading-zero": "always",
    "function-url-quotes": "always",
    "font-weight-notation": "numeric",
    "comment-whitespace-inside": "always",
    "rule-empty-line-before": ["always", {
      "except": ["first-nested"],
      "ignore": ["after-comment"]
    }],
    "at-rule-no-unknown": [true, {
      "ignoreAtRules": ["tailwind", "apply", "variants", "responsive", "screen"]
    }]
  }
}
"#
            .to_string(),
        ),
        (
            "css/.prettierrc",
            r#"{
  "printWidth": 100,
  "tabWidth": 2,
  "useTabs": false,
  "semi": true,
  "singleQuote": false,
  "quoteProps": "as-needed",
  "trailingComma": "es5",
  "bracketSpacing": true,
  "arrowParens": "always",
  "endOfLine": "lf",
  "overrides": [
    {
      "files": ["*.css", "*.scss", "*.less"],
      "options": {
        "singleQuote": false
      }
    }
  ]
}
"#
            .to_string(),
        ),
        // ==================== Kotlin ====================
        (
            "kotlin/.editorconfig",
            format!(
                r#"# EditorConfig for {} Plugin (Kotlin)

root = true

[*]
charset = utf-8
end_of_line = lf
insert_final_newline = true
trim_trailing_whitespace = true

[*.kt]
indent_style = space
indent_size = 4
continuation_indent_size = 4
max_line_length = 120

[*.kts]
indent_style = space
indent_size = 4
"#,
                name
            ),
        ),
        (
            "kotlin/detekt.yml",
            format!(
                r#"# Detekt Configuration for {} Plugin
# Documentation: https://detekt.dev/docs/intro

build:
  maxIssues: 0
  excludeCorrectable: false

config:
  validation: true
  warningsAsErrors: false

complexity:
  active: true
  LongParameterList:
    functionThreshold: 6
    constructorThreshold: 7
  LongMethod:
    threshold: 60
  LargeClass:
    threshold: 600
  ComplexMethod:
    threshold: 15

formatting:
  active: true
  android: false
  autoCorrect: true
  MaximumLineLength:
    maxLineLength: 120

naming:
  active: true
  VariableNaming:
    variablePattern: '[a-z][A-Za-z0-9]*'
  FunctionNaming:
    functionPattern: '[a-z][a-zA-Z0-9]*'

potential-bugs:
  active: true

style:
  active: true
  MaxLineLength:
    maxLineLength: 120
    excludeCommentStatements: true
"#,
                name
            ),
        ),
        // ==================== Dockerfile ====================
        (
            "dockerfile/.hadolint.yaml",
            format!(
                r#"# Hadolint Configuration for {} Plugin
# Documentation: https://github.com/hadolint/hadolint

# Ignore specific rules
ignored:
  - DL3008  # Pin versions in apt-get install
  - DL3009  # Delete the apt-get lists after installing
  - DL3015  # Avoid additional packages by specifying --no-install-recommends

# Trusted registries for base images
trustedRegistries:
  - docker.io
  - gcr.io
  - ghcr.io

# Label schema
label-schema:
  author: text
  version: semver

# Inline ignore pragmas
# Use # hadolint ignore=DL3006 in Dockerfile
"#,
                name
            ),
        ),
        // ==================== Scala ====================
        (
            "scala/.scalafmt.conf",
            format!(
                r#"# Scalafmt Configuration for {} Plugin
# Documentation: https://scalameta.org/scalafmt/

version = "3.7.3"

# Basic settings
maxColumn = 100
assumeStandardLibraryStripMargin = true
align.preset = more
docstrings.style = Asterisk

# Indentation
continuationIndent.defnSite = 2
continuationIndent.callSite = 2
continuationIndent.extendSite = 2

# Newlines
newlines.beforeMultiline = unfold
newlines.topLevelStatementBlankLines = [
  {{
    blanks = 1
  }}
]

# Rewrite rules
rewrite.rules = [
  RedundantBraces,
  RedundantParens,
  SortModifiers,
  PreferCurlyFors
]

# Trailing commas
trailingCommas = preserve

# Import organization
rewrite.scala3.convertToNewSyntax = true
runner.dialect = scala3
"#,
                name
            ),
        ),
        (
            "scala/.scalafix.conf",
            format!(
                r#"# Scalafix Configuration for {} Plugin
# Documentation: https://scalacenter.github.io/scalafix/

rules = [
  OrganizeImports,
  DisableSyntax,
  LeakingImplicitClassVal,
  NoAutoTupling,
  NoValInForComprehension,
  ProcedureSyntax,
  RedundantSyntax
]

DisableSyntax.noVars = true
DisableSyntax.noThrows = false
DisableSyntax.noNulls = true
DisableSyntax.noReturns = true
DisableSyntax.noAsInstanceOf = false
DisableSyntax.noIsInstanceOf = false
DisableSyntax.noXml = true
DisableSyntax.noFinalVal = true
DisableSyntax.noFinalize = true

OrganizeImports {{
  groups = [
    "re:javax?\\.",
    "scala.",
    "*",
    "com.example."
  ]
  removeUnused = true
  groupedImports = Merge
}}
"#,
                name
            ),
        ),
        // ==================== Dart ====================
        (
            "dart/analysis_options.yaml",
            format!(
                r#"# Dart Analyzer Configuration for {} Plugin
# Documentation: https://dart.dev/guides/language/analysis-options

include: package:lints/recommended.yaml

analyzer:
  exclude:
    - build/**
    - lib/generated/**
    - '**/*.g.dart'
    - '**/*.freezed.dart'

  strong-mode:
    implicit-casts: false
    implicit-dynamic: false

  errors:
    missing_required_param: error
    missing_return: error
    todo: ignore
    deprecated_member_use_from_same_package: ignore

  language:
    strict-casts: true
    strict-inference: true
    strict-raw-types: true

linter:
  rules:
    # Error rules
    - avoid_empty_else
    - avoid_print
    - avoid_relative_lib_imports
    - avoid_returning_null_for_future
    - avoid_slow_async_io
    - avoid_types_as_parameter_names
    - cancel_subscriptions
    - close_sinks
    - comment_references
    - control_flow_in_finally
    - empty_statements
    - hash_and_equals
    - invariant_booleans
    - iterable_contains_unrelated_type
    - list_remove_unrelated_type
    - literal_only_boolean_expressions
    - no_adjacent_strings_in_list
    - no_duplicate_case_values
    - prefer_void_to_null
    - test_types_in_equals
    - throw_in_finally
    - unnecessary_statements
    - unrelated_type_equality_checks
    - valid_regexps

    # Style rules
    - always_declare_return_types
    - always_put_control_body_on_new_line
    - always_require_non_null_named_parameters
    - annotate_overrides
    - avoid_bool_literals_in_conditional_expressions
    - avoid_catches_without_on_clauses
    - avoid_catching_errors
    - avoid_classes_with_only_static_members
    - avoid_function_literals_in_foreach_calls
    - avoid_init_to_null
    - avoid_null_checks_in_equality_operators
    - avoid_renaming_method_parameters
    - avoid_return_types_on_setters
    - avoid_returning_null
    - avoid_returning_this
    - avoid_shadowing_type_parameters
    - avoid_single_cascade_in_expression_statements
    - avoid_unnecessary_containers
    - await_only_futures
    - camel_case_extensions
    - camel_case_types
    - cascade_invocations
    - constant_identifier_names
    - curly_braces_in_flow_control_structures
    - directives_ordering
    - empty_catches
    - empty_constructor_bodies
    - file_names
    - implementation_imports
    - join_return_with_assignment
    - library_names
    - library_prefixes
    - lines_longer_than_80_chars
    - non_constant_identifier_names
    - null_closures
    - omit_local_variable_types
    - one_member_abstracts
    - only_throw_errors
    - overridden_fields
    - package_api_docs
    - package_prefixed_library_names
    - parameter_assignments
    - prefer_adjacent_string_concatenation
    - prefer_asserts_in_initializer_lists
    - prefer_collection_literals
    - prefer_conditional_assignment
    - prefer_const_constructors
    - prefer_const_constructors_in_immutables
    - prefer_const_declarations
    - prefer_const_literals_to_create_immutables
    - prefer_constructors_over_static_methods
    - prefer_contains
    - prefer_equal_for_default_values
    - prefer_final_fields
    - prefer_final_in_for_each
    - prefer_final_locals
    - prefer_for_elements_to_map_fromIterable
    - prefer_foreach
    - prefer_function_declarations_over_variables
    - prefer_generic_function_type_aliases
    - prefer_if_elements_to_conditional_expressions
    - prefer_if_null_operators
    - prefer_initializing_formals
    - prefer_inlined_adds
    - prefer_int_literals
    - prefer_interpolation_to_compose_strings
    - prefer_is_empty
    - prefer_is_not_empty
    - prefer_is_not_operator
    - prefer_iterable_whereType
    - prefer_single_quotes
    - prefer_spread_collections
    - prefer_typing_uninitialized_variables
    - provide_deprecation_message
    - recursive_getters
    - slash_for_doc_comments
    - sort_child_properties_last
    - sort_constructors_first
    - sort_unnamed_constructors_first
    - type_annotate_public_apis
    - type_init_formals
    - unawaited_futures
    - unnecessary_await_in_return
    - unnecessary_brace_in_string_interps
    - unnecessary_const
    - unnecessary_getters_setters
    - unnecessary_lambdas
    - unnecessary_new
    - unnecessary_null_aware_assignments
    - unnecessary_null_in_if_null_operators
    - unnecessary_overrides
    - unnecessary_parenthesis
    - unnecessary_this
    - use_full_hex_values_for_flutter_colors
    - use_function_type_syntax_for_parameters
    - use_rethrow_when_possible
    - use_setters_to_change_properties
    - use_string_buffers
    - use_to_and_as_if_applicable
    - void_checks
"#,
                name
            ),
        ),
    ]
}

/// Generate plugin manifest with all config mappings
fn generate_plugin_manifest(name: &str) -> String {
    format!(
        r#"# ============================================================================
# Linthis Plugin Manifest: {name}
# ============================================================================
#
# This file defines the plugin metadata and configuration file mappings.
# Documentation: https://github.com/zhlinh/linthis
#
# Structure:
#   [plugin]     - Plugin metadata (name, version, etc.)
#   [configs.*]  - Configuration file mappings by language

[plugin]
# Plugin name (required)
name = "{name}"

# Plugin version using semver (required)
version = "0.1.0"

# Short description
description = "{name} configuration plugin for linthis"

# Minimum linthis version required
linthis_version = ">=0.2.0"

# Supported languages (informational)
languages = ["rust", "python", "typescript", "go", "java", "cpp", "swift", "objectivec", "sql", "csharp", "lua", "css", "kotlin", "dockerfile", "scala", "dart"]

# License identifier (SPDX)
license = "MIT"

# Plugin authors
[[plugin.authors]]
name = "Your Name"
email = "your.email@example.com"

# ============================================================================
# Configuration File Mappings
# ============================================================================
#
# Format: [configs.<language>]
#         <tool> = "<path/to/config/file>"
#
# The path is relative to this manifest file.
# When users install this plugin, linthis will use these configs.

[configs.rust]
# Clippy linter configuration
clippy = "rust/clippy.toml"
# Rustfmt formatter configuration
rustfmt = "rust/rustfmt.toml"

[configs.python]
# Ruff linter and formatter configuration
ruff = "python/ruff.toml"

[configs.typescript]
# ESLint linter configuration (also works for JavaScript)
eslint = "typescript/.eslintrc.json"
# Prettier formatter configuration
prettier = "typescript/.prettierrc"

[configs.go]
# golangci-lint configuration
golangci-lint = "go/.golangci.yml"

[configs.java]
# Checkstyle configuration
checkstyle = "java/checkstyle.xml"

[configs.cpp]
# Clang-Format configuration
clang-format = "cpp/.clang-format"
# CPPLint configuration
cpplint = "cpp/CPPLINT.cfg"

[configs.swift]
# SwiftLint linter configuration
swiftlint = "swift/.swiftlint.yml"
# swift-format formatter configuration
swift-format = "swift/.swift-format"

[configs.objectivec]
# Clang-Format configuration
clang-format = "objectivec/.clang-format"

[configs.sql]
# SQLFluff linter and formatter configuration
sqlfluff = "sql/.sqlfluff"

[configs.csharp]
# dotnet-format configuration via .editorconfig
editorconfig = "csharp/.editorconfig"

[configs.lua]
# Luacheck linter configuration
luacheck = "lua/.luacheckrc"
# StyLua formatter configuration
stylua = "lua/stylua.toml"

[configs.css]
# Stylelint linter configuration
stylelint = "css/.stylelintrc.json"
# Prettier formatter configuration
prettier = "css/.prettierrc"

[configs.kotlin]
# EditorConfig for Kotlin
editorconfig = "kotlin/.editorconfig"
# Detekt linter configuration
detekt = "kotlin/detekt.yml"

[configs.dockerfile]
# Hadolint linter configuration
hadolint = "dockerfile/.hadolint.yaml"

[configs.scala]
# Scalafmt formatter configuration
scalafmt = "scala/.scalafmt.conf"
# Scalafix linter configuration
scalafix = "scala/.scalafix.conf"

[configs.dart]
# Dart analyzer configuration
analyzer = "dart/analysis_options.yaml"
"#,
        name = name
    )
}

/// Generate README for a new plugin
fn generate_plugin_readme(name: &str) -> String {
    format!(
        r#"# {name} Config Plugin

A linthis configuration plugin providing consistent linting and formatting rules.

## Supported Languages

| Language   | Linter/Formatter      | Config File             |
|------------|----------------------|-------------------------|
| Rust       | clippy, rustfmt      | `rust/clippy.toml`, `rust/rustfmt.toml` |
| Python     | ruff                 | `python/ruff.toml`      |
| TypeScript | eslint, prettier     | `typescript/.eslintrc.json`, `typescript/.prettierrc` |
| Go         | golangci-lint        | `go/.golangci.yml`      |
| Java       | checkstyle           | `java/checkstyle.xml`   |
| C/C++      | clang-format, cpplint| `cpp/.clang-format`, `cpp/CPPLINT.cfg` |
| Swift      | swiftlint, swift-format | `swift/.swiftlint.yml`, `swift/.swift-format` |
| Objective-C| clang-format         | `objectivec/.clang-format` |
| SQL        | sqlfluff             | `sql/.sqlfluff`         |
| C#         | dotnet-format        | `csharp/.editorconfig`  |
| Lua        | luacheck, stylua     | `lua/.luacheckrc`, `lua/stylua.toml` |
| CSS        | stylelint, prettier  | `css/.stylelintrc.json`, `css/.prettierrc` |
| Kotlin     | detekt               | `kotlin/.editorconfig`, `kotlin/detekt.yml` |
| Dockerfile | hadolint             | `dockerfile/.hadolint.yaml` |
| Scala      | scalafmt, scalafix   | `scala/.scalafmt.conf`, `scala/.scalafix.conf` |
| Dart       | dart analyzer        | `dart/analysis_options.yaml` |

## Usage

### Via Command Line

```bash
# Use this plugin for a single run
linthis --plugin https://github.com/your-org/{name}.git
```

### Via Configuration File

Add to your `.linthis/config.toml`:

```toml
[plugin]
sources = [
    {{ name = "{name}", url = "https://github.com/your-org/{name}.git" }},
]
```

### With Version Pinning

```toml
[plugin]
sources = [
    {{ name = "{name}", url = "https://github.com/your-org/{name}.git", ref = "v1.0.0" }},
]
```

## Customization

To override specific settings, you can:

1. **Layer plugins**: Add your overrides in a second plugin that loads after this one
2. **Local overrides**: Settings in your project's `.linthis/config.toml` override plugin settings
3. **Fork and modify**: Fork this repository and customize the configs

## Configuration Priority

Settings are applied in this order (later overrides earlier):

1. Built-in defaults
2. Plugin configs (in order listed in `sources`)
3. User config (`~/.linthis/config.toml`)
4. Project config (`.linthis/config.toml`)
5. CLI flags

## Contributing

1. Fork this repository
2. Make your changes
3. Test with: `linthis plugin validate .`
4. Submit a pull request

## License

MIT License - See LICENSE file for details.
"#,
        name = name
    )
}

/// Handle plugin subcommands
fn handle_plugin_command(action: PluginCommands) -> ExitCode {
    use linthis::plugin::{
        cache::PluginCache,
        manifest::PluginManifest,
    };

    match action {
        PluginCommands::Init { name } => {
            // Create a new plugin scaffold
            let plugin_dir = PathBuf::from(&name);
            if plugin_dir.exists() {
                eprintln!("{}: Directory '{}' already exists", "Error".red(), name);
                return ExitCode::from(1);
            }

            // Create directory structure
            let dirs = [
                "rust",
                "python",
                "typescript",
                "go",
                "java",
                "cpp",
                "swift",
                "objectivec",
                "sql",
                "csharp",
                "lua",
                "css",
                "kotlin",
                "dockerfile",
                "scala",
                "dart",
            ];
            if let Err(e) = std::fs::create_dir_all(&plugin_dir) {
                eprintln!("{}: Failed to create directory: {}", "Error".red(), e);
                return ExitCode::from(1);
            }

            for dir in dirs {
                if let Err(e) = std::fs::create_dir_all(plugin_dir.join(dir)) {
                    eprintln!(
                        "{}: Failed to create {} directory: {}",
                        "Error".red(),
                        dir,
                        e
                    );
                    return ExitCode::from(1);
                }
            }

            // Create example config files for each language
            let config_files = get_plugin_template_configs(&name);
            for (path, content) in config_files {
                let file_path = plugin_dir.join(path);
                if let Err(e) = std::fs::write(&file_path, content) {
                    eprintln!(
                        "{}: Failed to write {}: {}",
                        "Error".red(),
                        file_path.display(),
                        e
                    );
                    return ExitCode::from(1);
                }
            }

            // Create manifest with config mappings
            let manifest_content = generate_plugin_manifest(&name);
            let manifest_path = plugin_dir.join("linthis-plugin.toml");
            if let Err(e) = std::fs::write(manifest_path, manifest_content) {
                eprintln!("{}: Failed to write manifest: {}", "Error".red(), e);
                return ExitCode::from(1);
            }

            // Create README
            let readme_content = generate_plugin_readme(&name);
            let _ = std::fs::write(plugin_dir.join("README.md"), readme_content);

            // Create .gitignore
            let gitignore_content = "# Editor files\n*.swp\n*.swo\n*~\n.idea/\n.vscode/\n\n# OS files\n.DS_Store\nThumbs.db\n";
            let _ = std::fs::write(plugin_dir.join(".gitignore"), gitignore_content);

            println!("{} Created plugin scaffold at {}/", "✓".green(), name);
            println!();
            println!("Created files:");
            println!("  {} linthis-plugin.toml  - Plugin manifest", "•".cyan());
            println!("  {} README.md            - Documentation", "•".cyan());
            println!(
                "  {} rust/                - Rust configs (clippy, rustfmt)",
                "•".cyan()
            );
            println!(
                "  {} python/              - Python configs (ruff)",
                "•".cyan()
            );
            println!(
                "  {} typescript/          - TypeScript configs (eslint, prettier)",
                "•".cyan()
            );
            println!(
                "  {} go/                  - Go configs (golangci-lint)",
                "•".cyan()
            );
            println!(
                "  {} java/                - Java configs (checkstyle)",
                "•".cyan()
            );
            println!(
                "  {} cpp/                 - C/C++ configs (clang-format, cpplint)",
                "•".cyan()
            );
            println!(
                "  {} swift/               - Swift configs (swiftlint, swift-format)",
                "•".cyan()
            );
            println!(
                "  {} objectivec/          - Objective-C configs (clang-format)",
                "•".cyan()
            );
            println!(
                "  {} sql/                 - SQL configs (sqlfluff)",
                "•".cyan()
            );
            println!(
                "  {} csharp/              - C# configs (dotnet-format)",
                "•".cyan()
            );
            println!(
                "  {} lua/                 - Lua configs (luacheck, stylua)",
                "•".cyan()
            );
            println!(
                "  {} css/                 - CSS configs (stylelint, prettier)",
                "•".cyan()
            );
            println!(
                "  {} kotlin/              - Kotlin configs (detekt)",
                "•".cyan()
            );
            println!(
                "  {} dockerfile/          - Dockerfile configs (hadolint)",
                "•".cyan()
            );
            println!(
                "  {} scala/               - Scala configs (scalafmt, scalafix)",
                "•".cyan()
            );
            println!(
                "  {} dart/                - Dart configs (dart analyzer)",
                "•".cyan()
            );
            println!();
            println!("Next steps:");
            println!("  1. Review and customize the config files for your needs");
            println!("  2. Edit {}/linthis-plugin.toml with your details", name);
            println!("  3. Remove any languages you don't need");
            println!("  4. Push to a Git repository:");
            println!();
            println!("     git init && git add . && git commit -m \"Initial commit\"");
            println!(
                "     git remote add origin git@github.com:your-org/{}.git",
                name
            );
            println!("     git push -u origin main");

            ExitCode::SUCCESS
        }

        PluginCommands::List {
            verbose,
            global,
            cached,
        } => {
            // List cached (downloaded) plugins
            if cached {
                use linthis::plugin::cache::format_size;

                let cache = match PluginCache::new() {
                    Ok(c) => c,
                    Err(e) => {
                        eprintln!("{}: {}", "Error".red(), e);
                        return ExitCode::from(1);
                    }
                };

                match cache.list_cached() {
                    Ok(plugins) => {
                        if plugins.is_empty() {
                            println!("No cached plugins found.");
                            println!("\nCache: {}", cache.cache_dir().display());
                            return ExitCode::SUCCESS;
                        }

                        println!("{}", "Cached plugins:".bold());
                        for plugin in &plugins {
                            if verbose {
                                println!(
                                    "  {} {} ({})",
                                    "•".cyan(),
                                    plugin.name.bold(),
                                    plugin.url
                                );
                                println!("    Path: {}", plugin.cache_path.display());
                                println!("    Cached: {}", plugin.cached_at.format("%Y-%m-%d %H:%M"));
                                println!(
                                    "    Updated: {}",
                                    plugin.last_updated.format("%Y-%m-%d %H:%M")
                                );
                            } else {
                                println!("  {} {}", "•".cyan(), plugin.name);
                            }
                        }

                        // Show total cache size
                        if let Ok(size) = cache.cache_size() {
                            println!("\nTotal cache size: {}", format_size(size));
                        }
                        println!("Cache: {}", cache.cache_dir().display());
                    }
                    Err(e) => {
                        eprintln!("{}: Failed to list cached plugins: {}", "Error".red(), e);
                        return ExitCode::from(1);
                    }
                }

                return ExitCode::SUCCESS;
            }

            // List configured plugins
            use linthis::plugin::PluginConfigManager;

            let manager = if global {
                match PluginConfigManager::global() {
                    Ok(m) => m,
                    Err(e) => {
                        eprintln!("{}: {}", "Error".red(), e);
                        return ExitCode::from(1);
                    }
                }
            } else {
                match PluginConfigManager::project() {
                    Ok(m) => m,
                    Err(e) => {
                        eprintln!("{}: {}", "Error".red(), e);
                        return ExitCode::from(1);
                    }
                }
            };

            let config_type = if global { "global" } else { "project" };

            match manager.list_plugins() {
                Ok(plugins) => {
                    if plugins.is_empty() {
                        println!("No {} plugins configured.", config_type);
                        println!("\nConfig: {}", manager.config_path().display());
                        return ExitCode::SUCCESS;
                    }

                    println!(
                        "{} ({}):",
                        "Configured plugins".bold(),
                        config_type
                    );
                    for (name, url, git_ref) in &plugins {
                        if verbose {
                            if let Some(r) = git_ref {
                                println!("  {} {} ({}, ref: {})", "•".cyan(), name.bold(), url, r);
                            } else {
                                println!("  {} {} ({})", "•".cyan(), name.bold(), url);
                            }
                        } else {
                            println!("  {} {}", "•".cyan(), name);
                        }
                    }

                    println!("\nConfig: {}", manager.config_path().display());
                }
                Err(e) => {
                    eprintln!("{}: Failed to list plugins: {}", "Error".red(), e);
                    return ExitCode::from(1);
                }
            }

            ExitCode::SUCCESS
        }

        PluginCommands::Clean { all } => {
            let cache = match PluginCache::new() {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("{}: {}", "Error".red(), e);
                    return ExitCode::from(1);
                }
            };

            if all {
                match cache.clear_all() {
                    Ok(_) => {
                        println!("{} Cleared all cached plugins", "✓".green());
                    }
                    Err(e) => {
                        eprintln!("{}: Failed to clear cache: {}", "Error".red(), e);
                        return ExitCode::from(1);
                    }
                }
            } else {
                println!("Use --all to remove all cached plugins");
                println!("Or remove specific plugins manually from:");
                println!("  {}", cache.cache_dir().display());
            }

            ExitCode::SUCCESS
        }

        PluginCommands::Sync { global } => {
            use linthis::plugin::{fetcher::PluginFetcher, PluginConfigManager, PluginSource};

            let manager = if global {
                match PluginConfigManager::global() {
                    Ok(m) => m,
                    Err(e) => {
                        eprintln!("{}: {}", "Error".red(), e);
                        return ExitCode::from(1);
                    }
                }
            } else {
                match PluginConfigManager::project() {
                    Ok(m) => m,
                    Err(e) => {
                        eprintln!("{}: {}", "Error".red(), e);
                        return ExitCode::from(1);
                    }
                }
            };

            let config_type = if global { "global" } else { "project" };

            let plugins = match manager.list_plugins() {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("{}: Failed to read config: {}", "Error".red(), e);
                    return ExitCode::from(1);
                }
            };

            if plugins.is_empty() {
                println!("No {} plugins configured to sync.", config_type);
                println!("\nConfig: {}", manager.config_path().display());
                return ExitCode::SUCCESS;
            }

            println!(
                "{} {} plugin(s) from {} config...\n",
                "Syncing".cyan(),
                plugins.len(),
                config_type
            );

            let cache = match PluginCache::new() {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("{}: {}", "Error".red(), e);
                    return ExitCode::from(1);
                }
            };

            let fetcher = PluginFetcher::new();
            let mut success_count = 0;
            let mut fail_count = 0;
            let mut updated_count = 0;

            for (name, url, git_ref) in &plugins {
                let source = PluginSource {
                    name: name.clone(),
                    url: Some(url.clone()),
                    git_ref: git_ref.clone(),
                    enabled: true,
                };

                // Get old hash before sync (if cached)
                let cache_path = cache.url_to_cache_path(url);
                let old_hash = fetcher.get_local_commit_hash(&cache_path);

                print!("  {} {}... ", "↓".cyan(), name);
                // Always sync to latest (force = true)
                match fetcher.fetch(&source, &cache, true) {
                    Ok(cached_plugin) => {
                        let new_hash = cached_plugin.commit_hash.as_ref();
                        let was_updated = match (&old_hash, new_hash) {
                            (Some(old), Some(new)) => old != new,
                            (None, Some(_)) => true, // newly cloned
                            _ => false,
                        };

                        let hash_info = new_hash
                            .map(|h| &h[..7.min(h.len())])
                            .unwrap_or("unknown");

                        if was_updated {
                            if old_hash.is_some() {
                                let old_short = old_hash.as_ref()
                                    .map(|h| &h[..7.min(h.len())])
                                    .unwrap_or("unknown");
                                println!("{} {} -> {}", "✓".green(), old_short, hash_info);
                            } else {
                                println!("{} @ {}", "✓".green(), hash_info);
                            }
                            updated_count += 1;
                        } else {
                            println!("{} @ {} (up to date)", "✓".green(), hash_info);
                        }
                        success_count += 1;
                    }
                    Err(e) => {
                        println!("{}", "✗".red());
                        eprintln!("    Error: {}", e);
                        fail_count += 1;
                    }
                }
            }

            println!();
            if fail_count == 0 {
                if updated_count > 0 {
                    println!(
                        "{} Synced {} plugin(s), {} updated",
                        "✓".green(),
                        success_count,
                        updated_count
                    );
                } else {
                    println!(
                        "{} All {} plugin(s) up to date",
                        "✓".green(),
                        success_count
                    );
                }
            } else {
                println!(
                    "{} Synced {}/{} plugin(s), {} failed",
                    "⚠".yellow(),
                    success_count,
                    plugins.len(),
                    fail_count
                );
            }

            if fail_count > 0 {
                ExitCode::from(1)
            } else {
                ExitCode::SUCCESS
            }
        }

        PluginCommands::Validate { path } => {
            match PluginManifest::load(&path) {
                Ok(manifest) => {
                    // Validate the manifest
                    if let Err(e) = manifest.validate(&path) {
                        eprintln!("{}: {}", "Validation failed".red(), e);
                        return ExitCode::from(1);
                    }

                    println!("{} Plugin '{}' is valid", "✓".green(), manifest.plugin.name);
                    println!("  Version: {}", manifest.plugin.version);
                    println!("  Languages: {}", manifest.plugin.languages.join(", "));
                    println!("  Configs:");
                    for (lang, tools) in &manifest.configs {
                        for (tool, path) in tools {
                            println!("    {}/{}: {}", lang, tool, path);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("{}: {}", "Validation failed".red(), e);
                    return ExitCode::from(1);
                }
            }

            ExitCode::SUCCESS
        }

        PluginCommands::Add {
            alias,
            url,
            git_ref,
            global,
        } => {
            use linthis::plugin::PluginConfigManager;

            let manager = if global {
                match PluginConfigManager::global() {
                    Ok(m) => m,
                    Err(e) => {
                        eprintln!("{}: {}", "Error".red(), e);
                        return ExitCode::from(1);
                    }
                }
            } else {
                match PluginConfigManager::project() {
                    Ok(m) => m,
                    Err(e) => {
                        eprintln!("{}: {}", "Error".red(), e);
                        return ExitCode::from(1);
                    }
                }
            };

            let config_type = if global { "global" } else { "project" };

            match manager.add_plugin(&alias, &url, git_ref.as_deref()) {
                Ok(_) => {
                    println!(
                        "{} Added plugin '{}' to {} configuration",
                        "✓".green(),
                        alias.bold(),
                        config_type
                    );
                    println!();
                    println!("  Alias: {}", alias);
                    println!("  URL:   {}", url);
                    if let Some(ref_) = git_ref {
                        println!("  Ref:   {}", ref_);
                    }
                    println!("  Config: {}", manager.config_path().display());
                    println!();
                    println!("You can now use it with:");
                    println!("  linthis --plugin {}", alias);

                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("{}: {}", "Error".red(), e);
                    ExitCode::from(1)
                }
            }
        }

        PluginCommands::Remove { alias, global } => {
            use linthis::plugin::PluginConfigManager;

            let manager = if global {
                match PluginConfigManager::global() {
                    Ok(m) => m,
                    Err(e) => {
                        eprintln!("{}: {}", "Error".red(), e);
                        return ExitCode::from(1);
                    }
                }
            } else {
                match PluginConfigManager::project() {
                    Ok(m) => m,
                    Err(e) => {
                        eprintln!("{}: {}", "Error".red(), e);
                        return ExitCode::from(1);
                    }
                }
            };

            let config_type = if global { "global" } else { "project" };

            match manager.remove_plugin(&alias) {
                Ok(true) => {
                    println!(
                        "{} Removed plugin '{}' from {} configuration",
                        "✓".green(),
                        alias.bold(),
                        config_type
                    );
                    ExitCode::SUCCESS
                }
                Ok(false) => {
                    eprintln!(
                        "{}: Plugin alias '{}' not found in {} configuration",
                        "Warning".yellow(),
                        alias,
                        config_type
                    );
                    println!();
                    println!("Available plugins in {}:", manager.config_path().display());
                    match manager.list_plugins() {
                        Ok(plugins) => {
                            if plugins.is_empty() {
                                println!("  (none)");
                            } else {
                                for (name, url, ref_) in plugins {
                                    if let Some(r) = ref_ {
                                        println!("  {} {} ({}, ref: {})", "•".cyan(), name, url, r);
                                    } else {
                                        println!("  {} {} ({})", "•".cyan(), name, url);
                                    }
                                }
                            }
                        }
                        Err(_) => {
                            println!("  (unable to list plugins)");
                        }
                    }
                    ExitCode::from(1)
                }
                Err(e) => {
                    eprintln!("{}: {}", "Error".red(), e);
                    ExitCode::from(1)
                }
            }
        }

        PluginCommands::Apply { alias, global, language } => {
            use linthis::plugin::{loader::PluginLoader, PluginConfigManager, PluginSource};

            let manager = if global {
                match PluginConfigManager::global() {
                    Ok(m) => m,
                    Err(e) => {
                        eprintln!("{}: {}", "Error".red(), e);
                        return ExitCode::from(1);
                    }
                }
            } else {
                match PluginConfigManager::project() {
                    Ok(m) => m,
                    Err(e) => {
                        eprintln!("{}: {}", "Error".red(), e);
                        return ExitCode::from(1);
                    }
                }
            };

            let config_type = if global { "global" } else { "project" };

            // Get plugins to apply
            let plugins = match manager.list_plugins() {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("{}: Failed to read config: {}", "Error".red(), e);
                    return ExitCode::from(1);
                }
            };

            // Filter by alias if specified
            let plugins: Vec<_> = if let Some(ref alias_filter) = alias {
                plugins.into_iter().filter(|(name, _, _)| name == alias_filter).collect()
            } else {
                plugins
            };

            if plugins.is_empty() {
                if let Some(ref a) = alias {
                    eprintln!("{}: Plugin '{}' not found in {} config", "Error".red(), a, config_type);
                } else {
                    println!("No plugins configured in {} config.", config_type);
                }
                return ExitCode::from(1);
            }

            let loader = match PluginLoader::new() {
                Ok(l) => l,
                Err(e) => {
                    eprintln!("{}: {}", "Error".red(), e);
                    return ExitCode::from(1);
                }
            };

            let mut applied_count = 0;
            let project_root = std::env::current_dir().unwrap_or_default();

            for (name, url, git_ref) in &plugins {
                let source = PluginSource {
                    name: name.clone(),
                    url: Some(url.clone()),
                    git_ref: git_ref.clone(),
                    enabled: true,
                };

                match loader.load_configs(&[source], false) {
                    Ok(configs) => {
                        // Filter by language if specified
                        let configs: Vec<_> = if let Some(ref langs) = language {
                            configs.into_iter().filter(|c| langs.contains(&c.language)).collect()
                        } else {
                            configs
                        };

                        if configs.is_empty() {
                            continue;
                        }

                        println!("\n{} Applying configs from '{}':", "→".cyan(), name);
                        for config in &configs {
                            if let Some(filename) = config.config_path.file_name() {
                                let target = project_root.join(filename);
                                if target.exists() {
                                    println!(
                                        "  {} {}/{}: {} (skipped, exists)",
                                        "⊘".yellow(),
                                        config.language,
                                        config.tool,
                                        filename.to_string_lossy()
                                    );
                                } else {
                                    match std::fs::copy(&config.config_path, &target) {
                                        Ok(_) => {
                                            println!(
                                                "  {} {}/{}: {}",
                                                "✓".green(),
                                                config.language,
                                                config.tool,
                                                filename.to_string_lossy()
                                            );
                                            applied_count += 1;
                                        }
                                        Err(e) => {
                                            eprintln!(
                                                "  {} {}: {}",
                                                "✗".red(),
                                                filename.to_string_lossy(),
                                                e
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("{}: Failed to load plugin '{}': {}", "Warning".yellow(), name, e);
                    }
                }
            }

            println!();
            if applied_count > 0 {
                println!("{} Applied {} config file(s)", "✓".green(), applied_count);
                println!("\n{}: Add these to .gitignore if you don't want to commit them", "Tip".cyan());
            } else {
                println!("{} No new configs applied (all already exist)", "ℹ".blue());
            }

            ExitCode::SUCCESS
        }
    }
}

fn handle_config_command(action: ConfigCommands) -> ExitCode {
    use linthis::config::cli;

    match action {
        ConfigCommands::Add {
            field,
            value,
            global,
        } => cli::handle_config_add(field.as_str(), &value, global),
        ConfigCommands::Remove {
            field,
            value,
            global,
        } => cli::handle_config_remove(field.as_str(), &value, global),
        ConfigCommands::Clear { field, global } => cli::handle_config_clear(field.as_str(), global),
        ConfigCommands::Set {
            field,
            value,
            global,
        } => cli::handle_config_set(&field, &value, global),
        ConfigCommands::Unset { field, global } => cli::handle_config_unset(&field, global),
        ConfigCommands::Get { field, global } => cli::handle_config_get(&field, global),
        ConfigCommands::List { verbose, global } => cli::handle_config_list(verbose, global),
    }
}

/// Handle init subcommand
fn handle_init_command(global: bool, hook: Option<HookTool>, interactive: bool, force: bool) -> ExitCode {
    use colored::Colorize;
    use linthis::config::Config;

    let config_path = if global {
        // Global config path: ~/.linthis/config.toml
        let home = match dirs::home_dir() {
            Some(h) => h,
            None => {
                eprintln!("{}: Cannot determine home directory", "Error".red());
                return ExitCode::from(1);
            }
        };
        home.join(".linthis").join("config.toml")
    } else {
        // Project config path: .linthis/config.toml in current directory
        Config::project_config_path(&std::env::current_dir().unwrap_or_default())
    };

    if config_path.exists() && !force {
        eprintln!(
            "{}: {} already exists",
            "Warning".yellow(),
            config_path.display()
        );
        return ExitCode::from(1);
    }

    // Create parent directory if needed
    if let Some(parent) = config_path.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            eprintln!(
                "{}: Failed to create directory {}: {}",
                "Error".red(),
                parent.display(),
                e
            );
            return ExitCode::from(2);
        }
    }

    let content = Config::generate_default_toml();
    match std::fs::write(&config_path, content) {
        Ok(_) => {
            println!("{} Created {}", "✓".green(), config_path.display());
        }
        Err(e) => {
            eprintln!("{}: Failed to create config: {}", "Error".red(), e);
            return ExitCode::from(2);
        }
    }

    // Handle hook initialization
    // Warning: hooks only make sense for project-level config
    if global && hook.is_some() {
        eprintln!(
            "{}: Hooks can only be configured at project level, ignoring --hook flag",
            "Warning".yellow()
        );
        return ExitCode::SUCCESS;
    }

    // Determine which hook tool to use
    let hook_tool = if let Some(tool) = hook {
        Some(tool)
    } else if interactive && !global {
        prompt_for_hook_tool()
    } else {
        None
    };

    if let Some(tool) = hook_tool {
        if let Err(exit_code) = create_hook_config(&tool, force) {
            return exit_code;
        }
    }

    ExitCode::SUCCESS
}

/// Prompt user to select a hook tool interactively
fn prompt_for_hook_tool() -> Option<HookTool> {
    use colored::Colorize;
    use std::io::{self, Write};

    print!("\nWould you like to set up pre-commit hooks? (y/n) ");
    io::stdout().flush().ok()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input).ok()?;

    if !input.trim().eq_ignore_ascii_case("y") {
        return None;
    }

    println!("\nChoose a hook manager:");
    println!("  {}. {} (recommended, faster)", "1".cyan(), "prek".bold());
    println!("  {}. {} (standard)", "2".cyan(), "pre-commit".bold());
    println!("  {}. {} (simple)", "3".cyan(), "git hook".bold());
    print!("> ");
    io::stdout().flush().ok()?;

    input.clear();
    io::stdin().read_line(&mut input).ok()?;

    match input.trim() {
        "1" => Some(HookTool::Prek),
        "2" => Some(HookTool::PreCommit),
        "3" => Some(HookTool::Git),
        _ => {
            eprintln!("{}: Invalid choice", "Error".red());
            None
        }
    }
}

/// Check if a command is available in PATH
fn is_command_available(command: &str) -> bool {
    std::process::Command::new("which")
        .arg(command)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Install hooks using the specified tool
fn install_hooks(tool: &HookTool) -> Result<(), String> {
    use std::process::Command;

    let (cmd, tool_name) = match tool {
        HookTool::Prek => ("prek", "prek"),
        HookTool::PreCommit => ("pre-commit", "pre-commit"),
        HookTool::Git => return Ok(()), // Git hooks don't need install step
    };

    let output = Command::new(cmd)
        .arg("install")
        .output()
        .map_err(|e| format!("Failed to execute {} install: {}", tool_name, e))?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("{} install failed: {}", tool_name, stderr))
    }
}

/// Create hook configuration file based on the selected tool
fn create_hook_config(tool: &HookTool, force: bool) -> Result<(), ExitCode> {
    use colored::Colorize;
    use std::fs;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    match tool {
        HookTool::Prek | HookTool::PreCommit => {
            let config_path = std::path::PathBuf::from(".pre-commit-config.yaml");

            if config_path.exists() && !force {
                eprintln!(
                    "{}: {} already exists, skipping",
                    "Warning".yellow(),
                    config_path.display()
                );
                return Ok(());
            }

            let content = r#"repos:
  - repo: local
    hooks:
      - id: linthis
        name: linthis
        entry: linthis --staged --check-only
        language: system
        pass_filenames: false
"#;

            match fs::write(&config_path, content) {
                Ok(_) => {
                    let tool_name = match tool {
                        HookTool::Prek => "prek",
                        HookTool::PreCommit => "pre-commit",
                        _ => unreachable!(),
                    };
                    println!(
                        "{} Created {} ({}/pre-commit compatible)",
                        "✓".green(),
                        config_path.display(),
                        tool_name
                    );

                    // Check if tool is installed and auto-install hooks
                    let cmd_name = tool_name;
                    if is_command_available(cmd_name) {
                        println!("\n{} Detected installed", tool_name.cyan());
                        print!("{} Installing hooks... ", "→".cyan());
                        std::io::Write::flush(&mut std::io::stdout()).ok();

                        match install_hooks(tool) {
                            Ok(_) => {
                                println!("{}", "✓".green());
                                println!("\n{} Pre-commit hooks are ready!", "✓".green().bold());
                                println!("  Hooks will run automatically on {}", "git commit".cyan());
                            }
                            Err(e) => {
                                println!("{}", "✗".red());
                                eprintln!("{}: {}", "Warning".yellow(), e);
                                println!("\nPlease run manually: {}", format!("{} install", tool_name).cyan());
                            }
                        }
                    } else {
                        // Tool not installed, show installation instructions
                        // Both prek and pre-commit can be installed via pip
                        println!("\nNext steps:");
                        if matches!(tool, HookTool::Prek) {
                            println!("  1. Install prek: {}", "pip install prek".cyan());
                            println!("  2. Set up hooks: {}", "prek install".cyan());
                        } else {
                            println!("  1. Install pre-commit: {}", "pip install pre-commit".cyan());
                            println!("  2. Set up hooks: {}", "pre-commit install".cyan());
                        }
                    }
                    Ok(())
                }
                Err(e) => {
                    eprintln!(
                        "{}: Failed to create {}: {}",
                        "Error".red(),
                        config_path.display(),
                        e
                    );
                    Err(ExitCode::from(2))
                }
            }
        }
        HookTool::Git => {
            // Check if in a git repository
            let git_hooks_dir = std::path::PathBuf::from(".git/hooks");
            if !git_hooks_dir.exists() {
                eprintln!(
                    "{}: Not in a git repository, cannot create .git/hooks/pre-commit",
                    "Error".red()
                );
                return Err(ExitCode::from(1));
            }

            let hook_path = git_hooks_dir.join("pre-commit");

            if hook_path.exists() && !force {
                eprintln!(
                    "{}: {} already exists, skipping",
                    "Warning".yellow(),
                    hook_path.display()
                );
                return Ok(());
            }

            let content = r#"#!/bin/sh
linthis --staged --check-only
"#;

            match fs::write(&hook_path, content) {
                Ok(_) => {
                    // Make the hook executable
                    #[cfg(unix)]
                    {
                        let mut perms = fs::metadata(&hook_path)
                            .map_err(|e| {
                                eprintln!("{}: Failed to get file metadata: {}", "Error".red(), e);
                                ExitCode::from(2)
                            })?
                            .permissions();
                        perms.set_mode(0o755);
                        fs::set_permissions(&hook_path, perms).map_err(|e| {
                            eprintln!("{}: Failed to set permissions: {}", "Error".red(), e);
                            ExitCode::from(2)
                        })?;
                    }

                    println!("{} Created {}", "✓".green(), hook_path.display());
                    println!("\nNext steps:");
                    println!("  Make sure the hook is executable:");
                    println!("    {}", "chmod +x .git/hooks/pre-commit".cyan());
                    Ok(())
                }
                Err(e) => {
                    eprintln!(
                        "{}: Failed to create {}: {}",
                        "Error".red(),
                        hook_path.display(),
                        e
                    );
                    Err(ExitCode::from(2))
                }
            }
        }
    }
}

mod dirs {
    use std::path::PathBuf;

    pub fn home_dir() -> Option<PathBuf> {
        std::env::var("HOME")
            .ok()
            .map(PathBuf::from)
            .or_else(|| std::env::var("USERPROFILE").ok().map(PathBuf::from))
    }
}

/// Default config file contents for each linter/formatter
fn get_default_configs() -> Vec<(&'static str, &'static str)> {
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

/// Initialize default config files for all linters/formatters
fn init_linter_configs() -> ExitCode {
    use std::fs;
    use std::path::Path;

    let configs = get_default_configs();
    let mut created = 0;
    let mut skipped = 0;

    println!(
        "{}",
        "Generating default linter/formatter configs...".cyan()
    );

    for (filename, content) in configs {
        let path = Path::new(filename);
        if path.exists() {
            println!("  {} {} (already exists)", "⊘".yellow(), filename);
            skipped += 1;
        } else {
            match fs::write(path, content) {
                Ok(_) => {
                    println!("  {} {}", "✓".green(), filename);
                    created += 1;
                }
                Err(e) => {
                    eprintln!("  {} {} ({})", "✗".red(), filename, e);
                }
            }
        }
    }

    println!();
    println!(
        "Created {} config file{}, skipped {} existing",
        created,
        if created == 1 { "" } else { "s" },
        skipped
    );

    ExitCode::SUCCESS
}

/// Run benchmark comparing ruff vs flake8+black for Python
fn run_benchmark(cli: &Cli) -> ExitCode {
    use linthis::benchmark::{format_benchmark_table, run_python_benchmark};
    use linthis::utils::walker::{walk_paths, WalkerConfig};

    println!(
        "{}",
        "Running Python linting/formatting benchmark...".cyan()
    );
    println!("Comparing ruff vs flake8+black\n");

    // Get paths to scan (default to current directory if empty)
    let paths = if cli.paths.is_empty() {
        vec![PathBuf::from(".")]
    } else {
        cli.paths.clone()
    };

    // Configure walker for Python files only
    let walker_config = WalkerConfig {
        exclude_patterns: cli.exclude.clone().unwrap_or_default(),
        languages: vec![Language::Python],
        ..Default::default()
    };

    // Collect Python files
    let files = walk_paths(&paths, &walker_config);

    if files.is_empty() {
        println!("{}", "No Python files found to benchmark.".yellow());
        return ExitCode::SUCCESS;
    }

    println!("Found {} Python files", files.len());

    // Convert to Path references
    let file_refs: Vec<&std::path::Path> = files.iter().map(|p| p.as_path()).collect();

    // Run benchmark
    let comparison = run_python_benchmark(&file_refs);

    // Output results
    println!("{}", format_benchmark_table(&comparison));

    ExitCode::SUCCESS
}

/// Strip ANSI escape codes from a string for plain text output
fn strip_ansi_codes(s: &str) -> String {
    let ansi_regex = regex::Regex::new(r"\x1b\[[0-9;]*m").unwrap();
    ansi_regex.replace_all(s, "").to_string()
}

fn main() -> ExitCode {
    env_logger::init();

    let cli = Cli::parse();

    // Handle plugin subcommands first
    if let Some(Commands::Plugin { action }) = cli.command {
        return handle_plugin_command(action);
    }

    // Handle config subcommands
    if let Some(Commands::Config { action }) = cli.command {
        return handle_config_command(action);
    }

    // Handle init subcommand
    if let Some(Commands::Init { global, hook, interactive, force }) = cli.command {
        return handle_init_command(global, hook, interactive, force);
    }

    // Track loaded plugins for display
    let mut loaded_plugins: Vec<String> = Vec::new();

    // Load plugins: from --plugin flag, or from config files (project first, then global)
    {
        use linthis::plugin::{PluginConfigManager, PluginLoader, PluginSource};

        let plugins_to_load: Vec<(String, PluginSource)> = if let Some(ref plugin_name) = cli.plugin {
            // Use explicitly specified plugin
            vec![(plugin_name.clone(), PluginSource::new(plugin_name))]
        } else {
            // Try to load from config files: project first, then global
            let mut plugins = Vec::new();

            // Check project config first
            if let Ok(project_manager) = PluginConfigManager::project() {
                if let Ok(project_plugins) = project_manager.list_plugins() {
                    for (name, url, git_ref) in project_plugins {
                        let source = if let Some(ref r) = git_ref {
                            PluginSource::new(&url).with_ref(r)
                        } else {
                            PluginSource::new(&url)
                        };
                        plugins.push((name, source));
                    }
                }
            }

            // If no project plugins, check global config
            if plugins.is_empty() {
                if let Ok(global_manager) = PluginConfigManager::global() {
                    if let Ok(global_plugins) = global_manager.list_plugins() {
                        for (name, url, git_ref) in global_plugins {
                            let source = if let Some(ref r) = git_ref {
                                PluginSource::new(&url).with_ref(r)
                            } else {
                                PluginSource::new(&url)
                            };
                            plugins.push((name, source));
                        }
                    }
                }
            }

            plugins
        };

        if !plugins_to_load.is_empty() {
            let loader = match PluginLoader::with_verbose(cli.verbose) {
                Ok(l) => l,
                Err(e) => {
                    eprintln!(
                        "{}: Failed to initialize plugin loader: {}",
                        "Error".red(),
                        e
                    );
                    return ExitCode::from(1);
                }
            };

            for (plugin_name, source) in plugins_to_load {
                match loader.load_configs(&[source], cli.plugin_update) {
                    Ok(configs) => {
                        loaded_plugins.push(plugin_name.clone());
                        if cli.verbose {
                            eprintln!(
                                "Loaded {} config(s) from plugin '{}'",
                                configs.len(),
                                plugin_name
                            );
                        }
                        // Auto-apply plugin configs to .linthis/configs/{language}/
                        // Each language gets its own subdirectory to avoid conflicts
                        // (e.g., cpp/.clang-format vs oc/.clang-format)
                        let linthis_dir = std::env::current_dir()
                            .unwrap_or_default()
                            .join(".linthis");
                        let config_dir = linthis_dir.join("configs");

                        for config in &configs {
                            if let Some(filename) = config.config_path.file_name() {
                                // Create language-specific subdirectory
                                let lang_dir = config_dir.join(&config.language);
                                if std::fs::create_dir_all(&lang_dir).is_ok() {
                                    let target = lang_dir.join(filename);
                                    // Always update to latest plugin config
                                    if std::fs::copy(&config.config_path, &target).is_ok() {
                                        if cli.verbose {
                                            eprintln!(
                                                "  - {}/{}: {} -> .linthis/configs/{}/{}",
                                                config.language,
                                                config.tool,
                                                filename.to_string_lossy(),
                                                config.language,
                                                filename.to_string_lossy()
                                            );
                                        }
                                    }
                                }
                            }
                        }

                        // NOTE: We no longer create symlinks for CPPLINT.cfg in project root.
                        // linthis now passes cpplint config via command line args (--linelength, --filter)
                        // which allows per-language (cpp vs oc) configuration.
                        // Root symlinks would override this with a single cpp config for all files.
                    }
                    Err(e) => {
                        eprintln!(
                            "{}: Failed to load plugin '{}': {}",
                            "Warning".yellow(),
                            plugin_name,
                            e
                        );
                        // Continue with defaults - don't fail the entire run
                    }
                }
            }
        }
    }

    // Handle --plugin-update without --plugin (update all cached)
    if cli.plugin_update && cli.plugin.is_none() {
        use linthis::plugin::cache::PluginCache;
        use linthis::plugin::{PluginLoader, PluginSource};

        let cache = match PluginCache::new() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("{}: {}", "Error".red(), e);
                return ExitCode::from(1);
            }
        };

        match cache.list_cached() {
            Ok(plugins) => {
                if plugins.is_empty() {
                    println!("No cached plugins to update.");
                } else {
                    let loader = match PluginLoader::with_verbose(cli.verbose) {
                        Ok(l) => l,
                        Err(e) => {
                            eprintln!("{}: {}", "Error".red(), e);
                            return ExitCode::from(1);
                        }
                    };

                    for plugin in plugins {
                        let source = PluginSource {
                            name: plugin.name.clone(),
                            url: Some(plugin.url.clone()),
                            git_ref: plugin.git_ref.clone(),
                            enabled: true,
                        };

                        match loader.load_configs(&[source], true) {
                            Ok(_) => {
                                println!("{} Updated {}", "✓".green(), plugin.name);
                            }
                            Err(e) => {
                                eprintln!("{} Failed to update {}: {}", "✗".red(), plugin.name, e);
                            }
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("{}: Failed to list plugins: {}", "Error".red(), e);
                return ExitCode::from(1);
            }
        }

        return ExitCode::SUCCESS;
    }

    // Handle --init flag
    if cli.init {
        let config_path = linthis::config::Config::project_config_path(
            &std::env::current_dir().unwrap_or_default(),
        );
        if config_path.exists() {
            eprintln!(
                "{}: {} already exists",
                "Warning".yellow(),
                config_path.display()
            );
            return ExitCode::from(1);
        }

        let content = linthis::config::Config::generate_default_toml();
        match std::fs::write(&config_path, content) {
            Ok(_) => {
                println!("{} Created {}", "✓".green(), config_path.display());
                return ExitCode::SUCCESS;
            }
            Err(e) => {
                eprintln!("{}: Failed to create config: {}", "Error".red(), e);
                return ExitCode::from(2);
            }
        }
    }

    // Handle --init-configs flag
    if cli.init_configs {
        return init_linter_configs();
    }

    // Handle --benchmark flag
    if cli.benchmark {
        return run_benchmark(&cli);
    }

    // Determine run mode
    let mode = if cli.check_only {
        RunMode::CheckOnly
    } else if cli.format_only {
        RunMode::FormatOnly
    } else {
        RunMode::Both
    };

    // Parse languages
    let languages: Vec<Language> = cli
        .lang
        .unwrap_or_default()
        .iter()
        .filter_map(|s| Language::from_name(s))
        .collect();

    // Get paths (handle staged files)
    let paths = if cli.staged {
        match linthis::utils::get_staged_files() {
            Ok(files) => {
                if files.is_empty() {
                    if !cli.quiet {
                        println!("{}", "No staged files to check".yellow());
                    }
                    return ExitCode::SUCCESS;
                }
                files
            }
            Err(e) => {
                eprintln!("{}: {}", "Error getting staged files".red(), e);
                return ExitCode::from(2);
            }
        }
    } else if cli.paths.is_empty() {
        // Default to current directory if no paths specified
        vec![PathBuf::from(".")]
    } else {
        cli.paths
    };

    // Build exclusion patterns (defaults + gitignore + user-specified)
    let mut exclude_patterns: Vec<String> = if cli.no_default_excludes {
        Vec::new()
    } else {
        linthis::utils::DEFAULT_EXCLUDES
            .iter()
            .map(|s| s.to_string())
            .collect()
    };

    // Add .gitignore patterns if in a git repo and not disabled
    if !cli.no_gitignore && linthis::utils::is_git_repo() {
        let project_root = linthis::utils::get_project_root();
        let gitignore_patterns = linthis::utils::get_gitignore_patterns(&project_root);
        if cli.verbose && !gitignore_patterns.is_empty() {
            eprintln!(
                "Loaded {} patterns from .gitignore",
                gitignore_patterns.len()
            );
        }
        exclude_patterns.extend(gitignore_patterns);
    }

    exclude_patterns.extend(cli.exclude.unwrap_or_default());

    // Add excludes from project config file
    let project_root = linthis::utils::get_project_root();
    if let Some(project_config) = linthis::config::Config::load_project_config(&project_root) {
        if !project_config.excludes.is_empty() {
            if cli.verbose {
                eprintln!(
                    "Loaded {} exclude patterns from config",
                    project_config.excludes.len()
                );
            }
            exclude_patterns.extend(project_config.excludes);
        }
    }

    // Build options
    let options = RunOptions {
        paths,
        mode,
        languages,
        exclude_patterns,
        verbose: cli.verbose,
        quiet: cli.quiet,
        plugins: loaded_plugins,
    };

    // Parse output format
    let output_format = OutputFormat::parse(&cli.output).unwrap_or(OutputFormat::Human);

    if cli.verbose {
        eprintln!(
            "{}",
            "linthis - Multi-language Linter & Formatter".bold().cyan()
        );
        eprintln!("Mode: {:?}", mode);
        eprintln!("Paths: {:?}", options.paths);
    }

    // Run linthis
    match run(&options) {
        Ok(result) => {
            // Output results
            let output = format_result(&result, output_format);

            // Print to console
            if !cli.quiet || result.exit_code != 0 {
                if !output.is_empty() {
                    println!("{}", output);
                }
            }

            // Save to file by default (unless --no-save-result is specified)
            if !cli.no_save_result || cli.output_file.is_some() {
                use chrono::Local;
                use std::fs::{self, File};
                use std::io::Write;

                // Determine actual output path
                let output_file = if let Some(ref custom_path) = cli.output_file {
                    // Use specified path, create parent directory if needed
                    if let Some(parent) = custom_path.parent() {
                        if !parent.as_os_str().is_empty() {
                            let _ = fs::create_dir_all(parent);
                        }
                    }
                    custom_path.clone()
                } else {
                    // Use default path: .linthis/result/result-{timestamp}.txt
                    let result_dir = PathBuf::from(".linthis").join("result");
                    if let Err(e) = fs::create_dir_all(&result_dir) {
                        eprintln!(
                            "{}: Failed to create {}: {}",
                            "Warning".yellow(),
                            result_dir.display(),
                            e
                        );
                        return ExitCode::from(result.exit_code as u8);
                    }
                    let timestamp = Local::now().format("%Y%m%d-%H%M%S");
                    result_dir.join(format!("result-{}.txt", timestamp))
                };

                // Strip ANSI color codes for file output
                let plain_output = strip_ansi_codes(&output);

                match File::create(&output_file) {
                    Ok(mut file) => {
                        if let Err(e) = writeln!(file, "{}", plain_output) {
                            eprintln!(
                                "{}: Failed to write to {}: {}",
                                "Warning".yellow(),
                                output_file.display(),
                                e
                            );
                        } else if !cli.quiet {
                            eprintln!(
                                "{} Results saved to {}",
                                "✓".green(),
                                output_file.display()
                            );
                        }
                    }
                    Err(e) => {
                        eprintln!(
                            "{}: Failed to create {}: {}",
                            "Warning".yellow(),
                            output_file.display(),
                            e
                        );
                    }
                }

                // Clean up old result files if using default directory and keep_results > 0
                if !cli.no_save_result && cli.output_file.is_none() && cli.keep_results > 0 {
                    let result_dir = PathBuf::from(".linthis").join("result");
                    if let Ok(entries) = fs::read_dir(&result_dir) {
                        let mut result_files: Vec<_> = entries
                            .filter_map(|e| e.ok())
                            .filter(|e| {
                                e.file_name()
                                    .to_string_lossy()
                                    .starts_with("result-")
                                    && e.path().extension().map_or(false, |ext| ext == "txt")
                            })
                            .collect();

                        // Sort by modification time, newest first
                        result_files.sort_by(|a, b| {
                            let a_time = a.metadata().and_then(|m| m.modified()).ok();
                            let b_time = b.metadata().and_then(|m| m.modified()).ok();
                            b_time.cmp(&a_time)
                        });

                        // Remove files beyond keep_results limit
                        let files_to_remove = result_files.iter().skip(cli.keep_results);
                        let mut removed_count = 0;
                        for entry in files_to_remove {
                            if fs::remove_file(entry.path()).is_ok() {
                                removed_count += 1;
                            }
                        }
                        if removed_count > 0 && cli.verbose {
                            eprintln!(
                                "{} Cleaned up {} old result file(s)",
                                "✓".green(),
                                removed_count
                            );
                        }
                    }
                }
            }

            ExitCode::from(result.exit_code as u8)
        }
        Err(e) => {
            eprintln!("{}: {}", "Error".red().bold(), e);
            ExitCode::from(2)
        }
    }
}
