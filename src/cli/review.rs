//! CLI handler for the `linthis review` subcommand.

use std::fs;
use std::process::ExitCode;

use colored::Colorize;
use linthis::ai::{AiProvider, AiProviderConfig, AiProviderKind};
use linthis::config::Config;
use linthis::review::analyzer;
use linthis::review::background;
use linthis::review::diff;
use linthis::review::notifier;
use linthis::review::platform;
use linthis::review::report;
use linthis::review::reviewer;
use linthis::review::Assessment;

use crate::cli::helpers::resolve_ai_provider;

/// Options for the review command
pub struct ReviewCommandOptions {
    pub background: bool,
    pub auto_fix: bool,
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
    let retention = config.review.retention_days;
    match background::clean_artifacts(retention) {
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
    // Build args for the foreground review (everything except --background)
    let mut args: Vec<String> = Vec::new();

    if options.auto_fix {
        args.push("--auto-fix".to_string());
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
            eprintln!("{}: Failed to start background review: {}", "Error".red(), e);
            ExitCode::from(1)
        }
    }
}

fn handle_review_foreground(options: ReviewCommandOptions) -> ExitCode {
    let config = Config::load_merged(&std::env::current_dir().unwrap_or_default());

    // 1. Detect base ref
    let base_ref = match diff::detect_base_ref(options.base.as_deref()) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("{}: {}", "Error".red(), e);
            return ExitCode::from(1);
        }
    };
    let head_ref = &options.head;
    eprintln!(
        "{} Reviewing diff {}..{}",
        "→".cyan(),
        base_ref,
        head_ref
    );

    // 2. Collect diff
    let diff_result = match diff::collect_diff(&base_ref, head_ref) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("{}: {}", "Error".red(), e);
            return ExitCode::from(1);
        }
    };

    if diff_result.files.is_empty() {
        eprintln!("{} No changes found between {} and {}", "✓".green(), base_ref, head_ref);
        return ExitCode::SUCCESS;
    }

    eprintln!(
        "  {} file(s), +{} -{}",
        diff_result.files.len(),
        diff_result.total_additions,
        diff_result.total_deletions
    );

    // 3. Resolve AI provider
    let provider_str = resolve_ai_provider(
        options.provider.as_deref(),
        config.review.provider.as_deref().or(config.ai.provider.as_deref()),
    );
    let provider_kind: AiProviderKind = provider_str.parse().unwrap_or_default();
    let ai_config = create_provider_config(&provider_kind);

    let provider = AiProvider::new(ai_config);

    if !linthis::ai::is_provider_available(&provider_kind) {
        eprintln!(
            "{}: AI provider '{}' is not available",
            "Error".red(),
            provider_str
        );
        print_provider_hint(&provider_kind);
        return ExitCode::from(1);
    }

    eprintln!("{} Using AI provider: {}", "→".cyan(), provider_str);

    // 4. Run AI analysis
    eprintln!("{} Analyzing code changes...", "→".cyan());
    let review_result = match analyzer::analyze(&diff_result, &provider) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{}: AI review failed: {}", "Error".red(), e);
            return ExitCode::from(1);
        }
    };

    // 5. Generate and save report
    let report_content = match options.output.as_str() {
        "json" => report::generate_json_report(&review_result),
        _ => report::generate_markdown_report(&review_result),
    };

    let report_ext = if options.output == "json" { "json" } else { "md" };
    let report_path = match save_report(&report_content, report_ext) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{}: Failed to save report: {}", "Error".red(), e);
            // Print to stdout as fallback
            println!("{}", report_content);
            return ExitCode::from(1);
        }
    };
    eprintln!("{} Report saved: {}", "✓".green(), report_path);

    // Print full report content
    println!("{}", report_content);

    // Print boxed summary
    println!("{}", linthis::utils::output::format_review_box(&review_result));

    // 6. Handle auto-fix + PR creation if enabled
    let auto_fix = options.auto_fix || config.review.auto_fix;
    let pr_url = if auto_fix && !options.no_pr {
        match handle_auto_fix_pr(
            &options,
            &config,
            &review_result,
            &report_path,
            &base_ref,
            &provider_str,
        ) {
            Ok(url) => Some(url),
            Err(e) => {
                eprintln!("{}: Auto-fix/PR creation failed: {}", "Warning".yellow(), e);
                None
            }
        }
    } else {
        None
    };

    // 7. Send notifications
    if !config.review.notifications.is_empty() {
        let current_branch = get_current_branch().unwrap_or_else(|_| "unknown".to_string());
        let vars = notifier::build_template_vars(
            &review_result,
            &current_branch,
            &report_path,
            pr_url.as_deref(),
        );
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

    // 8. Exit code based on assessment
    match review_result.summary.assessment {
        Assessment::CriticalIssues => ExitCode::from(2),
        Assessment::NeedsWork => ExitCode::from(1),
        Assessment::Ready => ExitCode::SUCCESS,
    }
}

fn handle_auto_fix_pr(
    options: &ReviewCommandOptions,
    config: &Config,
    review_result: &linthis::review::ReviewResult,
    report_path: &str,
    base_branch: &str,
    _provider_str: &str,
) -> Result<String, String> {
    // Detect platform
    let domain = platform::detect_platform_domain()?;
    let platform_config = platform::resolve_platform(&domain, &config.review.platforms)
        .ok_or_else(|| format!("No platform config found for domain '{}'", domain))?;

    // Check CLI tool availability
    platform::check_tool_available(&platform_config)?;

    // Resolve reviewers
    let changed_files: Vec<String> = review_result.files.iter()
        .map(|f| f.path.display().to_string())
        .collect();
    let reviewers = reviewer::resolve_reviewers(
        &options.reviewers,
        &config.review.reviewers,
        &changed_files,
    );

    // Create fix branch
    let original_branch = get_current_branch()?;
    let fix_branch = platform::fix_branch_name(&original_branch);

    // Create and checkout the fix branch
    run_git(&["checkout", "-b", &fix_branch])?;

    // Stage and commit the review report
    run_git(&["add", report_path])?;
    run_git(&["commit", "-m", &format!("review: add code review report for {}", original_branch)])?;

    // Push the branch
    if !options.dry_run {
        run_git(&["push", "-u", "origin", &fix_branch])?;
    }

    // Generate PR description
    let pr_title = format!(
        "review: {} — {} issues ({})",
        review_result.summary.assessment,
        review_result.summary.total_issues,
        original_branch
    );
    let pr_description = report::generate_notification_summary(review_result);

    // Create PR
    let pr_result = platform::create_pr(
        &platform_config,
        &pr_title,
        &pr_description,
        base_branch,
        &reviewers,
        options.dry_run,
    )?;

    // Switch back to original branch
    let _ = run_git(&["checkout", &original_branch]);

    eprintln!("{} PR created: {}", "✓".green(), pr_result);
    Ok(pr_result)
}

fn create_provider_config(kind: &AiProviderKind) -> AiProviderConfig {
    let mut config = match kind {
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
    };

    // Set API key from environment
    config.api_key = match kind {
        AiProviderKind::Claude => std::env::var("ANTHROPIC_AUTH_TOKEN")
            .or_else(|_| std::env::var("ANTHROPIC_API_KEY"))
            .ok(),
        AiProviderKind::CodeBuddy => std::env::var("CODEBUDDY_API_KEY").ok(),
        AiProviderKind::OpenAi | AiProviderKind::CodexCli => {
            std::env::var("OPENAI_API_KEY").ok()
        }
        AiProviderKind::Gemini => std::env::var("GEMINI_API_KEY")
            .or_else(|_| std::env::var("GOOGLE_API_KEY"))
            .ok(),
        _ => None,
    };

    // Set endpoint from environment
    match kind {
        AiProviderKind::Claude => {
            if let Ok(base_url) = std::env::var("ANTHROPIC_BASE_URL") {
                config.endpoint = Some(base_url);
            }
        }
        AiProviderKind::CodeBuddy => {
            if let Ok(base_url) = std::env::var("CODEBUDDY_BASE_URL") {
                config.endpoint = Some(base_url);
            }
        }
        _ => {}
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
