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

mod cli;

use clap::{CommandFactory, FromArgMatches};
use colored::Colorize;
use std::path::PathBuf;
use std::process::ExitCode;

use cli::{
    collect_paths, handle_backup_command, handle_cache_command, handle_commit_msg_check,
    handle_complexity_command, handle_config_command, handle_doctor_command, handle_fix_command,
    handle_format_command, handle_hook_command, handle_ignore_command, handle_init_command,
    handle_license_command, handle_plugin_command, handle_report_command, handle_review_command,
    handle_disable_command, handle_enable_command, handle_security_command,
    handle_self_update_command, handle_shell_command, handle_status_command, init_linter_configs,
    perform_auto_sync, perform_self_update, print_fix_hint, run_benchmark, run_complexity_analysis,
    run_sast_scan, run_watch, strip_ansi_codes, Cli, Commands, ComplexityCommandOptions,
    FixCommandOptions, FormatCommandOptions, PathCollectionOptions, PathCollectionResult,
    ReviewCommandOptions,
};
use linthis::config::resolver::{ConfigResolver, ConfigSource, ResolvedConfig};
use linthis::lsp::{run_lsp_server_with_config, LspMode};
use linthis::utils::output::{format_result_with_hook_type, OutputFormat};
use linthis::{run, Language, RunMode, RunOptions, ToolInstallMode};
use std::sync::Arc;

/// Inject dynamic help text showing detected AI/agent providers into clap commands.
/// Check if help is being requested (--help, -h, or help subcommand).
/// Used to skip expensive provider detection when help isn't needed.
fn is_help_requested() -> bool {
    std::env::args().any(|a| a == "--help" || a == "-h" || a == "help")
}

fn inject_dynamic_help(cmd: &mut clap::Command) {
    use linthis::ai::provider::{detect_available_providers, ALL_AI_PROVIDERS};

    // Build dynamic help text for AI fix providers
    let providers = detect_available_providers();
    let mut ai_help = String::from("\nAI providers (current environment):\n");
    // Show available first, then unavailable
    let mut available: Vec<_> = providers.iter().filter(|(_, a)| *a).collect();
    let unavailable: Vec<_> = providers.iter().filter(|(_, a)| !*a).collect();
    available.extend(unavailable);
    for (kind, avail) in &available {
        let (_, name, desc) = ALL_AI_PROVIDERS.iter().find(|(k, _, _)| k == kind).unwrap();
        if *avail {
            ai_help.push_str(&format!("  \u{2713} {:<14} {} (available)\n", name, desc));
        } else {
            ai_help.push_str(&format!("    {:<14} {}\n", name, desc));
        }
    }

    // Inject into "fix" subcommand
    if let Some(fix_cmd) = cmd.find_subcommand_mut("fix") {
        let existing = fix_cmd
            .get_after_long_help()
            .map(|h| h.to_string())
            .unwrap_or_default();
        *fix_cmd = fix_cmd
            .clone()
            .after_long_help(format!("{}{}", existing, ai_help));
    }

    // Build dynamic help text for agent providers
    let mut agent_help = String::from("\nAgent providers (current environment):\n");
    let agent_detected = cli::hook::detect_agent_providers_lightweight();
    for (name, detected) in &agent_detected {
        if *detected {
            agent_help.push_str(&format!("  \u{2713} {} (detected)\n", name));
        } else {
            agent_help.push_str(&format!("    {}\n", name));
        }
    }

    // Inject into "hook add" subcommand
    if let Some(hook_cmd) = cmd.find_subcommand_mut("hook") {
        if let Some(install_cmd) = hook_cmd.find_subcommand_mut("add") {
            let existing = install_cmd
                .get_after_long_help()
                .map(|h| h.to_string())
                .unwrap_or_default();
            *install_cmd = install_cmd
                .clone()
                .after_long_help(format!("{}{}", existing, agent_help));
        }
    }
}

/// Dispatch early-return subcommands that don't need the main lint flow.
/// Returns `Some(ExitCode)` if a subcommand was handled, `None` otherwise.
fn dispatch_subcommand(command: Commands) -> Option<ExitCode> {
    match command {
        Commands::Plugin { action } => Some(handle_plugin_command(action)),
        Commands::Config { action } => Some(handle_config_command(action)),
        Commands::Hook { action } => Some(handle_hook_command(action)),
        Commands::Shell { action } => Some(handle_shell_command(action)),
        Commands::Cmsg { .. } | Commands::Init { .. } | Commands::AgentStream => {
            Some(dispatch_simple(command))
        }
        Commands::Security { .. } | Commands::License { .. } | Commands::Complexity { .. } => {
            Some(dispatch_analysis(command))
        }
        Commands::Format { .. } => Some(dispatch_format(command)),
        Commands::Fix { .. } => Some(dispatch_fix(command)),
        Commands::Review { .. } => Some(dispatch_review(command)),
        Commands::Watch { .. } => Some(dispatch_watch(command)),
        // Lint and Check fall through to main flow
        Commands::Lint { .. } | Commands::Check { .. } => None,
        other => Some(dispatch_utility(other)),
    }
}

/// Dispatch utility and management subcommands.
fn dispatch_utility(command: Commands) -> ExitCode {
    match command {
        Commands::Doctor { all, output } => handle_doctor_command(all, &output),
        Commands::Cache { action } => handle_cache_command(action),
        Commands::Lsp {
            mode,
            port,
            use_plugin,
        } => handle_lsp_subcommand(mode, port, use_plugin),
        Commands::Report { action } => handle_report_command(action),
        Commands::Backup { action } => handle_backup_command(action),
        Commands::Ignore { action } => handle_ignore_command(action),
        Commands::Update {
            check,
            force,
            target_version,
        } => handle_self_update_command(check, force, target_version),
        Commands::Disable { ttl, global } => handle_disable_command(ttl, global),
        Commands::Enable { global } => handle_enable_command(global),
        Commands::Status => handle_status_command(),
        _ => ExitCode::from(1),
    }
}

fn dispatch_simple(command: Commands) -> ExitCode {
    match command {
        Commands::Cmsg {
            msg_or_file,
            auto_fix,
            provider,
        } => handle_commit_msg_check(&msg_or_file, auto_fix, provider.as_deref()),
        Commands::Init {
            global,
            with_hook,
            force,
        } => handle_init_command(global, with_hook, force),
        Commands::AgentStream => linthis::agent_stream::handle_agent_stream(),
        _ => ExitCode::from(1),
    }
}

fn dispatch_watch(command: Commands) -> ExitCode {
    if let Commands::Watch {
        paths,
        check_only,
        format_only,
        debounce,
        notify,
        no_tui,
        clear,
        lang,
        exclude,
        verbose,
    } = command
    {
        handle_watch_subcommand(WatchSubcommandArgs {
            paths,
            check_only,
            format_only,
            debounce,
            notify,
            no_tui,
            clear,
            lang,
            exclude,
            verbose,
        })
    } else {
        ExitCode::from(1)
    }
}

fn dispatch_format(command: Commands) -> ExitCode {
    if let Commands::Format {
        paths,
        staged,
        modified,
        exclude,
        undo,
        source,
        list_backups,
        verbose,
        quiet,
    } = command
    {
        handle_format_command(FormatCommandOptions {
            paths,
            staged,
            modified,
            exclude,
            undo,
            source,
            list_backups,
            verbose,
            quiet,
        })
    } else {
        ExitCode::from(1)
    }
}

fn dispatch_fix(command: Commands) -> ExitCode {
    if let Commands::Fix {
        source,
        check,
        format_only,
        auto_fix,
        ai,
        provider,
        model,
        provider_args,
        max_suggestions,
        accept_all,
        jobs,
        file,
        line,
        message,
        rule,
        output,
        with_context,
        verbose,
        quiet,
        undo,
        list_backups,
    } = command
    {
        let (ai, accept_all) = if auto_fix {
            (true, true)
        } else {
            (ai, accept_all)
        };
        handle_fix_command(FixCommandOptions {
            source,
            check,
            format_only,
            ai,
            provider,
            model,
            provider_args,
            max_suggestions,
            accept_all,
            jobs,
            file,
            line,
            message,
            rule,
            output,
            with_context,
            verbose,
            quiet,
            undo,
            list_backups,
        })
    } else {
        ExitCode::from(1)
    }
}

fn dispatch_review(command: Commands) -> ExitCode {
    if let Commands::Review {
        background,
        auto_fix,
        auto_fix_mode,
        reviewers,
        provider,
        base,
        head,
        no_pr,
        notify,
        status,
        dry_run,
        clean,
        output,
    } = command
    {
        handle_review_command(ReviewCommandOptions {
            background,
            auto_fix,
            auto_fix_mode,
            reviewers,
            provider,
            base,
            head,
            no_pr,
            notify,
            status,
            dry_run,
            clean,
            output,
        })
    } else {
        ExitCode::from(1)
    }
}

fn dispatch_analysis(command: Commands) -> ExitCode {
    match command {
        Commands::Security {
            path,
            scan_type,
            severity,
            include_dev,
            fix,
            ignore,
            format,
            sbom,
            fail_on,
            sast_config,
            verbose,
        } => handle_security_command(cli::SecurityCommandParams {
            path,
            scan_type,
            severity,
            include_dev,
            fix,
            ignore,
            format,
            sbom,
            fail_on,
            sast_config,
            verbose,
        }),
        Commands::License {
            path,
            policy,
            policy_file,
            include_dev,
            format,
            sbom,
            fail_on_violation,
            verbose,
        } => handle_license_command(cli::LicenseCommandParams {
            path,
            policy,
            policy_file,
            include_dev,
            format,
            sbom,
            fail_on_violation,
            verbose,
        }),
        Commands::Complexity {
            path,
            staged,
            modified,
            include,
            exclude,
            threshold,
            preset,
            format,
            with_trends,
            trend_count,
            only_high,
            sort,
            no_parallel,
            verbose,
        } => handle_complexity_command(ComplexityCommandOptions {
            path,
            staged,
            modified,
            include,
            exclude,
            threshold,
            preset,
            format,
            with_trends,
            trend_count,
            only_high,
            sort,
            no_parallel,
            verbose,
        }),
        _ => ExitCode::from(1),
    }
}

/// Handle the `lsp` subcommand.
fn handle_lsp_subcommand(mode: String, port: u16, use_plugin: Option<Vec<String>>) -> ExitCode {
    let mut lsp_config_resolver = ConfigResolver::new();

    if let Some(ref plugin_specs) = use_plugin {
        use linthis::plugin::{PluginLoader, PluginSource};

        for spec in plugin_specs {
            let (url_or_path, git_ref) = parse_plugin_spec(spec);

            let name = plugin_name_from_path(&url_or_path);

            let source = if let Some(ref r) = git_ref {
                PluginSource::new(&url_or_path).with_ref(r)
            } else {
                PluginSource::new(&url_or_path)
            };

            if let Ok(loader) = PluginLoader::new() {
                if let Ok(configs) = loader.load_configs(&[source], false) {
                    for config in &configs {
                        lsp_config_resolver.add_config(ResolvedConfig::new(
                            config.language.clone(),
                            config.tool.clone(),
                            config.config_path.clone(),
                            ConfigSource::CliPlugin,
                            name.clone(),
                        ));
                    }
                    eprintln!(
                        "[lsp] Loaded {} config(s) from plugin '{}'",
                        configs.len(),
                        name
                    );
                }
            }
        }
    }

    let lsp_mode = match mode.parse::<LspMode>() {
        Ok(m) => m,
        Err(e) => {
            eprintln!("{}: {}", "Error".red(), e);
            return ExitCode::from(1);
        }
    };

    let runtime = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("{}: Failed to create async runtime: {}", "Error".red(), e);
            return ExitCode::from(1);
        }
    };

    let resolver = if lsp_config_resolver.is_empty() {
        None
    } else {
        Some(Arc::new(lsp_config_resolver))
    };

    match runtime.block_on(run_lsp_server_with_config(lsp_mode, port, resolver)) {
        Ok(_) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{}: LSP server error: {}", "Error".red(), e);
            ExitCode::from(1)
        }
    }
}

struct WatchSubcommandArgs {
    paths: Vec<PathBuf>,
    check_only: bool,
    format_only: bool,
    debounce: u64,
    notify: bool,
    no_tui: bool,
    clear: bool,
    lang: Option<Vec<String>>,
    exclude: Option<Vec<String>>,
    verbose: bool,
}

/// Handle the `watch` subcommand.
fn handle_watch_subcommand(args: WatchSubcommandArgs) -> ExitCode {
    let languages: Vec<Language> = args
        .lang
        .unwrap_or_default()
        .iter()
        .filter_map(|s| Language::from_name(s))
        .collect();

    let config = linthis::watch::WatchConfig {
        paths: args.paths,
        check_only: args.check_only,
        format_only: args.format_only,
        debounce_ms: args.debounce,
        notify: args.notify,
        no_tui: args.no_tui,
        clear: args.clear,
        verbose: args.verbose,
        languages,
        exclude_patterns: args.exclude.unwrap_or_default(),
    };

    match run_watch(config) {
        Ok(_) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{}: {}", "Error".red(), e);
            ExitCode::from(1)
        }
    }
}

/// Apply lint subcommand fields onto the top-level CLI struct so they fall through
/// to the main lint flow.
fn apply_lint_subcommand(cli: &mut Cli) {
    if let Some(Commands::Lint {
        paths,
        staged,
        modified,
        since,
        lang,
        exclude,
        no_default_excludes,
        no_gitignore,
        output,
        no_cache,
        verbose,
        quiet,
    }) = cli.command.take()
    {
        cli.paths = paths;
        cli.staged = staged;
        cli.modified = modified;
        cli.since = since;
        cli.lang = lang;
        cli.exclude = exclude;
        cli.no_default_excludes = no_default_excludes;
        cli.no_gitignore = no_gitignore;
        cli.output = output;
        cli.no_cache = no_cache;
        cli.verbose = verbose;
        cli.quiet = quiet;
        cli.check_only = true;
        cli.checks = Some(vec!["lint".to_string()]);
    }
}

/// Apply check subcommand fields onto the top-level CLI struct so they fall through
/// to the main lint flow.
fn apply_check_subcommand(cli: &mut Cli) {
    if let Some(Commands::Check {
        paths,
        staged,
        modified,
        since,
        checks,
        lang,
        exclude,
        no_default_excludes,
        no_gitignore,
        output,
        no_cache,
        verbose,
        quiet,
    }) = cli.command.take()
    {
        cli.paths = paths;
        cli.staged = staged;
        cli.modified = modified;
        cli.since = since;
        cli.checks = checks;
        cli.lang = lang;
        cli.exclude = exclude;
        cli.no_default_excludes = no_default_excludes;
        cli.no_gitignore = no_gitignore;
        cli.output = output;
        cli.no_cache = no_cache;
        cli.verbose = verbose;
        cli.quiet = quiet;
        cli.check_only = true;
    }
}

/// Validate flag dependencies that clap cannot express.
/// Returns `Some(ExitCode)` on validation failure.
fn validate_cli_flags(cli: &Cli) -> Option<ExitCode> {
    if cli.ai && !cli.fix {
        eprintln!("{}: --ai requires --fix or --auto-fix", "Error".red());
        return Some(ExitCode::from(2));
    }
    if cli.provider.is_some() && !cli.ai && !cli.auto_fix {
        eprintln!("{}: --provider requires --ai or --auto-fix", "Error".red());
        return Some(ExitCode::from(2));
    }
    if cli.accept_all && !cli.fix {
        eprintln!("{}: -y/--yes requires --fix or --auto-fix", "Error".red());
        return Some(ExitCode::from(2));
    }
    None
}

/// Parse a plugin spec string into (url_or_path, optional_git_ref).
fn parse_plugin_spec(spec: &str) -> (String, Option<String>) {
    if spec.contains('@') && !spec.starts_with('/') {
        let parts: Vec<&str> = spec.rsplitn(2, '@').collect();
        if parts.len() == 2 {
            (parts[1].to_string(), Some(parts[0].to_string()))
        } else {
            (spec.to_string(), None)
        }
    } else {
        (spec.to_string(), None)
    }
}

/// Derive a short plugin name from a URL or filesystem path.
fn plugin_name_from_path(url_or_path: &str) -> String {
    url_or_path
        .rsplit('/')
        .next()
        .unwrap_or(url_or_path)
        .trim_end_matches(".git")
        .to_string()
}

/// Load plugins and build a `ConfigResolver`.
/// Returns `(loaded_plugin_names, config_resolver)` or an error exit code.
/// Build a `PluginSource` from a URL/path and an optional git ref.
fn make_plugin_source(url_or_path: &str, git_ref: Option<&str>) -> linthis::plugin::PluginSource {
    let source = linthis::plugin::PluginSource::new(url_or_path);
    match git_ref {
        Some(r) => source.with_ref(r),
        None => source,
    }
}

/// Collect plugins specified on the CLI via `--use-plugin`.
fn collect_cli_plugins(
    plugin_specs: &[String],
    verbose: bool,
) -> Vec<(String, linthis::plugin::PluginSource)> {
    plugin_specs
        .iter()
        .map(|spec| {
            let (url_or_path, git_ref) = parse_plugin_spec(spec);
            let name = plugin_name_from_path(&url_or_path);
            let source = make_plugin_source(&url_or_path, git_ref.as_deref());
            if verbose {
                eprintln!("Using plugin from CLI: {} ({})", name, url_or_path);
            }
            (name, source)
        })
        .collect()
}

/// Convert a list of `(name, url, git_ref)` tuples from a plugin
/// config manager into `(name, PluginSource)` pairs, silently
/// returning an empty vec on any error.
fn list_plugins_from_manager(
    manager: Result<linthis::plugin::PluginConfigManager, linthis::plugin::PluginError>,
) -> Vec<(String, linthis::plugin::PluginSource)> {
    let manager = match manager {
        Ok(m) => m,
        Err(_) => return Vec::new(),
    };
    let entries = match manager.list_plugins() {
        Ok(p) => p,
        Err(_) => return Vec::new(),
    };
    entries
        .into_iter()
        .map(|(name, url, git_ref)| {
            let source = make_plugin_source(&url, git_ref.as_deref());
            (name, source)
        })
        .collect()
}

/// Load configs from a set of plugins and merge them into
/// `loaded_plugins` and `config_resolver`.
fn load_plugin_configs(
    plugins: Vec<(String, linthis::plugin::PluginSource)>,
    source_type: ConfigSource,
    verbose: bool,
    loaded_plugins: &mut Vec<String>,
    config_resolver: &mut ConfigResolver,
) -> Result<(), ExitCode> {
    if plugins.is_empty() {
        return Ok(());
    }

    let loader = linthis::plugin::PluginLoader::with_verbose(verbose).map_err(|e| {
        eprintln!(
            "{}: Failed to initialize plugin loader: {}",
            "Error".red(),
            e
        );
        ExitCode::from(1)
    })?;

    for (plugin_name, source) in plugins {
        match loader.load_configs(&[source], false) {
            Ok(configs) => {
                loaded_plugins.push(plugin_name.clone());
                if verbose {
                    eprintln!(
                        "Loaded {} config(s) from plugin '{}' (priority: {:?})",
                        configs.len(),
                        plugin_name,
                        source_type
                    );
                }
                for config in &configs {
                    config_resolver.add_config(ResolvedConfig::new(
                        config.language.clone(),
                        config.tool.clone(),
                        config.config_path.clone(),
                        source_type,
                        plugin_name.clone(),
                    ));
                    if verbose {
                        eprintln!(
                            "  - {}/{}: {} (from plugin cache)",
                            config.language,
                            config.tool,
                            config.config_path.display()
                        );
                    }
                }
            }
            Err(e) => {
                eprintln!(
                    "{}: Failed to load plugin '{}': {}",
                    "Warning".yellow(),
                    plugin_name,
                    e
                );
            }
        }
    }
    Ok(())
}

fn load_plugins(cli: &Cli) -> Result<(Vec<String>, ConfigResolver), ExitCode> {
    use linthis::plugin::PluginConfigManager;

    let mut loaded_plugins: Vec<String> = Vec::new();
    let mut config_resolver = ConfigResolver::new();

    if cli.no_plugin {
        return Ok((loaded_plugins, config_resolver));
    }

    let (cli_plugins, project_plugins, global_plugins) = if let Some(ref specs) = cli.use_plugin {
        (
            collect_cli_plugins(specs, cli.verbose),
            Vec::new(),
            Vec::new(),
        )
    } else {
        let project = list_plugins_from_manager(PluginConfigManager::project());
        let global = if project.is_empty() {
            list_plugins_from_manager(PluginConfigManager::global())
        } else {
            Vec::new()
        };
        (Vec::new(), project, global)
    };

    let all_plugins = [
        (cli_plugins, ConfigSource::CliPlugin),
        (project_plugins, ConfigSource::ProjectPlugin),
        (global_plugins, ConfigSource::GlobalPlugin),
    ];

    for (plugins, source_type) in all_plugins {
        load_plugin_configs(
            plugins,
            source_type,
            cli.verbose,
            &mut loaded_plugins,
            &mut config_resolver,
        )?;
    }

    Ok((loaded_plugins, config_resolver))
}

/// Handle the `--init` flag (create a default config file).
fn handle_init_flag() -> ExitCode {
    let config_path =
        linthis::config::Config::project_config_path(&std::env::current_dir().unwrap_or_default());
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
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{}: Failed to create config: {}", "Error".red(), e);
            ExitCode::from(2)
        }
    }
}

/// Parse a `LINTHIS_INSTALL_MODE` token into a `ToolInstallMode`.
///
/// Accepts `auto`, `prompt`, or `disabled` (case-insensitive, whitespace
/// trimmed). Unknown or empty values return `None` so the caller falls through
/// to config / default resolution.
fn parse_install_mode_env(raw: &str) -> Option<ToolInstallMode> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "auto" => Some(ToolInstallMode::Auto),
        "prompt" => Some(ToolInstallMode::Prompt),
        "disabled" => Some(ToolInstallMode::Disabled),
        _ => None,
    }
}

/// Resolve the `ToolInstallMode` from the `[tool_auto_install]` config section,
/// falling back to `Prompt` when the section is absent.
fn install_mode_from_config(runtime_config: &linthis::config::Config) -> ToolInstallMode {
    match &runtime_config.tool_auto_install {
        Some(cfg) if !cfg.enabled => ToolInstallMode::Disabled,
        Some(cfg) => match cfg.mode.as_str() {
            "auto" => ToolInstallMode::Auto,
            "disabled" => ToolInstallMode::Disabled,
            _ => ToolInstallMode::Prompt,
        },
        None => ToolInstallMode::Prompt,
    }
}

/// Resolve the `ToolInstallMode` from CLI flags, the `LINTHIS_INSTALL_MODE`
/// environment variable, and config.
///
/// Precedence (highest first):
///  1. `--no-tool-auto-install` flag → `Disabled`
///  2. `LINTHIS_INSTALL_MODE` env var (`auto` / `prompt` / `disabled`)
///  3. `[tool_auto_install]` config section
///  4. default → `Prompt`
fn resolve_tool_install_mode(
    no_tool_auto_install: bool,
    runtime_config: &linthis::config::Config,
) -> ToolInstallMode {
    if no_tool_auto_install {
        return ToolInstallMode::Disabled;
    }

    std::env::var("LINTHIS_INSTALL_MODE")
        .ok()
        .and_then(|raw| parse_install_mode_env(&raw))
        .unwrap_or_else(|| install_mode_from_config(runtime_config))
}

/// Resolve which checks to run from CLI flags and config.
fn resolve_checks_list(cli_checks: &Option<Vec<String>>, config_checks: &[String]) -> Vec<String> {
    let base = if let Some(ref cli_checks) = cli_checks {
        if cli_checks.iter().any(|c| c == "all") {
            vec!["lint".into(), "security".into(), "complexity".into()]
        } else {
            cli_checks.clone()
        }
    } else {
        config_checks.to_vec()
    };
    apply_skip_checks_env(base)
}

/// Names of the checks that `LINTHIS_SKIP_CHECKS` can filter out.
/// Kept in sync with `ChecksConfig::default_checks`.
const KNOWN_CHECKS: &[&str] = &["lint", "security", "complexity"];

/// Resolve a `LINTHIS_SKIP_CHECKS` token (≥3-char case-insensitive prefix, or
/// full name) to the canonical check name. Returns `None` on unknown tokens;
/// `Err(&"ambiguous")` would mean more than one match, but today's three checks
/// start with distinct letters so ambiguity is impossible.
fn resolve_skip_token(tok: &str) -> Option<&'static str> {
    let lower = tok.trim().to_lowercase();
    if lower.len() < 3 {
        return None;
    }
    let matches: Vec<&&str> = KNOWN_CHECKS
        .iter()
        .filter(|c| c.starts_with(&lower))
        .collect();
    match matches.as_slice() {
        [one] => Some(**one),
        _ => None,
    }
}

/// Apply `LINTHIS_SKIP_CHECKS=<names>` filtering to a resolved checks list.
/// Unknown or too-short tokens are reported to stderr and ignored (fail-safe).
fn apply_skip_checks_env(mut list: Vec<String>) -> Vec<String> {
    use colored::Colorize;
    let raw = match std::env::var("LINTHIS_SKIP_CHECKS") {
        Ok(v) if !v.trim().is_empty() => v,
        _ => return list,
    };

    let mut to_skip: Vec<&'static str> = Vec::new();
    let mut bad: Vec<String> = Vec::new();
    for tok in raw.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        match resolve_skip_token(tok) {
            Some(name) => {
                if !to_skip.contains(&name) {
                    to_skip.push(name);
                }
            }
            None => bad.push(tok.to_string()),
        }
    }

    if !bad.is_empty() {
        eprintln!(
            "{}: LINTHIS_SKIP_CHECKS: ignoring unknown token(s) [{}]. \
             Supported: lint, security, complexity (or any ≥3-char prefix).",
            "Warning".yellow(),
            bad.join(", ")
        );
    }

    if to_skip.is_empty() {
        return list;
    }

    let before = list.clone();
    list.retain(|c| !to_skip.contains(&c.as_str()));
    let removed: Vec<String> = before.into_iter().filter(|c| !list.contains(c)).collect();
    if !removed.is_empty() {
        eprintln!(
            "{}",
            format!(
                "⏭  linthis checks filtered via LINTHIS_SKIP_CHECKS={raw}: skipping {}",
                removed.join(", ")
            )
            .dimmed()
        );
    }
    list
}

/// Run the security SAST check and merge results into the main result.
fn run_security_check(
    result: &mut linthis::utils::types::RunResult,
    runtime_project_root: &std::path::Path,
    target_files: &[std::path::PathBuf],
    security_config: &linthis::config::SecurityChecksConfig,
    security_cache_path: &std::path::Path,
    no_cache: bool,
    quiet: bool,
) {
    let mut cache = PerFileCache::load(security_cache_path);
    let partition = cache.partition_files(target_files, no_cache);

    if !quiet {
        eprintln!("{}", PerFileCache::format_status("security", &partition));
    }

    let fresh_result = if !partition.changed.is_empty() {
        let r = run_sast_scan(runtime_project_root, &partition.changed, security_config);
        cache.update_from_sast(&partition.changed, &r);
        cache.save(security_cache_path);
        r
    } else {
        linthis::security::sast::SastResult {
            findings: vec![],
            by_severity: std::collections::HashMap::new(),
            by_tool: std::collections::HashMap::new(),
            scanner_status: vec![],
            unavailable_tools: vec![],
            duration_ms: 0,
            errors: vec![],
        }
    };

    let mut merged = fresh_result;
    let mut all_findings = partition.cached_findings;
    all_findings.append(&mut merged.findings);
    merged.findings = all_findings;
    merged.by_severity.clear();
    for f in &merged.findings {
        *merged
            .by_severity
            .entry(f.severity.to_string())
            .or_insert(0) += 1;
    }
    merged.by_tool.clear();
    for f in &merged.findings {
        *merged.by_tool.entry(f.source.clone()).or_insert(0) += 1;
    }

    let sec_fail_on = security_config.fail_on.clone().unwrap_or_default();
    let sec_errors = merged
        .findings
        .iter()
        .filter(|f| {
            matches!(
                f.severity,
                linthis::security::Severity::Critical | linthis::security::Severity::High
            )
        })
        .count();
    let sec_warnings = merged
        .findings
        .iter()
        .filter(|f| f.severity == linthis::security::Severity::Medium)
        .count();
    let sec_infos = merged
        .findings
        .iter()
        .filter(|f| {
            matches!(
                f.severity,
                linthis::security::Severity::Low
                    | linthis::security::Severity::None
                    | linthis::security::Severity::Unknown
            )
        })
        .count();
    let sec_exit = sec_fail_on.exit_code(sec_errors, sec_warnings, sec_infos);
    result.exit_code = std::cmp::max(result.exit_code, sec_exit);

    for ut in &merged.unavailable_tools {
        result
            .unavailable_tools
            .push(linthis::utils::types::UnavailableTool::new(
                &ut.tool,
                &ut.languages.join(", "),
                "sast",
                &ut.install_hint,
            ));
    }
    result.security = Some(merged);
    result.checks_run.push("security".to_string());
}

/// Run the complexity analysis check and merge results into the main result.
fn run_complexity_check(
    result: &mut linthis::utils::types::RunResult,
    runtime_project_root: &std::path::Path,
    target_files: &[std::path::PathBuf],
    complexity_config: &linthis::config::ComplexityChecksConfig,
    complexity_cache_path: &std::path::Path,
    no_cache: bool,
    quiet: bool,
) {
    let mut cache = PerFileCache::load(complexity_cache_path);
    let partition = cache.partition_files(target_files, no_cache);

    if !quiet {
        eprintln!("{}", PerFileCache::format_status("complexity", &partition));
    }

    if !partition.changed.is_empty() {
        let analysis_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            run_complexity_analysis(runtime_project_root, &partition.changed, complexity_config)
        }));
        match analysis_result {
            Ok(Ok(analysis)) => {
                cache.update_from_complexity(&partition.changed, &analysis);
                cache.save(complexity_cache_path);

                apply_complexity_exit_code(result, &analysis, complexity_config);
                result.complexity = Some(analysis);
            }
            Ok(Err(e)) => {
                if !quiet {
                    eprintln!("Complexity analysis error: {}", e);
                }
            }
            Err(_) => {
                if !quiet {
                    eprintln!("Complexity analysis encountered an internal error");
                }
            }
        }
    }

    if result.complexity.is_none() && partition.cache_hits > 0 {
        let cached_metrics = cache.get_cached_file_metrics(target_files);
        let mut cached_result = linthis::complexity::AnalysisResult::new();
        cached_result.files = cached_metrics;
        cached_result.calculate_summary();

        if let Some(t) = complexity_config.threshold {
            cached_result.thresholds.cyclomatic.good = t;
            cached_result.thresholds.cyclomatic.warning = t + 10;
            cached_result.thresholds.cyclomatic.high = t + 20;
        }
        if let Some(w) = complexity_config.warning_threshold {
            cached_result.thresholds.cyclomatic.warning = w;
        }
        if let Some(e) = complexity_config.error_threshold {
            cached_result.thresholds.cyclomatic.high = e;
        }
        cached_result.thresholds.cyclomatic.normalize();

        apply_complexity_exit_code(result, &cached_result, complexity_config);
        result.complexity = Some(cached_result);
    }
    result.checks_run.push("complexity".to_string());
}

/// Calculate and apply the complexity exit code onto the result.
fn apply_complexity_exit_code(
    result: &mut linthis::utils::types::RunResult,
    analysis: &linthis::complexity::AnalysisResult,
    complexity_config: &linthis::config::ComplexityChecksConfig,
) {
    let cx_fail_on = complexity_config.fail_on.clone().unwrap_or_default();
    let counts =
        linthis::complexity::count_cyclomatic(&analysis.files, &analysis.thresholds.cyclomatic);
    let cx_exit = cx_fail_on.exit_code(counts.errors, counts.warnings, counts.infos);
    result.exit_code = std::cmp::max(result.exit_code, cx_exit);
}

/// Save results to file and clean up old result files.
fn save_results(result: &linthis::utils::types::RunResult, output: &str, cli: &Cli) {
    use chrono::Local;
    use std::fs::{self, File};
    use std::io::Write;

    let project_root = linthis::utils::get_project_root();

    let output_file = if let Some(ref custom_path) = cli.output_file {
        if let Some(parent) = custom_path.parent() {
            if !parent.as_os_str().is_empty() {
                let _ = fs::create_dir_all(parent);
            }
        }
        custom_path.clone()
    } else {
        let result_dir = linthis::utils::get_result_dir();
        if let Err(e) = fs::create_dir_all(&result_dir) {
            eprintln!(
                "{}: Failed to create {}: {}",
                "Warning".yellow(),
                result_dir.display(),
                e
            );
            return;
        }
        let timestamp = Local::now().format("%Y%m%d-%H%M%S");
        result_dir.join(format!("result-{}.json", timestamp))
    };

    let file_content = if cli.output_file.is_some() {
        strip_ansi_codes(output)
    } else {
        linthis::utils::output::format_result_json(result)
    };

    match File::create(&output_file) {
        Ok(mut file) => {
            if let Err(e) = writeln!(file, "{}", file_content) {
                eprintln!(
                    "{}: Failed to write to {}: {}",
                    "Warning".yellow(),
                    output_file.display(),
                    e
                );
            } else if !cli.quiet {
                eprintln!("{} Results saved to {}", "✓".green(), output_file.display());
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

    if !cli.no_save_result && cli.output_file.is_none() {
        // CLI --keep-results overrides config; config overrides default (10)
        let keep = if cli.keep_results != 10 {
            cli.keep_results // explicitly set via CLI
        } else {
            linthis::config::Config::load_project_config(&project_root)
                .map(|c| c.retention.results)
                .unwrap_or(10)
        };
        if keep > 0 {
            cleanup_old_results(keep.max(1), cli.verbose);
        }
    }
}

/// Remove old result files beyond the keep limit.
fn cleanup_old_results(keep_results: usize, verbose: bool) {
    use std::fs;

    let result_dir = linthis::utils::get_result_dir();
    if let Ok(entries) = fs::read_dir(&result_dir) {
        let mut result_files: Vec<_> = entries
            .filter_map(|e| e.ok())
            .filter(|e| {
                let name = e.file_name().to_string_lossy().to_string();
                name.starts_with("result-") && (name.ends_with(".json") || name.ends_with(".txt"))
            })
            .collect();

        result_files.sort_by(|a, b| {
            let a_time = a.metadata().and_then(|m| m.modified()).ok();
            let b_time = b.metadata().and_then(|m| m.modified()).ok();
            b_time.cmp(&a_time)
        });

        let files_to_remove = result_files.iter().skip(keep_results);
        let mut removed_count = 0;
        for entry in files_to_remove {
            if fs::remove_file(entry.path()).is_ok() {
                removed_count += 1;
            }
        }
        if removed_count > 0 && verbose {
            eprintln!(
                "{} Cleaned up {} old result file(s)",
                "✓".green(),
                removed_count
            );
        }
    }
}

/// Print the failure summary with per-check details.
fn print_failure_summary(result: &linthis::utils::types::RunResult) {
    eprintln!();

    let checks_label = if result.checks_run.is_empty() {
        String::new()
    } else {
        format!(" [{}]", result.checks_run.join(", "))
    };
    let is_info_only = result.exit_code == 3;
    if is_info_only {
        eprintln!(
            "{} {}",
            "\u{26a0}".yellow().bold(),
            format!("Linthis check completed with issues{}", checks_label)
                .yellow()
                .bold()
        );
    } else {
        eprintln!(
            "{} {}",
            "\u{2717}".red().bold(),
            format!("Linthis check failed{}", checks_label).red().bold()
        );
    }

    // Formatting
    let fmt_errors = result
        .format_results
        .iter()
        .filter(|r| r.error.is_some())
        .count();
    if fmt_errors > 0 {
        eprintln!(
            "  {}: {}",
            "formatting".red(),
            format!("{} file(s) with errors", fmt_errors).red()
        );
    }

    print_lint_summary(result);
    print_security_summary(result);
    print_complexity_summary(result);
}

/// Print lint detail line in the failure summary.
fn print_lint_summary(result: &linthis::utils::types::RunResult) {
    if !result.checks_run.iter().any(|c| c == "lint") {
        return;
    }
    let lint_errors = result
        .issues
        .iter()
        .filter(|i| i.severity == linthis::utils::types::Severity::Error)
        .count();
    let lint_warnings = result
        .issues
        .iter()
        .filter(|i| i.severity == linthis::utils::types::Severity::Warning)
        .count();
    let lint_infos = result
        .issues
        .iter()
        .filter(|i| i.severity == linthis::utils::types::Severity::Info)
        .count();
    if lint_errors > 0 || lint_warnings > 0 || lint_infos > 0 {
        let mut parts = Vec::new();
        if lint_errors > 0 {
            parts.push(format!("{} error(s)", lint_errors));
        }
        if lint_warnings > 0 {
            parts.push(format!("{} warning(s)", lint_warnings));
        }
        if lint_infos > 0 {
            parts.push(format!("{} info", lint_infos));
        }
        eprintln!("  {}: {}", "lint".red(), parts.join(", ").red());
    } else if result.exit_code != 0 {
        eprintln!("  lint: {}", "\u{2713}".green());
    }
}

/// Print security detail line in the failure summary.
fn print_security_summary(result: &linthis::utils::types::RunResult) {
    if !result.checks_run.iter().any(|c| c == "security") {
        return;
    }
    let Some(ref sec) = result.security else {
        eprintln!("  security: {}", "\u{2713}".green());
        return;
    };
    let sec_errors = sec
        .findings
        .iter()
        .filter(|f| {
            matches!(
                f.severity,
                linthis::security::Severity::Critical | linthis::security::Severity::High
            )
        })
        .count();
    let sec_warnings = sec
        .findings
        .iter()
        .filter(|f| f.severity == linthis::security::Severity::Medium)
        .count();
    let sec_infos = sec
        .findings
        .iter()
        .filter(|f| {
            matches!(
                f.severity,
                linthis::security::Severity::Low
                    | linthis::security::Severity::None
                    | linthis::security::Severity::Unknown
            )
        })
        .count();
    if sec_errors > 0 || sec_warnings > 0 || sec_infos > 0 {
        let mut parts = Vec::new();
        if sec_errors > 0 {
            parts.push(format!("{} error(s)", sec_errors));
        }
        if sec_warnings > 0 {
            parts.push(format!("{} warning(s)", sec_warnings));
        }
        if sec_infos > 0 {
            parts.push(format!("{} info", sec_infos));
        }
        eprintln!("  {}: {}", "security".red(), parts.join(", ").red());
    } else {
        eprintln!("  security: {}", "\u{2713}".green());
    }
}

/// Print complexity detail line in the failure summary.
fn print_complexity_summary(result: &linthis::utils::types::RunResult) {
    if !result.checks_run.iter().any(|c| c == "complexity") {
        return;
    }
    let Some(ref cx) = result.complexity else {
        eprintln!("  complexity: {}", "\u{2713}".green());
        return;
    };
    let counts = linthis::complexity::count_cyclomatic(&cx.files, &cx.thresholds.cyclomatic);
    let (cx_errors, cx_warns, cx_infos) = (counts.errors, counts.warnings, counts.infos);
    if cx_errors > 0 || cx_warns > 0 || cx_infos > 0 {
        let mut parts = Vec::new();
        if cx_errors > 0 {
            parts.push(format!("{} error(s)", cx_errors));
        }
        if cx_warns > 0 {
            parts.push(format!("{} warning(s)", cx_warns));
        }
        if cx_infos > 0 {
            parts.push(format!("{} info", cx_infos));
        }
        eprintln!("  {}: {}", "complexity".red(), parts.join(", ").red());
    } else {
        eprintln!("  complexity: {}", "\u{2713}".green());
    }
}

/// Handle --fix mode (AI or interactive) after lint results are available.
/// Returns `Some(ExitCode)` if fix mode was entered, `None` otherwise.
fn handle_fix_mode(cli: &Cli, result: &linthis::utils::types::RunResult) -> Option<ExitCode> {
    if !cli.fix || result.issues.is_empty() {
        return None;
    }

    use cli::resolve_ai_provider;
    use linthis::config::Config;
    use linthis::interactive::{run_ai_fix_all, run_interactive, AiFixConfig};

    let project_root = linthis::utils::get_project_root();
    let config = Config::load_merged(&project_root);

    if cli.ai {
        let interactive_provider = if cli.provider.is_none()
            && std::env::var("LINTHIS_AI_PROVIDER").is_err()
            && config.ai.provider.is_none()
            && std::io::IsTerminal::is_terminal(&std::io::stdin())
        {
            cli::select_ai_provider_interactive()
        } else {
            None
        };

        let provider_ref = interactive_provider.as_deref().or(cli.provider.as_deref());

        let provider = resolve_ai_provider(provider_ref, config.ai.provider.as_deref());
        let ai_config = AiFixConfig::with_provider(&provider)
            .with_accept_all(cli.accept_all)
            .with_verbose(cli.verbose);

        if !cli.quiet {
            eprintln!(
                "\n{} Entering AI fix mode with provider: {}",
                "\u{2192}".cyan(),
                provider.cyan()
            );
        }

        let ai_result = run_ai_fix_all(result, &ai_config);

        if !cli.quiet && ai_result.applied > 0 {
            eprintln!(
                "{} Applied {} fix(es)",
                "\u{2713}".green(),
                ai_result.applied
            );
        }

        if ai_result.applied > 0 && ai_result.errors == 0 {
            return Some(ExitCode::SUCCESS);
        }
    } else {
        if !cli.quiet {
            eprintln!("\n{} Entering interactive fix mode", "\u{2192}".cyan());
        }

        let interactive_result = run_interactive(result);

        if !cli.quiet {
            let count = interactive_result.edited + interactive_result.ignored;
            if count > 0 {
                eprintln!("{} Processed {} issue(s)", "\u{2713}".green(), count);
            }
        }
    }

    Some(ExitCode::from(result.exit_code as u8))
}

/// Auto re-stage formatted files when running in staged mode (-s).
fn auto_restage_formatted(result: &linthis::utils::types::RunResult, quiet: bool) {
    if result.format_results.is_empty() {
        return;
    }
    let formatted_files: Vec<&PathBuf> = result
        .format_results
        .iter()
        .filter(|r| r.changed)
        .map(|r| &r.file_path)
        .collect();
    if formatted_files.is_empty() {
        return;
    }
    let mut cmd = std::process::Command::new("git");
    cmd.arg("add");
    for f in &formatted_files {
        cmd.arg((*f).as_os_str());
    }
    match cmd.output() {
        Ok(output) if output.status.success() => {
            if !quiet {
                eprintln!(
                    "{} Re-staged {} formatted file{}",
                    "\u{2713}".green(),
                    formatted_files.len(),
                    if formatted_files.len() == 1 { "" } else { "s" }
                );
            }
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            eprintln!(
                "{}: Failed to re-stage formatted files: {}",
                "Warning".yellow(),
                stderr.trim()
            );
        }
        Err(e) => {
            eprintln!("{}: Failed to run git add: {}", "Warning".yellow(), e);
        }
    }
}

/// Run additional checks (security, complexity) and merge into the result.
fn run_additional_checks(
    result: &mut linthis::utils::types::RunResult,
    cli: &Cli,
    runtime_config: &linthis::config::Config,
    runtime_project_root: &std::path::Path,
    resolved_paths: &[std::path::PathBuf],
) {
    let checks_list = resolve_checks_list(&cli.checks, &runtime_config.checks.run);

    if checks_list.iter().any(|c| c == "lint") {
        result.checks_run.push("lint".to_string());
    }

    // Use resolved paths (from -s/staged, -m/modified, or -i/explicit)
    // instead of cli.paths which may be empty for -s mode.
    let target_files: Vec<std::path::PathBuf> = resolved_paths
        .iter()
        .filter(|p| p.is_file())
        .cloned()
        .collect();

    let cache_dir = linthis::utils::get_cache_dir();
    let security_cache_path = cache_dir.join("security-cache.json");
    let complexity_cache_path = cache_dir.join("complexity-cache.json");

    if checks_list.iter().any(|c| c == "security") {
        let security_config = runtime_config.checks.security.clone().unwrap_or_default();
        run_security_check(
            result,
            runtime_project_root,
            &target_files,
            &security_config,
            &security_cache_path,
            cli.no_cache,
            cli.quiet,
        );
    }

    if checks_list.iter().any(|c| c == "complexity") {
        let complexity_config = runtime_config.checks.complexity.clone().unwrap_or_default();
        run_complexity_check(
            result,
            runtime_project_root,
            &target_files,
            &complexity_config,
            &complexity_cache_path,
            cli.no_cache,
            cli.quiet,
        );
    }
}

/// Process a successful lint result: run additional checks, output, save, fix.
fn process_lint_result(
    mut result: linthis::utils::types::RunResult,
    cli: &Cli,
    runtime_config: &linthis::config::Config,
    runtime_project_root: &std::path::Path,
    output_format: OutputFormat,
    hook_type: Option<String>,
    resolved_paths: &[std::path::PathBuf],
) -> ExitCode {
    if cli.staged {
        auto_restage_formatted(&result, cli.quiet);
    }

    let lint_fail_on = runtime_config
        .checks
        .lint
        .as_ref()
        .and_then(|c| c.fail_on.clone())
        .unwrap_or_default();
    result.calculate_exit_code_with_fail_on(&lint_fail_on);

    result.target_paths = resolved_paths
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect();

    run_additional_checks(
        &mut result,
        cli,
        runtime_config,
        runtime_project_root,
        resolved_paths,
    );

    // Merge security/complexity issues into result.issues so all formatters
    // (including hook box output) can see them
    result.merge_all_check_issues();

    // Re-apply rule filter to catch complexity/security issues added by merge.
    // The lib.rs run() filter only saw lint issues; we must filter again here.
    {
        let linthisignore =
            linthis::utils::linthisignore::LinthisIgnore::load(runtime_project_root);
        let mut rules = runtime_config.rules.clone();
        rules.disable.extend(linthisignore.rule_codes);
        let filter = linthis::rules::RuleFilter::from_config(&rules);
        result.issues = filter.filter_issues(std::mem::take(&mut result.issues));
    }

    // Recount: the lint pass counted before the merge above, so a run whose
    // only issues came from security/complexity reported "in 0 files".
    result.count_files_with_issues();

    // Re-attach for the same reason: `run()` hinted the lint issues, but the
    // security and complexity ones only joined the list above. Idempotent.
    linthis::utils::ignore_hint::attach(&mut result.issues);

    let output = format_result_with_hook_type(&result, output_format, hook_type.as_deref());

    if (!cli.quiet || result.exit_code != 0) && !output.is_empty() {
        println!("{}", output);
    }

    if !cli.no_save_result || cli.output_file.is_some() {
        save_results(&result, &output, cli);
    }

    if result.exit_code != 0 && !cli.quiet {
        print_failure_summary(&result);
    }

    if let Some(exit) = handle_fix_mode(cli, &result) {
        return exit;
    }

    if !cli.quiet && !result.issues.is_empty() {
        print_fix_hint(&result.issues);
    }

    ExitCode::from(result.exit_code as u8)
}

/// Handle the `--clear-cache` flag. Returns `Some(ExitCode)` when `main`
/// should return early, `None` to continue.
fn handle_clear_cache(cli: &Cli) -> Option<ExitCode> {
    if !cli.clear_cache {
        return None;
    }
    let project_root = linthis::utils::get_project_root();
    if let Err(e) = linthis::cache::LintCache::clear(&project_root) {
        eprintln!("{}: {}", "Error clearing cache".red(), e);
        return Some(ExitCode::from(2));
    }
    if !cli.quiet {
        println!("{} Cache cleared", "\u{2713}".green());
    }
    if cli.paths.is_empty() && !cli.check_only && !cli.format_only {
        return Some(ExitCode::SUCCESS);
    }
    None
}

/// Handle early-exit flags (`--init`, `--init-configs`, `--benchmark`).
/// Returns `Some(ExitCode)` when `main` should return early.
fn handle_early_flags(cli: &Cli) -> Option<ExitCode> {
    if cli.init {
        return Some(handle_init_flag());
    }
    if cli.init_configs {
        return Some(init_linter_configs());
    }
    if cli.benchmark {
        return Some(run_benchmark(cli));
    }
    None
}

/// Determine the `RunMode` from CLI flags.
fn determine_run_mode(cli: &Cli) -> RunMode {
    if cli.check_only && cli.format_only {
        RunMode::Both
    } else if cli.check_only {
        RunMode::CheckOnly
    } else if cli.format_only {
        RunMode::FormatOnly
    } else {
        RunMode::Both
    }
}

/// Collect target paths and exclude patterns from CLI options.
/// Returns `Err(ExitCode)` on failure or empty input.
fn collect_target_paths(cli: &Cli) -> Result<(Vec<PathBuf>, Vec<String>), ExitCode> {
    let path_options = PathCollectionOptions {
        staged: cli.staged,
        since: cli.since.clone(),
        modified: cli.modified,
        no_default_excludes: cli.no_default_excludes,
        no_gitignore: cli.no_gitignore,
        exclude: cli.exclude.clone().unwrap_or_default(),
        paths: cli.paths.clone(),
        verbose: cli.verbose,
    };

    match collect_paths(&path_options) {
        PathCollectionResult::Success(p, e) => Ok((p, e)),
        PathCollectionResult::Empty(msg) => {
            if !cli.quiet {
                println!("{}", msg);
            }
            Err(ExitCode::SUCCESS)
        }
        PathCollectionResult::Error(msg, code) => {
            eprintln!("{}", msg);
            Err(ExitCode::from(code as u8))
        }
    }
}

/// Parse the output format and optional hook type from CLI flags.
fn parse_output_format(cli: &Cli) -> (OutputFormat, Option<String>) {
    if let Some(ref hook) = cli.hook_mode {
        (OutputFormat::Hook, Some(hook.clone()))
    } else {
        (
            OutputFormat::parse(&cli.output).unwrap_or(OutputFormat::Human),
            None,
        )
    }
}

/// Handle subcommands. Lint/Check subcommands modify `cli` flags and
/// fall through; all others dispatch and return an exit code.
fn handle_subcommands(cli: &mut Cli) -> Option<ExitCode> {
    let command = cli.command.take()?;
    if matches!(command, Commands::Lint { .. }) {
        cli.command = Some(command);
        apply_lint_subcommand(cli);
        None
    } else if matches!(command, Commands::Check { .. }) {
        cli.command = Some(command);
        apply_check_subcommand(cli);
        None
    } else {
        dispatch_subcommand(command)
    }
}

/// Expand the `--auto-fix` convenience flag.
fn expand_auto_fix(cli: &mut Cli) {
    if cli.auto_fix {
        cli.fix = true;
        cli.ai = true;
        cli.accept_all = true;
    }
}

/// Perform self-update and plugin auto-sync checks.
fn run_update_checks() {
    let project_root = linthis::utils::get_project_root();
    let config = linthis::config::Config::load_merged(&project_root);
    perform_self_update(config.self_auto_update.as_ref());
    perform_auto_sync(config.plugin_auto_sync.as_ref());
}

/// Build the `RunOptions` for the main lint/format run.
fn build_run_options(
    cli: &Cli,
    loaded_plugins: Vec<String>,
    config_resolver: ConfigResolver,
    mode: RunMode,
    paths: Vec<PathBuf>,
    exclude_patterns: Vec<String>,
    tool_install_mode: ToolInstallMode,
) -> RunOptions {
    let languages: Vec<Language> = cli
        .lang
        .clone()
        .unwrap_or_default()
        .iter()
        .filter_map(|s| Language::from_name(s))
        .collect();

    RunOptions {
        paths,
        mode,
        languages,
        exclude_patterns,
        verbose: cli.verbose,
        quiet: cli.quiet,
        plugins: loaded_plugins,
        no_cache: cli.no_cache,
        // Always present, even when empty: `None` means "caller did not
        // resolve plugins", which makes linthis::run fall back to the
        // installed ones — the opposite of what --no-plugin asks for.
        config_resolver: Some(Arc::new(config_resolver)),
        tool_install_mode,
        hook_event: cli.hook_mode.clone(),
    }
}

/// Execute the lint/format run and process results.
fn execute_run(
    cli: &Cli,
    options: &RunOptions,
    runtime_config: &linthis::config::Config,
    runtime_project_root: &std::path::Path,
    mode: RunMode,
) -> ExitCode {
    let (output_format, hook_type) = parse_output_format(cli);

    if cli.verbose {
        eprintln!(
            "{}",
            "linthis - Multi-language Linter & Formatter".bold().cyan()
        );
        eprintln!("Mode: {:?}", mode);
        eprintln!("Paths: {:?}", options.paths);
    }

    if matches!(mode, RunMode::Both | RunMode::FormatOnly) {
        cli::create_backup(&options.paths, "format (linthis main command)", cli.quiet);
    }

    let resolved_paths = options.paths.clone();
    match run(options) {
        Ok(result) => process_lint_result(
            result,
            cli,
            runtime_config,
            runtime_project_root,
            output_format,
            hook_type,
            &resolved_paths,
        ),
        Err(e) => {
            eprintln!("{}: {}", "Error".red().bold(), e);
            ExitCode::from(2)
        }
    }
}

/// Early exits that run before any lint work: a `linthis disable` in effect
/// for this hook, then `--clear-cache`.
fn handle_early_exits(cli: &Cli) -> Option<ExitCode> {
    if hook_event_name(cli).is_some_and(|event| linthis::state::skip_hook(&event)) {
        return Some(ExitCode::SUCCESS);
    }
    handle_clear_cache(cli)
}

/// The hook event this invocation is serving, if any. `--hook-event <e>` names
/// it directly; a bare `-o hook` only says "some hook", which is treated as
/// pre-commit for gating purposes.
fn hook_event_name(cli: &Cli) -> Option<String> {
    cli.hook_mode
        .clone()
        .or_else(|| (cli.output.eq_ignore_ascii_case("hook")).then(|| "pre-commit".to_string()))
}

fn main() -> ExitCode {
    env_logger::init();

    let mut cmd = Cli::command();
    if is_help_requested() {
        inject_dynamic_help(&mut cmd);
    }
    let matches = cmd.get_matches();
    let mut cli = Cli::from_arg_matches(&matches).unwrap_or_else(|e| e.exit());

    // Global flag: record it before any subcommand dispatch, since format/fix/
    // watch build their own RunOptions and never see `cli`.
    linthis::set_plugins_disabled(cli.no_plugin);

    if let Some(exit) = handle_subcommands(&mut cli) {
        return exit;
    }

    // `linthis disable`: hooks that call this binary directly (plugin scripts,
    // legacy embedded hooks) identify themselves with --hook-event / -o hook.
    // Gate them here, before any backup or plugin work, so a disable applies
    // whatever hook flavour is installed. Manual runs have no hook output
    // format and are never gated.
    if let Some(exit) = handle_early_exits(&cli) {
        return exit;
    }

    expand_auto_fix(&mut cli);

    if let Some(exit) = validate_cli_flags(&cli) {
        return exit;
    }

    run_update_checks();

    // Migrate legacy project-local data dirs to global path (one-time, silent)
    linthis::utils::migrate_legacy_data_dirs();

    let (loaded_plugins, config_resolver) = match load_plugins(&cli) {
        Ok(result) => result,
        Err(exit) => return exit,
    };

    if let Some(exit) = handle_early_flags(&cli) {
        return exit;
    }

    let mode = determine_run_mode(&cli);

    let (paths, exclude_patterns) = match collect_target_paths(&cli) {
        Ok(result) => result,
        Err(exit) => return exit,
    };

    let runtime_project_root = linthis::utils::get_project_root();
    let runtime_config = linthis::config::Config::load_merged(&runtime_project_root);

    let tool_install_mode = resolve_tool_install_mode(cli.no_tool_auto_install, &runtime_config);

    let options = build_run_options(
        &cli,
        loaded_plugins,
        config_resolver,
        mode,
        paths,
        exclude_patterns,
        tool_install_mode,
    );

    // Pre-flight: auto-create .gitignore if missing in a git repo
    if !matches!(mode, RunMode::FormatOnly) {
        let is_pre_push = cli.hook_mode.as_deref() == Some("pre-push");
        if let Some(violations) =
            linthis::gitignore::check_and_create(&runtime_project_root, cli.quiet, is_pre_push)
        {
            if !violations.is_empty() {
                if !cli.quiet {
                    if is_pre_push {
                        eprintln!("Warning: Committed files should be ignored. Fix with:");
                        for v in &violations {
                            eprintln!("  git rm --cached {}", v);
                        }
                        eprintln!("  git commit --amend --no-edit");
                    } else {
                        eprintln!(
                            "Warning: Staged files should be ignored (remove with git rm --cached):"
                        );
                        for v in &violations {
                            eprintln!("  git rm --cached {}", v);
                        }
                    }
                }
                return ExitCode::from(1);
            }
        }
    }

    execute_run(&cli, &options, &runtime_config, &runtime_project_root, mode)
}

// PerFileCache is now in linthis::cache::checks_cache
use linthis::cache::PerFileCache;

#[cfg(test)]
mod skip_checks_tests {
    use super::{apply_skip_checks_env, resolve_skip_token};
    use std::sync::Mutex;

    // Env-var mutation is process-wide; serialize these tests.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn with_env<F: FnOnce() -> T, T>(value: Option<&str>, f: F) -> T {
        let _guard = ENV_LOCK.lock().unwrap();
        let prev = std::env::var("LINTHIS_SKIP_CHECKS").ok();
        match value {
            Some(v) => std::env::set_var("LINTHIS_SKIP_CHECKS", v),
            None => std::env::remove_var("LINTHIS_SKIP_CHECKS"),
        }
        let out = f();
        match prev {
            Some(v) => std::env::set_var("LINTHIS_SKIP_CHECKS", v),
            None => std::env::remove_var("LINTHIS_SKIP_CHECKS"),
        }
        out
    }

    fn base() -> Vec<String> {
        vec!["lint".into(), "security".into(), "complexity".into()]
    }

    #[test]
    fn resolves_full_names() {
        assert_eq!(resolve_skip_token("lint"), Some("lint"));
        assert_eq!(resolve_skip_token("security"), Some("security"));
        assert_eq!(resolve_skip_token("complexity"), Some("complexity"));
    }

    #[test]
    fn resolves_three_char_prefixes() {
        assert_eq!(resolve_skip_token("lin"), Some("lint"));
        assert_eq!(resolve_skip_token("sec"), Some("security"));
        assert_eq!(resolve_skip_token("com"), Some("complexity"));
    }

    #[test]
    fn resolves_case_insensitively() {
        assert_eq!(resolve_skip_token("LIN"), Some("lint"));
        assert_eq!(resolve_skip_token("Sec"), Some("security"));
    }

    #[test]
    fn rejects_too_short() {
        assert_eq!(resolve_skip_token("li"), None);
        assert_eq!(resolve_skip_token("s"), None);
        assert_eq!(resolve_skip_token(""), None);
    }

    #[test]
    fn rejects_unknown() {
        assert_eq!(resolve_skip_token("foo"), None);
        assert_eq!(resolve_skip_token("formatting"), None);
    }

    #[test]
    fn env_unset_is_noop() {
        let out = with_env(None, || apply_skip_checks_env(base()));
        assert_eq!(out, base());
    }

    #[test]
    fn env_empty_is_noop() {
        let out = with_env(Some(""), || apply_skip_checks_env(base()));
        assert_eq!(out, base());
    }

    #[test]
    fn env_filters_single_full_name() {
        let out = with_env(Some("lint"), || apply_skip_checks_env(base()));
        assert_eq!(out, vec!["security".to_string(), "complexity".to_string()]);
    }

    #[test]
    fn env_filters_via_prefix() {
        let out = with_env(Some("com"), || apply_skip_checks_env(base()));
        assert_eq!(out, vec!["lint".to_string(), "security".to_string()]);
    }

    #[test]
    fn env_filters_multiple() {
        let out = with_env(Some("lin,com"), || apply_skip_checks_env(base()));
        assert_eq!(out, vec!["security".to_string()]);
    }

    #[test]
    fn env_unknown_token_ignored() {
        // Unknown tokens are warned to stderr but do not filter anything.
        let out = with_env(Some("foo"), || apply_skip_checks_env(base()));
        assert_eq!(out, base());
    }

    #[test]
    fn env_mixed_valid_and_invalid() {
        let out = with_env(Some("lin,foo"), || apply_skip_checks_env(base()));
        assert_eq!(out, vec!["security".to_string(), "complexity".to_string()]);
    }
}

#[cfg(test)]
mod install_mode_tests {
    use super::{parse_install_mode_env, resolve_tool_install_mode};
    use linthis::config::{Config, ToolAutoInstallConfig};
    use linthis::ToolInstallMode;
    use std::sync::Mutex;

    // Env-var mutation is process-wide; serialize these tests.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn with_env<F: FnOnce() -> T, T>(value: Option<&str>, f: F) -> T {
        let _guard = ENV_LOCK.lock().unwrap();
        let prev = std::env::var("LINTHIS_INSTALL_MODE").ok();
        match value {
            Some(v) => std::env::set_var("LINTHIS_INSTALL_MODE", v),
            None => std::env::remove_var("LINTHIS_INSTALL_MODE"),
        }
        let out = f();
        match prev {
            Some(v) => std::env::set_var("LINTHIS_INSTALL_MODE", v),
            None => std::env::remove_var("LINTHIS_INSTALL_MODE"),
        }
        out
    }

    fn config_with_mode(mode: &str) -> Config {
        Config {
            tool_auto_install: Some(ToolAutoInstallConfig {
                enabled: true,
                mode: mode.to_string(),
            }),
            ..Config::default()
        }
    }

    #[test]
    fn parses_valid_tokens_case_insensitively() {
        assert_eq!(parse_install_mode_env("auto"), Some(ToolInstallMode::Auto));
        assert_eq!(
            parse_install_mode_env("  PROMPT "),
            Some(ToolInstallMode::Prompt)
        );
        assert_eq!(
            parse_install_mode_env("Disabled"),
            Some(ToolInstallMode::Disabled)
        );
    }

    #[test]
    fn rejects_unknown_or_empty_tokens() {
        assert_eq!(parse_install_mode_env(""), None);
        assert_eq!(parse_install_mode_env("yes"), None);
    }

    #[test]
    fn env_var_overrides_config() {
        // Config says prompt, but the env var forces auto (the CI case).
        let cfg = config_with_mode("prompt");
        let out = with_env(Some("auto"), || resolve_tool_install_mode(false, &cfg));
        assert_eq!(out, ToolInstallMode::Auto);
    }

    #[test]
    fn no_install_flag_beats_env_var() {
        // Explicit --no-tool-auto-install wins even over LINTHIS_INSTALL_MODE=auto.
        let cfg = Config::default();
        let out = with_env(Some("auto"), || resolve_tool_install_mode(true, &cfg));
        assert_eq!(out, ToolInstallMode::Disabled);
    }

    #[test]
    fn unknown_env_falls_through_to_config() {
        let cfg = config_with_mode("disabled");
        let out = with_env(Some("bogus"), || resolve_tool_install_mode(false, &cfg));
        assert_eq!(out, ToolInstallMode::Disabled);
    }

    #[test]
    fn unset_env_uses_config() {
        let cfg = config_with_mode("auto");
        let out = with_env(None, || resolve_tool_install_mode(false, &cfg));
        assert_eq!(out, ToolInstallMode::Auto);
    }

    #[test]
    fn unset_env_and_no_config_defaults_to_prompt() {
        let cfg = Config::default();
        let out = with_env(None, || resolve_tool_install_mode(false, &cfg));
        assert_eq!(out, ToolInstallMode::Prompt);
    }
}
