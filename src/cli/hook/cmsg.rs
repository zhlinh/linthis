// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Commit message validation: handle_commit_msg_check, handle_cmsg_auto_fix.

use colored::Colorize;
use std::process::ExitCode;

/// Read the commit message from a file path or treat the string as the message itself.
fn read_commit_msg(msg_or_file: &str) -> Result<(String, bool), ExitCode> {
    let path = std::path::Path::new(msg_or_file);
    if path.exists() {
        match std::fs::read_to_string(path) {
            Ok(content) => Ok((content, true)),
            Err(e) => {
                eprintln!(
                    "{}: Failed to read commit message file: {}",
                    "Error".red(),
                    e
                );
                Err(ExitCode::from(1))
            }
        }
    } else {
        Ok((msg_or_file.to_string(), false))
    }
}

/// Validate a commit message against the configured pattern and ticket requirement.
/// Returns a list of validation errors (empty if valid).
fn validate_commit_msg(
    first_line: &str,
    config: &linthis::config::Config,
) -> Result<Vec<String>, ExitCode> {
    use regex::Regex;
    let mut errors = Vec::new();

    let regex = match Regex::new(&config.cmsg.commit_msg_pattern) {
        Ok(r) => r,
        Err(e) => {
            eprintln!(
                "{}: Invalid commit message pattern in config: {}",
                "Error".red(),
                e
            );
            return Err(ExitCode::from(2));
        }
    };

    if !regex.is_match(first_line) {
        errors.push(
            "Does not match Conventional Commits format (type(scope)?: description). Valid types: feat, fix, docs, style, refactor, perf, test, build, ci, chore, revert".to_string()
        );
    }

    if config.cmsg.require_ticket {
        let ticket_pattern = config
            .cmsg
            .ticket_pattern
            .as_deref()
            .unwrap_or(r"\[\w+-\d+\]");
        let ticket_regex = match Regex::new(ticket_pattern) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("{}: Invalid ticket pattern in config: {}", "Error".red(), e);
                return Err(ExitCode::from(2));
            }
        };
        if !ticket_regex.is_match(first_line) {
            errors.push(format!(
                "Missing ticket reference (pattern: {}). Example: feat: [PROJ-123] add feature",
                ticket_pattern
            ));
        }
    }

    Ok(errors)
}

/// Print the ticket reference required error box.
fn print_ticket_error(first_line: &str, ticket_pattern: &str) {
    eprintln!("{}", "╭────────────────────────────────────────╮".red());
    eprintln!("{}", "│ 🔴 Ticket Reference Required          │".red());
    eprintln!("{}", "├────────────────────────────────────────┤".red());
    eprintln!("│ Your message:                          │");
    eprintln!("│   {}", first_line);
    eprintln!("│                                        │");
    eprintln!("│ Ticket reference is required.          │");
    eprintln!("│ Pattern: {}                            │", ticket_pattern);
    eprintln!("│                                        │");
    eprintln!("│ Example:                               │");
    eprintln!("│   feat: [PROJ-123] add feature         │");
    eprintln!("{}", "├────────────────────────────────────────┤".red());
    eprintln!("│ To skip this check:                    │");
    eprintln!("│   git commit --no-verify               │");
    eprintln!("{}", "╰────────────────────────────────────────╯".red());
}

pub fn handle_commit_msg_check(
    msg_or_file: &str,
    auto_fix: bool,
    provider: Option<&str>,
) -> ExitCode {
    use linthis::config::Config;

    let project_root = linthis::utils::get_project_root();
    let config = Config::load_merged(&project_root);

    let (commit_msg, is_file) = match read_commit_msg(msg_or_file) {
        Ok(r) => r,
        Err(code) => return code,
    };

    let first_line = commit_msg.lines().next().unwrap_or("").trim();
    if first_line.is_empty() || first_line.starts_with('#') {
        return ExitCode::SUCCESS;
    }

    let errors = match validate_commit_msg(first_line, &config) {
        Ok(e) => e,
        Err(code) => return code,
    };

    if errors.is_empty() {
        println!("{}", linthis::utils::output::format_cmsg_result(true, ""));
        let paths = linthis::utils::output::format_hook_paths_footer_pub(Some("commit-msg"));
        if !paths.is_empty() {
            println!("{}", paths);
        }
        return ExitCode::SUCCESS;
    }

    if auto_fix {
        let path = std::path::Path::new(msg_or_file);
        return handle_cmsg_auto_fix(
            &commit_msg,
            &errors,
            is_file,
            path,
            provider,
            config.ai.provider.as_deref(),
        );
    }

    if errors.iter().any(|e| e.contains("Conventional Commits")) {
        print_commit_msg_error(first_line);
    } else {
        let ticket_pattern = config
            .cmsg
            .ticket_pattern
            .as_deref()
            .unwrap_or(r"\[\w+-\d+\]");
        print_ticket_error(first_line, ticket_pattern);
    }
    ExitCode::from(1)
}

/// Resolve an AiProviderConfig from its kind.
fn ai_provider_config_from_kind(
    kind: &linthis::ai::AiProviderKind,
) -> linthis::ai::AiProviderConfig {
    use linthis::ai::{AiProviderConfig, AiProviderKind};
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

/// Handle AI auto-fix for commit messages
pub(crate) fn handle_cmsg_auto_fix(
    original_msg: &str,
    errors: &[String],
    is_file: bool,
    file_path: &std::path::Path,
    cli_provider: Option<&str>,
    config_provider: Option<&str>,
) -> ExitCode {
    use crate::cli::helpers::resolve_ai_provider;
    use linthis::ai::{AiProvider, AiProviderKind, AiProviderTrait};

    let provider_name = resolve_ai_provider(cli_provider, config_provider);
    let kind: AiProviderKind = match provider_name.parse() {
        Ok(k) => k,
        Err(_) => {
            eprintln!("{}: Unknown AI provider: {}", "Error".red(), provider_name);
            return ExitCode::from(2);
        }
    };

    let provider = AiProvider::new(ai_provider_config_from_kind(&kind));

    eprintln!(
        "{} Rewriting commit message with AI (provider: {})...",
        "→".cyan(),
        provider_name.cyan()
    );

    let first_line = original_msg.lines().next().unwrap_or("").trim();
    let rest_of_msg: String = original_msg.lines().skip(1).collect::<Vec<_>>().join("\n");
    let error_desc = errors.join("; ");

    let prompt = format!(
        "Rewrite the following git commit message to conform to the Conventional Commits format.\n\n\
         Original message: {}\n\nValidation errors: {}\n\n\
         Rules:\n- Format: type(scope)?: description\n\
         - Valid types: feat, fix, docs, style, refactor, perf, test, build, ci, chore, revert\n\
         - Keep the original intent and meaning\n\
         - Output ONLY the rewritten first line, nothing else (no quotes, no explanation)",
        first_line, error_desc
    );

    match provider.complete(&prompt, Some("You are a git commit message formatter. Output only the corrected commit message first line.")) {
        Ok(fixed_line) => {
            let fixed_line = fixed_line.trim().trim_matches('"').trim_matches('\'').trim();
            let fixed_msg = if rest_of_msg.is_empty() {
                format!("{}\n", fixed_line)
            } else {
                format!("{}\n{}", fixed_line, rest_of_msg)
            };

            if is_file {
                if let Err(e) = std::fs::write(file_path, &fixed_msg) {
                    eprintln!("{}: Failed to write fixed message: {}", "Error".red(), e);
                    return ExitCode::from(1);
                }
                eprintln!("{} Commit message rewritten: {} → {}", "✓".green(), first_line.dimmed(), fixed_line.green());
            } else {
                eprintln!("{} Suggested rewrite: {}", "✓".green(), fixed_line.green());
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{}: AI auto-fix failed: {}", "Error".red(), e);
            print_commit_msg_error(first_line);
            ExitCode::from(1)
        }
    }
}

/// Print commit message validation error
fn print_commit_msg_error(first_line: &str) {
    eprintln!(
        "{}",
        linthis::utils::output::format_cmsg_result(false, first_line)
    );
    let paths = linthis::utils::output::format_hook_paths_footer_pub(Some("commit-msg"));
    if !paths.is_empty() {
        eprintln!("{}", paths);
    }
}
