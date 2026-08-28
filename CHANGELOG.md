# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.28.3] - 2026-08-28

### Fixed

- keep a project's enable state out of the repository
- leave hand-written hooks alone instead of gutting them
- share hook files instead of taking them over

## [0.28.2] - 2026-08-24

### Added

- stop reporting flat dispatch as too complex
- tell the reader how to ignore each issue

### Fixed

- do not read function declarations out of comments
- drop results written by a different linthis version
- stop counting braces inside strings and comments

### Changed

- split activate, and format the languages we can format
- split the remaining functions over the complexity threshold
- give the diff parser an accumulator
- collapse the duplicated per-language branches

## [0.28.1] - 2026-08-21

### Added

- refresh plugins and hooks after an upgrade

### Fixed

- never let sync overwrite a hook linthis does not own
- let `hook sync` see and preserve config-owned hooks
- honor `linthis disable` for hooks that call linthis directly

## [0.28.0] - 2026-08-20

### Added

- add enable/disable with TTL and a status overview

### Fixed

- ignore git submodules when collecting files
- surface formatter failures in blocked hook box and report
- warn when config.toml exists but fails to parse
- exclude .linthis and build output from SAST scanning

### Fixed

- exclude `.linthis/`, VCS metadata, dependencies and build output from SAST file
  discovery, so `.linthis/secrets.toml` is no longer matched by its own patterns
- warn instead of staying silent when a `config.toml` exists but fails to parse;
  one bad value previously voided the whole file with no indication

## [0.27.0] - 2026-07-23

### Fixed

- bridge git-with-agent pre-commit to the repo's own hook
- bridge global pre-push to the repo's own local hook
- honor LINTHIS_INSTALL_MODE env var in install-mode resolution

## [0.26.1] - 2026-07-03

### Added

- extend inline linthis:ignore to all SAST tools

### Fixed

- tolerate shell-wrapper log noise in URLs and git output
- trust untrusted Homebrew tap and retry upgrade

## [0.26.0] - 2026-06-25

### Added

- show blocked end-hint in red and add --no-verify skip tip

### Fixed

- add polling backend so file-change test is deterministic
- use three-dot diff for pre-push file selection
- match directory globs for nested and absolute paths

## [0.25.2] - 2026-05-22

### Fixed

- update HookCommands::Install → Add in cli_tests

## [0.25.1] - 2026-05-18

### Changed

- rename install/uninstall to add/remove

### Documentation

- add .linthisignore feature documentation (en + zh)

## [0.25.0] - 2026-05-18

### Added

- add .linthisignore file support and linthis ignore command

## [0.24.0] - 2026-05-15

### Fixed

- skip missing staged files in git add instead of fatal error

## [0.23.0] - 2026-05-08

### Fixed

- post-commit gates strictly on sentinel; sentinel writes are loud

## [0.22.1] - 2026-04-29

### Fixed

- sanitize multi-line URL input from shell-wrapped git output

## [0.22.0] - 2026-04-28

### Added

- macOS bash_profile shim sources .bashrc for login shells
- status surfaces unmanaged source line in rc
- wire add/remove/status/init/completion handlers
- clap_complete-based completion-script generator
- marker-block rc-file edits with atomic write
- render state to per-shell source-file content
- add shell detection from $SHELL and --shell flag
- improve StateError context and document atomic save invariant
- add ShellState type and TOML round-trip
- scaffold linthis shell subcommand tree

### Fixed

- cargo fmt + restore stop-hook check at project scope
- status surfaces global agent skills in dedicated subsection
- surface non-NotFound I/O errors in bash_profile shim writer
- handle BrokenPipe gracefully in completion stdout
- document rollback scope, surface rollback failures, symmetric messages
- tighten rc.rs comment, replace loop, quote fish path
- strip leading whitespace from generated header

### Changed

- consolidate 7 duplicate home_dir helpers into utils::home_dir
- self-contained status hint, dedupe detection logic
- tighten state.rs visibility and lint allows

## [0.21.0] - 2026-04-27

### Added

- pre-push squash/fixup sentinel fast-path skips redundant round-2 fix+review
- pre-push squash/fixup isolates agent patch from user WIP
- pre-push agent-fix-then-review flow + worktree-aware review dir
- isolate pre-push lint check in temporary git worktree

## [0.20.0] - 2026-04-21

### Added

- show provider/model and split hook footer into aligned lines

### Fixed

- always include timer/paint preamble in --type git scripts
- tint content-row `│` to match the enclosing box colour
- paint sub-linthis output white for IDE VCS consoles
- wrap post-commit informational output for IDE VCS consoles
- route the "📄 Config" header to stdout
- redirect linthis/agent stderr to stdout in hook scripts
- route post-commit informational output to stdout
- keep restage heredoc terminator at column 0
- scope post-commit fixup staging to the committed files only

### Changed

- simplify LINTHIS_HOOK_COLOR values to auto|off|white

## [0.19.6] - 2026-04-20

### Added

- populate TOOL_INSTALLS table for all 16 tools
- introduce tools::install module with ToolInstallSpec scaffolding
- improve tool auto-install with multi-method fallback chains
- add Homebrew install method detection and update support
- show fix_commit_mode switch tip after fixup commits

### Fixed

- address final-review issues in tool-install matrix
- change missing tool install prompt default from N to Y

### Changed

- delegate lib.rs install helpers to tools::install

### Documentation

- format README table alignment and update Quick Start install command

## [0.19.5] - 2026-04-17

### Added

- support custom hook providers via [ai.custom_providers] config
- add --provider-args to linthis fix
- add --model flag to hook install
- add git-with-agent post-commit fixup support

## [0.19.4] - 2026-04-16

### Fixed

- add color with stream display

## [0.19.3] - 2026-04-15

### Added

- stream agent output with better format
- stream agent output with LINTHIS_AGENT_MAX_AUTO_FIX cap
- selectively skip hooks via LINTHIS_SKIP and LINTHIS_SKIP_CHECKS
- publish to Homebrew tap on release
- :LinthisRestore tries multiple recovery sources
- add --all-events/--all-types/--all to linthis hook install

### Fixed

- handle multibyte chars in placeholder pattern check
- format-on-save no longer blanks files on :x / :wq
- isolate cache tests via HOME env var to avoid interference

## [0.19.2] - 2026-04-13

### Fixed

- detect empty pre-push via stdin, not rev-list @{u}

### Documentation

- update README and feature docs with recent changes

## [0.19.1] - 2026-04-11

### Fixed

- remove stale CACHE_DIR reference in test

## [0.19.0] - 2026-04-11

### Added

- auto-fallback from HTTPS to SSH when git clone fails
- make diff patch retention count configurable
- save git patch files for linthis changes
- git type squash mode now uses stash snapshot like git-with-agent
- add fix_commit_mode support to git hook type and fix post-commit
- handle agent review fixes based on fix_commit_mode in pre-push
- show --fix-commit-mode in hook footer alongside --type

### Fixed

- skip hooks during rebase/merge/cherry-pick
- pre-push squash mode now blocks push for user review
- rename fixup commit message to cover both format and lint fixes
- dirty mode hints recommend linthis undo instead of git checkout
- pre-push fixup mode blocks push after creating fixup commit
- config list now shows nested tables with dotted key paths
- always show --fix-commit-mode in hook footer

### Performance

- defer AI provider detection to help-only, ~45x startup speedup

### Changed

- move undo/redo into linthis backup subcommands
- move diff/ to same level as backup/ to avoid interference
- squash mode creates fixup commit then squashes via reset

### Documentation

- add Chinese version of fix-commit-mode documentation
- add fix-commit-mode behavior matrix documentation

## [0.18.0] - 2026-04-10

### Added

- auto-create .gitignore when missing in git repos
- dirty mode asks user before staging with AskUserQuestion tool

### Fixed

- move gitignore check to main.rs and fix exit code + pre-push hints
- differentiate gitignore hints for pre-commit vs pre-push

### Changed

- migrate cache files to global directory
- migrate result/review/backup from project to global directory
- restructure agent skill steps by fix_commit_mode
- rename hook fix modes to squash/dirty/fixup

## [0.17.3] - 2026-04-08

### Added

- add three hook fix modes (one-commit, leave-on-dirty, two-commit)
- add hook fix_mode config and dotted key support for config get/set

### Fixed

- skip latest backup when pruning no-diff entries
- trim no-diff backups from newest end to preserve undo chain
- prioritize removing no-diff backups when over retention limit
- `backup diff` auto-skips backups with no changes
- print "Running [lint] check" even when all files are cached

## [0.17.2] - 2026-04-07

### Added

- add `linthis update/upgrade` subcommand for self-update
- add VCS abstraction layer with Git, SVN, and None providers

### Fixed

- simplify agent fix to direct mode, fix worktree index lock issue

### Changed

- merge update/upgrade into single subcommand with alias
- remove internal strategy labels from user-facing text

## [0.17.1] - 2026-04-07

### Fixed

- pass resolved paths to security/complexity checks and add worktree isolation

## [0.17.0] - 2026-04-05

### Added

- add linthis backup/undo/redo subcommands with unified diff

### Changed

- reduce dispatch_subcommand and handle_format_command complexity
- unify fix tip lines into single fix_tip_lines() function
- unify format and severity options, remove html subcommand

## [0.16.0] - 2026-04-04

### Added

- add unified [retention] config for results, backups, reviews, and cache
- prefer VS Code global user settings for .linthis search.exclude
- auto-exclude .linthis/ from JetBrains and VS Code search indexes
- prefer uv/pipx over pip for Python tool installation
- support fractional interval_days and treat 0 as 12 hours
- auto-add .linthis/ to global gitignore on first run
- add build verification, diff report, and parallel fix to agent skills
- improve tool install hints and default to auto-install

### Fixed

- change interval_days=0 to mean disabled instead of every 12 hours
- resolve compilation errors and dead code warnings
- resolve all clippy warnings (too many args, complex types, etc.)
- resolve lint issues in test fixtures and scripts
- resolve clippy warnings in main.rs
- resolve clippy warnings in security module

### Changed

- reduce cyclomatic complexity across 4 files (9 warnings → 1)
- reduce fix_file cyclomatic complexity from 62 to under 30
- reduce cyclomatic complexity in run_ai_fix_all and show_cached_suggestions
- reduce complexity in lib.rs, plugin.rs, review.rs, and cli modules

## [0.15.3] - 2026-03-30

### Fixed

- show correct threshold per severity level in complexity messages

## [0.15.2] - 2026-03-30

### Added

- unified fix and report for lint, security, and complexity issues

### Fixed

- show per-check breakdown in failure summary and fix hook pass/fail logic
- show security and complexity issues in hook output box
- restore complexity results from cache for correct exit code
- properly handle tag push and use stdin for pre-push
- skip pre-push check when no files to push
- support unified result JSON format in report show and trend loading

### Changed

- split hook.rs (5982 lines) into 11-file module directory
- reduce all hook.rs functions to <=20 cyclomatic complexity
- reduce remaining 4 functions to <=20 cyclomatic complexity
- reduce cyclomatic complexity across 10 files (15 functions → <=20)
- update main checks flow and hook output condition

## [0.15.1] - 2026-03-29

### Added

- unified fail_on, exit codes, cache, and output across all checks
- add `linthis lint` and `linthis check` subcommands

## [0.15.0] - 2026-03-26

### Added

- per-file cache, parallel scanners, language filtering
- unified --checks system with structured result JSON

### Fixed

- resolve clippy warnings across license, security, complexity, ai modules

### Documentation

- update documentation for security, complexity, and --checks features

## [0.14.5] - 2026-03-24

### Added

- add --staged, --modified flags and rename --format to --output
- add SAST source code security scanning

### Fixed

- add missing target arg to install_agent_plugin_from_dir test calls

## [0.14.4] - 2026-03-22

### Fixed

- replace Hook Failed/Passed with Blocked/Passed in hook output

### Changed

- extract cmsg box rendering into output.rs format_cmsg_result

### Documentation

- add linthis config get cmsg.commit_msg_pattern tip to agent cmsg skill
- clarify config resolution order in agent cmsg skill
- use linthis cmsg as authoritative validator in agent cmsg skill

## [0.14.3] - 2026-03-20

### Added

- improve tool auto-install with uv/pip fallback and better failure messages

## [0.14.2] - 2026-03-20

### Added

- merge linthis.toml top-level config on plugin add
- add `linthis hook list` command to show all installed hooks
- add --provider openclaw support for agent skill install and headless fix
- add target field to agent plugin entry for custom install paths

### Fixed

- clean up hook list output — remove repo line, add empty-state hints
- fix agent_is_installed detection for skill-dir providers
- make `hook list -g` show global-only, remove registry section

### Documentation

- update agent-hooks docs to reflect thin wrapper and add OpenClaw
- improve lt-lint steps with key commands table and re-stage workflow
- add re-stage note for formatted files in lt-lint skill

## [0.14.1] - 2026-03-19

### Added

- add companion skill cross-references for lt-lint and lt-cmsg

### Fixed

- add provider availability check and graceful degradation
- use hooks/hooks.json for provider-specific hook config

### Changed

- remove pre-commit hook type support entirely
- hide --type pre-commit to avoid confusion with --event

### Documentation

- remove hook/stop layer from plugin structure

## [0.14.0] - 2026-03-18

### Added

- support provider/model syntax in --provider flag
- add --provider-args flag to pass extra args to AI agent CLI
- remap -v to --version and remove verbose short flag
- add "all" option to interactive uninstall type/event prompts

### Fixed

- warn and skip /model when --provider-args already has --model

## [0.13.0] - 2026-03-17

### Added

- add auto-fix mode CLI arg and improve review subsystem
- generate and apply AI code fixes in auto-fix mode
- add configurable skill names and bordered review summary box

### Fixed

- skip agent review when all pushed files are excluded

### Changed

- rename [hooks] to [hook] with nested agent/review structure

## [0.12.0] - 2026-03-16

### Added

- add skip hint to pre-push review blocked box
- add braille dot spinner to shell timer and fix stop_timer line erasure
- optimize skill descriptions and body with bilingual keywords
- track skill_providers separately in TOML, improve sync output and skip empty commits

### Fixed

- modify icon for pre-push review box
- add JSON response format to stop hook prompt
- match skill name to directory name (lt-* not linthis-*)

### Changed

- flatten plugin directory structure with provider override

## [0.11.2] - 2026-03-16

### Fixed

- add #[cfg(unix)] guard to PermissionsExt import in detect_and_migrate_existing_hooks

## [0.11.1] - 2026-03-16

### Added

- thin wrapper hooks + sync + pre-push review improvements
- support syncing a specific plugin by alias

### Fixed

- include provider in TOML dedup key; disk-scan refreshes orphaned skills
- pass global flag through plugin sync and fix skill sync for global hooks
- use -i flags for per-file linthis args in pre-push script

### Documentation

- document 3-tier hook/plugin override system

## [0.11.0] - 2026-03-14

### Added

- implement 3-tier hook/plugin override system
- multi-type/event install/uninstall + per-event agent skills

### Fixed

- restage when files changed

### Documentation

- update reference docs for new features

## [0.10.0] - 2026-03-13

### Added

- auto re-stage formatted files in staged mode (-s)
- add `linthis format` subcommand with backup/undo support
- add platform detection with install hints
- add elapsed timer for agent fix in *-with-agent hooks
- add --auto-fix shorthand and cmsg auto-fix support
- add background review trigger to pre-push hook
- implement full review command handler with AI analysis
- implement Git platform detection and PR/MR creation
- implement reviewer management and history-based recommendation
- implement background process management
- implement Markdown report generation
- implement review-specific AI prompt templates
- implement AI review analyzer with retry and chunking
- implement git diff collection and parsing
- add review CLI handler skeleton and wire up dispatch
- add Commands::Review CLI definition
- add review module with core data types
- add ReviewConfig to config system
- add commit-msg agent auto-fix support and move cmsg config to [cmsg] section
- implement ObjC method length check, wire into CppChecker
- add count_sloc and extract_method_name helpers for ObjC method length check
- add oc_fn_length field to CppChecker, load from config with default 80
- add fn_length field to CppLanguageConfig for ObjC method length check

### Fixed

- add runtime validation for --ai, --provider, -y flag dependencies
- exclude braces from SLOC, detect no-space ObjC signatures, skip forward declarations
- clarify fn_length doc comment scope and add cpp test

### Changed

- convert agent rules to structured lint rule format

### Documentation

- add fn_length to default config template for ObjC

## [0.9.1] - 2026-03-13

### Added

- add --no-tool-auto-install CLI flag and wire config
- update RunOptions construction sites to use tool_install_mode
- add ToolInstallMode enum, update RunOptions and pre_flight_install
- add ToolAutoInstallConfig to config

### Fixed

- -c -f together now means Both (check+format) instead of CheckOnly
- default hook args now run both check+format (was -c -f which only checked)
- use --dangerously-skip-permissions for claude/codebuddy hook auto-fix
- add permission/auto-accept flags to agent fix headless commands
- add PartialEq to ToolAutoInstallConfig derive

## [0.9.0] - 2026-03-12

### Added

- add linthis cmsg command and commit-msg hook support

## [0.8.0] - 2026-03-11

### Added

- unify provider list and improve hook install help text
- add -m/--modified as primary flag for checking all locally modified files
- add --global support for all hook types
- add --type git-with-agent / prek-with-agent / pre-commit-with-agent

### Fixed

- correct headless CLI commands for all agent fix providers

## [0.7.0] - 2026-03-07

### Added

- add -g/--global flag for agent hook install/uninstall
- add -y/--yes as short alias for --accept-all
- add custom provider support with CLI and API modes
- add gemini, gemini-cli, and codex-cli providers
- add smart provider fallback for API/CLI pairs

### Fixed

- prevent clang-tidy from corrupting namespace declarations

### Changed

- rename template to cli_style and remove -like suffix

### Documentation

- typo
- replace --accept-all with -y in all documentation
- fix Kotlin linter description and add short CLI flag examples

## [0.6.1] - 2026-03-05

### Added

- add Stop Hook support for CodeBuddy agent provider

### Documentation

- update README

## [0.6.0] - 2026-03-04

### Added

- add agent hook system and simplify hook install CLI

### Fixed

- update videos to show plugin enabled by default
- correct Chinese anchor links for video tutorial cross-references
- use absolute paths for video assets in MkDocs
- ensure AI fixes handle C/C++ signature changes across files

### Documentation

- add video tutorials page and cross-reference videos in feature pages

## [0.5.3] - 2026-02-13

### Added

- show skip tip when too many clang-tidy issues detected
- add batch parallel AI fix mode and fix report scroll issue

### Fixed

- consistent clang-tidy results regardless of working directory

## [0.5.2] - 2026-02-09

### Added

- add spinner animation and elapsed time to progress indicators
- add spinner with elapsed time for CLI fix

### Fixed

- use spinner for all progress phases consistently
- use consistent 'oc' directory name for Objective-C configs
- position cursor on empty line below spinner
- improve CLI fix mode and re-check only modified files

### Performance

- reduce tokio features and add dev-release profile

## [0.5.1] - 2026-02-04

### Added

- add backup and restore functionality
- add iterative AI fix loop with --accept-all
- improve hook output with detailed help instructions
- enhance C++ checker, hook output width, and add report show command
- add use_plugin setting for direct plugin specification

### Fixed

- improve HTML report chart and issue layout
- simplify hook output tips section

### Changed

- update hook output to show linthis fix commands

## [0.5.0] - 2026-02-02

### Added

- add AI auto-fix options for hook install
- add CLI direct file editing mode
- use unified diff format for AI fix suggestions
- add --fix option to main command for integrated fix mode
- add [ai] section for AI provider configuration
- add CodeBuddy and CodeBuddy CLI as AI providers

### Fixed

- remove unused imports and prefix unused variables
- disable skip_with_suggestion and update mock provider
- improve diff parsing and add suggestion validation

### Changed

- rename --auto-apply to --accept-all

### Documentation

- add AI fix feature documentation
- add codebuddy API provider to hint
- add available providers hint in lint output
- add dangerous auto-fix hint in lint output
- update config path to .linthis/config.toml and add all 18 languages

## [0.3.0] - 2026-01-31

### Added

- replace init with new command for plugin creation
- add version bump support to publish scripts
- add priority-based config resolver for plugin configs

### Fixed

- add missing @types/glob dependency

### Documentation

- enhance plugin usage documentation
- add plugin configuration guide to README

## [0.2.1] - 2026-01-30

### Added

- add usePlugin setting for direct plugin specification for jetbrains
- add --use-plugin support for LSP server
- add usePlugin setting for direct plugin specification
- add --use-plugin option for direct plugin specification

## [0.2.0] - 2026-01-25

### Added

- add Neovim 0.9 compatibility and debug tools
- add AI-powered fix suggestions to interactive mode

### Fixed

- improve Neovim 0.9.x compatibility for remote installs
- use rustls-tls to avoid OpenSSL dependency on Linux

### Documentation

- simplify lazy.nvim config by removing redundant package.path

## [0.1.1] - 2026-01-23

### Added

- add Linthis branding to hook output

### Fixed

- align hook output box borders correctly
- resolve all cargo build warnings
- remove unsupported -w flag from hook commands
- use Marketplace-hosted images in plugin description
- remove external images from plugin description
- update LSP4IJ dependency to valid version 0.11.0
- use explicit branch name in git push commands

## [0.1.0] - 2026-01-22

### Added

- add advanced analysis features

## [0.0.13] - 2026-01-22

### Fixed

- resolve compilation warnings and clippy lints

## [0.0.12] - 2026-01-21

### Added

- add executable path and additional arguments settings
- add settings UI, format actions and documentation screenshots
- add JetBrains IDE plugin with LSP4IJ integration
- add lint on open and suggestion display for editor plugins
- improve format/lint on save and add comprehensive documentation
- add vscode-linthis

### Fixed

- configure JetBrains plugin dependencies and gradle wrapper
- replace star activation with specific language events

### Changed

- change JetBrains plugin package to com.mojeter.linthis

### Documentation

- add Editor Plugins section to README
- add Neovim plugin publishing guides
- add JetBrains plugin publishing guides
- list all supported languages in JetBrains plugin description
- add publishing guide and required files for marketplace
- mark some tasks as completed in roadmap
- update roadmap with completed items for v0.0.11
- add Chinese navigation translations for i18n
- add i18n support with Chinese translations

### Other

- Add tool name to diagnostic source (linthis-{tool})
- Rewrite lint to use CLI instead of LSP
- Fix format to use CLI instead of LSP
- Fix nvim-linthis root_dir function and add test script
- Add Neovim plugin (nvim-linthis)
- Bump VS Code extension version to 0.0.2
- Add icon for VS Code extension

## [0.0.11] - 2026-01-18

### Documentation

- translate feature docs
- add MkDocs with Material theme for Read the Docs

## [0.0.10] - 2026-01-18

### Added

- add support for Shell, Ruby, PHP, Scala, and C#
- add watch mode with TUI and file monitoring
- add custom regex rules, rule disable, and severity override
- add HTML report generation and analysis features
- add Language Server Protocol server for IDE integration

### Documentation

- add comprehensive documentation

## [0.0.9] - 2026-01-18

### Added

- add config migrate command for ESLint/Prettier/Black/isort migration
- enhance git hooks with pre-push, commit-msg, and detailed reports
- integrate cache into check flow and add large file detection
- add performance optimization with cache and incremental checking
- add Dart, Swift, Kotlin, and Lua language support
- add doctor command and enhance error handling

### Changed

- enhance LintisError with structured variants
- extract modules to reduce main.rs
- extract cli and templates modules from main.rs

### Documentation

- add config migrate and enhanced hooks documentation
- update README for new hook subcommand and fix cache path

## [0.0.8] - 2026-01-15

### Added

- improve fix mode with smart line matching and auto-recheck
- enhance fix mode with navigation and quickfix auto-launch
- add interative fix mode
- add context lines display for lint issues
- auto-load plugins and add --no-plugin skip option

### Fixed

- add warnings for invalid paths and improve plugin sync hints

### Other

-  feat: parallel processing check, format and recheck

## [0.0.7] - 2026-01-07

### Added

- add hook subcommand
- add clang-tidy skip option and filter third-party warnings
- add fail-on-warnings flag and improve git hook management

## [0.0.6] - 2026-01-06

### Fixed

- update clang-tidy test to use non-filtered error code

## [0.0.5] - 2026-01-06

### Added

- add plugin auto-sync and self-update functionality
- add auto-install cpplint

## [0.0.4] - 2026-01-04

### Added

- add whitespace fixers and improve OC support
- add language-specific config for cpp and objective-c
- unify config path to .linthis/ and add result auto-save
- add cpplint auto-fixer and clang-tidy integration
- add config CLI and pre-commit hooks integration
- add multi-language linter and formatter

### Fixed

- enhance comment from strings


