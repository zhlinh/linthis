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
    perform_auto_sync, perform_self_update, print_fix_hint, run_benchmark, run_watch,
    run_complexity_analysis, run_sast_scan, strip_ansi_codes, Cli, Commands,
    ComplexityCommandOptions, FixCommandOptions, FormatCommandOptions, PathCollectionOptions,
    PathCollectionResult, ReviewCommandOptions,
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
        let (_, name, desc) = ALL_AI_PROVIDERS
            .iter()
            .find(|(k, _, _)| k == kind)
            .unwrap();
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

fn main() -> ExitCode {
    env_logger::init();

    let mut cmd = Cli::command();
    inject_dynamic_help(&mut cmd);
    let matches = cmd.get_matches();
    let mut cli = Cli::from_arg_matches(&matches).unwrap_or_else(|e| e.exit());

    // Handle plugin subcommands first
    if let Some(Commands::Plugin { action }) = cli.command {
        return handle_plugin_command(action);
    }

    // Handle config subcommands
    if let Some(Commands::Config { action }) = cli.command {
        return handle_config_command(action);
    }

    // Handle hook subcommands
    if let Some(Commands::Hook { action }) = cli.command {
        return handle_hook_command(action);
    }

    // Handle cmsg subcommand (commit message validation)
    if let Some(Commands::Cmsg { msg_or_file, auto_fix, provider }) = cli.command {
        return handle_commit_msg_check(&msg_or_file, auto_fix, provider.as_deref());
    }

    // Handle init subcommand
    if let Some(Commands::Init { global, with_hook, force }) = cli.command {
        return handle_init_command(global, with_hook, force);
    }

    // Handle doctor subcommand
    if let Some(Commands::Doctor { all, output }) = cli.command {
        return handle_doctor_command(all, &output);
    }

    // Handle cache subcommand
    if let Some(Commands::Cache { action }) = cli.command {
        return handle_cache_command(action);
    }

    // Handle security subcommand
    if let Some(Commands::Security {
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
    }) = cli.command
    {
        return handle_security_command(
            path, scan_type, severity, include_dev, fix, ignore, format, sbom, fail_on,
            sast_config, verbose,
        );
    }

    // Handle license subcommand
    if let Some(Commands::License {
        path,
        policy,
        policy_file,
        include_dev,
        format,
        sbom,
        fail_on_violation,
        verbose,
    }) = cli.command
    {
        return handle_license_command(
            path, policy, policy_file, include_dev, format, sbom, fail_on_violation, verbose,
        );
    }

    // Handle complexity subcommand
    if let Some(Commands::Complexity {
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
        fail_on_high,
        verbose,
    }) = cli.command
    {
        return handle_complexity_command(ComplexityCommandOptions {
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
            fail_on_high,
            verbose,
        });
    }

    // Handle format subcommand
    if let Some(Commands::Format {
        paths,
        staged,
        modified,
        exclude,
        undo,
        source,
        list_backups,
        verbose,
        quiet,
    }) = cli.command
    {
        return handle_format_command(FormatCommandOptions {
            paths,
            staged,
            modified,
            exclude,
            undo,
            source,
            list_backups,
            verbose,
            quiet,
        });
    }

    // Handle fix subcommand
    if let Some(Commands::Fix {
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
    }) = cli.command
    {
        // --auto expands to --ai -y
        let (ai, accept_all) = if auto_fix {
            (true, true)
        } else {
            (ai, accept_all)
        };
        return handle_fix_command(FixCommandOptions {
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
        });
    }

    // Handle review subcommand
    if let Some(Commands::Review {
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
    }) = cli.command
    {
        return handle_review_command(ReviewCommandOptions {
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
        });
    }

    // Handle lsp subcommand
    if let Some(Commands::Lsp { mode, port, use_plugin }) = cli.command {
        // Build ConfigResolver for LSP (instead of copying configs)
        let mut lsp_config_resolver = ConfigResolver::new();

        // Load plugins before starting LSP
        if let Some(ref plugin_specs) = use_plugin {
            use linthis::plugin::{PluginLoader, PluginSource};

            for spec in plugin_specs {
                // Parse plugin spec: URL[@ref] or local path
                let (url_or_path, git_ref) = if spec.contains('@') && !spec.starts_with('/') {
                    let parts: Vec<&str> = spec.rsplitn(2, '@').collect();
                    if parts.len() == 2 {
                        (parts[1].to_string(), Some(parts[0].to_string()))
                    } else {
                        (spec.clone(), None)
                    }
                } else {
                    (spec.clone(), None)
                };

                let name = url_or_path
                    .rsplit('/')
                    .next()
                    .unwrap_or(&url_or_path)
                    .trim_end_matches(".git")
                    .to_string();

                let source = if let Some(ref r) = git_ref {
                    PluginSource::new(&url_or_path).with_ref(r)
                } else {
                    PluginSource::new(&url_or_path)
                };

                if let Ok(loader) = PluginLoader::new() {
                    if let Ok(configs) = loader.load_configs(&[source], false) {
                        // Add configs to resolver (no more copying to .linthis/configs/)
                        for config in &configs {
                            lsp_config_resolver.add_config(ResolvedConfig::new(
                                config.language.clone(),
                                config.tool.clone(),
                                config.config_path.clone(),
                                ConfigSource::CliPlugin,
                                name.clone(),
                            ));
                        }
                        eprintln!("[lsp] Loaded {} config(s) from plugin '{}'", configs.len(), name);
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

        // Run LSP server using tokio runtime with ConfigResolver
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
            Ok(_) => return ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("{}: LSP server error: {}", "Error".red(), e);
                return ExitCode::from(1);
            }
        }
    }

    // Handle report subcommand
    if let Some(Commands::Report { action }) = cli.command {
        return handle_report_command(action);
    }

    // Handle watch subcommand
    if let Some(Commands::Watch {
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
    }) = cli.command
    {
        // Parse languages
        let languages: Vec<Language> = lang
            .unwrap_or_default()
            .iter()
            .filter_map(|s| Language::from_name(s))
            .collect();

        let config = linthis::watch::WatchConfig {
            paths,
            check_only,
            format_only,
            debounce_ms: debounce,
            notify,
            no_tui,
            clear,
            verbose,
            languages,
            exclude_patterns: exclude.unwrap_or_default(),
        };

        match run_watch(config) {
            Ok(_) => return ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("{}: {}", "Error".red(), e);
                return ExitCode::from(1);
            }
        }
    }

    // Handle --clear-cache flag
    if cli.clear_cache {
        let project_root = linthis::utils::get_project_root();
        if let Err(e) = linthis::cache::LintCache::clear(&project_root) {
            eprintln!("{}: {}", "Error clearing cache".red(), e);
            return ExitCode::from(2);
        }
        if !cli.quiet {
            println!("{} Cache cleared", "✓".green());
        }
        // If only --clear-cache is specified, exit
        if cli.paths.is_empty() && !cli.check_only && !cli.format_only {
            return ExitCode::SUCCESS;
        }
    }

    // Expand --auto-fix to --fix --ai -y
    if cli.auto_fix {
        cli.fix = true;
        cli.ai = true;
        cli.accept_all = true;
    }

    // Validate flag dependencies (since we removed clap `requires` for --auto-fix compat)
    if cli.ai && !cli.fix {
        eprintln!("{}: --ai requires --fix or --auto-fix", "Error".red());
        return ExitCode::from(2);
    }
    if cli.provider.is_some() && !cli.ai && !cli.auto_fix {
        eprintln!("{}: --provider requires --ai or --auto-fix", "Error".red());
        return ExitCode::from(2);
    }
    if cli.accept_all && !cli.fix {
        eprintln!("{}: -y/--yes requires --fix or --auto-fix", "Error".red());
        return ExitCode::from(2);
    }

    // Perform self-update and auto-sync checks (before loading plugins)
    // Load config to get self_auto_update and plugin_auto_sync settings
    {
        let project_root = linthis::utils::get_project_root();
        let config = linthis::config::Config::load_merged(&project_root);

        // Perform self-update if configured
        let self_update_config = config.self_auto_update.as_ref();
        perform_self_update(self_update_config);

        // Perform auto-sync if configured
        let auto_sync_config = config.plugin_auto_sync.as_ref();
        perform_auto_sync(auto_sync_config);
    }

    // Track loaded plugins for display
    let mut loaded_plugins: Vec<String> = Vec::new();

    // Build ConfigResolver for plugin configs (instead of copying to .linthis/configs/)
    // Priority order: CLI plugins (2) > Project plugins (3) > Global plugins (4)
    // Local manual configs (1) are checked first by the resolver at runtime
    let mut config_resolver = ConfigResolver::new();

    // Load plugins: --use-plugin takes priority, then config files
    if !cli.no_plugin {
        use linthis::plugin::{PluginConfigManager, PluginLoader, PluginSource};

        // Track plugins with their source type for ConfigResolver
        let mut cli_plugins: Vec<(String, PluginSource)> = Vec::new();
        let mut project_plugins: Vec<(String, PluginSource)> = Vec::new();
        let mut global_plugins: Vec<(String, PluginSource)> = Vec::new();

        // Check --use-plugin first (takes priority over config files)
        if let Some(ref plugin_specs) = cli.use_plugin {
            for spec in plugin_specs {
                // Parse plugin spec: URL[@ref] or local path
                let (url_or_path, git_ref) = if spec.contains('@') && !spec.starts_with('/') {
                    // URL with ref: https://github.com/org/plugin.git@v1.0
                    let parts: Vec<&str> = spec.rsplitn(2, '@').collect();
                    if parts.len() == 2 {
                        (parts[1].to_string(), Some(parts[0].to_string()))
                    } else {
                        (spec.clone(), None)
                    }
                } else {
                    (spec.clone(), None)
                };

                // Generate plugin name from URL/path
                let name = url_or_path
                    .rsplit('/')
                    .next()
                    .unwrap_or(&url_or_path)
                    .trim_end_matches(".git")
                    .to_string();

                let source = if let Some(ref r) = git_ref {
                    PluginSource::new(&url_or_path).with_ref(r)
                } else {
                    PluginSource::new(&url_or_path)
                };

                if cli.verbose {
                    eprintln!("Using plugin from CLI: {} ({})", name, url_or_path);
                }
                cli_plugins.push((name, source));
            }
        } else {
            // No --use-plugin, load from config files (project first, then global)
            // Check project config first
            if let Ok(project_manager) = PluginConfigManager::project() {
                if let Ok(plugins) = project_manager.list_plugins() {
                    for (name, url, git_ref) in plugins {
                        let source = if let Some(ref r) = git_ref {
                            PluginSource::new(&url).with_ref(r)
                        } else {
                            PluginSource::new(&url)
                        };
                        project_plugins.push((name, source));
                    }
                }
            }

            // If no project plugins, check global config
            if project_plugins.is_empty() {
                if let Ok(global_manager) = PluginConfigManager::global() {
                    if let Ok(plugins) = global_manager.list_plugins() {
                        for (name, url, git_ref) in plugins {
                            let source = if let Some(ref r) = git_ref {
                                PluginSource::new(&url).with_ref(r)
                            } else {
                                PluginSource::new(&url)
                            };
                            global_plugins.push((name, source));
                        }
                    }
                }
            }
        }

        // Load all plugins and build ConfigResolver
        let all_plugins = [
            (cli_plugins, ConfigSource::CliPlugin),
            (project_plugins, ConfigSource::ProjectPlugin),
            (global_plugins, ConfigSource::GlobalPlugin),
        ];

        for (plugins, source_type) in all_plugins {
            if plugins.is_empty() {
                continue;
            }

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

            for (plugin_name, source) in plugins {
                match loader.load_configs(&[source], false) {
                    Ok(configs) => {
                        loaded_plugins.push(plugin_name.clone());
                        if cli.verbose {
                            eprintln!(
                                "Loaded {} config(s) from plugin '{}' (priority: {:?})",
                                configs.len(),
                                plugin_name,
                                source_type
                            );
                        }

                        // Add configs to resolver (no more copying to .linthis/configs/)
                        for config in &configs {
                            config_resolver.add_config(ResolvedConfig::new(
                                config.language.clone(),
                                config.tool.clone(),
                                config.config_path.clone(),
                                source_type,
                                plugin_name.clone(),
                            ));

                            if cli.verbose {
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
                        // Continue with defaults - don't fail the entire run
                    }
                }
            }
        }
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
    let mode = if cli.check_only && cli.format_only {
        RunMode::Both
    } else if cli.check_only {
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

    // Collect paths using the paths module
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

    let (paths, exclude_patterns) = match collect_paths(&path_options) {
        PathCollectionResult::Success(p, e) => (p, e),
        PathCollectionResult::Empty(msg) => {
            if !cli.quiet {
                println!("{}", msg);
            }
            return ExitCode::SUCCESS;
        }
        PathCollectionResult::Error(msg, code) => {
            eprintln!("{}", msg);
            return ExitCode::from(code as u8);
        }
    };

    // Load config for tool_auto_install and other runtime settings
    let runtime_project_root = linthis::utils::get_project_root();
    let runtime_config = linthis::config::Config::load_merged(&runtime_project_root);

    // Resolve tool_install_mode: CLI flag > config > default
    let tool_install_mode = if cli.no_tool_auto_install {
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
    };

    // Build options with ConfigResolver for plugin configs
    let options = RunOptions {
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
    };

    // Parse output format (hook_mode overrides output format)
    let (output_format, hook_type) = if let Some(ref hook) = cli.hook_mode {
        (OutputFormat::Hook, Some(hook.clone()))
    } else {
        (OutputFormat::parse(&cli.output).unwrap_or(OutputFormat::Human), None)
    };

    if cli.verbose {
        eprintln!(
            "{}",
            "linthis - Multi-language Linter & Formatter".bold().cyan()
        );
        eprintln!("Mode: {:?}", mode);
        eprintln!("Paths: {:?}", options.paths);
    }

    // Backup files before formatting (Both or FormatOnly mode)
    if matches!(mode, RunMode::Both | RunMode::FormatOnly) {
        cli::create_backup(&options.paths, "format (linthis main command)", cli.quiet);
    }

    // Run linthis
    match run(&options) {
        Ok(mut result) => {
            // Auto re-stage formatted files when running in staged mode (-s)
            if cli.staged && !result.format_results.is_empty() {
                let formatted_files: Vec<&PathBuf> = result
                    .format_results
                    .iter()
                    .filter(|r| r.changed)
                    .map(|r| &r.file_path)
                    .collect();
                if !formatted_files.is_empty() {
                    let mut cmd = std::process::Command::new("git");
                    cmd.arg("add");
                    for f in &formatted_files {
                        cmd.arg(f.as_os_str());
                    }
                    match cmd.output() {
                        Ok(output) if output.status.success() => {
                            if !cli.quiet {
                                eprintln!(
                                    "{} Re-staged {} formatted file{}",
                                    "✓".green(),
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
                            eprintln!(
                                "{}: Failed to run git add: {}",
                                "Warning".yellow(),
                                e
                            );
                        }
                    }
                }
            }

            // Record target paths for trend analysis scope tracking
            result.target_paths = cli.paths.iter().map(|p| p.to_string_lossy().to_string()).collect();

            // --- Run additional checks (--checks / config checks.run) ---
            let checks_list: Vec<String> = if let Some(ref cli_checks) = cli.checks {
                if cli_checks.iter().any(|c| c == "all") {
                    vec!["lint".into(), "security".into(), "complexity".into()]
                } else {
                    cli_checks.clone()
                }
            } else {
                runtime_config.checks.run.clone()
            };

            // Mark lint as run (it always runs unless explicitly excluded via --checks)
            if checks_list.iter().any(|c| c == "lint") {
                result.checks_run.push("lint".to_string());
            }

            // Run security check if in checks list
            if checks_list.iter().any(|c| c == "security") {
                let security_config = runtime_config
                    .checks
                    .security
                    .clone()
                    .unwrap_or_default();
                if !cli.quiet {
                    eprintln!("🔒 Running security check...");
                }
                // Pass specific files if -i was used, otherwise scan project root
                let security_files: Vec<std::path::PathBuf> = cli
                    .paths
                    .iter()
                    .filter(|p| p.is_file())
                    .map(|p| p.to_path_buf())
                    .collect();
                let sast_result = run_sast_scan(
                    &runtime_project_root,
                    &security_files,
                    &security_config,
                );
                if sast_result.critical_high_count() > 0 {
                    result.exit_code = std::cmp::max(result.exit_code, 1);
                }
                result.security = Some(sast_result);
                result.checks_run.push("security".to_string());
            }

            // Run complexity check if in checks list
            if checks_list.iter().any(|c| c == "complexity") {
                let complexity_config = runtime_config
                    .checks
                    .complexity
                    .clone()
                    .unwrap_or_default();
                if !cli.quiet {
                    eprintln!("📊 Running complexity check...");
                }
                // Use CLI-specified files if available
                let checked_files: Vec<std::path::PathBuf> = cli
                    .paths
                    .iter()
                    .filter(|p| p.is_file())
                    .map(|p| p.to_path_buf())
                    .collect();
                // Catch panics from complexity analyzer (e.g., parser bugs in certain files)
                let complexity_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    run_complexity_analysis(
                        &runtime_project_root,
                        &checked_files,
                        &complexity_config,
                    )
                }));
                match complexity_result {
                    Ok(Ok(analysis)) => {
                        if complexity_config.fail_on_high.unwrap_or(false)
                            && analysis.summary.high_complexity_files > 0
                        {
                            result.exit_code = std::cmp::max(result.exit_code, 1);
                        }
                        result.complexity = Some(analysis);
                    }
                    Ok(Err(e)) => {
                        if !cli.quiet {
                            eprintln!("Complexity analysis error: {}", e);
                        }
                    }
                    Err(_) => {
                        if !cli.quiet {
                            eprintln!("Complexity analysis encountered an internal error");
                        }
                    }
                }
                result.checks_run.push("complexity".to_string());
            }

            // Output results
            let output = format_result_with_hook_type(&result, output_format, hook_type.as_deref());

            // Print to console
            if (!cli.quiet || result.exit_code != 0) && !output.is_empty() {
                println!("{}", output);
            }

            // Save to file by default (unless --no-save-result is specified)
            // Default format is JSON for programmatic access (--last, --from-result)
            if !cli.no_save_result || cli.output_file.is_some() {
                use chrono::Local;
                use std::fs::{self, File};
                use std::io::Write;

                // Get project root for .linthis directory
                let project_root = linthis::utils::get_project_root();

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
                    // Use default path: <project_root>/.linthis/result/result-{timestamp}.json
                    let result_dir = project_root.join(".linthis").join("result");
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
                    result_dir.join(format!("result-{}.json", timestamp))
                };

                // Serialize result as JSON for default files, or use specified format for custom path
                let file_content = if cli.output_file.is_some() {
                    // Custom path: use the output format specified by user
                    strip_ansi_codes(&output)
                } else {
                    // Default path: always save as unified JSON for --last/--from-result support
                    linthis::utils::output::format_result_json(&result)
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
                                let name = e.file_name().to_string_lossy().to_string();
                                name.starts_with("result-")
                                    && (name.ends_with(".json") || name.ends_with(".txt"))
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

            // Show failure message if exit code is non-zero
            if result.exit_code != 0 && !cli.quiet {
                eprintln!();
                match result.exit_code {
                    1 => {
                        eprintln!("{} {} {}",
                            "✗".red().bold(),
                            "Linting failed due to errors.".red().bold(),
                            "Fix the errors above before committing.".red()
                        );
                    }
                    2 => {
                        eprintln!("{} {}",
                            "✗".red().bold(),
                            "Linting failed due to formatting errors.".red().bold()
                        );
                    }
                    3 => {
                        eprintln!("{} {}",
                            "⚠".yellow().bold(),
                            "Linting completed with warnings.".yellow().bold()
                        );
                    }
                    _ => {}
                }
            }

            // If --fix is specified and there are issues, enter fix mode
            if cli.fix && !result.issues.is_empty() {
                use cli::resolve_ai_provider;
                use linthis::config::Config;
                use linthis::interactive::{run_ai_fix_all, run_interactive, AiFixConfig};

                let project_root = linthis::utils::get_project_root();
                let config = Config::load_merged(&project_root);

                if cli.ai {
                    // Interactive provider selection when --ai without --provider
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

                    // AI-powered fix mode
                    let provider = resolve_ai_provider(
                        provider_ref,
                        config.ai.provider.as_deref(),
                    );
                    let ai_config = AiFixConfig::with_provider(&provider)
                        .with_accept_all(cli.accept_all)
                        .with_verbose(cli.verbose);

                    if !cli.quiet {
                        eprintln!(
                            "\n{} Entering AI fix mode with provider: {}",
                            "→".cyan(),
                            provider.cyan()
                        );
                    }

                    let ai_result = run_ai_fix_all(&result, &ai_config);

                    if !cli.quiet && ai_result.applied > 0 {
                        eprintln!(
                            "{} Applied {} fix(es)",
                            "✓".green(),
                            ai_result.applied
                        );
                    }

                    // Return success if all issues were fixed
                    if ai_result.applied > 0 && ai_result.errors == 0 {
                        return ExitCode::SUCCESS;
                    }
                } else {
                    // Interactive fix mode
                    if !cli.quiet {
                        eprintln!("\n{} Entering interactive fix mode", "→".cyan());
                    }

                    let interactive_result = run_interactive(&result);

                    if !cli.quiet {
                        let count = interactive_result.edited + interactive_result.ignored;
                        if count > 0 {
                            eprintln!(
                                "{} Processed {} issue(s)",
                                "✓".green(),
                                count
                            );
                        }
                    }
                }

                return ExitCode::from(result.exit_code as u8);
            }

            // Show hint for fix mode if there are issues
            if !cli.quiet && !result.issues.is_empty() {
                print_fix_hint(&result.issues);
            }

            ExitCode::from(result.exit_code as u8)
        }
        Err(e) => {
            eprintln!("{}: {}", "Error".red().bold(), e);
            ExitCode::from(2)
        }
    }
}
