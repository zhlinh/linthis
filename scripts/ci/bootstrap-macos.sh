#!/usr/bin/env bash
# Bootstrap macOS runner. Hosted macos-14 runners already include brew, go,
# ruby, java. We add the Python/JVM tool managers and .NET SDK only.
#
# Bootstrap NEVER installs tools that are listed in TOOL_INSTALLS.

set -euo pipefail

# Ensure brew is available (should be on hosted runners)
which brew

brew update || true

# Tool managers (uv, pipx) and runtime-level package managers for other langs.
brew install --quiet uv pipx composer luarocks coursier dotnet@8 || true
pipx ensurepath || true

# Make dotnet-format install target discoverable.
echo "$HOME/.dotnet/tools" >> "$GITHUB_PATH"

echo "--- macos bootstrap complete ---"
