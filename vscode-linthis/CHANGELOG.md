# Change Log

All notable changes to the "linthis" extension will be documented in this file.

## [0.2.0] - 2026-01-31

### Added
- `linthis.usePlugin` setting for direct plugin specification (Git URL, local path, or multiple plugins)
- `linthis.executable.path` setting for custom linthis executable path
- `linthis.executable.additionalArguments` setting for extra CLI arguments
- `linthis.lintOnOpen` setting to lint documents when opening files

### Fixed
- TypeScript compilation errors with missing @types/glob dependency
- Replaced star activation with specific language events for better performance

## [0.1.0] - 2025-01-18

### Added
- Improved format/lint on save functionality
- Comprehensive documentation and publishing guide

## [0.0.1] - Initial Release

- Initial release
- LSP client for linthis language server
- Support for 18+ programming languages
- Commands: lint, format, restart
- Configuration options for linting and formatting
