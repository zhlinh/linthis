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

use std::path::Path;
use std::process::Command;

/// Open a file in the user's preferred editor at a specific line.
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
/// * `Ok(())` if the editor was spawned successfully
/// * `Err(String)` with error message if spawning failed
pub fn open_in_editor(file: &Path, line: usize, column: Option<usize>) -> Result<(), String> {
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
    match cmd.spawn() {
        Ok(mut child) => {
            // Wait for the editor to close
            match child.wait() {
                Ok(status) => {
                    if status.success() {
                        Ok(())
                    } else {
                        Err(format!(
                            "Editor '{}' exited with status: {}",
                            editor,
                            status.code().unwrap_or(-1)
                        ))
                    }
                }
                Err(e) => Err(format!("Failed to wait for editor '{}': {}", editor, e)),
            }
        }
        Err(e) => Err(format!("Failed to launch editor '{}': {}", editor, e)),
    }
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
