//! AI review analysis logic.

use std::path::PathBuf;

use crate::ai::provider::AiProviderTrait;
use crate::review::diff::{chunk_diff, DiffResult, FileDiff};
use crate::review::prompts::{build_review_prompt, build_summary_review_prompt, review_system_prompt};
use crate::review::{
    Assessment, FileStatus, ReviewIssue, ReviewResult, ReviewSummary, ReviewedFile, Severity,
};

const DEFAULT_MAX_TOKENS_PER_CHUNK: usize = 8000;
const MAX_RETRIES: u32 = 3;
const INITIAL_RETRY_DELAY_MS: u64 = 2000;

/// Run AI review on a diff result.
pub fn analyze(diff: &DiffResult, provider: &dyn AiProviderTrait) -> Result<ReviewResult, String> {
    if diff.files.is_empty() {
        return Ok(empty_review_result(diff));
    }

    let system_prompt = review_system_prompt();

    // Chunk the diff
    let chunks = chunk_diff(&diff.files, DEFAULT_MAX_TOKENS_PER_CHUNK);

    let mut all_issues: Vec<ReviewIssue> = Vec::new();
    let mut file_summaries: Vec<(String, String)> = Vec::new();

    // Review each chunk
    for chunk in &chunks {
        if chunk.is_empty() {
            continue;
        }

        let file_refs: Vec<&FileDiff> = chunk.to_vec();
        let user_prompt = build_review_prompt(&file_refs);

        let response = call_with_retry(provider, &user_prompt, &system_prompt)?;
        let parsed = parse_review_response(&response)?;

        // Collect file summaries for cross-file review
        for file in chunk {
            let summary = format!(
                "{} (+{} -{})",
                match file.status {
                    crate::review::diff::DiffStatus::Added => "Added",
                    crate::review::diff::DiffStatus::Modified => "Modified",
                    crate::review::diff::DiffStatus::Deleted => "Deleted",
                    crate::review::diff::DiffStatus::Renamed => "Renamed",
                },
                file.additions,
                file.deletions
            );
            file_summaries.push((file.path.clone(), summary));
        }

        all_issues.extend(parsed);
    }

    // Cross-file summary review (only if multiple chunks)
    if chunks.len() > 1 {
        let summary_prompt = build_summary_review_prompt(&file_summaries);
        if let Ok(response) = call_with_retry(provider, &summary_prompt, &system_prompt) {
            if let Ok(parsed) = parse_review_response(&response) {
                all_issues.extend(parsed);
            }
        }
        // Non-critical: if summary review fails, we still have per-file results
    }

    // Determine assessment
    let assessment = determine_assessment(&all_issues);

    // Count by severity
    let critical_count = all_issues.iter().filter(|i| i.severity == Severity::Critical).count();
    let important_count = all_issues.iter().filter(|i| i.severity == Severity::Important).count();
    let minor_count = all_issues.iter().filter(|i| i.severity == Severity::Minor).count();
    let total_issues = all_issues.len();

    // Build reviewed file list
    let files: Vec<ReviewedFile> = diff.files.iter().map(|f| {
        let file_issues: Vec<ReviewIssue> = all_issues
            .iter()
            .filter(|i| i.file == std::path::Path::new(&f.path))
            .cloned()
            .collect();

        ReviewedFile {
            path: PathBuf::from(&f.path),
            status: match f.status {
                crate::review::diff::DiffStatus::Added => FileStatus::Added,
                crate::review::diff::DiffStatus::Modified => FileStatus::Modified,
                crate::review::diff::DiffStatus::Deleted => FileStatus::Deleted,
                crate::review::diff::DiffStatus::Renamed => FileStatus::Renamed {
                    from: PathBuf::from(f.old_path.clone().unwrap_or_default()),
                },
            },
            issues: file_issues,
        }
    }).collect();

    Ok(ReviewResult {
        summary: ReviewSummary {
            files_reviewed: diff.files.len(),
            total_issues,
            critical_count,
            important_count,
            minor_count,
            assessment: assessment.clone(),
            summary_text: String::new(),
        },
        files,
        issues: all_issues,
        base_ref: diff.base.clone(),
        head_ref: diff.head.clone(),
    })
}

fn empty_review_result(diff: &DiffResult) -> ReviewResult {
    ReviewResult {
        summary: ReviewSummary {
            files_reviewed: 0,
            total_issues: 0,
            critical_count: 0,
            important_count: 0,
            minor_count: 0,
            assessment: Assessment::Ready,
            summary_text: String::new(),
        },
        files: vec![],
        issues: vec![],
        base_ref: diff.base.clone(),
        head_ref: diff.head.clone(),
    }
}

fn call_with_retry(
    provider: &dyn AiProviderTrait,
    prompt: &str,
    system: &str,
) -> Result<String, String> {
    let mut delay = INITIAL_RETRY_DELAY_MS;

    for attempt in 0..MAX_RETRIES {
        match provider.complete(prompt, Some(system)) {
            Ok(response) => return Ok(response),
            Err(e) => {
                if attempt + 1 < MAX_RETRIES {
                    eprintln!(
                        "AI call failed (attempt {}/{}): {}. Retrying in {}ms...",
                        attempt + 1,
                        MAX_RETRIES,
                        e,
                        delay
                    );
                    std::thread::sleep(std::time::Duration::from_millis(delay));
                    delay = (delay * 2).min(30000); // exponential backoff, max 30s
                } else {
                    return Err(format!(
                        "AI call failed after {} attempts: {}",
                        MAX_RETRIES, e
                    ));
                }
            }
        }
    }

    Err("Unexpected: retry loop exhausted".to_string())
}

/// Parse the JSON response from AI into a list of ReviewIssue.
fn parse_review_response(response: &str) -> Result<Vec<ReviewIssue>, String> {
    // Try to extract JSON from response (AI might wrap in markdown fences)
    let json_str = extract_json(response);

    let parsed: serde_json::Value = serde_json::from_str(&json_str).map_err(|e| {
        format!(
            "Failed to parse AI response as JSON: {}. Response: {}",
            e,
            &response[..response.len().min(200)]
        )
    })?;

    Ok(parse_issues(&parsed))
}

fn extract_json(text: &str) -> String {
    // Strip markdown code fences if present
    let trimmed = text.trim();
    if trimmed.starts_with("```json") {
        let inner = trimmed.strip_prefix("```json").unwrap_or(trimmed);
        let inner = inner.strip_suffix("```").unwrap_or(inner);
        return inner.trim().to_string();
    }
    if trimmed.starts_with("```") {
        let inner = trimmed.strip_prefix("```").unwrap_or(trimmed);
        let inner = inner.strip_suffix("```").unwrap_or(inner);
        return inner.trim().to_string();
    }
    // Try to find JSON object boundaries
    if let Some(start) = trimmed.find('{') {
        if let Some(end) = trimmed.rfind('}') {
            return trimmed[start..=end].to_string();
        }
    }
    trimmed.to_string()
}

fn parse_issues(value: &serde_json::Value) -> Vec<ReviewIssue> {
    let Some(issues) = value.get("issues").and_then(|v| v.as_array()) else {
        return vec![];
    };

    issues
        .iter()
        .filter_map(|issue| {
            let severity = match issue.get("severity")?.as_str()? {
                "Critical" => Severity::Critical,
                "Important" => Severity::Important,
                _ => Severity::Minor,
            };
            // Map AI response fields to ReviewIssue fields
            // AI returns "title" and "description"; we use "title" as category, "description" as message
            let title = issue.get("title")?.as_str()?.to_string();
            let description = issue.get("description")?.as_str()?.to_string();
            let file_str = issue.get("file")?.as_str()?;

            Some(ReviewIssue {
                severity,
                category: title,
                file: PathBuf::from(file_str),
                line: issue.get("line").and_then(|v| v.as_u64()).map(|v| v as u32),
                message: description,
                suggestion: issue
                    .get("suggestion")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string()),
            })
        })
        .collect()
}

fn determine_assessment(issues: &[ReviewIssue]) -> Assessment {
    let has_critical = issues.iter().any(|i| i.severity == Severity::Critical);
    let has_important = issues.iter().any(|i| i.severity == Severity::Important);

    if has_critical {
        Assessment::CriticalIssues
    } else if has_important {
        Assessment::NeedsWork
    } else {
        Assessment::Ready
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_json_plain() {
        let input = r#"{"issues": []}"#;
        assert_eq!(extract_json(input), r#"{"issues": []}"#);
    }

    #[test]
    fn test_extract_json_from_markdown() {
        let input = "```json\n{\"issues\": []}\n```";
        assert_eq!(extract_json(input), r#"{"issues": []}"#);
    }

    #[test]
    fn test_extract_json_with_surrounding_text() {
        let input = "Here is the review:\n{\"issues\": []}\nEnd.";
        assert_eq!(extract_json(input), r#"{"issues": []}"#);
    }

    #[test]
    fn test_parse_review_response() {
        let response = r#"{
            "issues": [
                {
                    "severity": "Critical",
                    "file": "src/main.rs",
                    "line": 42,
                    "title": "SQL Injection",
                    "description": "Unsafe query",
                    "suggestion": "Use prepared statements"
                }
            ],
            "strengths": ["Good structure"],
            "recommendations": ["Add tests"],
            "assessment": "CriticalIssues"
        }"#;

        let result = parse_review_response(response).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].severity, Severity::Critical);
        assert_eq!(result[0].category, "SQL Injection");
        assert_eq!(result[0].message, "Unsafe query");
        assert_eq!(result[0].file, PathBuf::from("src/main.rs"));
    }

    #[test]
    fn test_determine_assessment_critical() {
        let issues = vec![ReviewIssue {
            severity: Severity::Critical,
            category: "security".to_string(),
            file: PathBuf::from("a.rs"),
            line: None,
            message: "d".to_string(),
            suggestion: None,
        }];
        assert_eq!(determine_assessment(&issues), Assessment::CriticalIssues);
    }

    #[test]
    fn test_determine_assessment_needs_work() {
        let issues = vec![ReviewIssue {
            severity: Severity::Important,
            category: "quality".to_string(),
            file: PathBuf::from("a.rs"),
            line: None,
            message: "d".to_string(),
            suggestion: None,
        }];
        assert_eq!(determine_assessment(&issues), Assessment::NeedsWork);
    }

    #[test]
    fn test_determine_assessment_ready() {
        let issues = vec![ReviewIssue {
            severity: Severity::Minor,
            category: "style".to_string(),
            file: PathBuf::from("a.rs"),
            line: None,
            message: "d".to_string(),
            suggestion: None,
        }];
        assert_eq!(determine_assessment(&issues), Assessment::Ready);

        assert_eq!(determine_assessment(&[]), Assessment::Ready);
    }
}
