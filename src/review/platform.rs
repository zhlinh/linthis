//! Git platform detection and PR/MR creation.

use std::collections::HashMap;
use std::process::Command;
use crate::config::PlatformConfig;

/// Built-in platform defaults
fn builtin_platforms() -> HashMap<String, PlatformConfig> {
    let mut map = HashMap::new();
    map.insert("github.com".to_string(), PlatformConfig {
        pr_create: "gh pr create".to_string(),
        pr_list: Some("gh pr list".to_string()),
        reviewer_flag: "--reviewer".to_string(),
        install_cmd: None,
        install_hint: Some("Install: https://cli.github.com/\n  Auth: gh auth login".to_string()),
    });
    map.insert("gitlab.com".to_string(), PlatformConfig {
        pr_create: "glab mr create".to_string(),
        pr_list: Some("glab mr list".to_string()),
        reviewer_flag: "--reviewer".to_string(),
        install_cmd: None,
        install_hint: Some("Install: https://gitlab.com/gitlab-org/cli\n  Auth: glab auth login".to_string()),
    });
    map
}

/// Detect the Git platform domain from the remote URL.
pub fn detect_platform_domain() -> Result<String, String> {
    let output = Command::new("git")
        .args(["remote", "get-url", "origin"])
        .output()
        .map_err(|e| format!("Failed to get remote URL: {}", e))?;

    if !output.status.success() {
        return Err("No 'origin' remote found".to_string());
    }

    let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
    extract_domain(&url).ok_or_else(|| format!("Could not parse domain from URL: {}", url))
}

/// Extract domain from a git remote URL.
pub fn extract_domain(url: &str) -> Option<String> {
    // SSH format: git@domain:path
    if let Some(rest) = url.strip_prefix("git@") {
        return rest.split(':').next().map(|s| s.to_string());
    }

    // SSH with explicit scheme: ssh://git@domain/path
    if url.starts_with("ssh://") {
        let without_scheme = url.strip_prefix("ssh://").unwrap();
        let without_user = if let Some(at) = without_scheme.find('@') {
            &without_scheme[at + 1..]
        } else {
            without_scheme
        };
        return without_user.split('/').next().map(|s| s.to_string());
    }

    // HTTPS format: https://domain/path
    if url.starts_with("https://") || url.starts_with("http://") {
        let without_scheme = url.split("://").nth(1)?;
        return without_scheme.split('/').next().map(|s| s.to_string());
    }

    None
}

/// Resolve the platform config for the current repo.
pub fn resolve_platform(
    domain: &str,
    user_platforms: &HashMap<String, PlatformConfig>,
) -> Option<PlatformConfig> {
    user_platforms.get(domain).cloned()
        .or_else(|| builtin_platforms().get(domain).cloned())
}

/// Check if the platform CLI tool is available, offering auto-install if possible.
pub fn check_tool_available(platform: &PlatformConfig) -> Result<(), String> {
    let tool = platform.pr_create.split_whitespace().next().unwrap_or("");
    let which_cmd = if cfg!(target_os = "windows") { "where" } else { "which" };
    let output = Command::new(which_cmd)
        .arg(tool)
        .output();

    match output {
        Ok(o) if o.status.success() => Ok(()),
        _ => {
            // Try auto-install if install_cmd is configured and we have a TTY
            if let Some(ref install_cmd) = platform.install_cmd {
                let hint_msg = platform.install_hint.as_deref().unwrap_or("Install CLI tool");
                if std::io::IsTerminal::is_terminal(&std::io::stdin()) {
                    eprintln!("⚠ {} is required for PR creation but not found.", tool);
                    eprint!("  {} Install now? [Y/n] ", hint_msg);

                    let mut answer = String::new();
                    if std::io::stdin().read_line(&mut answer).is_ok() {
                        let answer = answer.trim().to_lowercase();
                        if answer.is_empty() || answer == "y" || answer == "yes" {
                            eprintln!("  → Installing {}...", tool);
                            let status = Command::new("sh")
                                .arg("-c")
                                .arg(install_cmd)
                                .status();

                            match status {
                                Ok(s) if s.success() => {
                                    eprintln!("  ✓ {} installed successfully", tool);
                                    return Ok(());
                                }
                                Ok(s) => {
                                    return Err(format!(
                                        "Installation failed (exit {}). Try manually:\n  {}",
                                        s.code().unwrap_or(-1),
                                        install_cmd,
                                    ));
                                }
                                Err(e) => {
                                    return Err(format!(
                                        "Failed to run installer: {}. Try manually:\n  {}",
                                        e, install_cmd,
                                    ));
                                }
                            }
                        }
                    }
                }
                // Non-interactive or user declined
                return Err(format!(
                    "⚠ {} is required for PR creation.\n  {}\n  Install: {}",
                    tool, hint_msg, install_cmd,
                ));
            }

            // No install_cmd — fall back to static hints
            let hint = if let Some(ref hint) = platform.install_hint {
                hint.clone()
            } else {
                match tool {
                    "gh" => "Install: https://cli.github.com/\n  Auth: gh auth login".to_string(),
                    "glab" => "Install: https://gitlab.com/gitlab-org/cli\n  Auth: glab auth login".to_string(),
                    _ => "Ensure the CLI tool is installed and in PATH".to_string(),
                }
            };
            Err(format!("⚠ {} is required for PR creation.\n  {}", tool, hint))
        }
    }
}

/// Sanitize branch name (replace slashes with dashes).
pub fn sanitize_branch_name(name: &str) -> String {
    name.replace(['/', '\\', ' ', ':'], "-")
}

/// Build the fix branch name.
pub fn fix_branch_name(original_branch: &str) -> String {
    let sanitized = sanitize_branch_name(original_branch);
    let timestamp = chrono_timestamp();
    format!("review/fix-{}-{}", sanitized, timestamp)
}

fn chrono_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("{}", secs)
}

/// Expand template variables in a command string.
pub fn expand_template(template: &str, vars: &HashMap<String, String>) -> String {
    let mut result = template.to_string();
    for (key, value) in vars {
        result = result.replace(&format!("{{{{{}}}}}", key), value);
    }
    result
}

/// Create a PR/MR using the platform CLI.
pub fn create_pr(
    platform: &PlatformConfig,
    title: &str,
    description: &str,
    base_branch: &str,
    reviewers: &[String],
    dry_run: bool,
) -> Result<String, String> {
    let mut vars = HashMap::new();
    vars.insert("title".to_string(), title.to_string());
    vars.insert("description".to_string(), description.to_string());
    vars.insert("base".to_string(), base_branch.to_string());
    vars.insert("reviewers".to_string(), reviewers.join(","));

    let cmd_template = &platform.pr_create;

    // Build command with standard flags
    let mut parts: Vec<String> = cmd_template.split_whitespace().map(|s| s.to_string()).collect();

    // Add title and description if not already in template
    if !cmd_template.contains("{{title}}") {
        parts.extend(["--title".to_string(), title.to_string()]);
    }
    if !cmd_template.contains("{{description}}") {
        let desc_flag = if cmd_template.contains("glab") { "--description" } else { "--body" };
        parts.extend([desc_flag.to_string(), description.to_string()]);
    }

    // Add reviewers
    if !reviewers.is_empty() {
        parts.extend([platform.reviewer_flag.clone(), reviewers.join(",")]);
    }

    // Expand any template variables
    let expanded: Vec<String> = parts.iter().map(|p| expand_template(p, &vars)).collect();

    if dry_run {
        return Ok(format!("[dry-run] Would run: {}", expanded.join(" ")));
    }

    if expanded.is_empty() {
        return Err("Empty PR command".to_string());
    }

    let output = Command::new(&expanded[0])
        .args(&expanded[1..])
        .output()
        .map_err(|e| format!("Failed to create PR: {}", e))?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(stdout)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(format!("PR creation failed: {}", stderr))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_domain_ssh() {
        assert_eq!(extract_domain("git@github.com:user/repo.git"), Some("github.com".to_string()));
        assert_eq!(extract_domain("git@git.company.com:group/repo"), Some("git.company.com".to_string()));
    }

    #[test]
    fn test_extract_domain_https() {
        assert_eq!(extract_domain("https://github.com/user/repo.git"), Some("github.com".to_string()));
        assert_eq!(extract_domain("https://gitlab.com/user/repo.git"), Some("gitlab.com".to_string()));
    }

    #[test]
    fn test_extract_domain_ssh_scheme() {
        assert_eq!(extract_domain("ssh://git@github.com/user/repo.git"), Some("github.com".to_string()));
    }

    #[test]
    fn test_sanitize_branch_name() {
        assert_eq!(sanitize_branch_name("feature/auth"), "feature-auth");
        assert_eq!(sanitize_branch_name("fix/bug:123"), "fix-bug-123");
        assert_eq!(sanitize_branch_name("main"), "main");
    }

    #[test]
    fn test_expand_template() {
        let mut vars = HashMap::new();
        vars.insert("title".to_string(), "Fix bug".to_string());
        vars.insert("description".to_string(), "Details".to_string());

        let result = expand_template("cli mr create --title {{title}} --desc {{description}}", &vars);
        assert_eq!(result, "cli mr create --title Fix bug --desc Details");
    }

    #[test]
    fn test_resolve_platform_builtin() {
        let empty = HashMap::new();
        let github = resolve_platform("github.com", &empty);
        assert!(github.is_some());
        assert!(github.unwrap().pr_create.contains("gh"));

        let custom = resolve_platform("unknown.com", &empty);
        assert!(custom.is_none());
    }

    #[test]
    fn test_resolve_platform_user_override() {
        let mut user = HashMap::new();
        user.insert("github.com".to_string(), PlatformConfig {
            pr_create: "custom-gh pr create".to_string(),
            pr_list: None,
            reviewer_flag: "--assignee".to_string(),
            install_cmd: None,
            install_hint: None,
        });
        let result = resolve_platform("github.com", &user).unwrap();
        assert!(result.pr_create.contains("custom-gh"));
    }
}
