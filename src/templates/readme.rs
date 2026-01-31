// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Plugin manifest and README generation.
//!
//! This module provides functions for generating plugin manifest files
//! (linthis-plugin.toml) and README documentation for linthis plugins.

/// Generate plugin manifest with all config mappings
pub fn generate_plugin_manifest(name: &str) -> String {
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

/// Generate plugin manifest filtered by selected languages
pub fn generate_plugin_manifest_filtered(name: &str, languages: &[&str]) -> String {
    let mut configs = String::new();

    // Language config templates
    let lang_configs: &[(&str, &str)] = &[
        ("rust", r#"[configs.rust]
# Clippy linter configuration
clippy = "rust/clippy.toml"
# Rustfmt formatter configuration
rustfmt = "rust/rustfmt.toml"
"#),
        ("python", r#"[configs.python]
# Ruff linter and formatter configuration
ruff = "python/ruff.toml"
"#),
        ("typescript", r#"[configs.typescript]
# ESLint linter configuration (also works for JavaScript)
eslint = "typescript/.eslintrc.json"
# Prettier formatter configuration
prettier = "typescript/.prettierrc"
"#),
        ("go", r#"[configs.go]
# golangci-lint configuration
golangci-lint = "go/.golangci.yml"
"#),
        ("java", r#"[configs.java]
# Checkstyle configuration
checkstyle = "java/checkstyle.xml"
"#),
        ("cpp", r#"[configs.cpp]
# Clang-Format configuration
clang-format = "cpp/.clang-format"
# CPPLint configuration
cpplint = "cpp/CPPLINT.cfg"
"#),
        ("swift", r#"[configs.swift]
# SwiftLint linter configuration
swiftlint = "swift/.swiftlint.yml"
"#),
        ("oc", r#"[configs.oc]
# Clang-Format configuration
clang-format = "oc/.clang-format"
"#),
        ("sql", r#"[configs.sql]
# SQLFluff linter and formatter configuration
sqlfluff = "sql/.sqlfluff"
"#),
        ("csharp", r#"[configs.csharp]
# dotnet-format configuration via .editorconfig
editorconfig = "csharp/.editorconfig"
"#),
        ("lua", r#"[configs.lua]
# Luacheck linter configuration
luacheck = "lua/.luacheckrc"
# StyLua formatter configuration
stylua = "lua/stylua.toml"
"#),
        ("css", r#"[configs.css]
# Stylelint linter configuration
stylelint = "css/.stylelintrc.json"
# Prettier formatter configuration
prettier = "css/.prettierrc"
"#),
        ("kotlin", r#"[configs.kotlin]
# EditorConfig for Kotlin
editorconfig = "kotlin/.editorconfig"
# Detekt linter configuration
detekt = "kotlin/detekt.yml"
"#),
        ("dockerfile", r#"[configs.dockerfile]
# Hadolint linter configuration
hadolint = "dockerfile/.hadolint.yaml"
"#),
        ("scala", r#"[configs.scala]
# Scalafmt formatter configuration
scalafmt = "scala/.scalafmt.conf"
"#),
        ("dart", r#"[configs.dart]
# Dart analyzer configuration
analyzer = "dart/analysis_options.yaml"
"#),
    ];

    // Add only selected languages
    for (lang, config) in lang_configs {
        if languages.contains(lang) {
            configs.push_str(config);
            configs.push('\n');
        }
    }

    // Build languages list for manifest
    let lang_list: Vec<_> = languages.iter().map(|l| format!("\"{}\"", l)).collect();

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

# Supported languages
languages = [{languages}]

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

{configs}"#,
        name = name,
        languages = lang_list.join(", "),
        configs = configs.trim_end()
    )
}

/// Generate README for a new plugin
pub fn generate_plugin_readme(name: &str) -> String {
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
