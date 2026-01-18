# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Shell/Bash language support (ShellCheck + shfmt)
- Ruby language support (RuboCop)
- PHP language support (phpcs + php-cs-fixer)
- Scala language support (scalafix + scalafmt)
- C# language support (dotnet format)
- Watch mode for continuous file monitoring
- MkDocs documentation with Material theme

### Changed

- Improved error messages for missing tools

### Fixed

- Fixed file pattern matching on Windows

## [0.0.10] - 2024-XX-XX

### Added

- Lua language support (luacheck + stylua)
- Dart language support (dart analyze + dart format)
- Plugin auto-sync feature
- Self-update functionality
- Configuration migration from ESLint, Prettier, Black

### Changed

- Improved parallel processing performance
- Enhanced plugin caching mechanism

### Fixed

- Fixed gitignore pattern handling
- Fixed staged file detection

## [0.0.1] - 2024-XX-XX

### Added

- Initial release
- Support for Rust, Python, TypeScript, JavaScript, Go, Java, C++, Swift, Kotlin, Objective-C
- Plugin system
- Git hooks integration
- Format presets (Google, Airbnb, Standard)
- Configuration management CLI
- JSON and GitHub Actions output formats
