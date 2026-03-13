//! Git diff collection and parsing.

use std::process::Command;

use serde::{Deserialize, Serialize};

/// A parsed diff hunk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffHunk {
    pub old_start: u32,
    pub old_count: u32,
    pub new_start: u32,
    pub new_count: u32,
    pub content: String,
}

/// A parsed file diff
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDiff {
    pub path: String,
    pub old_path: Option<String>,
    pub status: DiffStatus,
    pub hunks: Vec<DiffHunk>,
    pub additions: usize,
    pub deletions: usize,
    pub is_binary: bool,
}

/// File diff status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiffStatus {
    Added,
    Modified,
    Deleted,
    Renamed,
}

/// Complete diff between two refs
#[derive(Debug, Clone)]
pub struct DiffResult {
    pub files: Vec<FileDiff>,
    pub base: String,
    pub head: String,
    pub total_additions: usize,
    pub total_deletions: usize,
}

/// Detect the base ref for diff comparison.
/// Priority: explicit base > remote tracking branch.
pub fn detect_base_ref(explicit_base: Option<&str>) -> Result<String, String> {
    if let Some(base) = explicit_base {
        return Ok(base.to_string());
    }

    // Try to get remote tracking branch
    let output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{upstream}"])
        .output()
        .map_err(|e| format!("Failed to run git: {}", e))?;

    if output.status.success() {
        let tracking = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !tracking.is_empty() {
            return Ok(tracking);
        }
    }

    // Fallback: try origin/main or origin/master
    for branch in &["origin/main", "origin/master"] {
        let check = Command::new("git")
            .args(["rev-parse", "--verify", branch])
            .output();
        if let Ok(out) = check {
            if out.status.success() {
                return Ok(branch.to_string());
            }
        }
    }

    Err("Could not detect base ref. Use --base to specify.".to_string())
}

/// Collect git diff between base and head refs.
pub fn collect_diff(base: &str, head: &str) -> Result<DiffResult, String> {
    let output = Command::new("git")
        .args(["diff", &format!("{}..{}", base, head)])
        .output()
        .map_err(|e| format!("Failed to run git diff: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("git diff failed: {}", stderr));
    }

    let raw_diff = String::from_utf8_lossy(&output.stdout).to_string();
    let files = parse_diff(&raw_diff);

    let total_additions = files.iter().map(|f| f.additions).sum();
    let total_deletions = files.iter().map(|f| f.deletions).sum();

    Ok(DiffResult {
        files,
        base: base.to_string(),
        head: head.to_string(),
        total_additions,
        total_deletions,
    })
}

/// Parse unified diff output into structured FileDiff entries.
pub fn parse_diff(raw: &str) -> Vec<FileDiff> {
    let mut files = Vec::new();
    let mut current_file: Option<FileDiff> = None;
    let mut current_hunk: Option<DiffHunk> = None;
    let mut in_binary = false;

    for line in raw.lines() {
        if line.starts_with("diff --git ") {
            // Save previous hunk and file
            if let Some(hunk) = current_hunk.take() {
                if let Some(ref mut file) = current_file {
                    file.hunks.push(hunk);
                }
            }
            if let Some(file) = current_file.take() {
                files.push(file);
            }
            in_binary = false;

            // Parse file paths from "diff --git a/path b/path"
            let parts: Vec<&str> = line.splitn(4, ' ').collect();
            let path = if parts.len() >= 4 {
                parts[3].strip_prefix("b/").unwrap_or(parts[3]).to_string()
            } else {
                "unknown".to_string()
            };

            current_file = Some(FileDiff {
                path,
                old_path: None,
                status: DiffStatus::Modified,
                hunks: Vec::new(),
                additions: 0,
                deletions: 0,
                is_binary: false,
            });
        } else if line.starts_with("new file mode") {
            if let Some(ref mut file) = current_file {
                file.status = DiffStatus::Added;
            }
        } else if line.starts_with("deleted file mode") {
            if let Some(ref mut file) = current_file {
                file.status = DiffStatus::Deleted;
            }
        } else if line.starts_with("rename from ") {
            if let Some(ref mut file) = current_file {
                file.status = DiffStatus::Renamed;
                file.old_path = Some(line.strip_prefix("rename from ").unwrap().to_string());
            }
        } else if line.starts_with("Binary files") {
            in_binary = true;
            if let Some(ref mut file) = current_file {
                file.is_binary = true;
            }
        } else if line.starts_with("@@ ") && !in_binary {
            // Save previous hunk
            if let Some(hunk) = current_hunk.take() {
                if let Some(ref mut file) = current_file {
                    file.hunks.push(hunk);
                }
            }

            // Parse hunk header: @@ -old_start,old_count +new_start,new_count @@
            let hunk = parse_hunk_header(line);
            current_hunk = Some(hunk);
        } else if let Some(ref mut hunk) = current_hunk {
            hunk.content.push_str(line);
            hunk.content.push('\n');

            if let Some(ref mut file) = current_file {
                if line.starts_with('+') && !line.starts_with("+++") {
                    file.additions += 1;
                } else if line.starts_with('-') && !line.starts_with("---") {
                    file.deletions += 1;
                }
            }
        }
    }

    // Save last hunk and file
    if let Some(hunk) = current_hunk {
        if let Some(ref mut file) = current_file {
            file.hunks.push(hunk);
        }
    }
    if let Some(file) = current_file {
        files.push(file);
    }

    files
}

fn parse_hunk_header(line: &str) -> DiffHunk {
    // @@ -1,5 +1,7 @@ optional context
    let mut old_start = 0u32;
    let mut old_count = 0u32;
    let mut new_start = 0u32;
    let mut new_count = 0u32;

    if let Some(rest) = line.strip_prefix("@@ -") {
        if let Some(at_pos) = rest.find(" @@") {
            let range_part = &rest[..at_pos];
            let parts: Vec<&str> = range_part.split(' ').collect();

            // Parse old range
            if let Some(old_range) = parts.first() {
                let old_parts: Vec<&str> = old_range.split(',').collect();
                old_start = old_parts[0].parse().unwrap_or(0);
                old_count = old_parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(1);
            }

            // Parse new range (strip leading +)
            if let Some(new_range) = parts.get(1) {
                let new_range = new_range.strip_prefix('+').unwrap_or(new_range);
                let new_parts: Vec<&str> = new_range.split(',').collect();
                new_start = new_parts[0].parse().unwrap_or(0);
                new_count = new_parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(1);
            }
        }
    }

    DiffHunk {
        old_start,
        old_count,
        new_start,
        new_count,
        content: String::new(),
    }
}

/// Estimate token count for a string (4 chars ≈ 1 token).
pub fn estimate_tokens(text: &str) -> usize {
    text.len() / 4
}

/// Chunk a diff by file. If a single file exceeds max_tokens, split by hunks.
pub fn chunk_diff(files: &[FileDiff], max_tokens: usize) -> Vec<Vec<&FileDiff>> {
    let mut chunks: Vec<Vec<&FileDiff>> = Vec::new();
    let mut current_chunk: Vec<&FileDiff> = Vec::new();
    let mut current_tokens = 0;

    for file in files {
        if file.is_binary {
            continue;
        }
        let file_tokens: usize = file.hunks.iter().map(|h| estimate_tokens(&h.content)).sum();

        if current_tokens + file_tokens > max_tokens && !current_chunk.is_empty() {
            chunks.push(current_chunk);
            current_chunk = Vec::new();
            current_tokens = 0;
        }

        current_chunk.push(file);
        current_tokens += file_tokens;
    }

    if !current_chunk.is_empty() {
        chunks.push(current_chunk);
    }

    if chunks.is_empty() {
        chunks.push(Vec::new());
    }

    chunks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty_diff() {
        let files = parse_diff("");
        assert!(files.is_empty());
    }

    #[test]
    fn test_parse_single_file_diff() {
        let raw = r#"diff --git a/src/main.rs b/src/main.rs
index abc1234..def5678 100644
--- a/src/main.rs
+++ b/src/main.rs
@@ -1,3 +1,4 @@
 fn main() {
+    println!("hello");
     println!("world");
 }
"#;
        let files = parse_diff(raw);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "src/main.rs");
        assert_eq!(files[0].status, DiffStatus::Modified);
        assert_eq!(files[0].additions, 1);
        assert_eq!(files[0].deletions, 0);
        assert_eq!(files[0].hunks.len(), 1);
    }

    #[test]
    fn test_parse_new_file() {
        let raw = r#"diff --git a/new.rs b/new.rs
new file mode 100644
index 0000000..abc1234
--- /dev/null
+++ b/new.rs
@@ -0,0 +1,3 @@
+fn hello() {
+    println!("hi");
+}
"#;
        let files = parse_diff(raw);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].status, DiffStatus::Added);
        assert_eq!(files[0].additions, 3);
    }

    #[test]
    fn test_parse_deleted_file() {
        let raw = r#"diff --git a/old.rs b/old.rs
deleted file mode 100644
index abc1234..0000000
--- a/old.rs
+++ /dev/null
@@ -1,2 +0,0 @@
-fn old() {}
-fn unused() {}
"#;
        let files = parse_diff(raw);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].status, DiffStatus::Deleted);
        assert_eq!(files[0].deletions, 2);
    }

    #[test]
    fn test_parse_renamed_file() {
        let raw = r#"diff --git a/old_name.rs b/new_name.rs
similarity index 95%
rename from old_name.rs
rename to new_name.rs
index abc1234..def5678 100644
--- a/old_name.rs
+++ b/new_name.rs
@@ -1,3 +1,3 @@
-fn old() {}
+fn renamed() {}
"#;
        let files = parse_diff(raw);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].status, DiffStatus::Renamed);
        assert_eq!(files[0].old_path, Some("old_name.rs".to_string()));
    }

    #[test]
    fn test_parse_binary_file() {
        let raw = r#"diff --git a/image.png b/image.png
Binary files a/image.png and b/image.png differ
"#;
        let files = parse_diff(raw);
        assert_eq!(files.len(), 1);
        assert!(files[0].is_binary);
    }

    #[test]
    fn test_parse_multi_file_diff() {
        let raw = r#"diff --git a/a.rs b/a.rs
index abc..def 100644
--- a/a.rs
+++ b/a.rs
@@ -1,1 +1,2 @@
 fn a() {}
+fn a2() {}
diff --git a/b.rs b/b.rs
index abc..def 100644
--- a/b.rs
+++ b/b.rs
@@ -1,1 +1,2 @@
 fn b() {}
+fn b2() {}
"#;
        let files = parse_diff(raw);
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].path, "a.rs");
        assert_eq!(files[1].path, "b.rs");
    }

    #[test]
    fn test_estimate_tokens() {
        assert_eq!(estimate_tokens(""), 0);
        assert_eq!(estimate_tokens("abcd"), 1);
        assert_eq!(estimate_tokens("12345678"), 2);
    }

    #[test]
    fn test_chunk_diff_single_chunk() {
        let files = vec![FileDiff {
            path: "a.rs".to_string(),
            old_path: None,
            status: DiffStatus::Modified,
            hunks: vec![DiffHunk {
                old_start: 1, old_count: 1, new_start: 1, new_count: 2,
                content: "small".to_string(),
            }],
            additions: 1, deletions: 0, is_binary: false,
        }];
        let chunks = chunk_diff(&files, 10000);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].len(), 1);
    }

    #[test]
    fn test_hunk_header_parsing() {
        let hunk = parse_hunk_header("@@ -1,5 +1,7 @@ fn main()");
        assert_eq!(hunk.old_start, 1);
        assert_eq!(hunk.old_count, 5);
        assert_eq!(hunk.new_start, 1);
        assert_eq!(hunk.new_count, 7);
    }
}
