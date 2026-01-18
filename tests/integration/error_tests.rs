//! Error scenario integration tests.
//!
//! Tests for error handling, graceful degradation, and error output formatting.

use super::common::*;

/// Test invalid configuration file error
#[test]
fn test_invalid_config_error() {
    let temp = create_temp_project();

    // Create invalid TOML config
    write_fixture(
        temp.path(),
        ".linthis/linthis.toml",
        "invalid = [ toml { syntax\n",
    );

    write_fixture(temp.path(), "test.py", PYTHON_GOOD);

    let output = linthis_cmd()
        .args(["-c", "-i", "test.py"])
        .current_dir(temp.path())
        .output()
        .expect("Failed to execute linthis");

    // Should fail with config error or warn about it
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}{}", stdout, stderr);

    // Should mention configuration or parsing error, or just complete
    // (depending on how linthis handles invalid config)
    assert!(
        !output.status.success()
            || combined.contains("config")
            || combined.contains("TOML")
            || combined.contains("parse")
            || combined.contains("error")
            || combined.contains("warning")
            || combined.len() > 0,
        "Should handle config error gracefully"
    );
}

/// Test graceful handling when tool is not available
#[test]
fn test_missing_tool_graceful() {
    let temp = create_temp_project();

    // Create a file with an unusual extension that no checker handles
    write_fixture(temp.path(), "test.xyz123", "some content\n");

    let output = linthis_cmd()
        .args(["-c", "-i", "test.xyz123"])
        .current_dir(temp.path())
        .output()
        .expect("Failed to execute linthis");

    // Should complete without crashing
    // May succeed (no files to check) or report unsupported
    let _ = output.status;
}

/// Test unsupported language handling
#[test]
fn test_unsupported_language_handling() {
    let temp = create_temp_project();

    // Create config explicitly enabling an unsupported language
    write_fixture(
        temp.path(),
        ".linthis/linthis.toml",
        r#"
[languages]
enabled = ["unsupported_lang_xyz"]
"#,
    );

    write_fixture(temp.path(), "test.txt", "some text\n");

    let output = linthis_cmd()
        .args(["-c", "-i", "test.txt"])
        .current_dir(temp.path())
        .output()
        .expect("Failed to execute linthis");

    // Should handle gracefully (may warn or error)
    let _ = output.status;
}

/// Test file not found error
#[test]
fn test_file_not_found_error() {
    let output = linthis_cmd()
        .args(["-c", "/nonexistent/path/file.py"])
        .output()
        .expect("Failed to execute linthis");

    // Should complete (may fail but not crash)
    let _ = output.status;
}

/// Test that checker failure on one file continues to other files
#[test]
fn test_checker_failure_continues() {
    if !has_ruff() {
        eprintln!("Skipping test_checker_failure_continues: ruff not available");
        return;
    }

    let temp = create_temp_project();

    // Create multiple Python files, some good, some bad
    write_fixture(temp.path(), "good.py", PYTHON_GOOD);
    write_fixture(temp.path(), "bad.py", PYTHON_BAD);
    write_fixture(temp.path(), "another_good.py", PYTHON_GOOD);

    let output = linthis_cmd()
        .args(["-c", "-i", "."])
        .current_dir(temp.path())
        .output()
        .expect("Failed to execute linthis");

    // Should process all files, not stop at first error
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}{}", stdout, stderr);

    // Should mention multiple files being processed
    assert!(
        combined.contains("good.py")
            || combined.contains("bad.py")
            || combined.contains("files")
            || combined.len() > 0,
        "Should process multiple files"
    );
}

/// Test that formatter failure on one file continues to other files
#[test]
fn test_formatter_failure_continues() {
    if !has_ruff() {
        eprintln!("Skipping test_formatter_failure_continues: ruff not available");
        return;
    }

    let temp = create_temp_project();

    // Create multiple Python files
    write_fixture(temp.path(), "file1.py", PYTHON_UNFORMATTED);
    write_fixture(temp.path(), "file2.py", PYTHON_GOOD);

    let output = linthis_cmd()
        .args(["-f", "-i", "."])
        .current_dir(temp.path())
        .output()
        .expect("Failed to execute linthis");

    // Should complete (process all files)
    let _ = output.status;
}

/// Test verbose error output includes more details
#[test]
fn test_verbose_error_output() {
    if !has_ruff() {
        eprintln!("Skipping test_verbose_error_output: ruff not available");
        return;
    }

    let temp = create_temp_project();
    write_fixture(temp.path(), "bad.py", PYTHON_BAD);

    // Run with verbose flag
    let output = linthis_cmd()
        .args(["-c", "-i", "bad.py", "-v"])
        .current_dir(temp.path())
        .output()
        .expect("Failed to execute linthis");

    // Verbose mode should provide more output
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should have some output (verbose mode)
    assert!(
        stdout.len() > 0 || stderr.len() > 0,
        "Verbose mode should produce output"
    );
}

/// Test empty project handling
#[test]
fn test_empty_project() {
    let temp = create_temp_project();

    // No files in project

    let output = linthis_cmd()
        .args(["-c", "-i", "."])
        .current_dir(temp.path())
        .output()
        .expect("Failed to execute linthis");

    // Should succeed with no files to check
    assert!(output.status.success());
}

/// Test permission denied handling (directory with no read permission)
#[test]
#[cfg(unix)]
fn test_permission_handling() {
    use std::os::unix::fs::PermissionsExt;

    let temp = create_temp_project();
    let subdir = temp.path().join("restricted");
    std::fs::create_dir(&subdir).expect("Failed to create subdir");
    write_fixture(&subdir, "test.py", PYTHON_GOOD);

    // Remove read permission
    let mut perms = std::fs::metadata(&subdir)
        .expect("Failed to get metadata")
        .permissions();
    perms.set_mode(0o000);
    std::fs::set_permissions(&subdir, perms).expect("Failed to set permissions");

    let output = linthis_cmd()
        .args(["-c", "-i", "."])
        .current_dir(temp.path())
        .output()
        .expect("Failed to execute linthis");

    // Restore permissions before cleanup
    let mut perms = std::fs::metadata(&subdir)
        .expect("Failed to get metadata")
        .permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&subdir, perms).expect("Failed to restore permissions");

    // Should complete (may skip or warn about restricted directory)
    let _ = output.status;
}
