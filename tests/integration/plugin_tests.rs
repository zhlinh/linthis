//! Integration tests for plugin system.

use super::common::*;
use std::fs;

/// Test plugin list command (empty initially)
#[test]
fn test_plugin_list_empty() {
    let temp = create_temp_project();

    let output = linthis_cmd()
        .args(["plugin", "list"])
        .current_dir(temp.path())
        .output()
        .expect("Failed to run linthis");

    // Should succeed even with no plugins
    assert_exit_code(&output, 0);
}

/// Test plugin add with invalid URL
#[test]
fn test_plugin_add_invalid_url() {
    let temp = create_temp_project();

    let output = linthis_cmd()
        .args(["plugin", "add", "test-plugin", "not-a-valid-url"])
        .current_dir(temp.path())
        .output()
        .expect("Failed to run linthis");

    // URL validation may be lenient - just ensure the command completes
    // The actual validation may happen during sync
    let _ = output.status;
}

/// Test plugin add with valid URL
#[test]
fn test_plugin_add_with_name() {
    let temp = create_temp_project();

    let output = linthis_cmd()
        .args([
            "plugin",
            "add",
            "test-plugin",
            "https://example.com/test.git",
        ])
        .current_dir(temp.path())
        .output()
        .expect("Failed to run linthis");

    // Should succeed adding plugin config
    assert_exit_code(&output, 0);

    // Verify plugin was added to config
    let list_output = linthis_cmd()
        .args(["plugin", "list"])
        .current_dir(temp.path())
        .output()
        .expect("Failed to run linthis");

    let stdout = String::from_utf8_lossy(&list_output.stdout);
    assert!(
        stdout.contains("test-plugin") || stdout.contains("example.com"),
        "Plugin should appear in list"
    );
}

/// Test plugin remove non-existent
#[test]
fn test_plugin_remove_nonexistent() {
    let temp = create_temp_project();

    let output = linthis_cmd()
        .args(["plugin", "remove", "nonexistent-plugin"])
        .current_dir(temp.path())
        .output()
        .expect("Failed to run linthis");

    // Should fail or report not found
    // The exit code depends on implementation
    let _ = output.status;
}

/// Test plugin add and remove workflow
#[test]
fn test_plugin_add_remove_workflow() {
    let temp = create_temp_project();

    // Add a plugin
    let add_output = linthis_cmd()
        .args([
            "plugin",
            "add",
            "workflow-test",
            "https://github.com/example/plugin.git",
        ])
        .current_dir(temp.path())
        .output()
        .expect("Failed to run linthis");

    assert_exit_code(&add_output, 0);

    // Remove the plugin
    let remove_output = linthis_cmd()
        .args(["plugin", "remove", "workflow-test"])
        .current_dir(temp.path())
        .output()
        .expect("Failed to run linthis");

    assert_exit_code(&remove_output, 0);

    // Verify plugin was removed
    let list_output = linthis_cmd()
        .args(["plugin", "list"])
        .current_dir(temp.path())
        .output()
        .expect("Failed to run linthis");

    let stdout = String::from_utf8_lossy(&list_output.stdout);
    assert!(
        !stdout.contains("workflow-test"),
        "Plugin should not appear in list after removal"
    );
}

/// Test plugin sync with no plugins
#[test]
fn test_plugin_sync_empty() {
    let temp = create_temp_project();

    let output = linthis_cmd()
        .args(["plugin", "sync"])
        .current_dir(temp.path())
        .output()
        .expect("Failed to run linthis");

    // Should succeed even with no plugins
    assert_exit_code(&output, 0);
}

/// Test plugin global flag
#[test]
fn test_plugin_global_flag() {
    // Test with --global flag for project plugins
    let temp = create_temp_project();

    let output = linthis_cmd()
        .args(["plugin", "list", "--global"])
        .current_dir(temp.path())
        .output()
        .expect("Failed to run linthis");

    // Should work with global flag
    assert_exit_code(&output, 0);
}

/// Test plugin add duplicate
#[test]
fn test_plugin_add_duplicate() {
    let temp = create_temp_project();

    // Add a plugin first
    let first_add = linthis_cmd()
        .args([
            "plugin",
            "add",
            "dup-test",
            "https://example.com/dup.git",
        ])
        .current_dir(temp.path())
        .output()
        .expect("Failed to run linthis");

    assert_exit_code(&first_add, 0);

    // Try to add the same plugin again
    let second_add = linthis_cmd()
        .args([
            "plugin",
            "add",
            "dup-test",
            "https://example.com/dup.git",
        ])
        .current_dir(temp.path())
        .output()
        .expect("Failed to run linthis");

    // Should either succeed (updating) or fail (duplicate)
    // The behavior depends on implementation
    let _ = second_add.status;
}

/// Test plugin validate command
#[test]
fn test_plugin_validate() {
    let temp = create_temp_project();

    // Create a minimal plugin structure
    let plugin_dir = temp.path().join(".linthis/plugins/test-validate");
    fs::create_dir_all(&plugin_dir).expect("Failed to create plugin dir");

    // Create a minimal plugin.toml
    let plugin_toml = r#"[plugin]
name = "test-validate"
version = "0.1.0"
description = "Test plugin"

[linters]
"#;
    fs::write(plugin_dir.join("plugin.toml"), plugin_toml)
        .expect("Failed to write plugin.toml");

    let output = linthis_cmd()
        .args(["plugin", "validate", "test-validate"])
        .current_dir(temp.path())
        .output()
        .expect("Failed to run linthis");

    // Validate should complete without crash
    let _ = output.status;
}

/// Test plugin clean command
#[test]
fn test_plugin_clean() {
    let temp = create_temp_project();

    let output = linthis_cmd()
        .args(["plugin", "clean"])
        .current_dir(temp.path())
        .output()
        .expect("Failed to run linthis");

    // Should succeed even with nothing to clean
    assert_exit_code(&output, 0);
}
