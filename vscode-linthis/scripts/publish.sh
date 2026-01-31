#!/bin/bash
# Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
# Use of this source code is governed by a MIT-style license.
#
# Publish linthis extension to VS Code Marketplace
#
# Prerequisites:
#   1. Node.js 18+ installed
#   2. npm installed
#   3. vsce installed (npm install -g @vscode/vsce)
#   4. VSCE_PAT environment variable set
#      Get token from: https://dev.azure.com/<org>/_usersSettings/tokens
#      (needs Marketplace > Manage scope)
#
# Usage:
#   ./scripts/publish.sh              # Build and publish
#   ./scripts/publish.sh --build      # Build only (no publish)
#   ./scripts/publish.sh --package    # Build and package VSIX (no publish)
#   ./scripts/publish.sh --dry-run    # Build and show what would be published

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

cd "$PROJECT_DIR"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Parse arguments
BUILD_ONLY=false
PACKAGE_ONLY=false
DRY_RUN=false

for arg in "$@"; do
    case $arg in
        --build)
            BUILD_ONLY=true
            ;;
        --package)
            PACKAGE_ONLY=true
            ;;
        --dry-run)
            DRY_RUN=true
            ;;
        --help|-h)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --build     Build only (no package or publish)"
            echo "  --package   Build and package VSIX (no publish)"
            echo "  --dry-run   Build and show what would be published"
            echo "  --help, -h  Show this help message"
            echo ""
            echo "Environment variables:"
            echo "  VSCE_PAT    Personal Access Token for VS Code Marketplace"
            exit 0
            ;;
        *)
            log_error "Unknown option: $arg"
            exit 1
            ;;
    esac
done

# Check prerequisites
log_info "Checking prerequisites..."

if ! command -v node &> /dev/null; then
    log_error "Node.js is not installed. Please install Node.js 18+."
    exit 1
fi

NODE_VERSION=$(node -v | sed 's/v//' | cut -d'.' -f1)
if [ "$NODE_VERSION" -lt 18 ]; then
    log_error "Node.js 18+ is required. Current version: $(node -v)"
    exit 1
fi

log_info "Node.js version: $(node -v)"

if ! command -v npm &> /dev/null; then
    log_error "npm is not installed."
    exit 1
fi

log_info "npm version: $(npm -v)"

# Install vsce if not present
if ! command -v vsce &> /dev/null; then
    log_warn "vsce not found. Installing @vscode/vsce globally..."
    npm install -g @vscode/vsce
fi

log_info "vsce version: $(vsce --version)"

# Get version from package.json
VERSION=$(node -p "require('./package.json').version")
NAME=$(node -p "require('./package.json').name")
log_info "Extension: $NAME v$VERSION"

# Install dependencies
log_info "Installing dependencies..."
npm ci

# Build the extension
log_info "Building extension..."
npm run build

if [ "$BUILD_ONLY" = true ]; then
    log_info "Build completed (--build flag specified, skipping package)"
    exit 0
fi

# Clean previous VSIX files
rm -f *.vsix

# Package the extension
log_info "Packaging extension..."
vsce package

VSIX_FILE=$(ls *.vsix 2>/dev/null | head -1)
if [ -z "$VSIX_FILE" ]; then
    log_error "No VSIX file generated"
    exit 1
fi

log_info "Packaged: $VSIX_FILE"

if [ "$PACKAGE_ONLY" = true ]; then
    log_info "Package completed (--package flag specified, skipping publish)"
    log_info "To install locally: code --install-extension $VSIX_FILE"
    exit 0
fi

if [ "$DRY_RUN" = true ]; then
    log_info "Dry run - would publish the following:"
    vsce show "$NAME" --json 2>/dev/null || log_warn "Extension not yet published"
    log_info "VSIX file: $VSIX_FILE"
    log_info "Dry run completed (--dry-run flag specified, skipping publish)"
    exit 0
fi

# Publish to VS Code Marketplace
if [ -z "$VSCE_PAT" ]; then
    log_error "VSCE_PAT is not set."
    log_error "Get your Personal Access Token from Azure DevOps:"
    log_error "  https://dev.azure.com/<org>/_usersSettings/tokens"
    log_error "Token needs 'Marketplace > Manage' scope."
    exit 1
fi

log_info "Publishing to VS Code Marketplace..."
vsce publish -p "$VSCE_PAT"

log_info "Successfully published $NAME v$VERSION to VS Code Marketplace!"
log_info "View at: https://marketplace.visualstudio.com/items?itemName=linthis.$NAME"
