#!/bin/bash
# Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
# Use of this source code is governed by a MIT-style license.
#
# Publish linthis plugin to JetBrains Marketplace
#
# Prerequisites:
#   1. JDK 17+ installed
#   2. JETBRAINS_MARKETPLACE_TOKEN environment variable set
#      Get token from: https://plugins.jetbrains.com/author/me/tokens
#
# For signed releases, also set:
#   - CERTIFICATE_CHAIN: Plugin signing certificate chain
#   - PRIVATE_KEY: Plugin signing private key
#   - PRIVATE_KEY_PASSWORD: Private key password
#
# Usage:
#   ./scripts/publish.sh           # Build, sign, and publish
#   ./scripts/publish.sh --build   # Build only (no publish)
#   ./scripts/publish.sh --dry-run # Build and sign, but don't publish

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
DRY_RUN=false

for arg in "$@"; do
    case $arg in
        --build)
            BUILD_ONLY=true
            ;;
        --dry-run)
            DRY_RUN=true
            ;;
        --help|-h)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --build     Build only (no publish)"
            echo "  --dry-run   Build and sign, but don't publish"
            echo "  --help, -h  Show this help message"
            echo ""
            echo "Environment variables:"
            echo "  JETBRAINS_MARKETPLACE_TOKEN  Required for publishing"
            echo "  CERTIFICATE_CHAIN            Optional: for signed releases"
            echo "  PRIVATE_KEY                  Optional: for signed releases"
            echo "  PRIVATE_KEY_PASSWORD         Optional: for signed releases"
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

if ! command -v java &> /dev/null; then
    log_error "Java is not installed. Please install JDK 17+."
    exit 1
fi

JAVA_VERSION=$(java -version 2>&1 | head -n 1 | cut -d'"' -f2 | cut -d'.' -f1)
if [ "$JAVA_VERSION" -lt 17 ]; then
    log_error "Java 17+ is required. Current version: $JAVA_VERSION"
    exit 1
fi

log_info "Java version: $(java -version 2>&1 | head -n 1)"

# Get version from build.gradle.kts
VERSION=$(grep 'version = ' build.gradle.kts | head -1 | sed 's/.*"\(.*\)".*/\1/')
log_info "Plugin version: $VERSION"

# Clean previous builds
log_info "Cleaning previous builds..."
./gradlew clean

# Build the plugin
log_info "Building plugin..."
./gradlew buildPlugin

# Verify build output
BUILD_OUTPUT="build/distributions"
if [ ! -d "$BUILD_OUTPUT" ]; then
    log_error "Build output directory not found: $BUILD_OUTPUT"
    exit 1
fi

ZIP_FILE=$(ls "$BUILD_OUTPUT"/*.zip 2>/dev/null | head -1)
if [ -z "$ZIP_FILE" ]; then
    log_error "No plugin ZIP file found in $BUILD_OUTPUT"
    exit 1
fi

log_info "Built: $ZIP_FILE"

if [ "$BUILD_ONLY" = true ]; then
    log_info "Build completed (--build flag specified, skipping publish)"
    exit 0
fi

# Sign the plugin (if credentials are available)
if [ -n "$CERTIFICATE_CHAIN" ] && [ -n "$PRIVATE_KEY" ]; then
    log_info "Signing plugin..."
    ./gradlew signPlugin

    SIGNED_FILE=$(ls "$BUILD_OUTPUT"/*-signed.zip 2>/dev/null | head -1)
    if [ -n "$SIGNED_FILE" ]; then
        log_info "Signed: $SIGNED_FILE"
    fi
else
    log_warn "Signing credentials not found. Plugin will be unsigned."
    log_warn "Set CERTIFICATE_CHAIN, PRIVATE_KEY, and PRIVATE_KEY_PASSWORD for signed releases."
fi

if [ "$DRY_RUN" = true ]; then
    log_info "Dry run completed (--dry-run flag specified, skipping publish)"
    exit 0
fi

# Publish to JetBrains Marketplace
if [ -z "$JETBRAINS_MARKETPLACE_TOKEN" ]; then
    log_error "JETBRAINS_MARKETPLACE_TOKEN is not set."
    log_error "Get your token from: https://plugins.jetbrains.com/author/me/tokens"
    exit 1
fi

log_info "Publishing to JetBrains Marketplace..."
./gradlew publishPlugin

log_info "Successfully published linthis plugin v$VERSION to JetBrains Marketplace!"
log_info "View at: https://plugins.jetbrains.com/plugin/29860-linthis"
