// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Auto-generate `.gitignore` when missing in a git repository.
//!
//! Detects the project languages by checking for marker files (Cargo.toml,
//! package.json, etc.) and generates a `.gitignore` with common rules plus
//! language-specific patterns.

use std::path::Path;
use std::process::Command;

/// Common gitignore patterns (always included).
const COMMON_TEMPLATE: &str = "\
# OS
.DS_Store
Thumbs.db
Desktop.ini

# IDE
.idea/
.vscode/
*.swp
*.swo
*~

# Environment
.env
.env.local
.env.*.local

# Linthis
.linthis/
";

/// Check if a .gitignore should be created and do so if needed.
/// Returns a list of staged files that match the new ignore patterns (violations).
/// Returns `None` if no action was needed (not a git repo, or .gitignore exists).
pub fn check_and_create(project_root: &Path, quiet: bool) -> Option<Vec<String>> {
    if !crate::utils::is_git_repo() {
        return None;
    }

    let gitignore_path = project_root.join(".gitignore");
    if gitignore_path.exists() {
        return None;
    }

    // Detect project languages and generate content
    let markers = detect_project_markers(project_root);
    let content = generate_gitignore_content(&markers);

    // Write .gitignore
    if let Err(e) = std::fs::write(&gitignore_path, &content) {
        if !quiet {
            eprintln!("Warning: Failed to create .gitignore: {}", e);
        }
        return None;
    }

    if !quiet {
        eprintln!("\u{26a0} Created .gitignore (was missing)");
    }

    // Check if any staged files match the new ignore patterns
    let violations = check_staged_against_ignores(project_root);
    Some(violations)
}

/// Detect project marker files to determine languages.
fn detect_project_markers(project_root: &Path) -> Vec<&'static str> {
    let checks: &[(&str, &str)] = &[
        ("Cargo.toml", "rust"),
        ("package.json", "node"),
        ("go.mod", "go"),
        ("pyproject.toml", "python"),
        ("setup.py", "python"),
        ("requirements.txt", "python"),
        ("Makefile", "cpp"),
        ("CMakeLists.txt", "cpp"),
        ("pom.xml", "java"),
        ("build.gradle", "java"),
        ("build.gradle.kts", "java"),
        ("Podfile", "swift"),
        ("Package.swift", "swift"),
    ];

    let mut seen = std::collections::HashSet::new();
    let mut markers = Vec::new();

    for (file, lang) in checks {
        if project_root.join(file).exists() && seen.insert(*lang) {
            markers.push(*lang);
        }
    }

    markers
}

/// Generate .gitignore content based on detected languages.
fn generate_gitignore_content(markers: &[&str]) -> String {
    let mut content = String::from(COMMON_TEMPLATE);

    for lang in markers {
        let section = match *lang {
            "rust" => "\n# Rust\ntarget/\n",
            "node" => "\n# Node.js\nnode_modules/\ndist/\nbuild/\n.next/\n",
            "go" => "\n# Go\nvendor/\n",
            "python" => "\n# Python\n__pycache__/\n*.pyc\n*.pyo\n.venv/\nvenv/\n*.egg-info/\n",
            "cpp" => "\n# C/C++\nbuild/\n*.o\n*.so\n*.dylib\n*.a\n",
            "java" => "\n# Java/Kotlin\ntarget/\nbuild/\n*.class\n.gradle/\n",
            "swift" => {
                "\n# Swift/Xcode\n.build/\nPods/\n*.xcworkspace/\nxcuserdata/\nDerivedData/\n"
            }
            _ => "",
        };
        content.push_str(section);
    }

    content
}

/// Check staged files against common ignore patterns.
/// Returns file paths that should probably be ignored.
fn check_staged_against_ignores(project_root: &Path) -> Vec<String> {
    let output = match Command::new("git")
        .args(["diff", "--cached", "--name-only"])
        .current_dir(project_root)
        .output()
    {
        Ok(o) if o.status.success() => o,
        _ => return Vec::new(),
    };

    let staged = String::from_utf8_lossy(&output.stdout);
    let patterns: &[&str] = &[
        "node_modules/",
        "__pycache__/",
        "target/",
        ".venv/",
        "venv/",
        "build/",
        "dist/",
        ".gradle/",
        ".idea/",
        ".vscode/",
        ".linthis/",
        ".env",
        ".DS_Store",
        "Thumbs.db",
        "Pods/",
        "DerivedData/",
    ];

    let ext_patterns: &[&str] = &[".pyc", ".pyo", ".class", ".o", ".so", ".dylib", ".a"];

    staged
        .lines()
        .filter(|line| {
            let line = line.trim();
            if line.is_empty() {
                return false;
            }
            // Check directory prefixes
            for pat in patterns {
                if line.starts_with(pat) || line.contains(&format!("/{}", pat)) {
                    return true;
                }
                // Exact file match (e.g. ".env", ".DS_Store")
                if !pat.ends_with('/') && line == *pat {
                    return true;
                }
            }
            // Check file extension patterns
            for ext in ext_patterns {
                if line.ends_with(ext) {
                    return true;
                }
            }
            false
        })
        .map(|s| s.to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_gitignore_no_markers() {
        let content = generate_gitignore_content(&[]);
        assert!(content.contains(".DS_Store"));
        assert!(content.contains(".linthis/"));
        assert!(!content.contains("node_modules"));
    }

    #[test]
    fn test_generate_gitignore_with_node() {
        let content = generate_gitignore_content(&["node"]);
        assert!(content.contains("node_modules/"));
        assert!(content.contains(".DS_Store"));
    }

    #[test]
    fn test_generate_gitignore_multiple_languages() {
        let content = generate_gitignore_content(&["rust", "python"]);
        assert!(content.contains("target/"));
        assert!(content.contains("__pycache__/"));
    }

    #[test]
    fn test_detect_markers_empty() {
        let markers = detect_project_markers(Path::new("/nonexistent"));
        assert!(markers.is_empty());
    }
}
