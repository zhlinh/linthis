#!/usr/bin/env bash
# Bootstrap Windows runner (runs under Git Bash via `shell: bash`).
# Hosted windows-latest runners ship with choco, scoop (if enabled), go,
# ruby, composer, .NET SDK. We top up uv + pipx + coursier only.
#
# Bootstrap NEVER installs tools that are listed in TOOL_INSTALLS.

set -euo pipefail

# Scoop is usually present; if not, install it.
if ! command -v scoop >/dev/null 2>&1; then
  powershell -Command "Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser -Force; iwr -useb get.scoop.sh | iex"
fi

# uv
powershell -Command "irm https://astral.sh/uv/install.ps1 | iex"

# pipx
python -m pip install --user pipx
python -m pipx ensurepath || true

# coursier
scoop install coursier || true

echo "--- windows bootstrap complete ---"
