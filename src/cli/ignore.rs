use std::process::ExitCode;

use crate::cli::commands::IgnoreCommands;

pub fn handle_ignore_command(action: IgnoreCommands) -> ExitCode {
    match action {
        IgnoreCommands::Add { pattern } => append_pattern(&pattern),
        IgnoreCommands::Remove { pattern } => remove_pattern(&pattern),
        IgnoreCommands::List => list_patterns(),
    }
}

fn project_root() -> std::path::PathBuf {
    linthis::utils::get_project_root()
}

fn append_pattern(entry: &str) -> ExitCode {
    match linthis::utils::linthisignore::LinthisIgnore::append(&project_root(), entry) {
        Ok(true) => {
            eprintln!("Already in .linthisignore: {entry}");
            ExitCode::SUCCESS
        }
        Ok(false) => {
            println!("Added to .linthisignore: {entry}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("Error writing .linthisignore: {e}");
            ExitCode::FAILURE
        }
    }
}

fn remove_pattern(entry: &str) -> ExitCode {
    let path = project_root().join(".linthisignore");

    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            eprintln!(".linthisignore does not exist");
            return ExitCode::FAILURE;
        }
        Err(e) => {
            eprintln!("Error reading .linthisignore: {e}");
            return ExitCode::FAILURE;
        }
    };

    if !content.lines().any(|l| l.trim() == entry) {
        eprintln!("Not found in .linthisignore: {entry}");
        return ExitCode::FAILURE;
    }

    let new_content: String = content
        .lines()
        .filter(|l| l.trim() != entry)
        .flat_map(|l| [l, "\n"])
        .collect();

    if let Err(e) = std::fs::write(&path, &new_content) {
        eprintln!("Error writing .linthisignore: {e}");
        return ExitCode::FAILURE;
    }

    println!("Removed from .linthisignore: {entry}");
    ExitCode::SUCCESS
}

fn list_patterns() -> ExitCode {
    let path = project_root().join(".linthisignore");
    match std::fs::read_to_string(&path) {
        Ok(content) => {
            let mut paths: Vec<&str> = Vec::new();
            let mut rules: Vec<&str> = Vec::new();
            for line in content.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                if let Some(code) = line.strip_prefix("rule:") {
                    rules.push(code.trim());
                } else {
                    paths.push(line);
                }
            }
            if paths.is_empty() && rules.is_empty() {
                println!(".linthisignore is empty");
            } else {
                if !paths.is_empty() {
                    println!("Path patterns:");
                    for p in paths {
                        println!("  {p}");
                    }
                }
                if !rules.is_empty() {
                    println!("Disabled rules:");
                    for r in rules {
                        println!("  {r}");
                    }
                }
            }
            ExitCode::SUCCESS
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            println!(".linthisignore does not exist");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("Error reading .linthisignore: {e}");
            ExitCode::FAILURE
        }
    }
}
