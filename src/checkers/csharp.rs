// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! C# language checker using dotnet format.

use crate::checkers::Checker;
use crate::utils::types::{LintIssue, Severity};
use crate::{Language, Result};
use regex::Regex;
use std::path::Path;
use std::process::Command;

/// C# checker using dotnet format with --verify-no-changes.
pub struct CSharpChecker;

impl CSharpChecker {
    pub fn new() -> Self {
        Self
    }

    /// Find the project/solution file for the given source file.
    fn find_project_file(&self, path: &Path) -> Option<std::path::PathBuf> {
        let mut current = path.parent()?;

        loop {
            // Check for .csproj files
            if let Ok(entries) = std::fs::read_dir(current) {
                for entry in entries.filter_map(|e| e.ok()) {
                    let entry_path = entry.path();
                    if let Some(ext) = entry_path.extension() {
                        if ext == "csproj" || ext == "sln" {
                            return Some(entry_path);
                        }
                    }
                }
            }

            // Move up to parent directory
            current = current.parent()?;
        }
    }

    /// Parse dotnet format output.
    /// Format: FILE(LINE,COL): SEVERITY CODE: MESSAGE
    fn parse_dotnet_output(&self, output: &str, path: &Path) -> Vec<LintIssue> {
        let mut issues = Vec::new();

        // Regex to match dotnet format diagnostic output
        // Example: Program.cs(10,5): warning IDE0005: Using directive is unnecessary
        let re = Regex::new(r"^(.+?)\((\d+),(\d+)\):\s*(error|warning|info)?\s*([A-Z]+\d+):\s*(.+)$")
            .unwrap();

        // Alternative format without position
        let re_simple = Regex::new(r"^(.+?):\s*(error|warning|info)?\s*([A-Z]+\d+):\s*(.+)$").unwrap();

        for line in output.lines() {
            if let Some(caps) = re.captures(line) {
                let file_path_str = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                let line_num: usize = caps
                    .get(2)
                    .and_then(|m| m.as_str().parse().ok())
                    .unwrap_or(1);
                let col: usize = caps
                    .get(3)
                    .and_then(|m| m.as_str().parse().ok())
                    .unwrap_or(1);
                let severity_str = caps.get(4).map(|m| m.as_str()).unwrap_or("warning");
                let code = caps.get(5).map(|m| m.as_str()).unwrap_or("");
                let message = caps.get(6).map(|m| m.as_str()).unwrap_or("");

                // Only include issues for the target file
                let issue_path = Path::new(file_path_str);
                if issue_path.file_name() != path.file_name() {
                    continue;
                }

                let severity = match severity_str {
                    "error" => Severity::Error,
                    "warning" => Severity::Warning,
                    _ => Severity::Info,
                };

                let issue = LintIssue::new(
                    path.to_path_buf(),
                    line_num,
                    message.to_string(),
                    severity,
                )
                .with_source("dotnet-format".to_string())
                .with_code(code.to_string())
                .with_column(col);

                issues.push(issue);
            } else if let Some(caps) = re_simple.captures(line) {
                let file_path_str = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                let severity_str = caps.get(2).map(|m| m.as_str()).unwrap_or("warning");
                let code = caps.get(3).map(|m| m.as_str()).unwrap_or("");
                let message = caps.get(4).map(|m| m.as_str()).unwrap_or("");

                // Only include issues for the target file
                let issue_path = Path::new(file_path_str);
                if issue_path.file_name() != path.file_name() {
                    continue;
                }

                let severity = match severity_str {
                    "error" => Severity::Error,
                    "warning" => Severity::Warning,
                    _ => Severity::Info,
                };

                let issue = LintIssue::new(path.to_path_buf(), 1, message.to_string(), severity)
                    .with_source("dotnet-format".to_string())
                    .with_code(code.to_string());

                issues.push(issue);
            }
        }

        issues
    }
}

impl Default for CSharpChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl Checker for CSharpChecker {
    fn name(&self) -> &str {
        "dotnet-format"
    }

    fn supported_languages(&self) -> &[Language] {
        &[Language::CSharp]
    }

    fn check(&self, path: &Path) -> Result<Vec<LintIssue>> {
        // Find project file for context
        let project_file = self.find_project_file(path);

        let output = if let Some(ref proj) = project_file {
            // Run dotnet format on the project with verify-no-changes
            Command::new("dotnet")
                .args([
                    "format",
                    "--verify-no-changes",
                    "--verbosity",
                    "diagnostic",
                ])
                .arg(proj)
                .output()
                .map_err(|e| {
                    crate::LintisError::checker(
                        "dotnet-format",
                        path,
                        format!("Failed to run: {}", e),
                    )
                })?
        } else {
            // Try to run on single file (may fail without project context)
            Command::new("dotnet")
                .args([
                    "format",
                    "--verify-no-changes",
                    "--verbosity",
                    "diagnostic",
                    "--include",
                ])
                .arg(path)
                .output()
                .map_err(|e| {
                    crate::LintisError::checker(
                        "dotnet-format",
                        path,
                        format!("Failed to run: {}", e),
                    )
                })?
        };

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        // Combine stdout and stderr for parsing
        let combined = format!("{}\n{}", stdout, stderr);
        let issues = self.parse_dotnet_output(&combined, path);

        Ok(issues)
    }

    fn is_available(&self) -> bool {
        Command::new("dotnet")
            .args(["format", "--version"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_dotnet_output() {
        let checker = CSharpChecker::new();
        let output = r#"Program.cs(10,5): warning IDE0005: Using directive is unnecessary
Program.cs(15,1): error CS0103: The name 'foo' does not exist"#;

        let issues = checker.parse_dotnet_output(output, Path::new("Program.cs"));

        assert_eq!(issues.len(), 2);

        let issue1 = &issues[0];
        assert_eq!(issue1.severity, Severity::Warning);
        assert_eq!(issue1.line, 10);
        assert_eq!(issue1.column, Some(5));
        assert_eq!(issue1.code, Some("IDE0005".to_string()));

        let issue2 = &issues[1];
        assert_eq!(issue2.severity, Severity::Error);
        assert_eq!(issue2.line, 15);
    }

    #[test]
    fn test_parse_empty_output() {
        let checker = CSharpChecker::new();
        let issues = checker.parse_dotnet_output("", Path::new("Program.cs"));
        assert!(issues.is_empty());
    }

    #[test]
    fn test_parse_filters_other_files() {
        let checker = CSharpChecker::new();
        let output = r#"Other.cs(10,5): warning IDE0005: Using directive is unnecessary
Program.cs(15,1): warning IDE0001: Simplify name"#;

        let issues = checker.parse_dotnet_output(output, Path::new("Program.cs"));
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].line, 15);
    }
}
