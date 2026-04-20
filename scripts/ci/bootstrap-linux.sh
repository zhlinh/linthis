#!/usr/bin/env bash
# Bootstrap Ubuntu runner with the pre-reqs that linthis cannot auto-install
# itself. Keep this script MINIMAL — every tool listed in TOOL_INSTALLS (ruff,
# stylua, shellcheck, cpplint, etc.) must be installed by linthis itself so
# the CI test is meaningful.
#
# Bootstrap NEVER installs tools that are listed in TOOL_INSTALLS.

set -euo pipefail

export DEBIAN_FRONTEND=noninteractive
sudo apt-get update -y
sudo apt-get install -y --no-install-recommends \
  ca-certificates \
  curl \
  build-essential \
  golang-go \
  composer \
  ruby-full \
  default-jdk \
  luarocks \
  unzip

# Ensure Go's bin directory is on PATH for subsequently-installed Go tools.
echo "$HOME/go/bin" >> "$GITHUB_PATH"

# uv (Python package + tool manager)
curl -LsSf https://astral.sh/uv/install.sh | sh
echo "$HOME/.local/bin" >> "$GITHUB_PATH"

# pipx (alternative Python tool manager)
python3 -m pip install --user pipx
python3 -m pipx ensurepath || true

# coursier (for Scala tools)
curl -fLo cs https://github.com/coursier/launchers/raw/master/cs-x86_64-pc-linux
chmod +x cs
sudo mv cs /usr/local/bin/cs
cs setup --yes

# .NET SDK (for dotnet-format)
curl -LsSf https://dot.net/v1/dotnet-install.sh -o dotnet-install.sh
chmod +x dotnet-install.sh
./dotnet-install.sh --channel 8.0 --install-dir "$HOME/.dotnet"
echo "$HOME/.dotnet" >> "$GITHUB_PATH"
echo "DOTNET_ROOT=$HOME/.dotnet" >> "$GITHUB_ENV"
rm dotnet-install.sh

echo "--- linux bootstrap complete ---"
