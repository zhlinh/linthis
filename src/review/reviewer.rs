//! Reviewer management and recommendation.

use crate::config::ReviewerConfig;
use std::collections::HashMap;
use std::process::Command;

/// Resolve reviewers from CLI args, config, or git history.
/// Priority: CLI args > config default > history recommendation.
pub fn resolve_reviewers(
    cli_reviewers: &Option<Vec<String>>,
    config: &ReviewerConfig,
    changed_files: &[String],
) -> Vec<String> {
    // CLI args take priority
    if let Some(reviewers) = cli_reviewers {
        if !reviewers.is_empty() {
            return reviewers.clone();
        }
    }

    // Config defaults
    if !config.default.is_empty() {
        return config.default.clone();
    }

    // History-based recommendation
    recommend_from_history(changed_files, 3)
}

/// Recommend reviewers based on git history of changed files.
pub fn recommend_from_history(changed_files: &[String], top_n: usize) -> Vec<String> {
    let current_user = get_current_git_user().unwrap_or_default();

    let mut contributor_counts: HashMap<String, usize> = HashMap::new();

    for file in changed_files {
        if let Ok(contributors) = get_file_contributors(file) {
            for name in contributors {
                if name != current_user && !name.is_empty() {
                    *contributor_counts.entry(name).or_insert(0) += 1;
                }
            }
        }
    }

    let mut sorted: Vec<(String, usize)> = contributor_counts.into_iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(&a.1));

    sorted
        .into_iter()
        .take(top_n)
        .map(|(name, _)| name)
        .collect()
}

fn get_current_git_user() -> Result<String, String> {
    let output = Command::new("git")
        .args(["config", "user.name"])
        .output()
        .map_err(|e| format!("Failed to get git user: {}", e))?;

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn get_file_contributors(file: &str) -> Result<Vec<String>, String> {
    let output = Command::new("git")
        .args(["log", "--format=%aN", "--", file])
        .output()
        .map_err(|e| format!("Failed to get file contributors: {}", e))?;

    if !output.status.success() {
        return Ok(vec![]);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.lines().map(|s| s.to_string()).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ReviewerConfig;

    #[test]
    fn test_resolve_reviewers_cli_priority() {
        let cli = Some(vec!["alice".to_string(), "bob".to_string()]);
        let config = ReviewerConfig {
            default: vec!["charlie".to_string()],
        };
        let result = resolve_reviewers(&cli, &config, &[]);
        assert_eq!(result, vec!["alice", "bob"]);
    }

    #[test]
    fn test_resolve_reviewers_config_fallback() {
        let config = ReviewerConfig {
            default: vec!["charlie".to_string()],
        };
        let result = resolve_reviewers(&None, &config, &[]);
        assert_eq!(result, vec!["charlie"]);
    }

    #[test]
    fn test_resolve_reviewers_empty_cli() {
        let cli = Some(vec![]);
        let config = ReviewerConfig {
            default: vec!["charlie".to_string()],
        };
        let result = resolve_reviewers(&cli, &config, &[]);
        assert_eq!(result, vec!["charlie"]);
    }
}
