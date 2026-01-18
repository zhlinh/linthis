//! Integration tests for config system.

use super::common::*;
use std::fs;

/// Test config list command
#[test]
fn test_config_list() {
    let temp = create_temp_project();

    // Create a minimal config file first
    let config_content = r#"# linthis configuration
"#;
    fs::write(temp.path().join(".linthis/config.toml"), config_content)
        .expect("Failed to write config file");

    let output = linthis_cmd()
        .args(["config", "list"])
        .current_dir(temp.path())
        .output()
        .expect("Failed to run linthis");

    assert_exit_code(&output, 0);
}

/// Test config get command
#[test]
fn test_config_get() {
    let temp = create_temp_project();

    // Create a minimal config file first
    fs::write(temp.path().join(".linthis/config.toml"), "# config\n")
        .expect("Failed to write config file");

    let output = linthis_cmd()
        .args(["config", "get", "excludes"])
        .current_dir(temp.path())
        .output()
        .expect("Failed to run linthis");

    // Should succeed or report key not found
    let _ = output.status;
}

/// Test config set command
#[test]
fn test_config_set() {
    let temp = create_temp_project();

    let output = linthis_cmd()
        .args(["config", "set", "test_key", "test_value"])
        .current_dir(temp.path())
        .output()
        .expect("Failed to run linthis");

    // Should succeed setting a config value
    assert_exit_code(&output, 0);
}

/// Test config set and get workflow
#[test]
fn test_config_set_get_workflow() {
    let temp = create_temp_project();

    // Set a config value
    let set_output = linthis_cmd()
        .args(["config", "set", "workflow_test", "my_value"])
        .current_dir(temp.path())
        .output()
        .expect("Failed to run linthis");

    assert_exit_code(&set_output, 0);

    // Get the config value
    let get_output = linthis_cmd()
        .args(["config", "get", "workflow_test"])
        .current_dir(temp.path())
        .output()
        .expect("Failed to run linthis");

    assert_exit_code(&get_output, 0);

    let stdout = String::from_utf8_lossy(&get_output.stdout);
    assert!(
        stdout.contains("my_value"),
        "Should retrieve the set value"
    );
}

/// Test init command creates config
#[test]
fn test_init_creates_config() {
    let temp = create_temp_project();

    // Remove .linthis to test clean init
    let _ = fs::remove_dir_all(temp.path().join(".linthis"));

    let output = linthis_cmd()
        .args(["init"])
        .current_dir(temp.path())
        .output()
        .expect("Failed to run linthis");

    assert_exit_code(&output, 0);

    // Verify .linthis directory was created
    assert!(
        temp.path().join(".linthis").exists(),
        ".linthis directory should be created"
    );
}

/// Test init with --force flag
#[test]
fn test_init_force() {
    let temp = create_temp_project();

    // First init
    let first_init = linthis_cmd()
        .args(["init"])
        .current_dir(temp.path())
        .output()
        .expect("Failed to run linthis");

    assert_exit_code(&first_init, 0);

    // Second init with --force
    let force_init = linthis_cmd()
        .args(["init", "--force"])
        .current_dir(temp.path())
        .output()
        .expect("Failed to run linthis");

    assert_exit_code(&force_init, 0);
}

/// Test init with --with-hook flag
#[test]
fn test_init_with_hook() {
    if !has_git() {
        eprintln!("Skipping test_init_with_hook: git not available");
        return;
    }

    let temp = create_temp_project();

    // Initialize git repo first
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(temp.path())
        .output()
        .expect("Failed to init git");

    let output = linthis_cmd()
        .args(["init", "--with-hook"])
        .current_dir(temp.path())
        .output()
        .expect("Failed to run linthis");

    assert_exit_code(&output, 0);

    // Verify git hook was created
    let hook_path = temp.path().join(".git/hooks/pre-commit");
    assert!(hook_path.exists(), "Pre-commit hook should be created");
}

/// Test config with project config file
#[test]
fn test_config_project_file() {
    let temp = create_temp_project();

    // Create a project config file
    let config_content = r#"[excludes]
patterns = ["vendor/**", "third_party/**"]

[languages]
enabled = ["python", "rust"]
"#;
    fs::write(temp.path().join(".linthis/config.toml"), config_content)
        .expect("Failed to write config file");

    let output = linthis_cmd()
        .args(["config", "list"])
        .current_dir(temp.path())
        .output()
        .expect("Failed to run linthis");

    assert_exit_code(&output, 0);

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Config list should show some configuration
    assert!(
        stdout.len() > 0 || !stdout.is_empty(),
        "Config list should produce output"
    );
}

/// Test hook install command
#[test]
fn test_hook_install() {
    if !has_git() {
        eprintln!("Skipping test_hook_install: git not available");
        return;
    }

    let temp = create_temp_project();

    // Initialize git repo
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(temp.path())
        .output()
        .expect("Failed to init git");

    let output = linthis_cmd()
        .args(["hook", "install"])
        .current_dir(temp.path())
        .output()
        .expect("Failed to run linthis");

    assert_exit_code(&output, 0);
}

/// Test hook uninstall command
#[test]
fn test_hook_uninstall() {
    if !has_git() {
        eprintln!("Skipping test_hook_uninstall: git not available");
        return;
    }

    let temp = create_temp_project();

    // Initialize git repo
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(temp.path())
        .output()
        .expect("Failed to init git");

    // Install hook first
    let _ = linthis_cmd()
        .args(["hook", "install"])
        .current_dir(temp.path())
        .output();

    let output = linthis_cmd()
        .args(["hook", "uninstall"])
        .current_dir(temp.path())
        .output()
        .expect("Failed to run linthis");

    assert_exit_code(&output, 0);
}
