//! Configuration CLI handlers for linthis config command

use colored::Colorize;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use toml_edit::{value, Array, DocumentMut};

use super::Config;

/// Get config file path based on global flag
fn get_config_path(global: bool) -> crate::Result<PathBuf> {
    if global {
        let home = dirs::home_dir().ok_or_else(|| {
            crate::LintisError::Config("Cannot find home directory".to_string())
        })?;
        let config_dir = home.join(".linthis");
        fs::create_dir_all(&config_dir).map_err(|e| {
            crate::LintisError::Config(format!("Failed to create config directory: {}", e))
        })?;
        Ok(config_dir.join("config.toml"))
    } else {
        Ok(PathBuf::from(".linthis.toml"))
    }
}

/// Ensure config file exists, create if not
fn ensure_config_file(config_path: &Path) -> crate::Result<()> {
    if !config_path.exists() {
        let default_content = Config::generate_default_toml();
        fs::write(config_path, default_content).map_err(|e| {
            crate::LintisError::Config(format!("Failed to create config file: {}", e))
        })?;
    }
    Ok(())
}

/// Load TOML document from config file
fn load_toml_doc(config_path: &Path) -> crate::Result<DocumentMut> {
    ensure_config_file(config_path)?;
    let content = fs::read_to_string(config_path).map_err(|e| {
        crate::LintisError::Config(format!("Failed to read config file: {}", e))
    })?;
    content.parse::<DocumentMut>().map_err(|e| {
        crate::LintisError::Config(format!("Failed to parse config file: {}", e))
    })
}

/// Save TOML document to config file
fn save_toml_doc(config_path: &Path, doc: &DocumentMut) -> crate::Result<()> {
    fs::write(config_path, doc.to_string()).map_err(|e| {
        crate::LintisError::Config(format!("Failed to write config file: {}", e))
    })
}

/// Add value to an array field
pub fn handle_config_add(field: &str, value: &str, global: bool) -> ExitCode {
    let config_path = match get_config_path(global) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{}: {}", "Error".red(), e);
            return ExitCode::from(1);
        }
    };

    let mut doc = match load_toml_doc(&config_path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("{}: {}", "Error".red(), e);
            return ExitCode::from(1);
        }
    };

    // Get or create array
    if !doc.contains_key(field) {
        doc[field] = toml_edit::Item::Value(toml_edit::Value::Array(Array::new()));
    }

    let arr = match doc.get_mut(field).and_then(|item| item.as_array_mut()) {
        Some(a) => a,
        None => {
            eprintln!(
                "{}: Field '{}' exists but is not an array",
                "Error".red(),
                field
            );
            return ExitCode::from(1);
        }
    };

    // Check for duplicates
    if arr.iter().any(|v| v.as_str() == Some(value)) {
        eprintln!(
            "{}: Value '{}' already exists in '{}'",
            "Warning".yellow(),
            value,
            field
        );
        return ExitCode::SUCCESS;
    }

    // Add value
    arr.push(value);

    if let Err(e) = save_toml_doc(&config_path, &doc) {
        eprintln!("{}: {}", "Error".red(), e);
        return ExitCode::from(1);
    }

    let config_type = if global { "global" } else { "project" };
    println!(
        "{} Added '{}' to {} in {} configuration",
        "✓".green(),
        value.bold(),
        field,
        config_type
    );

    ExitCode::SUCCESS
}

/// Remove value from an array field
pub fn handle_config_remove(field: &str, value: &str, global: bool) -> ExitCode {
    let config_path = match get_config_path(global) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{}: {}", "Error".red(), e);
            return ExitCode::from(1);
        }
    };

    if !config_path.exists() {
        eprintln!(
            "{}: Config file does not exist: {}",
            "Error".red(),
            config_path.display()
        );
        return ExitCode::from(1);
    }

    let mut doc = match load_toml_doc(&config_path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("{}: {}", "Error".red(), e);
            return ExitCode::from(1);
        }
    };

    let arr = doc.get_mut(field).and_then(|v| v.as_array_mut());

    let arr = match arr {
        Some(a) => a,
        None => {
            eprintln!("{}: Field '{}' not found or is not an array", "Error".red(), field);
            return ExitCode::from(1);
        }
    };

    // Find and remove value
    let initial_len = arr.len();
    arr.retain(|v| v.as_str() != Some(value));

    if arr.len() == initial_len {
        eprintln!(
            "{}: Value '{}' not found in '{}'",
            "Warning".yellow(),
            value,
            field
        );
        return ExitCode::SUCCESS;
    }

    if let Err(e) = save_toml_doc(&config_path, &doc) {
        eprintln!("{}: {}", "Error".red(), e);
        return ExitCode::from(1);
    }

    let config_type = if global { "global" } else { "project" };
    println!(
        "{} Removed '{}' from {} in {} configuration",
        "✓".green(),
        value.bold(),
        field,
        config_type
    );

    ExitCode::SUCCESS
}

/// Clear all values from an array field
pub fn handle_config_clear(field: &str, global: bool) -> ExitCode {
    let config_path = match get_config_path(global) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{}: {}", "Error".red(), e);
            return ExitCode::from(1);
        }
    };

    if !config_path.exists() {
        eprintln!(
            "{}: Config file does not exist: {}",
            "Error".red(),
            config_path.display()
        );
        return ExitCode::from(1);
    }

    let mut doc = match load_toml_doc(&config_path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("{}: {}", "Error".red(), e);
            return ExitCode::from(1);
        }
    };

    // Set field to empty array
    doc[field] = value(Array::new());

    if let Err(e) = save_toml_doc(&config_path, &doc) {
        eprintln!("{}: {}", "Error".red(), e);
        return ExitCode::from(1);
    }

    let config_type = if global { "global" } else { "project" };
    println!(
        "{} Cleared all values from {} in {} configuration",
        "✓".green(),
        field,
        config_type
    );

    ExitCode::SUCCESS
}

/// Validate and parse scalar field value
fn parse_scalar_value(field: &str, val: &str) -> crate::Result<toml_edit::Item> {
    match field {
        "max_complexity" => {
            let num = val.parse::<i64>().map_err(|_| {
                crate::LintisError::Config(
                    "max_complexity must be a positive integer".to_string(),
                )
            })?;
            if num < 0 {
                return Err(crate::LintisError::Config(
                    "max_complexity must be a positive integer".to_string(),
                ));
            }
            Ok(value(num))
        }
        "preset" => {
            if !["google", "standard", "airbnb"].contains(&val) {
                return Err(crate::LintisError::Config(
                    "preset must be one of: google, standard, airbnb".to_string(),
                ));
            }
            Ok(value(val))
        }
        "verbose" => {
            let _ = val.parse::<bool>().map_err(|_| {
                crate::LintisError::Config("verbose must be true or false".to_string())
            })?;
            Ok(value(val))
        }
        _ => Ok(value(val)),
    }
}

/// Set a scalar field value
pub fn handle_config_set(field: &str, value_str: &str, global: bool) -> ExitCode {
    let config_path = match get_config_path(global) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{}: {}", "Error".red(), e);
            return ExitCode::from(1);
        }
    };

    let parsed_value = match parse_scalar_value(field, value_str) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("{}: {}", "Error".red(), e);
            return ExitCode::from(1);
        }
    };

    let mut doc = match load_toml_doc(&config_path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("{}: {}", "Error".red(), e);
            return ExitCode::from(1);
        }
    };

    doc[field] = parsed_value;

    if let Err(e) = save_toml_doc(&config_path, &doc) {
        eprintln!("{}: {}", "Error".red(), e);
        return ExitCode::from(1);
    }

    let config_type = if global { "global" } else { "project" };
    println!(
        "{} Set {} = '{}' in {} configuration",
        "✓".green(),
        field.bold(),
        value_str,
        config_type
    );

    ExitCode::SUCCESS
}

/// Unset a scalar field
pub fn handle_config_unset(field: &str, global: bool) -> ExitCode {
    let config_path = match get_config_path(global) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{}: {}", "Error".red(), e);
            return ExitCode::from(1);
        }
    };

    if !config_path.exists() {
        eprintln!(
            "{}: Config file does not exist: {}",
            "Error".red(),
            config_path.display()
        );
        return ExitCode::from(1);
    }

    let mut doc = match load_toml_doc(&config_path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("{}: {}", "Error".red(), e);
            return ExitCode::from(1);
        }
    };

    if doc.get(field).is_none() {
        eprintln!(
            "{}: Field '{}' not found in configuration",
            "Warning".yellow(),
            field
        );
        return ExitCode::SUCCESS;
    }

    doc.remove(field);

    if let Err(e) = save_toml_doc(&config_path, &doc) {
        eprintln!("{}: {}", "Error".red(), e);
        return ExitCode::from(1);
    }

    let config_type = if global { "global" } else { "project" };
    println!(
        "{} Unset {} in {} configuration",
        "✓".green(),
        field.bold(),
        config_type
    );

    ExitCode::SUCCESS
}

/// Get value of a field
pub fn handle_config_get(field: &str, global: bool) -> ExitCode {
    let config_path = match get_config_path(global) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{}: {}", "Error".red(), e);
            return ExitCode::from(1);
        }
    };

    if !config_path.exists() {
        eprintln!(
            "{}: Config file does not exist: {}",
            "Error".red(),
            config_path.display()
        );
        return ExitCode::from(1);
    }

    let doc = match load_toml_doc(&config_path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("{}: {}", "Error".red(), e);
            return ExitCode::from(1);
        }
    };

    match doc.get(field) {
        Some(value) => {
            if let Some(arr) = value.as_array() {
                print!("[");
                for (i, v) in arr.iter().enumerate() {
                    if i > 0 {
                        print!(", ");
                    }
                    if let Some(s) = v.as_str() {
                        print!("\"{}\"", s);
                    } else {
                        print!("{}", v);
                    }
                }
                println!("]");
            } else {
                println!("{}", value);
            }
        }
        None => {
            eprintln!("{}: Field '{}' not found", "Error".red(), field);
            return ExitCode::from(1);
        }
    }

    ExitCode::SUCCESS
}

/// List all configuration values
pub fn handle_config_list(verbose: bool, global: bool) -> ExitCode {
    let config_path = match get_config_path(global) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{}: {}", "Error".red(), e);
            return ExitCode::from(1);
        }
    };

    if !config_path.exists() {
        let config_type = if global { "global" } else { "project" };
        eprintln!(
            "{}: No {} configuration file found at {}",
            "Warning".yellow(),
            config_type,
            config_path.display()
        );
        return ExitCode::from(1);
    }

    let doc = match load_toml_doc(&config_path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("{}: {}", "Error".red(), e);
            return ExitCode::from(1);
        }
    };

    let config_type = if global { "Global" } else { "Project" };
    println!("{} Configuration ({})", config_type.bold(), config_path.display());
    println!();

    if doc.is_empty() {
        println!("  {}", "(empty)".dimmed());
        return ExitCode::SUCCESS;
    }

    // Print configuration items
    for (key, value) in doc.iter() {
        if verbose {
            println!("{} = {}", key.cyan().bold(), value);
        } else {
            println!("{} = {}", key, value);
        }
    }

    ExitCode::SUCCESS
}

/// Fallback for home directory if dirs crate is not available
mod dirs {
    use std::path::PathBuf;

    pub fn home_dir() -> Option<PathBuf> {
        std::env::var("HOME")
            .ok()
            .map(PathBuf::from)
            .or_else(|| std::env::var("USERPROFILE").ok().map(PathBuf::from))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_config_add_includes() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join(".linthis.toml");

        // Create empty config
        fs::write(&config_path, "").unwrap();

        let mut doc = load_toml_doc(&config_path).unwrap();

        // Add to includes
        if !doc.contains_key("includes") {
            doc["includes"] = toml_edit::Item::Value(toml_edit::Value::Array(Array::new()));
        }
        let arr = doc.get_mut("includes").unwrap().as_array_mut().unwrap();
        arr.push("src/**");
        arr.push("lib/**");

        save_toml_doc(&config_path, &doc).unwrap();

        let config = Config::load(&config_path).unwrap();
        assert_eq!(config.includes, vec!["src/**", "lib/**"]);
    }

    #[test]
    fn test_config_add_dedup() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join(".linthis.toml");

        fs::write(&config_path, "").unwrap();

        let mut doc = load_toml_doc(&config_path).unwrap();

        // Use manual key checking instead of entry().or_insert()
        if !doc.contains_key("excludes") {
            doc["excludes"] = toml_edit::Item::Value(toml_edit::Value::Array(Array::new()));
        }
        let arr = doc.get_mut("excludes").unwrap().as_array_mut().unwrap();

        // Add same value twice
        arr.push("*.log");
        if !arr.iter().any(|v| v.as_str() == Some("*.log")) {
            arr.push("*.log");
        }

        save_toml_doc(&config_path, &doc).unwrap();

        let config = Config::load(&config_path).unwrap();
        assert_eq!(config.excludes, vec!["*.log"]);
    }

    #[test]
    fn test_config_set_max_complexity() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join(".linthis.toml");

        fs::write(&config_path, "").unwrap();

        let mut doc = load_toml_doc(&config_path).unwrap();
        doc["max_complexity"] = value(25i64);
        save_toml_doc(&config_path, &doc).unwrap();

        let config = Config::load(&config_path).unwrap();
        assert_eq!(config.max_complexity, Some(25));
    }

    #[test]
    fn test_parse_scalar_value() {
        assert!(parse_scalar_value("max_complexity", "20").is_ok());
        assert!(parse_scalar_value("max_complexity", "abc").is_err());
        assert!(parse_scalar_value("max_complexity", "-1").is_err());

        assert!(parse_scalar_value("preset", "google").is_ok());
        assert!(parse_scalar_value("preset", "invalid").is_err());

        assert!(parse_scalar_value("verbose", "true").is_ok());
        assert!(parse_scalar_value("verbose", "xyz").is_err());
    }
}
