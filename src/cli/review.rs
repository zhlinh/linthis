//! CLI handler for the `linthis review` subcommand.

use std::fs;
use std::process::ExitCode;

use colored::Colorize;
use linthis::ai::{AiProvider, AiProviderConfig, AiProviderKind};
use linthis::config::Config;
use linthis::review::analyzer;
use linthis::review::background;
use linthis::review::diff;
use linthis::review::fixer;
use linthis::review::notifier;
use linthis::review::platform;
use linthis::review::report;
use linthis::review::reviewer;
use linthis::review::{Assessment, AutoFixMode};

use crate::cli::helpers::resolve_ai_provider;

/// Options for the review command
pub struct ReviewCommandOptions {
    pub background: bool,
    pub auto_fix: bool,
    pub auto_fix_mode: Option<String>,
    pub reviewers: Option<Vec<String>>,
    pub provider: Option<String>,
    pub base: Option<String>,
    pub head: String,
    pub no_pr: bool,
    pub notify: Option<Vec<String>>,
    pub status: bool,
    pub dry_run: bool,
    pub clean: bool,
    pub output: String,
}

/// Handle the review command
pub fn handle_review_command(options: ReviewCommandOptions) -> ExitCode {
    if options.status {
        return handle_review_status();
    }
    if options.clean {
        return handle_review_clean();
    }
    if options.background {
        return handle_review_background(options);
    }
    handle_review_foreground(options)
}

fn handle_review_status() -> ExitCode {
    let statuses = background::check_status();
    if statuses.is_empty() {
        println!("No reviews in progress");
    } else {
        for status in &statuses {
            println!("{}", status);
        }
    }
    ExitCode::SUCCESS
}

fn handle_review_clean() -> ExitCode {
    let config = Config::load_merged(&std::env::current_dir().unwrap_or_default());
    // Prefer retention.reviews (count-based); fall back to review.retention_days (days-based)
    let result = if config.retention.reviews > 0 {
        background::clean_artifacts_by_count(config.retention.reviews)
    } else {
        background::clean_artifacts(config.review.retention_days)
    };
    match result {
        Ok(count) => {
            println!("Cleaned {} review artifact(s)", count);
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{}: {}", "Error".red(), e);
            ExitCode::from(1)
        }
    }
}

fn handle_review_background(options: ReviewCommandOptions) -> ExitCode {
    let config = Config::load_merged(&std::env::current_dir().unwrap_or_default());

    // Build args for the foreground review (everything except --background)
    let mut args: Vec<String> = Vec::new();

    if options.auto_fix {
        args.push("--auto-fix".to_string());
    }
    if let Some(ref mode) = options.auto_fix_mode {
        // Explicit CLI mode takes priority
        args.extend(["--auto-fix-mode".to_string(), mode.clone()]);
    } else {
        // Background/hook context: use hook_auto_fix_mode from config
        args.extend([
            "--auto-fix-mode".to_string(),
            config.hook.review.auto_fix_mode.to_string(),
        ]);
    }
    if let Some(ref reviewers) = options.reviewers {
        for r in reviewers {
            args.extend(["--reviewer".to_string(), r.clone()]);
        }
    }
    if let Some(ref provider) = options.provider {
        args.extend(["--provider".to_string(), provider.clone()]);
    }
    if let Some(ref base) = options.base {
        args.extend(["--base".to_string(), base.clone()]);
    }
    if options.head != "HEAD" {
        args.extend(["--head".to_string(), options.head.clone()]);
    }
    if options.no_pr {
        args.push("--no-pr".to_string());
    }
    if let Some(ref notify) = options.notify {
        for n in notify {
            args.extend(["--notify".to_string(), n.clone()]);
        }
    }
    if options.dry_run {
        args.push("--dry-run".to_string());
    }
    if options.output != "markdown" {
        args.extend(["--output".to_string(), options.output.clone()]);
    }

    match background::spawn_background_review(&args) {
        Ok(_pid) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!(
                "{}: Failed to start background review: {}",
                "Error".red(),
                e
            );
            ExitCode::from(1)
        }
    }
}

/// Collected context for the review setup phase.
struct ReviewContext {
    config: Config,
    base_ref: String,
    diff_result: diff::DiffResult,
    provider_str: String,
    ai_config: AiProviderConfig,
}

/// Execute review setup: detect base ref, collect diff, resolve AI provider.
fn setup_review(options: &ReviewCommandOptions) -> Result<ReviewContext, ExitCode> {
    let config = Config::load_merged(&std::env::current_dir().unwrap_or_default());

    let base_ref = match diff::detect_base_ref(options.base.as_deref()) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("{}: {}", "Error".red(), e);
            return Err(ExitCode::from(1));
        }
    };
    let head_ref = &options.head;
    eprintln!("{} Reviewing diff {}..{}", "→".cyan(), base_ref, head_ref);

    let diff_result = match diff::collect_diff(&base_ref, head_ref) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("{}: {}", "Error".red(), e);
            return Err(ExitCode::from(1));
        }
    };

    if diff_result.files.is_empty() {
        eprintln!(
            "{} No changes found between {} and {}",
            "✓".green(),
            base_ref,
            head_ref
        );
        return Err(ExitCode::SUCCESS);
    }

    eprintln!(
        "  {} file(s), +{} -{}",
        diff_result.files.len(),
        diff_result.total_additions,
        diff_result.total_deletions
    );

    let provider_str = resolve_ai_provider(
        options.provider.as_deref(),
        config
            .review
            .provider
            .as_deref()
            .or(config.ai.provider.as_deref()),
    );
    let provider_kind: AiProviderKind = provider_str.parse().unwrap_or_default();
    let ai_config = create_provider_config(&provider_kind);

    if !linthis::ai::is_provider_available(&provider_kind) {
        eprintln!(
            "{}: AI provider '{}' is not available",
            "Error".red(),
            provider_str
        );
        print_provider_hint(&provider_kind);
        return Err(ExitCode::from(1));
    }

    eprintln!("{} Using AI provider: {}", "→".cyan(), provider_str);

    Ok(ReviewContext {
        config,
        base_ref,
        diff_result,
        provider_str,
        ai_config,
    })
}

/// Run AI analysis and generate the review report. Returns (review_result, report_content, report_path).
fn run_analysis_and_report(
    diff_result: &diff::DiffResult,
    ai_config: &AiProviderConfig,
    output_format: &str,
) -> Result<(linthis::review::ReviewResult, String, String), ExitCode> {
    let provider = AiProvider::new(ai_config.clone());

    eprintln!("{} Analyzing code changes...", "→".cyan());
    let review_result = match analyzer::analyze(diff_result, &provider) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{}: AI review failed: {}", "Error".red(), e);
            return Err(ExitCode::from(1));
        }
    };

    let report_content = match output_format {
        "json" => report::generate_json_report(&review_result),
        _ => report::generate_markdown_report(&review_result),
    };

    let report_ext = if output_format == "json" {
        "json"
    } else {
        "md"
    };
    let report_path = match save_report(&report_content, report_ext) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{}: Failed to save report: {}", "Error".red(), e);
            println!("{}", report_content);
            return Err(ExitCode::from(1));
        }
    };
    eprintln!("{} Report saved: {}", "✓".green(), report_path);

    println!("{}", report_content);
    println!(
        "{}",
        linthis::utils::output::format_review_box(&review_result)
    );

    Ok((review_result, report_content, report_path))
}

/// Handle the auto-fix phase: resolve mode, dispatch to the appropriate handler.
fn handle_auto_fix_phase(
    options: &ReviewCommandOptions,
    config: &Config,
    review_result: &mut linthis::review::ReviewResult,
    report_path: &str,
    base_ref: &str,
    provider_str: &str,
    ai_config: &AiProviderConfig,
) -> (Option<String>, Option<ExitCode>) {
    let auto_fix = options.auto_fix || config.review.auto_fix;

    let auto_fix_mode = if let Some(ref mode_str) = options.auto_fix_mode {
        match mode_str.parse::<AutoFixMode>() {
            Ok(m) => m,
            Err(e) => {
                eprintln!("{}: {}", "Error".red(), e);
                return (None, Some(ExitCode::from(1)));
            }
        }
    } else {
        config.review.auto_fix_mode.clone()
    };

    if !auto_fix {
        return (None, None);
    }

    match auto_fix_mode {
        AutoFixMode::Pr if !options.no_pr => {
            match handle_auto_fix_pr(
                options,
                config,
                review_result,
                report_path,
                base_ref,
                provider_str,
                ai_config,
            ) {
                Ok(url) => (Some(url), None),
                Err(e) => {
                    eprintln!("{}: Auto-fix/PR creation failed: {}", "Warning".yellow(), e);
                    (None, None)
                }
            }
        }
        AutoFixMode::Commit => {
            if let Err(e) = handle_auto_fix_commit(review_result, ai_config) {
                eprintln!("{}: Auto-fix commit failed: {}", "Warning".yellow(), e);
            }
            (None, Some(ExitCode::from(1)))
        }
        AutoFixMode::Apply => {
            if let Err(e) = handle_auto_fix_apply(review_result, ai_config) {
                eprintln!("{}: Auto-fix apply failed: {}", "Warning".yellow(), e);
            }
            (None, Some(ExitCode::from(1)))
        }
        _ => (None, None),
    }
}

/// Send review notifications if configured.
fn send_review_notifications(
    config: &Config,
    review_result: &linthis::review::ReviewResult,
    report_path: &str,
    pr_url: Option<&str>,
) {
    if config.review.notifications.is_empty() {
        return;
    }

    let current_branch = get_current_branch().unwrap_or_else(|_| "unknown".to_string());
    let vars = notifier::build_template_vars(review_result, &current_branch, report_path, pr_url);
    let results = notifier::send_notifications(&config.review.notifications, &vars);
    let failures: Vec<_> = results.iter().filter(|r| r.is_err()).collect();
    if !failures.is_empty() {
        eprintln!(
            "{}: {} notification(s) failed",
            "Warning".yellow(),
            failures.len()
        );
    }
}

fn handle_review_foreground(options: ReviewCommandOptions) -> ExitCode {
    // 1-3. Setup: detect base, collect diff, resolve provider
    let ctx = match setup_review(&options) {
        Ok(ctx) => ctx,
        Err(code) => return code,
    };

    // 4-5. Run AI analysis and generate report
    let (mut review_result, _report_content, report_path) =
        match run_analysis_and_report(&ctx.diff_result, &ctx.ai_config, &options.output) {
            Ok(r) => r,
            Err(code) => return code,
        };

    // 6. Handle auto-fix if enabled
    let (pr_url, force_exit) = handle_auto_fix_phase(
        &options,
        &ctx.config,
        &mut review_result,
        &report_path,
        &ctx.base_ref,
        &ctx.provider_str,
        &ctx.ai_config,
    );

    // 7. Send notifications
    send_review_notifications(&ctx.config, &review_result, &report_path, pr_url.as_deref());

    // 8. Exit code: commit/apply modes force exit 1 to block push
    if let Some(exit_code) = force_exit {
        return exit_code;
    }

    match review_result.summary.assessment {
        Assessment::CriticalIssues => ExitCode::from(2),
        Assessment::NeedsWork => ExitCode::from(1),
        Assessment::Ready => ExitCode::SUCCESS,
    }
}

/// Count fixable issues (Critical or Important with a line number).
fn count_fixable_issues(review_result: &linthis::review::ReviewResult) -> usize {
    review_result
        .issues
        .iter()
        .filter(|i| {
            matches!(
                i.severity,
                linthis::review::Severity::Critical | linthis::review::Severity::Important
            ) && i.line.is_some()
        })
        .count()
}

/// Generate and apply fixes, returning the fix report if any fixes were applied.
fn generate_fixes(
    review_result: &mut linthis::review::ReviewResult,
    ai_config: &AiProviderConfig,
    fixable_count: usize,
) -> Option<linthis::review::fixer::FixReport> {
    if fixable_count == 0 {
        return None;
    }

    eprintln!(
        "{} Generating fixes for {} issue(s)...",
        "→".cyan(),
        fixable_count
    );
    let report = fixer::generate_and_apply_fixes(review_result, ai_config);
    eprintln!(
        "{} Fixed {}/{} issue(s)",
        "✓".green(),
        report.applied,
        fixable_count
    );

    Some(report)
}

/// Create a fix branch, commit fixes, and push.
fn create_fix_branch_and_push(
    options: &ReviewCommandOptions,
    fix_report: &Option<linthis::review::fixer::FixReport>,
    report_path: &str,
    original_branch: &str,
) -> Result<(String, usize), String> {
    let fix_branch = platform::fix_branch_name(original_branch);

    run_git(&["checkout", "-b", &fix_branch])?;

    if let Some(ref fr) = fix_report {
        for file in &fr.modified_files {
            run_git(&["add", &file.display().to_string()])?;
        }
    }

    run_git(&["add", report_path])?;

    let fix_count = fix_report.as_ref().map_or(0, |r| r.applied);
    let commit_msg = if fix_count > 0 {
        format!(
            "review: auto-fix {} issue(s) in {}",
            fix_count, original_branch
        )
    } else {
        format!("review: add code review report for {}", original_branch)
    };
    run_git(&["commit", "-m", &commit_msg])?;

    if !options.dry_run {
        run_git(&["push", "-u", "origin", &fix_branch])?;
    }

    Ok((fix_branch, fix_count))
}

/// Create PR on the platform and return its URL.
fn create_pr_for_fixes(
    options: &ReviewCommandOptions,
    review_result: &linthis::review::ReviewResult,
    config: &Config,
    base_branch: &str,
    original_branch: &str,
    fix_count: usize,
) -> Result<String, String> {
    let domain = platform::detect_platform_domain()?;
    let platform_config = platform::resolve_platform(&domain, &config.review.platforms)
        .ok_or_else(|| format!("No platform config found for domain '{}'", domain))?;

    platform::check_tool_available(&platform_config)?;

    let changed_files: Vec<String> = review_result
        .files
        .iter()
        .map(|f| f.path.display().to_string())
        .collect();
    let reviewers =
        reviewer::resolve_reviewers(&options.reviewers, &config.review.reviewers, &changed_files);

    let pr_title = if fix_count > 0 {
        format!(
            "review: auto-fix {} issue(s) — {} ({})",
            fix_count, review_result.summary.assessment, original_branch
        )
    } else {
        format!(
            "review: {} — {} issues ({})",
            review_result.summary.assessment, review_result.summary.total_issues, original_branch
        )
    };
    let pr_description = report::generate_notification_summary(review_result);

    platform::create_pr(
        &platform_config,
        &pr_title,
        &pr_description,
        base_branch,
        &reviewers,
        options.dry_run,
    )
}

fn handle_auto_fix_pr(
    options: &ReviewCommandOptions,
    config: &Config,
    review_result: &mut linthis::review::ReviewResult,
    report_path: &str,
    base_branch: &str,
    _provider_str: &str,
    ai_config: &linthis::ai::AiProviderConfig,
) -> Result<String, String> {
    let fixable_count = count_fixable_issues(review_result);
    let fix_report = generate_fixes(review_result, ai_config, fixable_count);

    let original_branch = get_current_branch()?;

    let (_fix_branch, fix_count) =
        create_fix_branch_and_push(options, &fix_report, report_path, &original_branch)?;

    let pr_result = create_pr_for_fixes(
        options,
        review_result,
        config,
        base_branch,
        &original_branch,
        fix_count,
    )?;

    // Switch back to original branch and restore original files
    let _ = run_git(&["checkout", &original_branch]);
    if let Some(ref fr) = fix_report {
        for file in &fr.modified_files {
            let _ = run_git(&["checkout", "--", &file.display().to_string()]);
        }
    }

    eprintln!("{} PR created: {}", "✓".green(), pr_result);
    Ok(pr_result)
}

fn handle_auto_fix_commit(
    review_result: &mut linthis::review::ReviewResult,
    ai_config: &linthis::ai::AiProviderConfig,
) -> Result<(), String> {
    let fixable_count = count_fixable_issues(review_result);

    if fixable_count == 0 {
        eprintln!("{} No fixable issues found", "→".cyan());
        return Ok(());
    }

    eprintln!(
        "{} Generating fixes for {} issue(s)...",
        "→".cyan(),
        fixable_count
    );
    let fix_report = fixer::generate_and_apply_fixes(review_result, ai_config);
    eprintln!(
        "{} Fixed {}/{} issue(s)",
        "✓".green(),
        fix_report.applied,
        fixable_count
    );

    if fix_report.applied == 0 {
        return Ok(());
    }

    // Stage fixed files
    for file in &fix_report.modified_files {
        run_git(&["add", &file.display().to_string()])?;
    }

    // Commit on current branch
    let commit_msg = format!("review: auto-fix {} issue(s)", fix_report.applied);
    run_git(&["commit", "-m", &commit_msg])?;

    eprintln!(
        "{} Committed {} fix(es) on current branch (not pushed)",
        "✓".green(),
        fix_report.applied
    );
    Ok(())
}

fn handle_auto_fix_apply(
    review_result: &mut linthis::review::ReviewResult,
    ai_config: &linthis::ai::AiProviderConfig,
) -> Result<(), String> {
    let fixable_count = count_fixable_issues(review_result);

    if fixable_count == 0 {
        eprintln!("{} No fixable issues found", "→".cyan());
        return Ok(());
    }

    eprintln!(
        "{} Generating fixes for {} issue(s)...",
        "→".cyan(),
        fixable_count
    );
    let fix_report = fixer::generate_and_apply_fixes(review_result, ai_config);
    eprintln!(
        "{} Applied {}/{} fix(es) to working tree (not committed)",
        "✓".green(),
        fix_report.applied,
        fixable_count
    );

    if !fix_report.modified_files.is_empty() {
        eprintln!("{} Modified files:", "→".cyan());
        for file in &fix_report.modified_files {
            eprintln!("  {}", file.display());
        }
    }

    Ok(())
}

/// Create the base provider config from the provider kind.
fn create_base_config(kind: &AiProviderKind) -> AiProviderConfig {
    match kind {
        AiProviderKind::Claude => AiProviderConfig::claude(),
        AiProviderKind::ClaudeCli => AiProviderConfig::claude_cli(),
        AiProviderKind::CodeBuddy => AiProviderConfig::codebuddy(),
        AiProviderKind::CodeBuddyCli => AiProviderConfig::codebuddy_cli(),
        AiProviderKind::OpenAi => AiProviderConfig::openai(),
        AiProviderKind::CodexCli => AiProviderConfig::codex_cli(),
        AiProviderKind::Gemini => AiProviderConfig::gemini(),
        AiProviderKind::GeminiCli => AiProviderConfig::gemini_cli(),
        AiProviderKind::Local => AiProviderConfig::local(),
        AiProviderKind::Custom(name) => AiProviderConfig {
            kind: AiProviderKind::Custom(name.clone()),
            ..AiProviderConfig::default()
        },
        AiProviderKind::Mock => AiProviderConfig::mock(),
    }
}

/// Resolve the API key for the given provider kind from environment variables.
fn resolve_api_key(kind: &AiProviderKind) -> Option<String> {
    match kind {
        AiProviderKind::Claude => std::env::var("ANTHROPIC_AUTH_TOKEN")
            .or_else(|_| std::env::var("ANTHROPIC_API_KEY"))
            .ok(),
        AiProviderKind::CodeBuddy => std::env::var("CODEBUDDY_API_KEY").ok(),
        AiProviderKind::OpenAi | AiProviderKind::CodexCli => std::env::var("OPENAI_API_KEY").ok(),
        AiProviderKind::Gemini => std::env::var("GEMINI_API_KEY")
            .or_else(|_| std::env::var("GOOGLE_API_KEY"))
            .ok(),
        _ => None,
    }
}

/// Resolve the endpoint for the given provider kind from environment variables.
fn resolve_endpoint(kind: &AiProviderKind) -> Option<String> {
    match kind {
        AiProviderKind::Claude => std::env::var("ANTHROPIC_BASE_URL").ok(),
        AiProviderKind::CodeBuddy => std::env::var("CODEBUDDY_BASE_URL").ok(),
        _ => None,
    }
}

fn create_provider_config(kind: &AiProviderKind) -> AiProviderConfig {
    let mut config = create_base_config(kind);
    config.api_key = resolve_api_key(kind);
    if let Some(endpoint) = resolve_endpoint(kind) {
        config.endpoint = Some(endpoint);
    }
    config
}

fn print_provider_hint(kind: &AiProviderKind) {
    match kind {
        AiProviderKind::Claude => {
            eprintln!("Set ANTHROPIC_AUTH_TOKEN or ANTHROPIC_API_KEY environment variable");
        }
        AiProviderKind::ClaudeCli => {
            eprintln!("Install Claude CLI (claude command must be available)");
        }
        AiProviderKind::OpenAi => {
            eprintln!("Set OPENAI_API_KEY environment variable");
        }
        AiProviderKind::CodexCli => {
            eprintln!("Install Codex CLI (npm install -g @openai/codex)");
        }
        AiProviderKind::Gemini => {
            eprintln!("Set GEMINI_API_KEY or GOOGLE_API_KEY environment variable");
        }
        AiProviderKind::GeminiCli => {
            eprintln!("Install Gemini CLI (npm install -g @google/gemini-cli)");
        }
        AiProviderKind::CodeBuddy => {
            eprintln!("Set CODEBUDDY_API_KEY environment variable");
        }
        AiProviderKind::CodeBuddyCli => {
            eprintln!("Install CodeBuddy CLI (codebuddy command must be available)");
        }
        AiProviderKind::Local => {
            eprintln!("Set LINTHIS_AI_ENDPOINT to your local LLM endpoint");
        }
        _ => {}
    }
}

fn save_report(content: &str, ext: &str) -> Result<String, String> {
    let dir = background::review_dir()?;
    let ts = background::timestamp();
    let path = dir.join(format!("{}.{}", ts, ext));
    fs::write(&path, content).map_err(|e| format!("Failed to write report: {}", e))?;
    Ok(path.display().to_string())
}

fn get_current_branch() -> Result<String, String> {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .map_err(|e| format!("Failed to get current branch: {}", e))?;

    if !output.status.success() {
        return Err("Failed to get current branch".to_string());
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn run_git(args: &[&str]) -> Result<String, String> {
    let output = std::process::Command::new("git")
        .args(args)
        .output()
        .map_err(|e| format!("git {} failed: {}", args.first().unwrap_or(&""), e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "git {} failed: {}",
            args.first().unwrap_or(&""),
            stderr.trim()
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}
