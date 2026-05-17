use std::fs::OpenOptions;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

pub struct LinthisIgnore {
    pub path_patterns: Vec<String>,
    pub rule_codes: Vec<String>,
}

impl LinthisIgnore {
    #[must_use]
    pub fn load(project_root: &Path) -> Self {
        let path = project_root.join(".linthisignore");
        match std::fs::read_to_string(&path) {
            Ok(content) => Self::parse(&content),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Self {
                path_patterns: Vec::new(),
                rule_codes: Vec::new(),
            },
            Err(e) => {
                eprintln!("Warning: failed to read .linthisignore: {e}");
                Self {
                    path_patterns: Vec::new(),
                    rule_codes: Vec::new(),
                }
            }
        }
    }

    /// Append `entry` to `<project_root>/.linthisignore`.
    ///
    /// Returns `Ok(true)` when the entry was already present (no-op),
    /// `Ok(false)` when it was newly written, or `Err` on I/O failure.
    pub fn append(project_root: &Path, entry: &str) -> std::io::Result<bool> {
        let path = project_root.join(".linthisignore");

        let mut file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(&path)?;

        let mut content = String::new();
        file.read_to_string(&mut content)?;

        if content.lines().any(|l| l.trim() == entry) {
            return Ok(true);
        }

        file.seek(SeekFrom::End(0))?;
        writeln!(file, "{entry}")?;
        Ok(false)
    }

    fn parse(content: &str) -> Self {
        let mut path_patterns = Vec::new();
        let mut rule_codes = Vec::new();

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some(code) = line.strip_prefix("rule:") {
                let code = code.trim();
                if !code.is_empty() {
                    rule_codes.push(code.to_string());
                }
            } else {
                path_patterns.push(line.to_string());
            }
        }

        Self {
            path_patterns,
            rule_codes,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_empty() {
        let r = LinthisIgnore::parse("");
        assert!(r.path_patterns.is_empty());
        assert!(r.rule_codes.is_empty());
    }

    #[test]
    fn parse_comments_and_blanks_ignored() {
        let r = LinthisIgnore::parse("# comment\n\n  # another\n");
        assert!(r.path_patterns.is_empty());
        assert!(r.rule_codes.is_empty());
    }

    #[test]
    fn parse_path_patterns() {
        let r = LinthisIgnore::parse("vendor/**\n*.generated.go\nbuild/\n");
        assert_eq!(
            r.path_patterns,
            vec!["vendor/**", "*.generated.go", "build/"]
        );
        assert!(r.rule_codes.is_empty());
    }

    #[test]
    fn parse_rule_codes() {
        let r =
            LinthisIgnore::parse("rule:E501\nrule:clippy::too_many_arguments\nrule:complexity/*\n");
        assert!(r.path_patterns.is_empty());
        assert_eq!(
            r.rule_codes,
            vec!["E501", "clippy::too_many_arguments", "complexity/*"]
        );
    }

    #[test]
    fn parse_mixed() {
        let content = "# Ignore generated files\nvendor/**\n\nrule:E501\n*.pb.go\nrule:clippy::*\n";
        let r = LinthisIgnore::parse(content);
        assert_eq!(r.path_patterns, vec!["vendor/**", "*.pb.go"]);
        assert_eq!(r.rule_codes, vec!["E501", "clippy::*"]);
    }

    #[test]
    fn parse_rule_with_whitespace() {
        let r = LinthisIgnore::parse("rule:  E501  \n");
        assert_eq!(r.rule_codes, vec!["E501"]);
    }

    #[test]
    fn load_missing_file_returns_empty() {
        let tmp = std::env::temp_dir().join("linthis_no_such_dir_xyz");
        let r = LinthisIgnore::load(&tmp);
        assert!(r.path_patterns.is_empty());
        assert!(r.rule_codes.is_empty());
    }
}
