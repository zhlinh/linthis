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

use clap::Parser;
use colored::Colorize;
use std::path::PathBuf;
use std::process::ExitCode;

use cli::{
    collect_paths, handle_cache_command, handle_complexity_command, handle_config_command,
    handle_doctor_command, handle_fix_command, handle_hook_command, handle_init_command,
    handle_license_command, handle_plugin_command, handle_report_command, handle_security_command,
    init_linter_configs, perform_auto_sync, perform_self_update, print_fix_hint, run_benchmark,
    run_watch, strip_ansi_codes, Cli, Commands, ComplexityCommandOptions, FixCommandOptions,
    PathCollectionOptions, PathCollectionResult,
};
use linthis::lsp::{run_lsp_server, LspMode};
use linthis::utils::output::{format_result_with_hook_type, OutputFormat};
use linthis::{run, Language, RunMode, RunOptions};

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

    // Handle hook subcommands
    if let Some(Commands::Hook { action }) = cli.command {
        return handle_hook_command(action);
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
        severity,
        include_dev,
        fix,
        ignore,
        format,
        sbom,
        fail_on,
        verbose,
    }) = cli.command
    {
        return handle_security_command(
            path, severity, include_dev, fix, ignore, format, sbom, fail_on, verbose,
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

    // Handle fix subcommand
    if let Some(Commands::Fix {
        source,
        check,
        format_only,
        ai,
        provider,
        model,
        max_suggestions,
        auto_apply,
        jobs,
        file,
        line,
        message,
        rule,
        output,
        with_context,
        verbose,
        quiet,
    }) = cli.command
    {
        return handle_fix_command(FixCommandOptions {
            source,
            check,
            format_only,
            ai,
            provider,
            model,
            max_suggestions,
            auto_apply,
            jobs,
            file,
            line,
            message,
            rule,
            output,
            with_context,
            verbose,
            quiet,
        });
    }

    // Handle lsp subcommand
    if let Some(Commands::Lsp { mode, port }) = cli.command {
        let lsp_mode = match mode.parse::<LspMode>() {
            Ok(m) => m,
            Err(e) => {
                eprintln!("{}: {}", "Error".red(), e);
                return ExitCode::from(1);
            }
        };

        // Run LSP server using tokio runtime
        let runtime = match tokio::runtime::Runtime::new() {
            Ok(rt) => rt,
            Err(e) => {
                eprintln!("{}: Failed to create async runtime: {}", "Error".red(), e);
                return ExitCode::from(1);
            }
        };

        match runtime.block_on(run_lsp_server(lsp_mode, port)) {
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

    // Load plugins from config files (project first, then global)
    if !cli.no_plugin {
        use linthis::plugin::{PluginConfigManager, PluginLoader, PluginSource};

        let mut plugins_to_load: Vec<(String, PluginSource)> = Vec::new();

        // Check project config first
        if let Ok(project_manager) = PluginConfigManager::project() {
            if let Ok(project_plugins) = project_manager.list_plugins() {
                for (name, url, git_ref) in project_plugins {
                    let source = if let Some(ref r) = git_ref {
                        PluginSource::new(&url).with_ref(r)
                    } else {
                        PluginSource::new(&url)
                    };
                    plugins_to_load.push((name, source));
                }
            }
        }

        // If no project plugins, check global config
        if plugins_to_load.is_empty() {
            if let Ok(global_manager) = PluginConfigManager::global() {
                if let Ok(global_plugins) = global_manager.list_plugins() {
                    for (name, url, git_ref) in global_plugins {
                        let source = if let Some(ref r) = git_ref {
                            PluginSource::new(&url).with_ref(r)
                        } else {
                            PluginSource::new(&url)
                        };
                        plugins_to_load.push((name, source));
                    }
                }
            }
        }

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
                match loader.load_configs(&[source], false) {
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
                                    if std::fs::copy(&config.config_path, &target).is_ok()
                                        && cli.verbose {
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

    // Collect paths using the paths module
    let path_options = PathCollectionOptions {
        staged: cli.staged,
        since: cli.since.clone(),
        uncommitted: cli.uncommitted,
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

    // Build options
    let options = RunOptions {
        paths,
        mode,
        languages,
        exclude_patterns,
        verbose: cli.verbose,
        quiet: cli.quiet,
        plugins: loaded_plugins,
        no_cache: cli.no_cache,
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

    // Run linthis
    match run(&options) {
        Ok(result) => {
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
                    // Default path: always save as JSON for --last/--from-result support
                    serde_json::to_string_pretty(&result).unwrap_or_else(|_| output.clone())
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

            // Show hint for fix mode if there are issues
            if !cli.quiet && !result.issues.is_empty() {
                print_fix_hint();
            }

            ExitCode::from(result.exit_code as u8)
        }
        Err(e) => {
            eprintln!("{}: {}", "Error".red().bold(), e);
            ExitCode::from(2)
        }
    }
}
