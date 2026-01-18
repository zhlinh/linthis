//! Language-specific integration tests.
//!
//! Tests for Go, TypeScript, C++, Java, Dart, Kotlin, Lua, and Swift.
//! Tests are skipped if the required tools are not available.

use super::common::*;

// ============================================================================
// Go Tests
// ============================================================================

#[test]
fn test_go_check() {
    if !has_golangci_lint() {
        eprintln!("Skipping test_go_check: golangci-lint not available");
        return;
    }

    let temp = create_temp_project();
    write_fixture(temp.path(), "main.go", GO_GOOD);

    // Create go.mod for the project
    write_fixture(temp.path(), "go.mod", "module test\n\ngo 1.21\n");

    let output = linthis_cmd()
        .args(["-c", "-i", "main.go"])
        .current_dir(temp.path())
        .output()
        .expect("Failed to execute linthis");

    // Good code should pass
    assert!(output.status.success() || output.status.code() == Some(0));
}

#[test]
fn test_go_format() {
    if !has_goimports() {
        eprintln!("Skipping test_go_format: goimports not available");
        return;
    }

    let temp = create_temp_project();
    write_fixture(temp.path(), "main.go", GO_UNFORMATTED);
    write_fixture(temp.path(), "go.mod", "module test\n\ngo 1.21\n");

    let output = linthis_cmd()
        .args(["-f", "-i", "main.go"])
        .current_dir(temp.path())
        .output()
        .expect("Failed to execute linthis");

    // Format should succeed
    assert!(output.status.success() || output.status.code() == Some(0));
}

// ============================================================================
// TypeScript Tests
// ============================================================================

#[test]
fn test_typescript_check() {
    if !has_eslint() {
        eprintln!("Skipping test_typescript_check: eslint not available");
        return;
    }

    let temp = create_temp_project();
    write_fixture(temp.path(), "index.ts", TS_GOOD);

    // Create minimal package.json
    write_fixture(
        temp.path(),
        "package.json",
        r#"{"name": "test", "version": "1.0.0"}"#,
    );

    let output = linthis_cmd()
        .args(["-c", "-i", "index.ts"])
        .current_dir(temp.path())
        .output()
        .expect("Failed to execute linthis");

    // Check should complete (may have warnings/errors depending on eslint config)
    let _ = output.status;
}

#[test]
fn test_typescript_format() {
    if !has_prettier() {
        eprintln!("Skipping test_typescript_format: prettier not available");
        return;
    }

    let temp = create_temp_project();
    write_fixture(temp.path(), "index.ts", TS_UNFORMATTED);

    let output = linthis_cmd()
        .args(["-f", "-i", "index.ts"])
        .current_dir(temp.path())
        .output()
        .expect("Failed to execute linthis");

    // Format should complete
    let _ = output.status;
}

// ============================================================================
// C++ Tests
// ============================================================================

#[test]
fn test_cpp_check() {
    if !has_cpplint() {
        eprintln!("Skipping test_cpp_check: cpplint not available");
        return;
    }

    let temp = create_temp_project();
    write_fixture(temp.path(), "main.cpp", CPP_GOOD);

    let output = linthis_cmd()
        .args(["-c", "-i", "main.cpp"])
        .current_dir(temp.path())
        .output()
        .expect("Failed to execute linthis");

    // Check should complete
    let _ = output.status;
}

#[test]
fn test_cpp_format() {
    if !has_clang_format() {
        eprintln!("Skipping test_cpp_format: clang-format not available");
        return;
    }

    let temp = create_temp_project();
    write_fixture(temp.path(), "main.cpp", CPP_BAD);

    let output = linthis_cmd()
        .args(["-f", "-i", "main.cpp"])
        .current_dir(temp.path())
        .output()
        .expect("Failed to execute linthis");

    // Format should complete
    let _ = output.status;
}

// ============================================================================
// Java Tests
// ============================================================================

#[test]
fn test_java_check() {
    if !has_checkstyle() {
        eprintln!("Skipping test_java_check: checkstyle not available");
        return;
    }

    let temp = create_temp_project();
    write_fixture(temp.path(), "Hello.java", JAVA_GOOD);

    let output = linthis_cmd()
        .args(["-c", "-i", "Hello.java"])
        .current_dir(temp.path())
        .output()
        .expect("Failed to execute linthis");

    // Check should complete
    let _ = output.status;
}

#[test]
fn test_java_format() {
    if !has_clang_format() {
        eprintln!("Skipping test_java_format: clang-format not available");
        return;
    }

    let temp = create_temp_project();
    write_fixture(temp.path(), "Hello.java", JAVA_BAD);

    let output = linthis_cmd()
        .args(["-f", "-i", "Hello.java"])
        .current_dir(temp.path())
        .output()
        .expect("Failed to execute linthis");

    // Format should complete
    let _ = output.status;
}

// ============================================================================
// Dart Tests
// ============================================================================

#[test]
fn test_dart_check() {
    if !has_dart() {
        eprintln!("Skipping test_dart_check: dart not available");
        return;
    }

    let temp = create_temp_project();
    write_fixture(temp.path(), "main.dart", DART_GOOD);

    // Create minimal pubspec.yaml
    write_fixture(
        temp.path(),
        "pubspec.yaml",
        "name: test\nenvironment:\n  sdk: '>=3.0.0 <4.0.0'\n",
    );

    let output = linthis_cmd()
        .args(["-c", "-i", "main.dart"])
        .current_dir(temp.path())
        .output()
        .expect("Failed to execute linthis");

    // Check should complete
    let _ = output.status;
}

#[test]
fn test_dart_format() {
    if !has_dart() {
        eprintln!("Skipping test_dart_format: dart not available");
        return;
    }

    let temp = create_temp_project();
    write_fixture(temp.path(), "main.dart", DART_BAD);

    let output = linthis_cmd()
        .args(["-f", "-i", "main.dart"])
        .current_dir(temp.path())
        .output()
        .expect("Failed to execute linthis");

    // Format should complete
    let _ = output.status;
}

// ============================================================================
// Kotlin Tests
// ============================================================================

#[test]
fn test_kotlin_check() {
    if !has_ktlint() {
        eprintln!("Skipping test_kotlin_check: ktlint not available");
        return;
    }

    let temp = create_temp_project();
    write_fixture(temp.path(), "Main.kt", KOTLIN_GOOD);

    let output = linthis_cmd()
        .args(["-c", "-i", "Main.kt"])
        .current_dir(temp.path())
        .output()
        .expect("Failed to execute linthis");

    // Check should complete
    let _ = output.status;
}

#[test]
fn test_kotlin_format() {
    if !has_ktlint() {
        eprintln!("Skipping test_kotlin_format: ktlint not available");
        return;
    }

    let temp = create_temp_project();
    write_fixture(temp.path(), "Main.kt", KOTLIN_BAD);

    let output = linthis_cmd()
        .args(["-f", "-i", "Main.kt"])
        .current_dir(temp.path())
        .output()
        .expect("Failed to execute linthis");

    // Format should complete
    let _ = output.status;
}

// ============================================================================
// Lua Tests
// ============================================================================

#[test]
fn test_lua_check() {
    if !has_luacheck() {
        eprintln!("Skipping test_lua_check: luacheck not available");
        return;
    }

    let temp = create_temp_project();
    write_fixture(temp.path(), "main.lua", LUA_GOOD);

    let output = linthis_cmd()
        .args(["-c", "-i", "main.lua"])
        .current_dir(temp.path())
        .output()
        .expect("Failed to execute linthis");

    // Check should complete
    let _ = output.status;
}

#[test]
fn test_lua_format() {
    if !has_stylua() {
        eprintln!("Skipping test_lua_format: stylua not available");
        return;
    }

    let temp = create_temp_project();
    write_fixture(temp.path(), "main.lua", LUA_BAD);

    let output = linthis_cmd()
        .args(["-f", "-i", "main.lua"])
        .current_dir(temp.path())
        .output()
        .expect("Failed to execute linthis");

    // Format should complete
    let _ = output.status;
}

// ============================================================================
// Swift Tests
// ============================================================================

#[test]
fn test_swift_check() {
    if !has_swiftlint() {
        eprintln!("Skipping test_swift_check: swiftlint not available");
        return;
    }

    let temp = create_temp_project();
    write_fixture(temp.path(), "main.swift", SWIFT_GOOD);

    let output = linthis_cmd()
        .args(["-c", "-i", "main.swift"])
        .current_dir(temp.path())
        .output()
        .expect("Failed to execute linthis");

    // Check should complete
    let _ = output.status;
}

#[test]
fn test_swift_format() {
    if !has_swift_format() {
        eprintln!("Skipping test_swift_format: swift-format not available");
        return;
    }

    let temp = create_temp_project();
    write_fixture(temp.path(), "main.swift", SWIFT_BAD);

    let output = linthis_cmd()
        .args(["-f", "-i", "main.swift"])
        .current_dir(temp.path())
        .output()
        .expect("Failed to execute linthis");

    // Format should complete
    let _ = output.status;
}
