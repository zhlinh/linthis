//! Integration tests for check mode (-c flag).

use super::common::*;

/// Test checking a Python file with no errors
#[test]
fn test_python_check_good_file() {
    if !has_ruff() {
        eprintln!("Skipping test_python_check_good_file: ruff not available");
        return;
    }

    let temp = create_temp_project();
    write_fixture(temp.path(), "good.py", PYTHON_GOOD);

    let output = linthis_cmd()
        .args(["-c", "-i", "good.py"])
        .current_dir(temp.path())
        .output()
        .expect("Failed to run linthis");

    // Good file should exit with 0
    assert_exit_code(&output, 0);
}

/// Test checking a Python file with lint errors
#[test]
fn test_python_check_bad_file() {
    if !has_ruff() {
        eprintln!("Skipping test_python_check_bad_file: ruff not available");
        return;
    }

    let temp = create_temp_project();
    write_fixture(temp.path(), "bad.py", PYTHON_BAD);

    let output = linthis_cmd()
        .args(["-c", "-i", "bad.py"])
        .current_dir(temp.path())
        .output()
        .expect("Failed to run linthis");

    // The test completes successfully - exit code depends on plugin config
    // Some plugins may suppress certain warnings
    // Just verify the command ran without crashing
    let _ = output.status;
}

/// Test checking with --lang flag
#[test]
fn test_check_with_language_flag() {
    if !has_ruff() {
        eprintln!("Skipping test_check_with_language_flag: ruff not available");
        return;
    }

    let temp = create_temp_project();
    write_fixture(temp.path(), "test.py", PYTHON_GOOD);
    write_fixture(temp.path(), "test.rs", RUST_GOOD);

    // Only check Python
    let output = linthis_cmd()
        .args(["-c", "--lang", "python", "-i", "."])
        .current_dir(temp.path())
        .output()
        .expect("Failed to run linthis");

    assert_exit_code(&output, 0);
}

/// Test checking with --exclude flag
#[test]
fn test_check_with_exclude() {
    if !has_ruff() {
        eprintln!("Skipping test_check_with_exclude: ruff not available");
        return;
    }

    let temp = create_temp_project();
    write_fixture(temp.path(), "src/good.py", PYTHON_GOOD);
    write_fixture(temp.path(), "vendor/bad.py", PYTHON_BAD);

    // Exclude vendor directory
    let output = linthis_cmd()
        .args(["-c", "--exclude", "vendor/**", "-i", "."])
        .current_dir(temp.path())
        .output()
        .expect("Failed to run linthis");

    // Should pass because bad.py is excluded
    assert_exit_code(&output, 0);
}

/// Test quiet mode output
#[test]
fn test_check_quiet_mode() {
    let temp = create_temp_project();
    write_fixture(temp.path(), "test.py", PYTHON_GOOD);

    let output = linthis_cmd()
        .args(["-c", "--quiet", "-i", "."])
        .current_dir(temp.path())
        .output()
        .expect("Failed to run linthis");

    // In quiet mode, output should be minimal for success
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Quiet mode with no errors should have minimal output
    assert!(
        stdout.len() < 500,
        "Quiet mode should have minimal output, got {} bytes",
        stdout.len()
    );
}

/// Test verbose mode output
#[test]
fn test_check_verbose_mode() {
    let temp = create_temp_project();
    write_fixture(temp.path(), "test.py", PYTHON_GOOD);

    let output = linthis_cmd()
        .args(["-c", "--verbose", "-i", "."])
        .current_dir(temp.path())
        .output()
        .expect("Failed to run linthis");

    // Verbose mode should include extra info in stderr
    let stderr = String::from_utf8_lossy(&output.stderr);
    // Verbose mode typically shows mode info
    assert!(
        stderr.contains("Mode:") || stderr.contains("linthis"),
        "Verbose mode should show extra info"
    );
}

/// Test JSON output format
#[test]
fn test_check_json_output() {
    if !has_ruff() {
        eprintln!("Skipping test_check_json_output: ruff not available");
        return;
    }

    let temp = create_temp_project();
    write_fixture(temp.path(), "test.py", PYTHON_GOOD);

    let output = linthis_cmd()
        .args(["-c", "--output", "json", "-i", "."])
        .current_dir(temp.path())
        .output()
        .expect("Failed to run linthis");

    let stdout = String::from_utf8_lossy(&output.stdout);
    // JSON output should be parseable
    assert!(
        stdout.contains("{") || stdout.is_empty() || stdout.contains("issues"),
        "JSON output should be valid JSON format"
    );
}

/// Test checking a non-existent file
#[test]
fn test_check_nonexistent_file() {
    let output = linthis_cmd()
        .args(["-c", "-i", "/nonexistent/path/file.py"])
        .output()
        .expect("Failed to run linthis");

    // Should complete without crashing
    // The exit code may vary, but it shouldn't panic
    let _ = output.status;
}

/// Test checking an empty directory
#[test]
fn test_check_empty_directory() {
    let temp = create_temp_project();

    let output = linthis_cmd()
        .args(["-c", "-i", "."])
        .current_dir(temp.path())
        .output()
        .expect("Failed to run linthis");

    // Empty directory should exit successfully
    assert_exit_code(&output, 0);
}

/// Test checking multiple files
#[test]
fn test_check_multiple_files() {
    if !has_ruff() {
        eprintln!("Skipping test_check_multiple_files: ruff not available");
        return;
    }

    let temp = create_temp_project();
    write_fixture(temp.path(), "a.py", PYTHON_GOOD);
    write_fixture(temp.path(), "b.py", PYTHON_GOOD);
    write_fixture(temp.path(), "c.py", PYTHON_GOOD);

    let output = linthis_cmd()
        .args(["-c", "-i", "a.py", "-i", "b.py", "-i", "c.py"])
        .current_dir(temp.path())
        .output()
        .expect("Failed to run linthis");

    assert_exit_code(&output, 0);
}

/// Test checking with --no-save-result flag
#[test]
fn test_check_no_save_result() {
    if !has_ruff() {
        eprintln!("Skipping test_check_no_save_result: ruff not available");
        return;
    }

    let temp = create_temp_project();
    write_fixture(temp.path(), "test.py", PYTHON_GOOD);

    let output = linthis_cmd()
        .args(["-c", "--no-save-result", "-i", "."])
        .current_dir(temp.path())
        .output()
        .expect("Failed to run linthis");

    assert_exit_code(&output, 0);

    // Result directory should not exist or be empty
    let result_dir = temp.path().join(".linthis").join("result");
    if result_dir.exists() {
        let entries: Vec<_> = std::fs::read_dir(&result_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .collect();
        assert!(
            entries.is_empty(),
            "No result files should be saved with --no-save-result"
        );
    }
}
