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
    find_latest_result_file, handle_config_command, handle_hook_command, handle_init_command,
    handle_plugin_command, init_linter_configs, perform_auto_sync, perform_self_update,
    print_fix_hint, run_benchmark, strip_ansi_codes, Cli, Commands,
};
use linthis::interactive::run_interactive;
use linthis::utils::output::{format_result, OutputFormat};
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

    // Handle --fix without -c: load result file and enter interactive mode
    // If --fix is used with -c, we'll run check first then enter interactive mode later
    if let Some(ref source) = cli.fix {
        // --fix without -c: load from file
        if !cli.check_only && !cli.format_only {
            let path = if source == "last" {
                match find_latest_result_file() {
                    Some(p) => p,
                    None => {
                        eprintln!(
                            "{}: No result files found in .linthis/result/",
                            "Error".red()
                        );
                        eprintln!("  Run {} first to generate a result file.", "linthis -c".cyan());
                        return ExitCode::from(1);
                    }
                }
            } else {
                PathBuf::from(source)
            };

            if !path.exists() {
                eprintln!("{}: Result file not found: {}", "Error".red(), path.display());
                return ExitCode::from(1);
            }

            println!(
                "{} Loading results from: {}",
                "→".cyan(),
                path.display()
            );

            match std::fs::read_to_string(&path) {
                Ok(content) => {
                    match serde_json::from_str::<linthis::utils::types::RunResult>(&content) {
                        Ok(result) => {
                            if result.issues.is_empty() {
                                println!("{}", "No issues in the saved result.".green());
                                return ExitCode::SUCCESS;
                            }
                            println!(
                                "  Found {} issue{} from previous run\n",
                                result.issues.len(),
                                if result.issues.len() == 1 { "" } else { "s" }
                            );
                            let interactive_result = run_interactive(&result);

                            // Recheck modified files if any changes were made
                            if !interactive_result.modified_files.is_empty() {
                                use linthis::utils::language::language_from_path;
                                use std::collections::HashMap;

                                println!();
                                println!("{}", "═".repeat(60).dimmed());
                                println!("  {}", "Rechecking modified files...".bold());
                                println!("{}", "─".repeat(60).dimmed());

                                // Build a map of file -> language from original issues
                                let mut file_languages: HashMap<PathBuf, Language> = HashMap::new();
                                for issue in &result.issues {
                                    if let Some(lang) = issue.language {
                                        file_languages.insert(issue.file_path.clone(), lang);
                                    }
                                }

                                // Recheck each modified file
                                let modified_count = interactive_result.modified_files.len();
                                let mut recheck_issues = Vec::new();

                                for (i, file) in interactive_result.modified_files.iter().enumerate() {
                                    eprint!("\r⏳ Rechecking {}/{}...", i + 1, modified_count);
                                    use std::io::Write;
                                    std::io::stderr().flush().ok();

                                    let lang = file_languages
                                        .get(file)
                                        .copied()
                                        .or_else(|| language_from_path(file));

                                    if let Some(lang) = lang {
                                        if let Some(checker) = linthis::get_checker(lang) {
                                            if checker.is_available() {
                                                match checker.check(file) {
                                                    Ok(file_issues) => {
                                                        for mut issue in file_issues {
                                                            issue.language = Some(lang);
                                                            recheck_issues.push(issue);
                                                        }
                                                    }
                                                    Err(e) => {
                                                        eprintln!("\n  Check error for {}: {}", file.display(), e);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }

                                eprint!("\r");
                                use std::io::Write;
                                std::io::stderr().flush().ok();

                                // Print recheck results
                                let remaining_count = recheck_issues.len();
                                let fixed_count = interactive_result.edited + interactive_result.ignored;

                                if remaining_count == 0 {
                                    println!(
                                        "  {} All issues in modified files have been resolved!",
                                        "✓".green().bold()
                                    );
                                    println!("  {} file(s) modified, {} issue(s) fixed", modified_count, fixed_count);
                                } else {
                                    println!(
                                        "  {} {} remaining issue(s) in modified files",
                                        "⚠".yellow(),
                                        remaining_count
                                    );
                                    println!("  {} file(s) modified, {} issue(s) fixed", modified_count, fixed_count);
                                    println!();

                                    // Show remaining issues
                                    use linthis::utils::types::Severity;
                                    let errors = recheck_issues.iter().filter(|i| i.severity == Severity::Error).count();
                                    let warnings = recheck_issues.iter().filter(|i| i.severity == Severity::Warning).count();

                                    for issue in &recheck_issues {
                                        let severity_badge = match issue.severity {
                                            Severity::Error => "ERROR".red().bold(),
                                            Severity::Warning => "WARNING".yellow(),
                                            Severity::Info => "INFO".blue(),
                                        };

                                        let location = if let Some(col) = issue.column {
                                            format!("{}:{}:{}", issue.file_path.display(), issue.line, col)
                                        } else {
                                            format!("{}:{}", issue.file_path.display(), issue.line)
                                        };

                                        println!("  {} {} {}", severity_badge, location, issue.message);
                                    }

                                    println!();
                                    println!("  Summary: {} error(s), {} warning(s)", errors, warnings);
                                }

                                println!("{}", "═".repeat(60).dimmed());
                                println!();
                            }

                            return ExitCode::from(result.exit_code as u8);
                        }
                        Err(e) => {
                            eprintln!(
                                "{}: Failed to parse result file as JSON: {}",
                                "Error".red(),
                                e
                            );
                            eprintln!("  Result files are saved in JSON format by default.");
                            eprintln!("  Make sure the file is a valid JSON result file.");
                            return ExitCode::from(2);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("{}: Failed to read result file: {}", "Error".red(), e);
                    return ExitCode::from(2);
                }
            }
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
                                    if std::fs::copy(&config.config_path, &target).is_ok() {
                                        if cli.verbose {
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

    // Build exclusion patterns FIRST (defaults + gitignore + user-specified)
    // This must be done before getting staged files so we can filter them
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

    // Add excludes from project config file
    let project_root = linthis::utils::get_project_root();
    if let Some(project_config) = linthis::config::Config::load_project_config(&project_root) {
        if !project_config.excludes.is_empty() {
            if cli.verbose {
                eprintln!(
                    "Loaded {} exclude patterns from config",
                    project_config.excludes.len()
                );
            }
            exclude_patterns.extend(project_config.excludes);
        }
    }

    // Get paths (handle staged files) and apply exclusion filters
    let paths = if cli.staged {
        match linthis::utils::get_staged_files() {
            Ok(files) => {
                if files.is_empty() {
                    if !cli.quiet {
                        println!("{}", "No staged files to check".yellow());
                    }
                    return ExitCode::SUCCESS;
                }

                // Filter staged files using exclusion patterns
                use linthis::utils::walker::build_glob_set;
                let glob_set = build_glob_set(&exclude_patterns);
                let filtered_files: Vec<PathBuf> = files
                    .into_iter()
                    .filter(|path| {
                        // Check if file should be excluded
                        if let Some(ref gs) = glob_set {
                            // Check relative path from git root
                            if let Ok(relative) = path.strip_prefix(&project_root) {
                                if gs.is_match(relative) {
                                    if cli.verbose {
                                        eprintln!("Excluding: {}", relative.display());
                                    }
                                    return false;
                                }

                                // Check all subpaths starting from each component
                                // This handles patterns like "third_party/**" matching "vpncomm/third_party/..."
                                let components: Vec<_> = relative.components().collect();
                                for i in 0..components.len() {
                                    let subpath: PathBuf = components[i..].iter().collect();
                                    if gs.is_match(&subpath) {
                                        if cli.verbose {
                                            eprintln!("Excluding: {} (matches from subpath {})", relative.display(), subpath.display());
                                        }
                                        return false;
                                    }
                                }
                            }
                        }
                        true
                    })
                    .collect();

                if filtered_files.is_empty() {
                    if !cli.quiet {
                        println!("{}", "No staged files to check after exclusions".yellow());
                    }
                    return ExitCode::SUCCESS;
                }

                if cli.verbose {
                    eprintln!("Checking {} staged file(s) after exclusions", filtered_files.len());
                }

                filtered_files
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

    // Build options
    let options = RunOptions {
        paths,
        mode,
        languages,
        exclude_patterns,
        verbose: cli.verbose,
        quiet: cli.quiet,
        plugins: loaded_plugins,
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
            let output = format_result(&result, output_format);

            // Print to console
            if !cli.quiet || result.exit_code != 0 {
                if !output.is_empty() {
                    println!("{}", output);
                }
            }

            // Run interactive fix mode if --fix was used with -c
            if cli.fix.is_some() && !result.issues.is_empty() {
                let interactive_result = run_interactive(&result);

                // Recheck modified files if any changes were made
                if !interactive_result.modified_files.is_empty() {
                    use linthis::utils::language::language_from_path;
                    use std::collections::HashMap;

                    println!();
                    println!("{}", "═".repeat(60).dimmed());
                    println!("  {}", "Rechecking modified files...".bold());
                    println!("{}", "─".repeat(60).dimmed());

                    // Build a map of file -> language from original issues
                    let mut file_languages: HashMap<PathBuf, Language> = HashMap::new();
                    for issue in &result.issues {
                        if let Some(lang) = issue.language {
                            file_languages.insert(issue.file_path.clone(), lang);
                        }
                    }

                    // Recheck each modified file
                    let modified_count = interactive_result.modified_files.len();
                    let mut recheck_issues = Vec::new();

                    for (i, file) in interactive_result.modified_files.iter().enumerate() {
                        if !cli.quiet {
                            eprint!("\r⏳ Rechecking {}/{}...", i + 1, modified_count);
                            use std::io::Write;
                            std::io::stderr().flush().ok();
                        }

                        // Get language from original issues, or detect it
                        let lang = file_languages
                            .get(file)
                            .copied()
                            .or_else(|| language_from_path(file));

                        if let Some(lang) = lang {
                            // Use the internal function to check the file
                            if let Some(checker) = linthis::get_checker(lang) {
                                if checker.is_available() {
                                    match checker.check(file) {
                                        Ok(file_issues) => {
                                            for mut issue in file_issues {
                                                issue.language = Some(lang);
                                                recheck_issues.push(issue);
                                            }
                                        }
                                        Err(e) => {
                                            if cli.verbose {
                                                eprintln!("\n  Check error for {}: {}", file.display(), e);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    if !cli.quiet {
                        eprint!("\r");
                        use std::io::Write;
                        std::io::stderr().flush().ok();
                    }

                    // Print recheck results
                    let remaining_count = recheck_issues.len();
                    let fixed_count = interactive_result.edited + interactive_result.ignored;

                    if remaining_count == 0 {
                        println!(
                            "  {} All issues in modified files have been resolved!",
                            "✓".green().bold()
                        );
                        println!("  {} file(s) modified, {} issue(s) fixed", modified_count, fixed_count);
                    } else {
                        println!(
                            "  {} {} remaining issue(s) in modified files",
                            "⚠".yellow(),
                            remaining_count
                        );
                        println!("  {} file(s) modified, {} issue(s) fixed", modified_count, fixed_count);
                        println!();

                        // Show remaining issues
                        use linthis::utils::types::Severity;
                        let errors = recheck_issues.iter().filter(|i| i.severity == Severity::Error).count();
                        let warnings = recheck_issues.iter().filter(|i| i.severity == Severity::Warning).count();

                        for issue in &recheck_issues {
                            let severity_badge = match issue.severity {
                                Severity::Error => "ERROR".red().bold(),
                                Severity::Warning => "WARNING".yellow(),
                                Severity::Info => "INFO".blue(),
                            };

                            let location = if let Some(col) = issue.column {
                                format!("{}:{}:{}", issue.file_path.display(), issue.line, col)
                            } else {
                                format!("{}:{}", issue.file_path.display(), issue.line)
                            };

                            println!("  {} {} {}", severity_badge, location, issue.message);
                        }

                        println!();
                        println!("  Summary: {} error(s), {} warning(s)", errors, warnings);
                    }

                    println!("{}", "═".repeat(60).dimmed());
                    println!();
                }
            }

            // Save to file by default (unless --no-save-result is specified)
            // Default format is JSON for programmatic access (--last, --from-result)
            if !cli.no_save_result || cli.output_file.is_some() {
                use chrono::Local;
                use std::fs::{self, File};
                use std::io::Write;

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
                    // Use default path: .linthis/result/result-{timestamp}.json
                    let result_dir = PathBuf::from(".linthis").join("result");
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

            // Show hint for --fix mode if there are issues and not already using --fix
            if !cli.quiet && cli.fix.is_none() && !result.issues.is_empty() {
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
