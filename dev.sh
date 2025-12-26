#!/bin/bash
# Development helper script for linthis
# See .specify/memory/constitution.md for full development guidelines

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

usage() {
    echo "Usage: ./dev.sh <command>"
    echo ""
    echo "Commands:"
    echo "  build     Build release binary"
    echo "  install   Build and install to ~/.cargo/bin"
    echo "  check     Run clippy, fmt check, and tests"
    echo "  fix       Auto-fix clippy warnings and format code"
    echo "  test      Run all tests"
    echo "  all       Run check, build, and install (recommended before testing)"
    echo ""
    echo "Examples:"
    echo "  ./dev.sh install   # Build and install latest version"
    echo "  ./dev.sh check     # Verify code quality before commit"
    echo "  ./dev.sh all       # Full development cycle"
}

cmd_build() {
    echo "Building release..."
    cargo build --release
    echo "Build complete: target/release/linthis"
}

cmd_install() {
    echo "Building release..."
    cargo build --release

    echo "Installing..."
    # Copy to cargo bin
    cp target/release/linthis ~/.cargo/bin/
    echo "Installed to: ~/.cargo/bin/linthis"

    # Also update venv if it exists and has linthis
    if [ -f "$HOME/.venv/bin/linthis" ]; then
        cp target/release/linthis "$HOME/.venv/bin/"
        echo "Updated: ~/.venv/bin/linthis"
    fi

    # Verify installation
    echo ""
    echo "Installed version:"
    $(which linthis) --version
}

cmd_check() {
    echo "Running clippy..."
    cargo clippy --all-targets --all-features -- -D warnings
    echo ""
    echo "Checking format..."
    cargo fmt --check
    echo ""
    echo "Running tests..."
    cargo test
    echo ""
    echo "All checks passed!"
}

cmd_fix() {
    echo "Fixing clippy warnings..."
    cargo clippy --fix --allow-dirty --allow-staged
    echo ""
    echo "Formatting code..."
    cargo fmt
    echo ""
    echo "All fixes applied!"
}

cmd_test() {
    echo "Running tests..."
    cargo test
}

cmd_all() {
    cmd_check
    echo ""
    cmd_install
}

case "${1:-}" in
    build)
        cmd_build
        ;;
    install)
        cmd_install
        ;;
    check)
        cmd_check
        ;;
    fix)
        cmd_fix
        ;;
    test)
        cmd_test
        ;;
    all)
        cmd_all
        ;;
    -h|--help|help|"")
        usage
        ;;
    *)
        echo "Unknown command: $1"
        echo ""
        usage
        exit 1
        ;;
esac
