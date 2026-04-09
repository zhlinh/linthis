// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Watch mode CLI handler.

use std::io::Write;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use linthis::tui::{self, App, EventHandler};
use linthis::utils::types::RunResult;
use linthis::watch::{Debouncer, FileWatcher, WatchConfig};
use linthis::{run, RunMode, RunOptions, Severity};

/// Run watch mode
pub fn run_watch(config: WatchConfig) -> Result<(), String> {
    println!("🔍 Starting watch mode...");

    // Validate paths
    for path in &config.paths {
        if !path.exists() {
            return Err(format!("Path does not exist: {}", path.display()));
        }
    }

    // Create file watcher
    let watcher = FileWatcher::new(config.paths.clone())
        .map_err(|e| format!("Failed to create file watcher: {}", e))?;

    println!(
        "👀 Watching {} path(s) for changes...",
        watcher.watched_paths().len()
    );
    for path in watcher.watched_paths() {
        println!("   {}", path.display());
    }
    println!();

    // Run initial lint
    println!("\x1b[36m⠋\x1b[0m Running initial lint...");
    let result = run_lint(&config)?;
    print_result_summary(&result);

    if config.no_tui {
        // Simple stdout mode
        run_simple_watch(watcher, config)
    } else {
        // TUI mode
        run_tui_watch(watcher, config, result)
    }
}

/// Run watch with simple stdout output
fn run_simple_watch(watcher: FileWatcher, config: WatchConfig) -> Result<(), String> {
    let mut debouncer = Debouncer::new(config.debounce_ms);

    println!("Press Ctrl+C to stop watching.");
    println!();

    loop {
        // Collect events
        while let Some(event) = watcher.try_recv() {
            if config.verbose {
                println!(
                    "📝 File changed: {} ({:?})",
                    event.path.display(),
                    event.kind
                );
            }
            debouncer.add_event(event);
        }

        // Check for ready events
        let ready = debouncer.get_ready_events();
        if !ready.is_empty() {
            let paths: Vec<_> = ready.iter().map(|e| e.path.display().to_string()).collect();

            if config.clear {
                // Clear screen
                print!("\x1b[2J\x1b[H");
                std::io::stdout().flush().ok();
            }

            println!();
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            println!("🔄 Files changed: {}", paths.join(", "));
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

            // Run lint
            match run_lint(&config) {
                Ok(result) => {
                    print_result_summary(&result);

                    // Send notification if enabled
                    #[cfg(feature = "notifications")]
                    if config.notify {
                        let watch_result = linthis::watch::WatchResult::from_run_result(&result);
                        linthis::watch::notify_issues(&watch_result);
                    }
                }
                Err(e) => {
                    eprintln!("❌ Lint error: {}", e);
                }
            }
        }

        // Small sleep to avoid busy loop
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// Process a lint run triggered by file changes or a forced rerun.
/// Updates app state and returns the new last-lint timestamp on success.
fn process_tui_lint(
    app: &mut App,
    config: &WatchConfig,
    ready: &[linthis::watch::WatchEvent],
    terminal: &mut tui::Tui,
) -> Option<Instant> {
    let paths: Vec<PathBuf> = ready.iter().map(|e| e.path.clone()).collect();
    app.watch_state.mark_checking(&paths);
    app.set_status("Running...");

    // Redraw to show checking status
    terminal.draw(|frame| tui::draw(frame, app)).ok();

    match run_lint(config) {
        Ok(result) => {
            app.update_results(&result);

            #[cfg(feature = "notifications")]
            if config.notify {
                let watch_result = linthis::watch::WatchResult::from_run_result(&result);
                linthis::watch::notify_issues(&watch_result);
            }

            Some(Instant::now())
        }
        Err(e) => {
            app.set_status(format!("Error: {}", e));
            None
        }
    }
}

/// Handle a single TUI event (key press, resize, tick).
fn handle_tui_event(app: &mut App, event_handler: &EventHandler) {
    if let Ok(event) = event_handler.try_next().ok_or("").map_err(|_| ()) {
        match event {
            tui::Event::Key(key) => {
                tui::event::handle_key_event(app, key);
            }
            tui::Event::Resize(_, _) | tui::Event::Tick => {}
            _ => {}
        }
    }
}

/// Run watch with TUI
fn run_tui_watch(
    watcher: FileWatcher,
    config: WatchConfig,
    initial_result: RunResult,
) -> Result<(), String> {
    let mut terminal =
        tui::init_terminal().map_err(|e| format!("Failed to initialize TUI: {}", e))?;

    let mut app = App::new(config.clone());
    app.update_results(&initial_result);

    let event_handler = EventHandler::new(Duration::from_millis(100));
    let mut debouncer = Debouncer::new(config.debounce_ms);
    let mut last_lint = Instant::now();
    let min_lint_interval = Duration::from_secs(1);

    while app.is_running() {
        terminal
            .draw(|frame| tui::draw(frame, &app))
            .map_err(|e| format!("Failed to draw TUI: {}", e))?;

        while let Some(event) = watcher.try_recv() {
            debouncer.add_event(event);
        }

        let ready = debouncer.get_ready_events();
        let should_lint = !ready.is_empty() || app.take_force_rerun();

        if should_lint && last_lint.elapsed() >= min_lint_interval {
            if let Some(new_time) =
                process_tui_lint(&mut app, &config, &ready, &mut terminal)
            {
                last_lint = new_time;
            }
        }

        handle_tui_event(&mut app, &event_handler);
    }

    tui::restore_terminal(&mut terminal)
        .map_err(|e| format!("Failed to restore terminal: {}", e))?;

    println!("Watch mode stopped.");
    Ok(())
}

/// Run a lint check with current config
fn run_lint(config: &WatchConfig) -> Result<RunResult, String> {
    let mode = if config.check_only {
        RunMode::CheckOnly
    } else if config.format_only {
        RunMode::FormatOnly
    } else {
        RunMode::Both
    };

    let options = RunOptions {
        paths: config.paths.clone(),
        mode,
        languages: config.languages.clone(),
        exclude_patterns: config.exclude_patterns.clone(),
        verbose: config.verbose,
        quiet: true, // Suppress normal output in watch mode
        plugins: Vec::new(),
        no_cache: false,
        config_resolver: None,
        tool_install_mode: linthis::ToolInstallMode::Disabled,
        hook_event: None,
    };

    run(&options).map_err(|e| e.to_string())
}

/// Print a summary of lint results
fn print_result_summary(result: &RunResult) {
    let mut errors = 0;
    let mut warnings = 0;
    let mut infos = 0;

    for issue in &result.issues {
        match issue.severity {
            Severity::Error => errors += 1,
            Severity::Warning => warnings += 1,
            Severity::Info => infos += 1,
        }
    }

    if result.issues.is_empty() {
        println!("✅ All clear! No issues found.");
    } else {
        println!(
            "📊 Found {} issue(s): {} error(s), {} warning(s), {} info",
            result.issues.len(),
            errors,
            warnings,
            infos
        );

        // Show first few issues
        let show_count = result.issues.len().min(5);
        for issue in result.issues.iter().take(show_count) {
            let severity_symbol = match issue.severity {
                Severity::Error => "❌",
                Severity::Warning => "⚠️ ",
                Severity::Info => "ℹ️ ",
            };

            println!(
                "   {} {}:{}  {}",
                severity_symbol,
                issue.file_path.display(),
                issue.line,
                if issue.message.len() > 60 {
                    format!("{}...", &issue.message[..57])
                } else {
                    issue.message.clone()
                }
            );
        }

        if result.issues.len() > show_count {
            println!("   ... and {} more", result.issues.len() - show_count);
        }
    }
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_watch_config_default() {
        let config = WatchConfig::default();
        assert_eq!(config.debounce_ms, 300);
        assert!(!config.check_only);
        assert!(!config.format_only);
        assert!(!config.no_tui);
    }
}
