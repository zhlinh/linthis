//! Review-specific AI prompt templates.

use crate::review::diff::FileDiff;

/// Build the system prompt for code review.
pub fn review_system_prompt() -> String {
    r#"You are an expert code reviewer. Analyze the provided git diff and produce a structured review.

Review dimensions:
1. Code Quality — separation of concerns, error handling, DRY, edge cases
2. Architecture — design soundness, scalability, performance implications
3. Security — OWASP top 10, injection vulnerabilities, auth issues
4. Naming & Style — convention adherence, readability
5. Error Handling — proper error propagation, no silent failures

Response format (strict JSON):
{
  "issues": [
    {
      "severity": "Critical|Important|Minor",
      "file": "path/to/file.rs",
      "line": 42,
      "title": "Short issue title",
      "description": "Detailed explanation",
      "suggestion": "How to fix (optional, null if none)"
    }
  ],
  "strengths": ["Positive observation 1", "Positive observation 2"],
  "recommendations": ["Suggestion 1", "Suggestion 2"],
  "assessment": "Ready|NeedsWork|CriticalIssues"
}

Rules:
- Only report real issues. Do not invent problems.
- "Critical" = bugs, security vulnerabilities, data loss risks
- "Important" = architectural problems, missing error handling, test gaps
- "Minor" = style, naming, documentation
- Be specific: include file paths and line numbers when possible
- Keep each description concise (1-2 sentences)
- Return ONLY valid JSON, no markdown fences"#.to_string()
}

/// Build the user prompt from a set of file diffs.
pub fn build_review_prompt(files: &[&FileDiff]) -> String {
    let mut prompt = String::from("Review the following code changes:\n\n");

    for file in files {
        if file.is_binary {
            prompt.push_str(&format!("Binary file: {} (skipped)\n\n", file.path));
            continue;
        }

        prompt.push_str(&format!("### File: {} ({:?})\n", file.path, file.status));
        for hunk in &file.hunks {
            prompt.push_str(&format!(
                "```diff (lines {}-{})\n{}\n```\n",
                hunk.new_start,
                hunk.new_start + hunk.new_count,
                hunk.content.trim_end()
            ));
        }
        prompt.push('\n');
    }

    prompt
}

/// Build a cross-file summary review prompt.
pub fn build_summary_review_prompt(file_summaries: &[(String, String)]) -> String {
    let mut prompt = String::from(
        "You previously reviewed individual file diffs. Now review for cross-file concerns:\n\n"
    );
    prompt.push_str("Look for:\n");
    prompt.push_str("- API contract mismatches between files\n");
    prompt.push_str("- Missing updates in related files\n");
    prompt.push_str("- Inconsistent patterns across files\n\n");

    for (path, summary) in file_summaries {
        prompt.push_str(&format!("**{}**: {}\n", path, summary));
    }

    prompt.push_str("\nReturn only NEW cross-file issues not found in per-file reviews. Same JSON format.\n");
    prompt
}

/// Build a prompt for generating PR/MR description from review results.
pub fn build_pr_description_prompt(review_summary: &str) -> String {
    format!(
        "Based on this code review report, generate a concise PR description in Markdown:\n\n{}\n\n\
         Format:\n## Summary\n<2-3 bullet points of what changed>\n\n## Review Notes\n<key findings>\n\n\
         Return ONLY the PR description, no other text.",
        review_summary
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::review::diff::{DiffHunk, DiffStatus, FileDiff};

    #[test]
    fn test_system_prompt_contains_dimensions() {
        let prompt = review_system_prompt();
        assert!(prompt.contains("Code Quality"));
        assert!(prompt.contains("Security"));
        assert!(prompt.contains("JSON"));
    }

    #[test]
    fn test_build_review_prompt_single_file() {
        let file = FileDiff {
            path: "src/main.rs".to_string(),
            old_path: None,
            status: DiffStatus::Modified,
            hunks: vec![DiffHunk {
                old_start: 1,
                old_count: 3,
                new_start: 1,
                new_count: 4,
                content: "+println!(\"hello\");\n".to_string(),
            }],
            additions: 1,
            deletions: 0,
            is_binary: false,
        };
        let prompt = build_review_prompt(&[&file]);
        assert!(prompt.contains("src/main.rs"));
        assert!(prompt.contains("println"));
    }

    #[test]
    fn test_build_review_prompt_skips_binary() {
        let file = FileDiff {
            path: "image.png".to_string(),
            old_path: None,
            status: DiffStatus::Modified,
            hunks: vec![],
            additions: 0,
            deletions: 0,
            is_binary: true,
        };
        let prompt = build_review_prompt(&[&file]);
        assert!(prompt.contains("Binary file"));
        assert!(prompt.contains("skipped"));
    }
}
