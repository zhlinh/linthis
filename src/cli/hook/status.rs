// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Hook status, list, and check commands.

use colored::Colorize;
use std::path::PathBuf;
use std::process::ExitCode;

use super::agent::{
    agent_is_installed, agent_skill_path, agent_stop_hook_settings_path, ALL_AGENT_PROVIDERS,
};
use super::metadata::{load_installed_hooks, InstalledHooksFile};
use super::{find_git_root, global_hooks_dir, is_linthis_hook_file};
use crate::cli::commands::HookEvent;

/// Where git will actually look for hooks in this repository, and where that
/// was decided.
struct HooksPath {
    /// The one directory git runs hooks from.
    effective: PathBuf,
    /// Origin of a `core.hooksPath` setting, as git reports it, or `None` when
    /// the setting is absent and git falls back to `$GIT_DIR/hooks`.
    origin: Option<String>,
}

impl HooksPath {
    /// Whether `dir` is the directory git actually runs.
    fn is_effective(&self, dir: &std::path::Path) -> bool {
        same_dir(dir, &self.effective)
    }
}

/// Resolve the hooks directory the way git does.
///
/// `core.hooksPath` can be set at repository, user or system level and the most
/// specific one wins; when it is set, git consults that directory *only* and
/// never falls back. Asking git rather than reimplementing that precedence is
/// what keeps this honest — reading only `--global`, as this used to, reports a
/// directory git may never open.
fn resolve_hooks_path(git_root: &std::path::Path) -> HooksPath {
    let effective = git_output(git_root, &["rev-parse", "--git-path", "hooks"])
        .map(|p| {
            let path = PathBuf::from(&p);
            if path.is_absolute() {
                path
            } else {
                git_root.join(path)
            }
        })
        .unwrap_or_else(|| git_root.join(".git/hooks"));

    // `--show-origin` prints "file:<config path>\t<value>"; the config path is
    // what tells the user where to go and change it.
    let origin = git_output(
        git_root,
        &["config", "--show-origin", "--get", "core.hooksPath"],
    )
    .and_then(|line| line.split('\t').next().map(|o| o.to_string()))
    .map(|o| o.trim_start_matches("file:").to_string());

    HooksPath { effective, origin }
}

/// Run git in `dir` and return its trimmed stdout, or `None` if it failed.
fn git_output(dir: &std::path::Path, args: &[&str]) -> Option<String> {
    let out = std::process::Command::new("git")
        .current_dir(dir)
        .args(args)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    (!s.is_empty()).then_some(s)
}

/// Compare two directories, resolving symlinks when possible.
///
/// `/tmp` is a symlink to `/private/tmp` on macOS, so a plain path comparison
/// reports the same directory as two.
fn same_dir(a: &std::path::Path, b: &std::path::Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => a == b,
    }
}

/// The lines of a hook script git will actually execute: neither blank nor
/// commented out.
fn active_lines(content: &str) -> impl Iterator<Item = &str> {
    content
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
}

/// Whether running this hook actually invokes linthis.
///
/// `content.contains("linthis")` also matches a mention in a comment, or an
/// invocation someone commented out, and then reports a hook as installed that
/// does nothing.
fn runs_linthis(content: &str) -> bool {
    active_lines(content).any(|l| l.contains("linthis"))
}

/// Whether the hook runs no commands at all — an empty file, or one whose body
/// is entirely commented out, as tool-installed stubs often are.
fn is_noop_hook(content: &str) -> bool {
    active_lines(content).next().is_none()
}

/// Print project-level hook status for each event. Returns true if any hook is installed.
fn print_project_hook_status(
    git_root: &std::path::Path,
    hook_events: &[HookEvent],
    hooks_path: &HooksPath,
) -> bool {
    let dir = git_root.join(".git/hooks");
    let live = hooks_path.is_effective(&dir);
    let mut any_installed = false;

    println!("{}", "Project Hooks (.git/hooks/):".bold());
    // Only worth saying when there is something here to be ignored.
    if !live
        && hook_events
            .iter()
            .any(|e| dir.join(e.hook_filename()).exists())
    {
        println!(
            "  {} git runs {} instead — nothing here is executed",
            "⚠".yellow(),
            hooks_path.effective.display()
        );
    }
    for event in hook_events {
        let hook_path = dir.join(event.hook_filename());
        if hook_path.exists() {
            any_installed = true;
            let tag = if live {
                "[project]"
            } else {
                "[project, never runs]"
            };
            println!("{} {} {}", "✓".green(), hook_path.display(), tag);
            println!("    {}", event.description().dimmed());
            print_hook_content_analysis(&hook_path, live);
        } else {
            println!("{} {} (not installed)", "✗".red(), event.hook_filename());
        }
    }
    any_installed
}

/// Print analysis of detected tools in a hook file (linthis, prek, pre-commit, husky).
///
/// Only lines git would execute count. A tool's stub whose body is commented
/// out mentions itself in the comments, and reporting that as an installed tool
/// is how a repository ends up with no checks and a status screen full of ticks.
fn print_hook_content_analysis(hook_path: &std::path::Path, live: bool) {
    let Ok(content) = std::fs::read_to_string(hook_path) else {
        return;
    };

    if is_noop_hook(&content) {
        println!(
            "    {} runs no commands — every line is blank or commented out",
            "⚠".yellow()
        );
        return;
    }

    let mut found = Vec::new();
    for (needle, label) in [
        ("linthis", "linthis"),
        ("prek", "prek"),
        ("pre-commit", "pre-commit"),
        ("husky", "husky"),
    ] {
        if active_lines(&content).any(|l| l.contains(needle)) {
            found.push(label);
        }
    }

    if found.is_empty() {
        println!("    {} Custom hook", "ℹ".cyan());
        return;
    }
    for label in &found {
        let mark = if *label == "linthis" {
            "✓".green()
        } else {
            "ℹ".cyan()
        };
        println!("    {} {}", mark, label);
    }
    if live && !found.contains(&"linthis") {
        println!(
            "    {} does not invoke linthis — no lint check runs on this event",
            "⚠".yellow()
        );
    }
}

/// Print global hook status section.
fn print_global_hook_status(hook_events: &[HookEvent], hooks_path: &HooksPath) {
    println!();
    println!("{}", "Global Hooks (~/.config/git/hooks/):".bold());

    match hooks_path.origin {
        Some(ref origin) => {
            println!(
                "  {} = {}  ({})",
                "core.hooksPath".cyan(),
                hooks_path.effective.display(),
                format!("set in {}", origin).dimmed()
            );
        }
        None => {
            println!(
                "  {} (core.hooksPath not set — git uses {})",
                "ℹ".cyan(),
                hooks_path.effective.display()
            );
        }
    }

    let Some(ref ghooks_dir) = global_hooks_dir() else {
        println!("  {} No global linthis hooks installed", "ℹ".cyan());
        return;
    };
    let live = hooks_path.is_effective(ghooks_dir);
    if !live
        && hook_events
            .iter()
            .any(|e| ghooks_dir.join(e.hook_filename()).exists())
    {
        println!(
            "  {} git runs {} instead — nothing here is executed",
            "⚠".yellow(),
            hooks_path.effective.display()
        );
    }

    let mut any_global_hook = false;
    for event in hook_events {
        let hook_path = ghooks_dir.join(event.hook_filename());
        if !hook_path.exists() {
            continue;
        }
        any_global_hook = true;
        let installed = is_linthis_hook_file(&hook_path);
        let tag = match (installed, live) {
            (true, true) => "[global]".to_string(),
            (true, false) => "[global, never runs]".to_string(),
            (false, _) => "[global, not by linthis]".to_string(),
        };
        let mark = if installed && live {
            "✓".green()
        } else {
            "⚠".yellow()
        };
        println!("{} {} {}", mark, hook_path.display(), tag);
    }
    if !any_global_hook {
        println!("  {} No global linthis hooks installed", "ℹ".cyan());
    }
}

/// Warn when nothing in the directory git actually runs will invoke linthis.
///
/// This is the case the old output hid: a repository-level `core.hooksPath`
/// points git at a directory holding another tool's stub, the global linthis
/// hook sits in a directory git never opens, and every line on screen is a
/// tick. Commits then go through unchecked until CI catches them.
fn warn_if_linthis_never_runs(git_root: &std::path::Path, hooks_path: &HooksPath) {
    let pre_commit = hooks_path.effective.join("pre-commit");
    let runs = std::fs::read_to_string(&pre_commit)
        .map(|c| runs_linthis(&c))
        .unwrap_or(false);
    if runs {
        return;
    }

    println!();
    println!(
        "{} {}",
        "⚠".yellow().bold(),
        "linthis does not run on commit in this repository"
            .yellow()
            .bold()
    );
    if pre_commit.exists() {
        println!(
            "  {} exists but never invokes linthis.",
            pre_commit.display()
        );
    } else {
        println!(
            "  No pre-commit hook in {}.",
            hooks_path.effective.display()
        );
    }
    if let Some(ref origin) = hooks_path.origin {
        println!(
            "  git looks there because {} is set in {}.",
            "core.hooksPath".cyan(),
            origin
        );
    }

    println!("  {}", "Fix with either:".bold());
    if hooks_path.origin.is_some() {
        println!(
            "    {}   {}",
            "git config --unset core.hooksPath".cyan(),
            "# fall back to the global hooks".dimmed()
        );
    }
    println!(
        "    {}   {}",
        "linthis hook add --event pre-commit --force".cyan(),
        format!("# install into {}", hooks_path.effective.display()).dimmed()
    );
    let _ = git_root;
}

/// Print agent integration status for a single scope. Returns true if any
/// agent is installed under that scope.
fn print_agent_status_for_scope(
    base: &std::path::Path,
    global: bool,
    skill_names: Option<&linthis::config::AgentSkillNamesConfig>,
) -> bool {
    let title = if global {
        "Agent Integration (Global skills)"
    } else {
        "Agent Integration (Project skills)"
    };
    println!("\n{}", title.bold());
    let events = [
        HookEvent::PreCommit,
        HookEvent::CommitMsg,
        HookEvent::PrePush,
    ];
    let mut any_installed = false;
    for p in ALL_AGENT_PROVIDERS {
        if agent_is_installed(base, p, global, skill_names) {
            any_installed = true;
            println!("{} {}", "✓".green(), p);
            for event in &events {
                let path = agent_skill_path(base, p, global, event, skill_names);
                if path.exists() {
                    println!(
                        "  {} {} ({})",
                        "✓".green().dimmed(),
                        path.display(),
                        event.hook_filename()
                    );
                }
            }
            if let Some(settings_path) = agent_stop_hook_settings_path(base, p) {
                let has_stop_hook = settings_path.exists()
                    && std::fs::read_to_string(&settings_path)
                        .map(|c| c.contains("linthis"))
                        .unwrap_or(false);
                if has_stop_hook {
                    println!(
                        "  {} Stop Hook ({})",
                        "✓".green().dimmed(),
                        settings_path.display()
                    );
                }
            }
        } else {
            println!("{} {} (not installed)", "✗".red(), p);
        }
    }
    any_installed
}

/// Show git hook status
pub(crate) fn handle_hook_status() -> ExitCode {
    let git_root = match find_git_root() {
        Some(root) => root,
        None => {
            eprintln!("{}: Not in a git repository", "Error".red());
            return ExitCode::from(1);
        }
    };

    let prek_config = std::path::Path::new(".pre-commit-config.yaml");

    println!("{}", "Git Hook Status".bold());
    println!("Repository: {}", git_root.display());
    println!();

    let hook_events = [
        HookEvent::PreCommit,
        HookEvent::PrePush,
        HookEvent::CommitMsg,
    ];

    let hooks_path = resolve_hooks_path(&git_root);
    let any_hook_installed = print_project_hook_status(&git_root, &hook_events, &hooks_path);
    print_global_hook_status(&hook_events, &hooks_path);
    warn_if_linthis_never_runs(&git_root, &hooks_path);

    // Check for prek/pre-commit config
    if prek_config.exists() {
        println!("\n{} {}", "✓".green(), prek_config.display());
        if let Ok(content) = std::fs::read_to_string(prek_config) {
            if content.contains("linthis") {
                println!("  {} Contains linthis configuration", "✓".green());
            } else {
                println!("  {} No linthis configuration found", "⚠".yellow());
            }
        }
    }

    println!("\n{}", "Available hooks:".bold());
    println!("  {} - runs before each commit", "pre-commit".cyan());
    println!("  {} - runs before push to remote", "pre-push".cyan());
    println!(
        "  {} - validates commit message format",
        "commit-msg".cyan()
    );

    let skill_names_cfg = linthis::config::Config::load_merged(&git_root)
        .hook
        .agent
        .skill_names;
    let any_agent_project = print_agent_status_for_scope(&git_root, false, Some(&skill_names_cfg));
    let any_agent_global = match linthis::utils::home_dir() {
        Some(ref home) => print_agent_status_for_scope(home, true, Some(&skill_names_cfg)),
        None => {
            println!("\n{}", "Agent Integration (Global skills)".bold());
            println!(
                "  {} HOME / USERPROFILE not set — cannot check global skills",
                "ℹ".cyan()
            );
            false
        }
    };
    let any_agent_installed = any_agent_project || any_agent_global;

    println!("\n{}", "Commands:".bold());
    if !any_hook_installed {
        println!("  Add pre-commit:  {}", "linthis hook add".cyan());
        println!(
            "  Add pre-push:    {}",
            "linthis hook add --event pre-push".cyan()
        );
        println!(
            "  Add commit-msg:  {}",
            "linthis hook add --event commit-msg".cyan()
        );
    } else {
        println!(
            "  Add hook:    {}",
            "linthis hook add --event <event>".cyan()
        );
        println!(
            "  Remove hook: {}",
            "linthis hook remove --event <event>".cyan()
        );
        println!("  Remove all:  {}", "linthis hook remove --all".cyan());
    }
    if !any_agent_installed {
        println!("  Add agent:   {}", "linthis hook add --type agent".cyan());
    } else {
        println!(
            "  Add agent:   {}",
            "linthis hook add --type agent --provider <name>".cyan()
        );
        println!("  Remove all:  {}", "linthis hook remove --all".cyan());
    }

    ExitCode::SUCCESS
}

/// Detect the hook type (git, git-with-agent, prek, prek-with-agent) from script content.
///
/// Falls back to the TOML registry if content analysis is inconclusive.
fn detect_hook_type_from_content(
    content: &str,
    toml: &InstalledHooksFile,
    scope: &str,
    project: &std::path::Path,
    event: &HookEvent,
) -> String {
    // Content-based detection
    let has_agent = content.contains("_LINTHIS_AGENT_OK") || content.contains("agent");
    let has_prek = content.contains("prek");

    if has_prek && has_agent {
        return "prek-with-agent".to_string();
    }
    if has_prek {
        return "prek".to_string();
    }
    if has_agent
        && (content.contains("claude")
            || content.contains("codex")
            || content.contains("openclaw")
            || content.contains("gemini")
            || content.contains("codebuddy")
            || content.contains("droid")
            || content.contains("auggie")
            || content.contains("cursor-agent"))
    {
        return "git-with-agent".to_string();
    }

    // Fall back to TOML registry
    let project_str = project.to_string_lossy();
    let event_str = event.hook_filename();
    for hook in &toml.hooks {
        if hook.scope == scope
            && hook.event == event_str
            && (scope == "global" || hook.project == project_str.as_ref())
        {
            return hook.hook_type.clone();
        }
    }

    "git".to_string()
}

/// Detect the provider name from script content or TOML registry.
fn detect_provider_from_content(
    content: &str,
    toml: &InstalledHooksFile,
    scope: &str,
    project: &std::path::Path,
    event: &HookEvent,
) -> String {
    // Content-based detection
    let providers = [
        ("claude", "claude"),
        ("codex", "codex"),
        ("gemini", "gemini"),
        ("cursor-agent", "cursor"),
        ("droid", "droid"),
        ("auggie", "auggie"),
        ("codebuddy", "codebuddy"),
        ("openclaw", "openclaw"),
    ];
    for (pattern, name) in &providers {
        if content.contains(pattern)
            && (content.contains("_LINTHIS_AGENT_OK") || content.contains("agent"))
        {
            return name.to_string();
        }
    }

    // Fall back to TOML registry
    let project_str = project.to_string_lossy();
    let event_str = event.hook_filename();
    for hook in &toml.hooks {
        if hook.scope == scope
            && hook.event == event_str
            && (scope == "global" || hook.project == project_str.as_ref())
        {
            return hook.provider.clone();
        }
    }

    String::new()
}

/// List shell hooks in a given hooks directory. Returns the count of hooks found.
fn list_shell_hooks(
    hooks_dir: &std::path::Path,
    scope: &str,
    project: &std::path::Path,
    hook_events: &[HookEvent],
    toml: &InstalledHooksFile,
) -> usize {
    let mut count = 0;
    let mut any = false;
    for event in hook_events {
        let hook_path = hooks_dir.join(event.hook_filename());
        if !hook_path.exists() {
            continue;
        }
        let content = std::fs::read_to_string(&hook_path).unwrap_or_default();
        if !content.contains("linthis") {
            continue;
        }
        any = true;
        count += 1;

        let hook_type = detect_hook_type_from_content(&content, toml, scope, project, event);
        let provider = detect_provider_from_content(&content, toml, scope, project, event);

        println!(
            "  {} {} {} {}",
            "✓".green(),
            event.hook_filename(),
            format!("[{}]", hook_type).dimmed(),
            if provider.is_empty() {
                String::new()
            } else {
                format!("(provider: {})", provider)
            }
        );
    }
    if !any {
        println!("  {} No linthis shell hooks installed", "—".dimmed());
    }
    count
}

/// List agent skills for a given base directory. Returns the count of skill entries found.
fn list_agent_skills(
    base: &std::path::Path,
    global: bool,
    hook_events: &[HookEvent],
    skill_names: Option<&linthis::config::AgentSkillNamesConfig>,
) -> usize {
    let mut count = 0;
    let mut any = false;
    for p in ALL_AGENT_PROVIDERS {
        if !agent_is_installed(base, p, global, skill_names) {
            continue;
        }
        any = true;
        let mut event_tags: Vec<&str> = Vec::new();
        for event in hook_events {
            let path = agent_skill_path(base, p, global, event, skill_names);
            if path.exists() {
                count += 1;
                event_tags.push(event.hook_filename());
            }
        }
        let stop_hook = agent_stop_hook_settings_path(base, p)
            .map(|sp| {
                sp.exists()
                    && std::fs::read_to_string(&sp)
                        .map(|c| c.contains("linthis"))
                        .unwrap_or(false)
            })
            .unwrap_or(false);
        println!(
            "  {} {} [{}]{}",
            "✓".green(),
            p,
            event_tags.join(", "),
            if stop_hook {
                format!(" + {}", "stop-hook".dimmed())
            } else {
                String::new()
            }
        );
    }
    if !any {
        let label = if global {
            "No global agent skills installed"
        } else {
            "No agent skills installed"
        };
        println!("  {} {}", "—".dimmed(), label);
    }
    count
}

/// Print the summary footer for `hook list`.
fn print_list_footer(count: usize, global: bool) {
    println!();
    if count == 0 {
        if global {
            println!("No global hooks installed.");
            println!(
                "  Use {} to view project hooks.",
                "linthis hook list".cyan()
            );
        } else {
            println!("No project hooks installed.");
            println!(
                "  Use {} to view global hooks.",
                "linthis hook list -g".cyan()
            );
        }
    } else {
        let hint = if global {
            format!(" (use {} for project hooks)", "linthis hook list".cyan())
        } else {
            format!(" (use {} for global hooks)", "linthis hook list -g".cyan())
        };
        println!("{} {} hook entries found{}", "Total:".bold(), count, hint);
    }
}

/// List all installed linthis hooks.
pub(crate) fn handle_hook_list(global: bool) -> ExitCode {
    let toml = load_installed_hooks();

    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let skill_names_cfg = linthis::config::Config::load_merged(&cwd)
        .hook
        .agent
        .skill_names;
    let skill_names = Some(&skill_names_cfg);

    let hook_events = [
        HookEvent::PreCommit,
        HookEvent::PrePush,
        HookEvent::CommitMsg,
    ];
    let scope_label = if global { "Global" } else { "Project" };

    println!("{}", format!("Installed Hooks ({})", scope_label).bold());
    println!();

    let mut count: usize = 0;

    if global {
        println!("{}", "Shell Hooks (~/.config/git/hooks/)".bold());
        if let Some(ref ghooks) = global_hooks_dir() {
            count += list_shell_hooks(ghooks, "global", &PathBuf::new(), &hook_events, &toml);
        } else {
            println!("  {} No global linthis shell hooks installed", "—".dimmed());
        }

        println!();
        println!("{}", "Agent Skills (~/)".bold());
        if let Some(ref home_dir) = linthis::utils::home_dir() {
            count += list_agent_skills(home_dir, true, &hook_events, skill_names);
        }
    } else {
        let git_root = find_git_root();
        let project_root_display = git_root
            .as_ref()
            .map(|r| r.display().to_string())
            .unwrap_or_else(|| "(not in a git repository)".to_string());

        println!("{}", "Shell Hooks (.git/hooks/)".bold());
        if let Some(ref root) = git_root {
            count += list_shell_hooks(&root.join(".git/hooks"), "local", root, &hook_events, &toml);
        } else {
            println!("  {} {}", "—".dimmed(), project_root_display);
        }

        println!();
        println!("{}", "Agent Skills".bold());
        if let Some(ref root) = git_root {
            count += list_agent_skills(root, false, &hook_events, skill_names);
        } else {
            println!("  {} {}", "—".dimmed(), project_root_display);
        }
    }

    print_list_footer(count, global);
    ExitCode::SUCCESS
}

/// Check hook file for multiple tool conflicts. Returns (has_conflicts, warnings).
fn check_hook_tool_conflicts(hook_path: &std::path::Path) -> (bool, Vec<&'static str>) {
    let mut warnings = Vec::new();
    if !hook_path.exists() {
        return (false, warnings);
    }
    if let Ok(content) = std::fs::read_to_string(hook_path) {
        let tools = [
            content.contains("prek"),
            content.contains("pre-commit"),
            content.contains("husky"),
            content.contains("linthis"),
        ];
        let tool_count = tools.iter().filter(|&&x| x).count();
        if tool_count > 1 {
            println!(
                "{} Multiple hook tools detected in {}",
                "⚠".yellow(),
                hook_path.display()
            );
            if content.contains("linthis") {
                println!("  {} linthis", "✓".green());
            }
            if content.contains("prek") {
                println!("  {} prek", "⚠".yellow());
            }
            if content.contains("pre-commit") {
                println!("  {} pre-commit", "⚠".yellow());
            }
            if content.contains("husky") {
                println!("  {} husky", "⚠".yellow());
            }
            warnings.push("Consider using only one hook management tool");
            return (true, warnings);
        }
    }
    (false, warnings)
}

/// Check for hook conflicts
pub(crate) fn handle_hook_check() -> ExitCode {
    let git_root = match find_git_root() {
        Some(root) => root,
        None => {
            eprintln!("{}: Not in a git repository", "Error".red());
            return ExitCode::from(1);
        }
    };

    let hook_path = git_root.join(".git/hooks/pre-commit");
    let prek_config = std::path::Path::new(".pre-commit-config.yaml");
    let husky_dir = std::path::Path::new(".husky");

    println!("{}", "Checking for hook conflicts...".bold());
    println!();

    let (mut has_conflicts, mut warnings) = check_hook_tool_conflicts(&hook_path);

    if prek_config.exists() {
        if let Ok(content) = std::fs::read_to_string(prek_config) {
            if content.contains("linthis") && !hook_path.exists() {
                has_conflicts = true;
                println!(
                    "{} {} exists but no hook installed",
                    "⚠".yellow(),
                    prek_config.display()
                );
                warnings.push("Run 'prek install' or 'pre-commit install' to activate hooks");
            }
        }
    }

    if husky_dir.exists() && husky_dir.join("pre-commit").exists() {
        println!(
            "{} Husky detected: {}",
            "ℹ".cyan(),
            husky_dir.join("pre-commit").display()
        );
        warnings.push("Husky manages its own hooks in .husky/ directory");
        warnings.push("To use linthis with husky, add linthis command to .husky/pre-commit");
    }

    println!();
    if has_conflicts {
        println!("{}", "Conflicts detected:".yellow().bold());
        for warning in warnings {
            println!("  • {}", warning);
        }
        println!();
        println!("{}", "Recommendations:".bold());
        println!(
            "  • Use {} to see current hook setup",
            "linthis hook status".cyan()
        );
        println!("  • Choose one hook tool and stick with it");
        println!("  • For teams, document hook setup in README");
    } else {
        println!("{} No conflicts detected", "✓".green().bold());
    }

    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_commented_out_stub_runs_nothing() {
        // The shape that let unchecked commits through: code-review-graph's
        // stub, whose whole body is commented out.
        let stub = "#!/bin/sh\n\
                    # Installed by code-review-graph. Remove this file to disable checks.\n\
                    #if command -v code-review-graph >/dev/null 2>&1; then\n\
                    #    code-review-graph update || true\n\
                    #fi\n";
        assert!(is_noop_hook(stub));
        assert!(!runs_linthis(stub));
    }

    #[test]
    fn a_mention_in_a_comment_is_not_an_invocation() {
        let mentioned = "#!/bin/sh\n# linthis used to run here\nexit 0\n";
        assert!(!runs_linthis(mentioned), "a comment is not a command");
        assert!(!is_noop_hook(mentioned), "`exit 0` is still a command");
    }

    #[test]
    fn a_commented_out_invocation_is_not_an_invocation() {
        let disabled = "#!/bin/sh\n#linthis hook run --event pre-commit --type git\n";
        assert!(!runs_linthis(disabled));
        assert!(is_noop_hook(disabled));
    }

    #[test]
    fn the_real_managed_block_counts_as_installed() {
        let real = "#!/bin/sh\n\
                    # >>> linthis managed block >>>\n\
                    if command -v linthis >/dev/null 2>&1; then\n\
                    \x20 linthis hook run --event pre-commit --type git --global \"$@\" || exit $?\n\
                    fi\n";
        assert!(runs_linthis(real));
        assert!(!is_noop_hook(real));
    }

    #[test]
    fn an_empty_hook_runs_nothing() {
        assert!(is_noop_hook(""));
        assert!(is_noop_hook("\n   \n"));
        assert!(is_noop_hook("#!/bin/sh\n"));
    }

    #[test]
    fn a_foreign_hook_is_neither_noop_nor_linthis() {
        let husky = "#!/bin/sh\n. \"$(dirname \"$0\")/_/husky.sh\"\nnpx lint-staged\n";
        assert!(!is_noop_hook(husky));
        assert!(!runs_linthis(husky));
    }
}
