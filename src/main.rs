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
    /// Files or directories to check (can be specified multiple times)
    /// Examples: -p src -p lib, --path ./plugin
    #[arg(short = 'p', long = "path")]
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

    /// Initialize a new .linthis.toml configuration file
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
    /// Examples: --plugin official, --plugin https://github.com/org/config.git
    #[arg(long)]
    plugin: Option<String>,

    /// Force update cached plugins
    #[arg(long)]
    plugin_update: bool,

    /// Plugin subcommands (init, list, clean)
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(clap::Subcommand, Debug)]
enum Commands {
    /// Plugin management commands
    Plugin {
        #[command(subcommand)]
        action: PluginCommands,
    },
}

#[derive(clap::Subcommand, Debug)]
enum PluginCommands {
    /// Initialize a new plugin
    Init {
        /// Plugin name
        name: String,
    },
    /// List cached plugins
    List {
        /// Show detailed information
        #[arg(short, long)]
        verbose: bool,
    },
    /// Clean cached plugins
    Clean {
        /// Remove all cached plugins
        #[arg(long)]
        all: bool,
    },
    /// Validate a plugin manifest
    Validate {
        /// Path to plugin directory
        path: PathBuf,
    },
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
languages = ["rust", "python", "typescript", "go", "java", "cpp"]

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

## Usage

### Via Command Line

```bash
# Use this plugin for a single run
linthis --plugin https://github.com/your-org/{name}.git
```

### Via Configuration File

Add to your `.linthis.toml`:

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
2. **Local overrides**: Settings in your project's `.linthis.toml` override plugin settings
3. **Fork and modify**: Fork this repository and customize the configs

## Configuration Priority

Settings are applied in this order (later overrides earlier):

1. Built-in defaults
2. Plugin configs (in order listed in `sources`)
3. User config (`~/.linthis/config.toml`)
4. Project config (`.linthis.toml`)
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
        cache::{format_size, PluginCache},
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
            let dirs = ["rust", "python", "typescript", "go", "java", "cpp"];
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

        PluginCommands::List { verbose } => {
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
                        return ExitCode::SUCCESS;
                    }

                    println!("{}", "Cached plugins:".bold());
                    for plugin in &plugins {
                        if verbose {
                            println!("  {} {} ({})", "•".cyan(), plugin.name.bold(), plugin.url);
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

fn main() -> ExitCode {
    env_logger::init();

    let cli = Cli::parse();

    // Handle plugin subcommands first
    if let Some(Commands::Plugin { action }) = cli.command {
        return handle_plugin_command(action);
    }

    // Handle --plugin flag (fetch/use plugin)
    if let Some(ref plugin_name) = cli.plugin {
        use linthis::plugin::{PluginLoader, PluginSource};

        let source = PluginSource::new(plugin_name);
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

        match loader.load_configs(&[source], cli.plugin_update) {
            Ok(configs) => {
                if cli.verbose {
                    eprintln!(
                        "Loaded {} config(s) from plugin '{}'",
                        configs.len(),
                        plugin_name
                    );
                    for config in &configs {
                        eprintln!(
                            "  - {}/{}: {}",
                            config.language,
                            config.tool,
                            config.config_path.display()
                        );
                    }
                }
                // Note: Actual config application would integrate with the linter/formatter runners
                // For now, we just log the loaded configs
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

    // Build options
    let options = RunOptions {
        paths,
        mode,
        languages,
        exclude_patterns,
        verbose: cli.verbose,
        quiet: cli.quiet,
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
            if !cli.quiet || result.exit_code != 0 {
                let output = format_result(&result, output_format);
                if !output.is_empty() {
                    println!("{}", output);
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
