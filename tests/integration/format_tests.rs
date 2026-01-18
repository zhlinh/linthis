//! Integration tests for format mode (-f flag).

use super::common::*;

/// Test formatting a Python file
#[test]
fn test_python_format_file() {
    if !has_ruff() {
        eprintln!("Skipping test_python_format_file: ruff not available");
        return;
    }

    let temp = create_temp_project();
    write_fixture(temp.path(), "test.py", PYTHON_UNFORMATTED);

    let output = linthis_cmd()
        .args(["-f", "-i", "test.py"])
        .current_dir(temp.path())
        .output()
        .expect("Failed to run linthis");

    // Format should succeed
    assert_exit_code(&output, 0);

    // File should be modified (formatted)
    let content = std::fs::read_to_string(temp.path().join("test.py")).unwrap();
    assert!(
        content != PYTHON_UNFORMATTED,
        "File should be modified after formatting"
    );
}

/// Test formatting a Rust file
#[test]
fn test_rust_format_file() {
    if !has_rustfmt() {
        eprintln!("Skipping test_rust_format_file: rustfmt not available");
        return;
    }

    let temp = create_temp_project();
    write_fixture(temp.path(), "test.rs", RUST_UNFORMATTED);

    let output = linthis_cmd()
        .args(["-f", "-i", "test.rs"])
        .current_dir(temp.path())
        .output()
        .expect("Failed to run linthis");

    // Format should succeed
    assert_exit_code(&output, 0);

    // File should be modified (formatted)
    let content = std::fs::read_to_string(temp.path().join("test.rs")).unwrap();
    assert!(
        content != RUST_UNFORMATTED,
        "File should be modified after formatting"
    );
}

/// Test format with quiet mode
#[test]
fn test_format_quiet_mode() {
    if !has_ruff() {
        eprintln!("Skipping test_format_quiet_mode: ruff not available");
        return;
    }

    let temp = create_temp_project();
    write_fixture(temp.path(), "test.py", PYTHON_GOOD);

    let output = linthis_cmd()
        .args(["-f", "--quiet", "-i", "test.py"])
        .current_dir(temp.path())
        .output()
        .expect("Failed to run linthis");

    assert_exit_code(&output, 0);
    // Quiet mode should have minimal output
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.len() < 500,
        "Quiet mode should have minimal output"
    );
}

/// Test formatting multiple files
#[test]
fn test_format_multiple_files() {
    if !has_ruff() {
        eprintln!("Skipping test_format_multiple_files: ruff not available");
        return;
    }

    let temp = create_temp_project();
    write_fixture(temp.path(), "a.py", PYTHON_UNFORMATTED);
    write_fixture(temp.path(), "b.py", PYTHON_UNFORMATTED);

    let output = linthis_cmd()
        .args(["-f", "-i", "a.py", "-i", "b.py"])
        .current_dir(temp.path())
        .output()
        .expect("Failed to run linthis");

    assert_exit_code(&output, 0);
}

/// Test formatting with exclude
#[test]
fn test_format_with_exclude() {
    if !has_ruff() {
        eprintln!("Skipping test_format_with_exclude: ruff not available");
        return;
    }

    let temp = create_temp_project();
    write_fixture(temp.path(), "src/test.py", PYTHON_UNFORMATTED);
    write_fixture(temp.path(), "vendor/test.py", PYTHON_UNFORMATTED);

    let output = linthis_cmd()
        .args(["-f", "--exclude", "vendor/**", "-i", "."])
        .current_dir(temp.path())
        .output()
        .expect("Failed to run linthis");

    assert_exit_code(&output, 0);

    // vendor file should NOT be formatted
    let vendor_content = std::fs::read_to_string(temp.path().join("vendor/test.py")).unwrap();
    assert_eq!(
        vendor_content, PYTHON_UNFORMATTED,
        "Excluded file should not be formatted"
    );
}

/// Test formatting already formatted file
#[test]
fn test_format_already_formatted() {
    if !has_ruff() {
        eprintln!("Skipping test_format_already_formatted: ruff not available");
        return;
    }

    let temp = create_temp_project();
    write_fixture(temp.path(), "test.py", PYTHON_GOOD);

    let output = linthis_cmd()
        .args(["-f", "-i", "test.py"])
        .current_dir(temp.path())
        .output()
        .expect("Failed to run linthis");

    assert_exit_code(&output, 0);

    // File should remain unchanged
    let content = std::fs::read_to_string(temp.path().join("test.py")).unwrap();
    assert_eq!(content, PYTHON_GOOD, "Already formatted file should remain unchanged");
}
