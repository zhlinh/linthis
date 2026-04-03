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
    collect_paths, handle_cache_command, handle_commit_msg_check, handle_complexity_command,
    handle_config_command, handle_doctor_command, handle_fix_command, handle_format_command,
    handle_hook_command, handle_init_command, handle_license_command, handle_plugin_command,
    handle_report_command, handle_review_command, handle_security_command, init_linter_configs,
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

    // Inject into "hook install" subcommand
    if let Some(hook_cmd) = cmd.find_subcommand_mut("hook") {
        if let Some(install_cmd) = hook_cmd.find_subcommand_mut("install") {
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
        Commands::Cmsg {
            msg_or_file,
            auto_fix,
            provider,
        } => Some(handle_commit_msg_check(
            &msg_or_file,
            auto_fix,
            provider.as_deref(),
        )),
        Commands::Init {
            global,
            with_hook,
            force,
        } => Some(handle_init_command(global, with_hook, force)),
        Commands::Doctor { all, output } => Some(handle_doctor_command(all, &output)),
        Commands::Cache { action } => Some(handle_cache_command(action)),
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
        } => Some(handle_security_command(cli::SecurityCommandParams {
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
        })),
        Commands::License {
            path,
            policy,
            policy_file,
            include_dev,
            format,
            sbom,
            fail_on_violation,
            verbose,
        } => Some(handle_license_command(cli::LicenseCommandParams {
            path,
            policy,
            policy_file,
            include_dev,
            format,
            sbom,
            fail_on_violation,
            verbose,
        })),
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
        } => Some(handle_complexity_command(ComplexityCommandOptions {
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
        })),
        Commands::Format {
            paths,
            staged,
            modified,
            exclude,
            undo,
            source,
            list_backups,
            verbose,
            quiet,
        } => Some(handle_format_command(FormatCommandOptions {
            paths,
            staged,
            modified,
            exclude,
            undo,
            source,
            list_backups,
            verbose,
            quiet,
        })),
        Commands::Fix {
            source,
            check,
            format_only,
            auto_fix,
            ai,
            provider,
            model,
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
        } => {
            let (ai, accept_all) = if auto_fix {
                (true, true)
            } else {
                (ai, accept_all)
            };
            Some(handle_fix_command(FixCommandOptions {
                source,
                check,
                format_only,
                ai,
                provider,
                model,
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
            }))
        }
        Commands::Review {
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
        } => Some(handle_review_command(ReviewCommandOptions {
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
        })),
        Commands::Lsp {
            mode,
            port,
            use_plugin,
        } => Some(handle_lsp_subcommand(mode, port, use_plugin)),
        Commands::Report { action } => Some(handle_report_command(action)),
        Commands::Watch {
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
        } => Some(handle_watch_subcommand(WatchSubcommandArgs {
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
        })),
        // Lint and Check fall through to main flow
        Commands::Lint { .. } | Commands::Check { .. } => None,
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
    let languages: Vec<Language> = args.lang
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

/// Resolve the `ToolInstallMode` from CLI flags and config.
fn resolve_tool_install_mode(
    no_tool_auto_install: bool,
    runtime_config: &linthis::config::Config,
) -> ToolInstallMode {
    if no_tool_auto_install {
        ToolInstallMode::Disabled
    } else {
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
}

/// Resolve which checks to run from CLI flags and config.
fn resolve_checks_list(cli_checks: &Option<Vec<String>>, config_checks: &[String]) -> Vec<String> {
    if let Some(ref cli_checks) = cli_checks {
        if cli_checks.iter().any(|c| c == "all") {
            vec!["lint".into(), "security".into(), "complexity".into()]
        } else {
            cli_checks.clone()
        }
    } else {
        config_checks.to_vec()
    }
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
    let cx_high = analysis.thresholds.cyclomatic.high;
    let cx_warning = analysis.thresholds.cyclomatic.warning;
    let cx_threshold = analysis.thresholds.cyclomatic.good;
    let cx_errors = analysis
        .files
        .iter()
        .flat_map(|f| &f.functions)
        .filter(|func| func.metrics.cyclomatic > cx_high)
        .count();
    let cx_warns = analysis
        .files
        .iter()
        .flat_map(|f| &f.functions)
        .filter(|func| func.metrics.cyclomatic > cx_warning && func.metrics.cyclomatic <= cx_high)
        .count();
    let cx_infos = analysis
        .files
        .iter()
        .flat_map(|f| &f.functions)
        .filter(|func| {
            func.metrics.cyclomatic > cx_threshold && func.metrics.cyclomatic <= cx_warning
        })
        .count();
    let cx_exit = cx_fail_on.exit_code(cx_errors, cx_warns, cx_infos);
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
        let result_dir = project_root.join(".linthis").join("result");
        if let Err(e) = fs::create_dir_all(&result_dir) {
            eprintln!(
                "{}: Failed to create {}: {}",
                "Warning".yellow(),
                result_dir.display(),
                e
            );
            return;
        }
        // Ensure .linthis/ is in .gitignore so it doesn't pollute the user's repo
        linthis::utils::ensure_gitignore_has_linthis(&project_root);
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

    let result_dir = PathBuf::from(".linthis").join("result");
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
    let cx_high = cx.thresholds.cyclomatic.high;
    let cx_warning = cx.thresholds.cyclomatic.warning;
    let cx_good = cx.thresholds.cyclomatic.good;
    let cx_errors = cx
        .files
        .iter()
        .flat_map(|f| &f.functions)
        .filter(|func| func.metrics.cyclomatic > cx_high)
        .count();
    let cx_warns = cx
        .files
        .iter()
        .flat_map(|f| &f.functions)
        .filter(|func| func.metrics.cyclomatic > cx_warning && func.metrics.cyclomatic <= cx_high)
        .count();
    let cx_infos = cx
        .files
        .iter()
        .flat_map(|f| &f.functions)
        .filter(|func| func.metrics.cyclomatic > cx_good && func.metrics.cyclomatic <= cx_warning)
        .count();
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
) {
    let checks_list = resolve_checks_list(&cli.checks, &runtime_config.checks.run);

    if checks_list.iter().any(|c| c == "lint") {
        result.checks_run.push("lint".to_string());
    }

    let target_files: Vec<std::path::PathBuf> = cli
        .paths
        .iter()
        .filter(|p| p.is_file())
        .map(|p| p.to_path_buf())
        .collect();

    let cache_dir = runtime_project_root.join(".linthis");
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

    result.target_paths = cli
        .paths
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect();

    run_additional_checks(&mut result, cli, runtime_config, runtime_project_root);

    // Merge security/complexity issues into result.issues so all formatters
    // (including hook box output) can see them
    result.merge_all_check_issues();

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
        config_resolver: if config_resolver.is_empty() {
            None
        } else {
            Some(Arc::new(config_resolver))
        },
        tool_install_mode,
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

    match run(options) {
        Ok(result) => process_lint_result(
            result,
            cli,
            runtime_config,
            runtime_project_root,
            output_format,
            hook_type,
        ),
        Err(e) => {
            eprintln!("{}: {}", "Error".red().bold(), e);
            ExitCode::from(2)
        }
    }
}

fn main() -> ExitCode {
    env_logger::init();

    let mut cmd = Cli::command();
    inject_dynamic_help(&mut cmd);
    let matches = cmd.get_matches();
    let mut cli = Cli::from_arg_matches(&matches).unwrap_or_else(|e| e.exit());

    if let Some(exit) = handle_subcommands(&mut cli) {
        return exit;
    }

    if let Some(exit) = handle_clear_cache(&cli) {
        return exit;
    }

    expand_auto_fix(&mut cli);

    if let Some(exit) = validate_cli_flags(&cli) {
        return exit;
    }

    run_update_checks();

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

    execute_run(&cli, &options, &runtime_config, &runtime_project_root, mode)
}

// PerFileCache is now in linthis::cache::checks_cache
use linthis::cache::PerFileCache;
