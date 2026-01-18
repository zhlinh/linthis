// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Plugin template configuration files for various programming languages.
//!
//! This module generates template configuration files for linting and formatting tools
//! across multiple programming languages including Rust, Python, TypeScript, Go, Java,
//! C++, Swift, Objective-C, SQL, C#, Lua, CSS, Kotlin, Dockerfile, Scala, and Dart.

/// Generate template config files for a new plugin
///
/// Returns a vector of (path, content) tuples for all supported language configurations.
pub fn get_plugin_template_configs(name: &str) -> Vec<(&'static str, String)> {
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
