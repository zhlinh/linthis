#!/usr/bin/env bash
# Bootstrap Windows runner (runs under Git Bash via `shell: bash`).
# Hosted windows-latest runners ship with choco, go, ruby, composer, .NET SDK.
# We top up scoop, uv, pipx and coursier only.
#
# Bootstrap NEVER installs tools that are listed in TOOL_INSTALLS.

set -euo pipefail

# Scoop is not shipped on the hosted image; install it if missing. Its shims
# land in %USERPROFILE%\scoop\shims.
if ! command -v scoop >/dev/null 2>&1; then
	powershell -Command "Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser -Force; iwr -useb get.scoop.sh | iex"
fi

# The scoop installer only updates the PowerShell session + the registry, neither
# of which this bash session or the runner's step PATH sees. Note `$HOME` here is
# MSYS-style (`/c/Users/runneradmin`), so GITHUB_PATH entries must use the
# Windows-style `$USERPROFILE` or the test step's `cmd` will never resolve them.
export PATH="$USERPROFILE/scoop/shims:$PATH"
echo "$USERPROFILE/scoop/shims" >>"$GITHUB_PATH"

# uv (Python tool manager) — installs to %USERPROFILE%\.local\bin.
powershell -Command "irm https://astral.sh/uv/install.ps1 | iex"
echo "$USERPROFILE/.local/bin" >>"$GITHUB_PATH"

# pipx (alternative Python tool manager)
python -m pip install --user pipx
python -m pipx ensurepath || true

# coursier (for Scala tools). scoop's coursier manifest ships `coursier.jar`,
# which provides neither a `cs` nor a `coursier` command, so download the native
# launcher directly — mirrors bootstrap-linux.sh.
powershell -Command 'New-Item -ItemType Directory -Force -Path "$env:LOCALAPPDATA\coursier" | Out-Null; Invoke-WebRequest -UseBasicParsing -Uri https://github.com/coursier/launchers/raw/master/cs-x86_64-pc-win32.zip -OutFile "$env:TEMP\cs-x86_64-pc-win32.zip"; Expand-Archive -Path "$env:TEMP\cs-x86_64-pc-win32.zip" -DestinationPath "$env:LOCALAPPDATA\coursier" -Force; Rename-Item -Path "$env:LOCALAPPDATA\coursier\cs-x86_64-pc-win32.exe" -NewName cs.exe -Force'
echo "$LOCALAPPDATA/coursier" >>"$GITHUB_PATH"

# Bin dirs for the tools linthis installs during the test step. GITHUB_PATH
# applies to later steps, so exporting them here is enough — the tools do not
# have to exist yet.
echo "$USERPROFILE/.dotnet/tools" >>"$GITHUB_PATH"
echo "$APPDATA/Composer/vendor/bin" >>"$GITHUB_PATH"
echo "$LOCALAPPDATA/Coursier/data/bin" >>"$GITHUB_PATH"
echo "$USERPROFILE/.luarocks/bin" >>"$GITHUB_PATH"

echo "--- windows bootstrap complete ---"
