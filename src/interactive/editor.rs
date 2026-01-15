// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Cross-platform editor integration for opening files at specific lines.

use std::fs;
use std::path::Path;
use std::process::Command;
use similar::{ChangeTag, TextDiff};

/// Result of opening a file in an editor
#[derive(Debug)]
pub struct EditorResult {
    /// Whether the editor operation succeeded
    pub success: bool,
    /// Lines that were changed (if any)
    pub changes: Vec<LineChange>,
    /// Error message if operation failed
    pub error: Option<String>,
}

/// Information about a changed line
#[derive(Debug, Clone)]
pub struct LineChange {
    pub line_number: usize,
    pub old_content: String,
    pub new_content: String,
}

/// Open a file in the user's preferred editor at a specific line and detect changes.
///
/// # Platform Support
/// - Unix: Uses $EDITOR environment variable, defaults to vim
/// - Windows: Uses $EDITOR if set, otherwise tries code, notepad++, then notepad
///
/// # Editor-specific line number arguments
/// - vim/nvim/vi: +{line}
/// - code (VS Code): --goto {file}:{line}:{column}
/// - emacs: +{line} {file}
/// - nano: +{line} {file}
/// - sublime/subl: {file}:{line}
/// - notepad++: -n{line} {file}
/// - atom: {file}:{line}
///
/// # Arguments
/// * `file` - Path to the file to open
/// * `line` - Line number (1-indexed)
/// * `column` - Optional column number (1-indexed)
///
/// # Returns
/// * `EditorResult` with change information and success status
pub fn open_in_editor(file: &Path, line: usize, column: Option<usize>) -> EditorResult {
    // Read file content before editing
    let original_content = match fs::read_to_string(file) {
        Ok(content) => content,
        Err(e) => {
            return EditorResult {
                success: false,
                changes: vec![],
                error: Some(format!("Failed to read file: {}", e)),
            }
        }
    };
    let editor = get_editor();
    let editor_lower = editor.to_lowercase();

    // Determine editor type and build command
    let mut cmd = Command::new(&editor);

    // Get the base name of the editor for matching
    let editor_name = Path::new(&editor_lower)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(&editor_lower);

    match editor_name {
        // VS Code family
        "code" | "code-insiders" | "codium" => {
            let col = column.unwrap_or(1);
            cmd.arg("--goto")
                .arg(format!("{}:{}:{}", file.display(), line, col));
        }
        // Vim family
        "vim" | "nvim" | "vi" | "gvim" | "mvim" => {
            cmd.arg(format!("+{}", line)).arg(file);
        }
        // Emacs
        "emacs" | "emacsclient" => {
            cmd.arg(format!("+{}", line)).arg(file);
        }
        // Nano
        "nano" => {
            cmd.arg(format!("+{}", line)).arg(file);
        }
        // Sublime Text
        "sublime" | "subl" | "sublime_text" => {
            let col = column.unwrap_or(1);
            cmd.arg(format!("{}:{}:{}", file.display(), line, col));
        }
        // Notepad++
        "notepad++" => {
            cmd.arg(format!("-n{}", line)).arg(file);
        }
        // Atom (deprecated but still used)
        "atom" => {
            let col = column.unwrap_or(1);
            cmd.arg(format!("{}:{}:{}", file.display(), line, col));
        }
        // Helix
        "hx" | "helix" => {
            let col = column.unwrap_or(1);
            cmd.arg(format!("{}:{}:{}", file.display(), line, col));
        }
        // Kakoune
        "kak" => {
            cmd.arg(format!("+{}", line)).arg(file);
        }
        // JetBrains IDEs (idea, goland, pycharm, etc.) via command line
        name if name.contains("idea") || name.contains("goland") || name.contains("pycharm") => {
            let col = column.unwrap_or(1);
            cmd.arg("--line")
                .arg(line.to_string())
                .arg("--column")
                .arg(col.to_string())
                .arg(file);
        }
        // Default: try vim-style +line argument
        _ => {
            cmd.arg(format!("+{}", line)).arg(file);
        }
    }

    // Spawn the editor
    let spawn_result = cmd.spawn();

    match spawn_result {
        Ok(mut child) => {
            // Wait for the editor to close
            match child.wait() {
                Ok(status) => {
                    if !status.success() {
                        return EditorResult {
                            success: false,
                            changes: vec![],
                            error: Some(format!(
                                "Editor '{}' exited with status: {}",
                                editor,
                                status.code().unwrap_or(-1)
                            )),
                        };
                    }

                    // Read file content after editing
                    let new_content = match fs::read_to_string(file) {
                        Ok(content) => content,
                        Err(e) => {
                            return EditorResult {
                                success: false,
                                changes: vec![],
                                error: Some(format!("Failed to read file after editing: {}", e)),
                            }
                        }
                    };

                    // Detect changes
                    let changes = detect_changes(&original_content, &new_content);

                    EditorResult {
                        success: true,
                        changes,
                        error: None,
                    }
                }
                Err(e) => EditorResult {
                    success: false,
                    changes: vec![],
                    error: Some(format!("Failed to wait for editor '{}': {}", editor, e)),
                },
            }
        }
        Err(e) => EditorResult {
            success: false,
            changes: vec![],
            error: Some(format!("Failed to launch editor '{}': {}", editor, e)),
        },
    }
}

/// Detect changes between two versions of file content using proper diff algorithm
fn detect_changes(original: &str, new: &str) -> Vec<LineChange> {
    let mut changes = Vec::new();

    // Use the similar crate's TextDiff to compute proper line-based diff
    let diff = TextDiff::from_lines(original, new);

    // Track current line number in new version
    let mut new_line_num = 0;

    // Process each change operation
    for change in diff.iter_all_changes() {
        match change.tag() {
            ChangeTag::Equal => {
                // Line unchanged, just increment counter
                new_line_num += 1;
            }
            ChangeTag::Delete => {
                // Line deleted from old version
                changes.push(LineChange {
                    line_number: new_line_num + 1, // Position in new file where deletion occurred
                    old_content: change.to_string().trim_end().to_string(),
                    new_content: String::new(), // Deleted, so new content is empty
                });
            }
            ChangeTag::Insert => {
                // Line inserted in new version
                new_line_num += 1;
                changes.push(LineChange {
                    line_number: new_line_num,
                    old_content: String::new(), // Inserted, so old content is empty
                    new_content: change.to_string().trim_end().to_string(),
                });
            }
        }
    }

    changes
}

/// Get the user's preferred editor from environment variables.
///
/// Checks in order:
/// 1. $EDITOR
/// 2. $VISUAL
/// 3. Platform-specific defaults
fn get_editor() -> String {
    // Check EDITOR first
    if let Ok(editor) = std::env::var("EDITOR") {
        if !editor.is_empty() {
            return editor;
        }
    }

    // Check VISUAL
    if let Ok(visual) = std::env::var("VISUAL") {
        if !visual.is_empty() {
            return visual;
        }
    }

    // Platform-specific defaults
    #[cfg(windows)]
    {
        // On Windows, try to find a reasonable editor
        // Check if common editors are available in PATH
        for editor in &["code", "notepad++", "notepad"] {
            if which_exists(editor) {
                return editor.to_string();
            }
        }
        "notepad".to_string()
    }

    #[cfg(not(windows))]
    {
        // On Unix, default to vim
        "vim".to_string()
    }
}

/// Check if a command exists in PATH (Windows-compatible)
#[cfg(windows)]
fn which_exists(cmd: &str) -> bool {
    use std::process::Stdio;
    Command::new("where")
        .arg(cmd)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_editor_default() {
        // This test depends on environment, just ensure it returns something
        let editor = get_editor();
        assert!(!editor.is_empty());
    }
}
