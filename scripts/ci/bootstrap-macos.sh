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

# Bin dirs for the tools linthis installs during the test step.
echo "$HOME/.dotnet/tools" >> "$GITHUB_PATH"
echo "$HOME/.luarocks/bin" >> "$GITHUB_PATH"
echo "$HOME/Library/Application Support/Coursier/bin" >> "$GITHUB_PATH"
echo "$(composer config -g home 2>/dev/null || echo "$HOME/.composer")/vendor/bin" >> "$GITHUB_PATH"
echo "$(ruby -e 'puts Gem.user_dir')/bin" >> "$GITHUB_PATH"

echo "--- macos bootstrap complete ---"
