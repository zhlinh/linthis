// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Plugin management commands.
//!
//! This module provides functions for managing linthis plugins including
//! initialization, listing, syncing, validation, and configuration.

use colored::Colorize;
use std::path::PathBuf;
use std::process::ExitCode;

use super::commands::PluginCommands;
use linthis::templates::{generate_plugin_manifest_filtered, generate_plugin_readme, get_plugin_template_configs};

/// Check if any plugins have available updates
pub fn check_plugins_for_updates(plugins: &[(String, String, Option<String>)]) -> bool {
    use linthis::plugin::{fetcher::PluginFetcher, PluginCache};

    let cache = match PluginCache::new() {
        Ok(c) => c,
        Err(_) => return false,
    };

    let fetcher = PluginFetcher::new();

    for (_name, url, git_ref) in plugins {
        let cache_path = cache.url_to_cache_path(url);
        if fetcher.has_updates(&cache_path, url, git_ref.as_deref()) {
            return true;
        }
    }

    false
}

/// Helper function to sync a list of plugins
pub fn sync_plugins(plugins: &[(String, String, Option<String>)]) -> Result<(), ()> {
    use linthis::plugin::{fetcher::PluginFetcher, PluginCache, PluginSource};

    let cache = match PluginCache::new() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("  {}: {}", "Error".red(), e);
            return Err(());
        }
    };

    let fetcher = PluginFetcher::new();
    let mut success = true;

    for (name, url, git_ref) in plugins {
        let source = PluginSource {
            name: name.clone(),
            url: Some(url.clone()),
            git_ref: git_ref.clone(),
            enabled: true,
        };

        print!("  {} {}... ", "↓".cyan(), name);
        match fetcher.fetch(&source, &cache, true) {
            Ok(cached_plugin) => {
                let hash_info = cached_plugin
                    .commit_hash
                    .as_ref()
                    .map(|h| &h[..7.min(h.len())])
                    .unwrap_or("unknown");
                println!("{} @ {}", "✓".green(), hash_info);
            }
            Err(e) => {
                println!("{}", "✗".red());
                eprintln!("    Error: {}", e);
                success = false;
            }
        }
    }

    if success {
        Ok(())
    } else {
        Err(())
    }
}

/// Handle plugin subcommands
pub fn handle_plugin_command(action: PluginCommands) -> ExitCode {
    use linthis::plugin::{
        cache::PluginCache,
        manifest::PluginManifest,
    };

    match action {
        PluginCommands::New { name, languages, force } => {
            // Create a new plugin from template
            let plugin_dir = PathBuf::from(&name);
            if plugin_dir.exists() {
                if force {
                    if let Err(e) = std::fs::remove_dir_all(&plugin_dir) {
                        eprintln!("{}: Failed to remove existing directory: {}", "Error".red(), e);
                        return ExitCode::from(1);
                    }
                } else {
                    eprintln!("{}: Directory '{}' already exists (use --force to overwrite)", "Error".red(), name);
                    return ExitCode::from(1);
                }
            }

            // All supported languages
            let all_langs = [
                ("rust", "Rust configs (clippy, rustfmt)"),
                ("python", "Python configs (ruff)"),
                ("typescript", "TypeScript configs (eslint, prettier)"),
                ("go", "Go configs (golangci-lint)"),
                ("java", "Java configs (checkstyle)"),
                ("cpp", "C/C++ configs (clang-format, cpplint)"),
                ("swift", "Swift configs (swiftlint)"),
                ("oc", "Objective-C configs (clang-format)"),
                ("sql", "SQL configs (sqlfluff)"),
                ("csharp", "C# configs (dotnet-format)"),
                ("lua", "Lua configs (luacheck, stylua)"),
                ("css", "CSS configs (stylelint, prettier)"),
                ("kotlin", "Kotlin configs (detekt)"),
                ("dockerfile", "Dockerfile configs (hadolint)"),
                ("scala", "Scala configs (scalafmt)"),
                ("dart", "Dart configs (dart analyzer)"),
            ];

            // Filter languages if specified
            let selected_langs: Vec<_> = if let Some(ref filter) = languages {
                let filter_lower: Vec<_> = filter.iter().map(|s| s.to_lowercase()).collect();
                all_langs.iter()
                    .filter(|(lang, _)| filter_lower.contains(&lang.to_string()))
                    .collect()
            } else {
                all_langs.iter().collect()
            };

            if selected_langs.is_empty() {
                eprintln!("{}: No valid languages specified", "Error".red());
                eprintln!("Available languages: {}", all_langs.iter().map(|(l, _)| *l).collect::<Vec<_>>().join(", "));
                return ExitCode::from(1);
            }

            // Create plugin directory
            if let Err(e) = std::fs::create_dir_all(&plugin_dir) {
                eprintln!("{}: Failed to create directory: {}", "Error".red(), e);
                return ExitCode::from(1);
            }

            // Create language directories
            for (lang, _) in &selected_langs {
                if let Err(e) = std::fs::create_dir_all(plugin_dir.join(lang)) {
                    eprintln!("{}: Failed to create {} directory: {}", "Error".red(), lang, e);
                    return ExitCode::from(1);
                }
            }

            // Get selected language names for template generation
            let lang_names: Vec<&str> = selected_langs.iter().map(|(l, _)| *l).collect();

            // Create example config files for selected languages
            let config_files = get_plugin_template_configs(&name);
            for (path, content) in config_files {
                // Only write config if it's for a selected language
                let should_include = lang_names.iter().any(|lang| path.starts_with(lang));
                if should_include {
                    let file_path = plugin_dir.join(path);
                    if let Err(e) = std::fs::write(&file_path, content) {
                        eprintln!("{}: Failed to write {}: {}", "Error".red(), file_path.display(), e);
                        return ExitCode::from(1);
                    }
                }
            }

            // Create manifest with config mappings (filtered by selected languages)
            let manifest_content = generate_plugin_manifest_filtered(&name, &lang_names);
            let manifest_path = plugin_dir.join("linthis-plugin.toml");
            if let Err(e) = std::fs::write(&manifest_path, &manifest_content) {
                eprintln!("{}: Failed to write manifest: {}", "Error".red(), e);
                return ExitCode::from(1);
            }

            // Create README
            let readme_content = generate_plugin_readme(&name);
            let _ = std::fs::write(plugin_dir.join("README.md"), readme_content);

            // Create .gitignore
            let gitignore_content = "# Editor files\n*.swp\n*.swo\n*~\n.idea/\n.vscode/\n\n# OS files\n.DS_Store\nThumbs.db\n";
            let _ = std::fs::write(plugin_dir.join(".gitignore"), gitignore_content);

            println!("{} Created plugin '{}' with {} language(s)", "✓".green(), name, selected_langs.len());
            println!();
            println!("Created files:");
            println!("  {} linthis-plugin.toml  - Plugin manifest", "•".cyan());
            println!("  {} README.md            - Documentation", "•".cyan());
            for (lang, desc) in &selected_langs {
                println!("  {} {:<17} - {}", "•".cyan(), format!("{}/", lang), desc);
            }
            println!();
            println!("Next steps:");
            println!("  1. cd {}", name);
            println!("  2. Review and customize the config files");
            println!("  3. Edit linthis-plugin.toml with your details");
            println!("  4. Push to a Git repository:");
            println!();
            println!("     cd {} && git init && git add . && git commit -m \"Initial commit\"", name);
            println!("     git remote add origin git@github.com:your-org/{}.git", name);
            println!("     git push -u origin main");
            println!();
            println!("  5. Use the plugin:");
            println!("     linthis --use-plugin ./{}", name);

            ExitCode::SUCCESS
        }

        PluginCommands::List {
            verbose,
            global,
            cached,
        } => {
            // List cached (downloaded) plugins
            if cached {
                use linthis::plugin::cache::format_size;

                let cache = match PluginCache::new() {
                    Ok(c) => c,
                    Err(e) => {
                        eprintln!("{}: {}", "Error".red(), e);
                        return ExitCode::from(1);
                    }
                };

                match cache.list_cached() {
                    Ok(plugins) => {
                        if plugins.is_empty() {
                            println!("No cached plugins found.");
                            println!("\nCache: {}", cache.cache_dir().display());
                            return ExitCode::SUCCESS;
                        }

                        println!("{}", "Cached plugins:".bold());
                        for plugin in &plugins {
                            if verbose {
                                println!(
                                    "  {} {} ({})",
                                    "•".cyan(),
                                    plugin.name.bold(),
                                    plugin.url
                                );
                                println!("    Path: {}", plugin.cache_path.display());
                                println!("    Cached: {}", plugin.cached_at.format("%Y-%m-%d %H:%M"));
                                println!(
                                    "    Updated: {}",
                                    plugin.last_updated.format("%Y-%m-%d %H:%M")
                                );
                            } else {
                                println!("  {} {}", "•".cyan(), plugin.name);
                            }
                        }

                        // Show total cache size
                        if let Ok(size) = cache.cache_size() {
                            println!("\nTotal cache size: {}", format_size(size));
                        }
                        println!("Cache: {}", cache.cache_dir().display());
                    }
                    Err(e) => {
                        eprintln!("{}: Failed to list cached plugins: {}", "Error".red(), e);
                        return ExitCode::from(1);
                    }
                }

                return ExitCode::SUCCESS;
            }

            // List configured plugins
            use linthis::plugin::PluginConfigManager;

            let manager = if global {
                match PluginConfigManager::global() {
                    Ok(m) => m,
                    Err(e) => {
                        eprintln!("{}: {}", "Error".red(), e);
                        return ExitCode::from(1);
                    }
                }
            } else {
                match PluginConfigManager::project() {
                    Ok(m) => m,
                    Err(e) => {
                        eprintln!("{}: {}", "Error".red(), e);
                        return ExitCode::from(1);
                    }
                }
            };

            let config_type = if global { "global" } else { "project" };

            match manager.list_plugins() {
                Ok(plugins) => {
                    if plugins.is_empty() {
                        println!("No {} plugins configured.", config_type);
                        println!("\nConfig: {}", manager.config_path().display());
                        return ExitCode::SUCCESS;
                    }

                    println!(
                        "{} ({}):",
                        "Configured plugins".bold(),
                        config_type
                    );
                    for (name, url, git_ref) in &plugins {
                        if verbose {
                            if let Some(r) = git_ref {
                                println!("  {} {} ({}, ref: {})", "•".cyan(), name.bold(), url, r);
                            } else {
                                println!("  {} {} ({})", "•".cyan(), name.bold(), url);
                            }
                        } else {
                            println!("  {} {}", "•".cyan(), name);
                        }
                    }

                    println!("\nConfig: {}", manager.config_path().display());
                }
                Err(e) => {
                    eprintln!("{}: Failed to list plugins: {}", "Error".red(), e);
                    return ExitCode::from(1);
                }
            }

            ExitCode::SUCCESS
        }

        PluginCommands::Clean { all } => {
            let cache = match PluginCache::new() {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("{}: {}", "Error".red(), e);
                    return ExitCode::from(1);
                }
            };

            if all {
                match cache.clear_all() {
                    Ok(_) => {
                        println!("{} Cleared all cached plugins", "✓".green());
                    }
                    Err(e) => {
                        eprintln!("{}: Failed to clear cache: {}", "Error".red(), e);
                        return ExitCode::from(1);
                    }
                }
            } else {
                println!("Use --all to remove all cached plugins");
                println!("Or remove specific plugins manually from:");
                println!("  {}", cache.cache_dir().display());
            }

            ExitCode::SUCCESS
        }

        PluginCommands::Sync { global } => {
            use linthis::plugin::{fetcher::PluginFetcher, PluginConfigManager, PluginSource};

            let manager = if global {
                match PluginConfigManager::global() {
                    Ok(m) => m,
                    Err(e) => {
                        eprintln!("{}: {}", "Error".red(), e);
                        return ExitCode::from(1);
                    }
                }
            } else {
                match PluginConfigManager::project() {
                    Ok(m) => m,
                    Err(e) => {
                        eprintln!("{}: {}", "Error".red(), e);
                        return ExitCode::from(1);
                    }
                }
            };

            let config_type = if global { "global" } else { "project" };

            let plugins = match manager.list_plugins() {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("{}: Failed to read config: {}", "Error".red(), e);
                    return ExitCode::from(1);
                }
            };

            if plugins.is_empty() {
                println!("No {} plugins configured to sync.", config_type);
                println!("\nConfig: {}", manager.config_path().display());

                // Check if the other config has plugins and provide helpful hints
                if !global {
                    // If project config is empty, check global config
                    if let Ok(global_mgr) = PluginConfigManager::global() {
                        if let Ok(global_plugins) = global_mgr.list_plugins() {
                            if !global_plugins.is_empty() {
                                println!(
                                    "\n{} Found {} global plugin(s).",
                                    "ℹ".cyan(),
                                    global_plugins.len()
                                );
                                println!("To sync global plugins, run:");
                                println!("  {}", "linthis plugin sync -g".bold());
                            }
                        }
                    }
                } else {
                    // If global config is empty, check project config
                    if let Ok(project_mgr) = PluginConfigManager::project() {
                        if let Ok(project_plugins) = project_mgr.list_plugins() {
                            if !project_plugins.is_empty() {
                                println!(
                                    "\n{} Found {} project plugin(s).",
                                    "ℹ".cyan(),
                                    project_plugins.len()
                                );
                                println!("To sync project plugins, run:");
                                println!("  {}", "linthis plugin sync".bold());
                            }
                        }
                    }
                }

                return ExitCode::SUCCESS;
            }

            println!(
                "{} {} plugin(s) from {} config...\n",
                "Syncing".cyan(),
                plugins.len(),
                config_type
            );

            let cache = match PluginCache::new() {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("{}: {}", "Error".red(), e);
                    return ExitCode::from(1);
                }
            };

            let fetcher = PluginFetcher::new();
            let mut success_count = 0;
            let mut fail_count = 0;
            let mut updated_count = 0;

            for (name, url, git_ref) in &plugins {
                let source = PluginSource {
                    name: name.clone(),
                    url: Some(url.clone()),
                    git_ref: git_ref.clone(),
                    enabled: true,
                };

                // Get old hash before sync (if cached)
                let cache_path = cache.url_to_cache_path(url);
                let old_hash = fetcher.get_local_commit_hash(&cache_path);

                print!("  {} {}... ", "↓".cyan(), name);
                // Always sync to latest (force = true)
                match fetcher.fetch(&source, &cache, true) {
                    Ok(cached_plugin) => {
                        let new_hash = cached_plugin.commit_hash.as_ref();
                        let was_updated = match (&old_hash, new_hash) {
                            (Some(old), Some(new)) => old != new,
                            (None, Some(_)) => true, // newly cloned
                            _ => false,
                        };

                        let hash_info = new_hash
                            .map(|h| &h[..7.min(h.len())])
                            .unwrap_or("unknown");

                        if was_updated {
                            if old_hash.is_some() {
                                let old_short = old_hash.as_ref()
                                    .map(|h| &h[..7.min(h.len())])
                                    .unwrap_or("unknown");
                                println!("{} {} -> {}", "✓".green(), old_short, hash_info);
                            } else {
                                println!("{} @ {}", "✓".green(), hash_info);
                            }
                            updated_count += 1;
                        } else {
                            println!("{} @ {} (up to date)", "✓".green(), hash_info);
                        }
                        success_count += 1;
                    }
                    Err(e) => {
                        println!("{}", "✗".red());
                        eprintln!("    Error: {}", e);
                        fail_count += 1;
                    }
                }
            }

            println!();
            if fail_count == 0 {
                if updated_count > 0 {
                    println!(
                        "{} Synced {} plugin(s), {} updated",
                        "✓".green(),
                        success_count,
                        updated_count
                    );
                } else {
                    println!(
                        "{} All {} plugin(s) up to date",
                        "✓".green(),
                        success_count
                    );
                }
            } else {
                println!(
                    "{} Synced {}/{} plugin(s), {} failed",
                    "⚠".yellow(),
                    success_count,
                    plugins.len(),
                    fail_count
                );
            }

            if fail_count > 0 {
                ExitCode::from(1)
            } else {
                ExitCode::SUCCESS
            }
        }

        PluginCommands::Validate { path } => {
            match PluginManifest::load(&path) {
                Ok(manifest) => {
                    // Validate the manifest
                    if let Err(e) = manifest.validate(&path) {
                        eprintln!("{}: {}", "Validation failed".red(), e);
                        return ExitCode::from(1);
                    }

                    println!("{} Plugin '{}' is valid", "✓".green(), manifest.plugin.name);
                    println!("  Version: {}", manifest.plugin.version);
                    println!("  Languages: {}", manifest.plugin.languages.join(", "));
                    println!("  Configs:");
                    for (lang, tools) in &manifest.configs {
                        for (tool, path) in tools {
                            println!("    {}/{}: {}", lang, tool, path);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("{}: {}", "Validation failed".red(), e);
                    return ExitCode::from(1);
                }
            }

            ExitCode::SUCCESS
        }

        PluginCommands::Add {
            alias,
            url,
            git_ref,
            global,
        } => {
            use linthis::plugin::PluginConfigManager;

            let manager = if global {
                match PluginConfigManager::global() {
                    Ok(m) => m,
                    Err(e) => {
                        eprintln!("{}: {}", "Error".red(), e);
                        return ExitCode::from(1);
                    }
                }
            } else {
                match PluginConfigManager::project() {
                    Ok(m) => m,
                    Err(e) => {
                        eprintln!("{}: {}", "Error".red(), e);
                        return ExitCode::from(1);
                    }
                }
            };

            let config_type = if global { "global" } else { "project" };

            match manager.add_plugin(&alias, &url, git_ref.as_deref()) {
                Ok(_) => {
                    println!(
                        "{} Added plugin '{}' to {} configuration",
                        "✓".green(),
                        alias.bold(),
                        config_type
                    );
                    println!();
                    println!("  Alias: {}", alias);
                    println!("  URL:   {}", url);
                    if let Some(ref_) = git_ref {
                        println!("  Ref:   {}", ref_);
                    }
                    println!("  Config: {}", manager.config_path().display());
                    println!();
                    println!("Plugin will be automatically loaded when running linthis.");

                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("{}: {}", "Error".red(), e);
                    ExitCode::from(1)
                }
            }
        }

        PluginCommands::Remove { alias, global } => {
            use linthis::plugin::PluginConfigManager;

            let manager = if global {
                match PluginConfigManager::global() {
                    Ok(m) => m,
                    Err(e) => {
                        eprintln!("{}: {}", "Error".red(), e);
                        return ExitCode::from(1);
                    }
                }
            } else {
                match PluginConfigManager::project() {
                    Ok(m) => m,
                    Err(e) => {
                        eprintln!("{}: {}", "Error".red(), e);
                        return ExitCode::from(1);
                    }
                }
            };

            let config_type = if global { "global" } else { "project" };

            match manager.remove_plugin(&alias) {
                Ok(true) => {
                    println!(
                        "{} Removed plugin '{}' from {} configuration",
                        "✓".green(),
                        alias.bold(),
                        config_type
                    );
                    ExitCode::SUCCESS
                }
                Ok(false) => {
                    eprintln!(
                        "{}: Plugin alias '{}' not found in {} configuration",
                        "Warning".yellow(),
                        alias,
                        config_type
                    );
                    println!();
                    println!("Available plugins in {}:", manager.config_path().display());
                    match manager.list_plugins() {
                        Ok(plugins) => {
                            if plugins.is_empty() {
                                println!("  (none)");
                            } else {
                                for (name, url, ref_) in plugins {
                                    if let Some(r) = ref_ {
                                        println!("  {} {} ({}, ref: {})", "•".cyan(), name, url, r);
                                    } else {
                                        println!("  {} {} ({})", "•".cyan(), name, url);
                                    }
                                }
                            }
                        }
                        Err(_) => {
                            println!("  (unable to list plugins)");
                        }
                    }
                    ExitCode::from(1)
                }
                Err(e) => {
                    eprintln!("{}: {}", "Error".red(), e);
                    ExitCode::from(1)
                }
            }
        }

        PluginCommands::Apply { alias, global, language } => {
            use linthis::plugin::{loader::PluginLoader, PluginConfigManager, PluginSource};

            let manager = if global {
                match PluginConfigManager::global() {
                    Ok(m) => m,
                    Err(e) => {
                        eprintln!("{}: {}", "Error".red(), e);
                        return ExitCode::from(1);
                    }
                }
            } else {
                match PluginConfigManager::project() {
                    Ok(m) => m,
                    Err(e) => {
                        eprintln!("{}: {}", "Error".red(), e);
                        return ExitCode::from(1);
                    }
                }
            };

            let config_type = if global { "global" } else { "project" };

            // Get plugins to apply
            let plugins = match manager.list_plugins() {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("{}: Failed to read config: {}", "Error".red(), e);
                    return ExitCode::from(1);
                }
            };

            // Filter by alias if specified
            let plugins: Vec<_> = if let Some(ref alias_filter) = alias {
                plugins.into_iter().filter(|(name, _, _)| name == alias_filter).collect()
            } else {
                plugins
            };

            if plugins.is_empty() {
                if let Some(ref a) = alias {
                    eprintln!("{}: Plugin '{}' not found in {} config", "Error".red(), a, config_type);
                } else {
                    println!("No plugins configured in {} config.", config_type);
                }
                return ExitCode::from(1);
            }

            let loader = match PluginLoader::new() {
                Ok(l) => l,
                Err(e) => {
                    eprintln!("{}: {}", "Error".red(), e);
                    return ExitCode::from(1);
                }
            };

            let mut applied_count = 0;
            let project_root = std::env::current_dir().unwrap_or_default();

            for (name, url, git_ref) in &plugins {
                let source = PluginSource {
                    name: name.clone(),
                    url: Some(url.clone()),
                    git_ref: git_ref.clone(),
                    enabled: true,
                };

                match loader.load_configs(&[source], false) {
                    Ok(configs) => {
                        // Filter by language if specified
                        let configs: Vec<_> = if let Some(ref langs) = language {
                            configs.into_iter().filter(|c| langs.contains(&c.language)).collect()
                        } else {
                            configs
                        };

                        if configs.is_empty() {
                            continue;
                        }

                        println!("\n{} Applying configs from '{}':", "→".cyan(), name);
                        for config in &configs {
                            if let Some(filename) = config.config_path.file_name() {
                                let target = project_root.join(filename);
                                if target.exists() {
                                    println!(
                                        "  {} {}/{}: {} (skipped, exists)",
                                        "⊘".yellow(),
                                        config.language,
                                        config.tool,
                                        filename.to_string_lossy()
                                    );
                                } else {
                                    match std::fs::copy(&config.config_path, &target) {
                                        Ok(_) => {
                                            println!(
                                                "  {} {}/{}: {}",
                                                "✓".green(),
                                                config.language,
                                                config.tool,
                                                filename.to_string_lossy()
                                            );
                                            applied_count += 1;
                                        }
                                        Err(e) => {
                                            eprintln!(
                                                "  {} {}: {}",
                                                "✗".red(),
                                                filename.to_string_lossy(),
                                                e
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("{}: Failed to load plugin '{}': {}", "Warning".yellow(), name, e);
                    }
                }
            }

            println!();
            if applied_count > 0 {
                println!("{} Applied {} config file(s)", "✓".green(), applied_count);
                println!("\n{}: Add these to .gitignore if you don't want to commit them", "Tip".cyan());
            } else {
                println!("{} No new configs applied (all already exist)", "ℹ".blue());
            }

            ExitCode::SUCCESS
        }
    }
}
