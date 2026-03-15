#!/usr/bin/env bash
# install.sh - linthis installation script
# Supports macOS / Linux
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/zhlinh/linthis/main/install.sh | bash
#   bash install.sh [-y]      # -y: use defaults for all prompts, no interaction (CI-friendly)

set -e

# Color definitions
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

info()    { echo -e "${BLUE}[INFO]${NC} $*"; }
success() { echo -e "${GREEN}[OK]${NC} $*"; }
warn()    { echo -e "${YELLOW}[WARN]${NC} $*"; }
error()   { echo -e "${RED}[ERROR]${NC} $*"; }

# -y / --yes: use defaults for all prompts, no interaction
YES_ALL=false
for _arg in "$@"; do
    [[ "$_arg" == "-y" || "$_arg" == "--yes" ]] && YES_ALL=true
done

ask_yes_no() {
    local prompt="$1"
    local default="${2:-y}"
    local hint
    if [[ "$default" == "y" ]]; then
        hint="[Y/n]"
    else
        hint="[y/N]"
    fi
    if $YES_ALL; then
        echo -e "${CYAN}$prompt ${hint}:${NC} (auto: $default)"
        [[ "$default" == "y" ]] && return 0 || return 1
    fi
    while true; do
        read -r -p "$(echo -e "${CYAN}$prompt ${hint}:${NC} ")" answer </dev/tty
        answer="${answer:-$default}"
        case "$answer" in
            [Yy]*) return 0 ;;
            [Nn]*) return 1 ;;
            *) echo "Please enter y or n" ;;
        esac
    done
}

echo ""
echo -e "${GREEN}============================================${NC}"
echo -e "${GREEN}   linthis Installation Script${NC}"
echo -e "${GREEN}============================================${NC}"
echo ""

# ============================================================
# Step 1: Check and install Python environment and linthis
# ============================================================
info "Step 1/5: Checking Python environment and installing linthis..."

HAS_UV=false
HAS_PIP=false
INSTALL_CMD=""

if command -v uv &>/dev/null; then
    HAS_UV=true
    success "Detected uv: $(uv --version)"
fi

if command -v pip3 &>/dev/null; then
    HAS_PIP=true
    success "Detected pip3"
elif command -v pip &>/dev/null; then
    HAS_PIP=true
    success "Detected pip"
fi

if ! $HAS_UV && ! $HAS_PIP; then
    warn "pip or uv not found, attempting to install Python environment..."

    if [[ "$(uname)" == "Darwin" ]]; then
        if command -v brew &>/dev/null; then
            info "Installing Python via Homebrew..."
            brew install python3
        else
            error "Homebrew not found. Please install Homebrew (https://brew.sh) or Python 3 manually."
            exit 1
        fi
    elif command -v apt-get &>/dev/null; then
        info "Installing Python via apt..."
        sudo apt-get update && sudo apt-get install -y python3 python3-pip
    elif command -v yum &>/dev/null; then
        info "Installing Python via yum..."
        sudo yum install -y python3 python3-pip
    elif command -v dnf &>/dev/null; then
        info "Installing Python via dnf..."
        sudo dnf install -y python3 python3-pip
    else
        error "Cannot auto-install Python. Please install Python 3 and pip manually."
        exit 1
    fi

    if command -v pip3 &>/dev/null; then
        HAS_PIP=true
    elif command -v pip &>/dev/null; then
        HAS_PIP=true
    else
        error "pip still not found after Python installation. Please install manually."
        exit 1
    fi
fi

# Select install tool
if $HAS_UV; then
    INSTALL_CMD="uv pip install --upgrade"
elif command -v pip3 &>/dev/null; then
    INSTALL_CMD="pip3 install --upgrade"
else
    INSTALL_CMD="pip install --upgrade"
fi

# Install linthis
if command -v linthis &>/dev/null; then
    _installed_ver=$(linthis --version 2>/dev/null | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' | head -1)
    success "linthis already installed: ${_installed_ver:-installed}"
    info "Checking for latest version on PyPI..."
    _latest_ver=$(curl -fsSL --max-time 5 https://pypi.org/pypi/linthis/json 2>/dev/null \
        | grep -oE '"version":"[0-9]+\.[0-9]+\.[0-9]+"' | head -1 | grep -oE '[0-9]+\.[0-9]+\.[0-9]+')
    if [[ -n "$_latest_ver" && "$_installed_ver" == "$_latest_ver" ]]; then
        success "Already at latest version, skipping upgrade"
    else
        if [[ -n "$_latest_ver" ]]; then
            info "New version available: ${_installed_ver:-unknown} -> $_latest_ver"
        fi
        if ask_yes_no "Upgrade linthis?" "y"; then
            info "Running: $INSTALL_CMD linthis..."
            $INSTALL_CMD linthis
        fi
    fi
else
    info "Running: $INSTALL_CMD linthis..."
    $INSTALL_CMD linthis
fi

# Verify installation
if ! command -v linthis &>/dev/null; then
    error "linthis installation failed. Check your PATH or run manually: $INSTALL_CMD linthis"
    exit 1
fi
success "linthis installed successfully"
echo ""

# ============================================================
# Step 2: Install linthis plugin
# ============================================================
info "Step 2/5: Install linthis plugin..."

DEFAULT_PLUGIN_URL="https://github.com/zhlinh/linthis-plugin-template"
DEFAULT_PLUGIN_NAME="template"

echo "  Default plugin: $DEFAULT_PLUGIN_URL"
echo ""
echo "  1. Install default plugin (recommended)"
echo "  2. Install a custom plugin (enter URL)"
echo "  0. Skip"
echo ""

if $YES_ALL; then
    plugin_choice="1"
    info "Plugin install: auto -> 1 (default plugin)"
else
    read -r -p "$(echo -e "${CYAN}Select plugin option [1/2/0, default 1]:${NC} ")" plugin_choice </dev/tty
    [[ -z "$plugin_choice" ]] && plugin_choice="1"
fi

PLUGIN_URL=""
PLUGIN_NAME=""

if [[ "$plugin_choice" == "1" ]]; then
    PLUGIN_URL="$DEFAULT_PLUGIN_URL"
    PLUGIN_NAME="$DEFAULT_PLUGIN_NAME"
elif [[ "$plugin_choice" == "2" ]]; then
    if $YES_ALL; then
        PLUGIN_URL="$DEFAULT_PLUGIN_URL"
        PLUGIN_NAME="$DEFAULT_PLUGIN_NAME"
        info "Custom plugin URL: auto -> $DEFAULT_PLUGIN_URL"
    else
        read -r -p "$(echo -e "${CYAN}Enter plugin Git URL:${NC} ")" PLUGIN_URL </dev/tty
        read -r -p "$(echo -e "${CYAN}Enter plugin name (alias):${NC} ")" PLUGIN_NAME </dev/tty
        [[ -z "$PLUGIN_URL" ]] && { warn "No URL provided, skipping plugin install"; PLUGIN_URL=""; }
        [[ -z "$PLUGIN_NAME" ]] && PLUGIN_NAME="custom"
    fi
else
    info "Skipping plugin installation"
    info "You can install later with: linthis plugin add -g <name> <git-url>"
fi

if [[ -n "$PLUGIN_URL" ]]; then
    if linthis plugin list -g 2>/dev/null | grep -q "$PLUGIN_NAME"; then
        info "Plugin '$PLUGIN_NAME' already exists, syncing updates..."
        if linthis plugin sync -g; then
            success "Plugin '$PLUGIN_NAME' updated (global)"
        else
            warn "Plugin update failed. Run manually: linthis plugin sync -g"
        fi
    elif linthis plugin add -g "$PLUGIN_NAME" "$PLUGIN_URL"; then
        success "Plugin '$PLUGIN_NAME' installed (global)"
    else
        warn "Plugin installation failed. Check network or run manually:"
        warn "  linthis plugin add -g $PLUGIN_NAME $PLUGIN_URL"
    fi
fi
echo ""

# ============================================================
# Step 3: Install Git Hooks
# ============================================================
info "Step 3/5: Install Git Hooks..."

# Uninstall all previous global hooks first to ensure a clean state
info "Uninstalling all previous global hooks for a clean state..."
linthis hook uninstall -g --all -y 2>/dev/null || true
echo ""

echo "  1. git            - lint check only"
echo "  2. git-with-agent - lint check + AI auto-fix (default)"
echo "  0. Skip"
echo ""
if $YES_ALL; then
    hook_type_choice="2"
    info "Git Hooks type: auto -> git-with-agent"
else
    read -r -p "$(echo -e "${CYAN}Select Git Hooks type [1/2/0, default 2]:${NC} ")" hook_type_choice </dev/tty
    [[ -z "$hook_type_choice" ]] && hook_type_choice="2"
fi

if [[ "$hook_type_choice" == "1" ]]; then
    HOOK_TYPE="git"
elif [[ "$hook_type_choice" == "2" ]]; then
    HOOK_TYPE="git-with-agent"
else
    HOOK_TYPE=""
fi

HOOK_EVENTS=""

if [[ -n "$HOOK_TYPE" ]]; then
    echo ""
    echo -e "\033[2m  Selected type: --type $HOOK_TYPE\033[0m"
    echo ""
    echo "  1. pre-commit  - auto lint on every git commit"
    echo "  2. commit-msg  - check commit message format (Conventional Commits)"
    echo "  3. pre-push    - auto lint before git push"
    echo ""
    if $YES_ALL; then
        hook_choice="1 2 3"
        info "Hook events: auto -> 1 2 3 (pre-commit commit-msg pre-push)"
    else
        read -r -p "$(echo -e "${CYAN}Select hook events [1/2/3, default 1 2 3]:${NC} ")" hook_choice </dev/tty
        [[ -z "$hook_choice" ]] && hook_choice="1 2 3"
    fi

    [[ "$hook_choice" == *"1"* ]] && HOOK_EVENTS="$HOOK_EVENTS --event pre-commit"
    [[ "$hook_choice" == *"2"* ]] && HOOK_EVENTS="$HOOK_EVENTS --event commit-msg"
    [[ "$hook_choice" == *"3"* ]] && HOOK_EVENTS="$HOOK_EVENTS --event pre-push"

    if [[ -n "$HOOK_EVENTS" ]]; then
        # shellcheck disable=SC2086
        if linthis hook install -g --type $HOOK_TYPE $HOOK_EVENTS -y; then
            success "Global Git Hooks installed (--type $HOOK_TYPE)"
        else
            warn "Git Hooks installation failed"
        fi
    else
        info "Skipping Git Hooks installation"
        info "You can run later: linthis hook install -g --type $HOOK_TYPE --event pre-commit --event commit-msg --event pre-push"
    fi
else
    info "Skipping Git Hooks installation"
    info "You can run later: linthis hook install -g --type git-with-agent --event pre-commit --event commit-msg --event pre-push"
fi

echo ""

# ============================================================
# Agent hook: detect available providers and let user choose
# ============================================================

# Detect available AI agent providers
AVAILABLE_PROVIDERS=()
command -v claude     &>/dev/null && AVAILABLE_PROVIDERS+=("claude")
command -v codebuddy  &>/dev/null && AVAILABLE_PROVIDERS+=("codebuddy")
command -v cursor     &>/dev/null && AVAILABLE_PROVIDERS+=("cursor")
command -v copilot    &>/dev/null && AVAILABLE_PROVIDERS+=("copilot")
# Also add claude if not detected but user may have it configured
if [[ ${#AVAILABLE_PROVIDERS[@]} -eq 0 ]]; then
    AVAILABLE_PROVIDERS+=("claude")  # fallback default
fi

if ask_yes_no "Install global agent hook for AI code review?" "y"; then

    # Display detected providers
    echo ""
    echo "  Detected providers:"
    _idx=1
    _provider_map=()
    for _p in "${AVAILABLE_PROVIDERS[@]}"; do
        if [[ "$_p" == "claude" ]]; then
            echo "  $_idx. claude     (Claude Code - default)"
        else
            echo "  $_idx. $_p"
        fi
        _provider_map+=("$_p")
        ((_idx++))
    done
    # Always offer manual entry
    echo "  $_idx. Other (enter manually)"
    echo ""

    SELECTED_PROVIDER=""
    # Find default index (claude = 1 if available, else 1)
    _default_provider_idx=1
    for i in "${!_provider_map[@]}"; do
        if [[ "${_provider_map[$i]}" == "claude" ]]; then
            _default_provider_idx=$(( i + 1 ))
            break
        fi
    done

    if $YES_ALL; then
        SELECTED_PROVIDER="claude"
        info "Agent provider: auto -> claude"
    else
        read -r -p "$(echo -e "${CYAN}Select provider [default $_default_provider_idx]:${NC} ")" _prov_choice </dev/tty
        [[ -z "$_prov_choice" ]] && _prov_choice="$_default_provider_idx"

        if [[ "$_prov_choice" == "$_idx" ]]; then
            read -r -p "$(echo -e "${CYAN}Enter provider name:${NC} ")" SELECTED_PROVIDER </dev/tty
        elif [[ "$_prov_choice" =~ ^[0-9]+$ ]] && (( _prov_choice >= 1 && _prov_choice < _idx )); then
            SELECTED_PROVIDER="${_provider_map[$(( _prov_choice - 1 ))]}"
        else
            warn "Invalid selection, defaulting to claude"
            SELECTED_PROVIDER="claude"
        fi
    fi

    if [[ -z "$SELECTED_PROVIDER" ]]; then
        warn "No provider selected, defaulting to claude"
        SELECTED_PROVIDER="claude"
    fi

    # Model selection
    SELECTED_MODEL=""
    echo ""
    if [[ "$SELECTED_PROVIDER" == "claude" ]]; then
        echo "  Available Claude models:"
        echo "  1. claude-sonnet-4-5  (recommended, balanced)"
        echo "  2. claude-opus-4-5    (most capable)"
        echo "  3. claude-haiku-4-5   (fastest)"
        echo "  0. Skip (use provider default)"
        echo ""
        if $YES_ALL; then
            SELECTED_MODEL="claude-sonnet-4-5"
            info "Model: auto -> claude-sonnet-4-5"
        else
            read -r -p "$(echo -e "${CYAN}Select model [1/2/3/0, default 1]:${NC} ")" _model_choice </dev/tty
            [[ -z "$_model_choice" ]] && _model_choice="1"
            case "$_model_choice" in
                1) SELECTED_MODEL="claude-sonnet-4-5" ;;
                2) SELECTED_MODEL="claude-opus-4-5" ;;
                3) SELECTED_MODEL="claude-haiku-4-5" ;;
                0) SELECTED_MODEL="" ;;
                *) SELECTED_MODEL="claude-sonnet-4-5" ;;
            esac
        fi
    else
        echo "  Enter model name for provider '$SELECTED_PROVIDER' (leave blank for default):"
        if $YES_ALL; then
            SELECTED_MODEL=""
            info "Model: auto -> (provider default)"
        else
            read -r -p "$(echo -e "${CYAN}Model name [leave blank for default]:${NC} ")" SELECTED_MODEL </dev/tty
        fi
    fi

    # Select hook events for agent
    _agent_default=""
    [[ "$HOOK_EVENTS" == *"pre-commit"* ]] && _agent_default="${_agent_default}1 "
    [[ "$HOOK_EVENTS" == *"commit-msg"* ]] && _agent_default="${_agent_default}2 "
    [[ "$HOOK_EVENTS" == *"pre-push"*   ]] && _agent_default="${_agent_default}3 "
    _agent_default="${_agent_default%% }"
    [[ -z "$_agent_default" ]] && _agent_default="1 2 3"

    echo ""
    echo -e "\033[2m  Selected: --type agent --provider $SELECTED_PROVIDER${SELECTED_MODEL:+ --model $SELECTED_MODEL}\033[0m"
    echo ""
    echo "  1. pre-commit  - AI code review on every git commit"
    echo "  2. commit-msg  - check commit message format"
    echo "  3. pre-push    - AI code review before git push"
    echo ""
    if $YES_ALL; then
        agent_choice="$_agent_default"
        info "Agent hook events: auto -> $_agent_default"
    else
        read -r -p "$(echo -e "${CYAN}Select hook events [1/2/3, default $_agent_default]:${NC} ")" agent_choice </dev/tty
        [[ -z "$agent_choice" ]] && agent_choice="$_agent_default"
    fi

    AGENT_EVENTS=""
    [[ "$agent_choice" == *"1"* ]] && AGENT_EVENTS="$AGENT_EVENTS --event pre-commit"
    [[ "$agent_choice" == *"2"* ]] && AGENT_EVENTS="$AGENT_EVENTS --event commit-msg"
    [[ "$agent_choice" == *"3"* ]] && AGENT_EVENTS="$AGENT_EVENTS --event pre-push"

    if [[ -n "$AGENT_EVENTS" ]]; then
        MODEL_FLAG=""
        [[ -n "$SELECTED_MODEL" ]] && MODEL_FLAG="--model $SELECTED_MODEL"
        # shellcheck disable=SC2086
        if linthis hook install -g --type agent --provider "$SELECTED_PROVIDER" $MODEL_FLAG $AGENT_EVENTS -y; then
            success "Global agent hook installed (provider: $SELECTED_PROVIDER${SELECTED_MODEL:+, model: $SELECTED_MODEL})"
        else
            warn "Agent hook installation failed"
        fi
    else
        info "Skipping agent hook installation"
    fi
else
    info "Skipping agent hook installation"
    info "You can run later: linthis hook install -g --type agent --provider claude --event pre-commit --event commit-msg --event pre-push"
fi
echo ""

# ============================================================
# Step 4: Configure shell aliases
# ============================================================
info "Step 4/5: Configure shell aliases..."

if ask_yes_no "Add common aliases? (lt=linthis, lts=linthis -s, ltm=linthis -m)" "y"; then
    # Detect shell config file
    SHELL_RC=""
    if [[ -n "$ZSH_VERSION" ]] || [[ "$SHELL" == *"zsh"* ]]; then
        SHELL_RC="$HOME/.zshrc"
    elif [[ -n "$BASH_VERSION" ]] || [[ "$SHELL" == *"bash"* ]]; then
        SHELL_RC="$HOME/.bashrc"
    fi

    if [[ -z "$SHELL_RC" ]]; then
        SHELL_RC="$HOME/.bashrc"
        warn "Could not determine shell type, defaulting to $SHELL_RC"
    fi

    ALIAS_BLOCK='
# linthis aliases
alias lt="linthis"
alias lts="linthis -s"
alias ltm="linthis -m"'

    if grep -q 'alias lt="linthis"' "$SHELL_RC" 2>/dev/null; then
        success "Aliases already exist in $SHELL_RC"
    else
        echo "$ALIAS_BLOCK" >> "$SHELL_RC"
        success "Aliases added to $SHELL_RC"
        info "Run: source $SHELL_RC  (or reopen your terminal)"
    fi
else
    info "Skipping alias configuration"
    info "You can add these aliases manually:"
    echo '  alias lt="linthis"'
    echo '  alias lts="linthis -s"'
    echo '  alias ltm="linthis -m"'
fi
echo ""

# ============================================================
# Step 5: Dependency check
# ============================================================
info "Step 5/5: Running linthis doctor to check dependencies..."

linthis doctor 2>&1 || true

echo ""
info "Depending on your project type, you may need additional lint tool dependencies:"
echo ""
echo "  [1] Android (Java/Kotlin)        - checkstyle, detekt"
echo "  [2] iOS (OC/C++/Swift)           - clang-format, clang-tidy, cpplint, swiftlint"
echo "  [3] Ohos/HarmonyOS (TypeScript)  - node, eslint, typescript, @typescript-eslint"
echo "  [4] All three"
echo "  [5] Skip"
echo ""
if $YES_ALL; then
    dep_choice="5"
    info "Dependency install: auto -> 5 (skip)"
else
    read -r -p "$(echo -e "${CYAN}Select [1/2/3/4/5, default 5 skip]:${NC} ")" dep_choice </dev/tty
fi

install_android_deps() {
    info "Installing Android lint dependencies..."

    if [[ "$(uname)" == "Darwin" ]]; then
        if command -v brew &>/dev/null; then
            if ! command -v checkstyle &>/dev/null; then
                info "Installing checkstyle..."
                brew install checkstyle
            else
                success "checkstyle already installed"
            fi
            if ! command -v detekt &>/dev/null; then
                info "Installing detekt..."
                brew install detekt
            else
                success "detekt already installed"
            fi
        else
            warn "Homebrew not found. Please install checkstyle and detekt manually."
        fi
    else
        if ! command -v java &>/dev/null; then
            warn "Java not found. checkstyle and detekt require a Java runtime."
            if command -v apt-get &>/dev/null; then
                info "Installing OpenJDK..."
                sudo apt-get install -y default-jdk
            elif command -v yum &>/dev/null; then
                sudo yum install -y java-17-openjdk
            elif command -v dnf &>/dev/null; then
                sudo dnf install -y java-17-openjdk
            fi
        fi

        if ! command -v checkstyle &>/dev/null; then
            info "Installing checkstyle..."
            CHECKSTYLE_VERSION="10.21.4"
            CHECKSTYLE_DIR="$HOME/.local/share/checkstyle"
            mkdir -p "$CHECKSTYLE_DIR" "$HOME/.local/bin"
            curl -fsSL "https://github.com/checkstyle/checkstyle/releases/download/checkstyle-${CHECKSTYLE_VERSION}/checkstyle-${CHECKSTYLE_VERSION}-all.jar" \
                -o "$CHECKSTYLE_DIR/checkstyle.jar"
            cat > "$HOME/.local/bin/checkstyle" <<'WRAPPER'
#!/bin/sh
exec java -jar "$HOME/.local/share/checkstyle/checkstyle.jar" "$@"
WRAPPER
            chmod +x "$HOME/.local/bin/checkstyle"
            success "checkstyle installed to ~/.local/bin/checkstyle"
        else
            success "checkstyle already installed"
        fi

        if ! command -v detekt &>/dev/null; then
            info "Installing detekt..."
            DETEKT_VERSION="1.23.7"
            mkdir -p "$HOME/.local/bin"
            curl -fsSL "https://github.com/detekt/detekt/releases/download/v${DETEKT_VERSION}/detekt-cli-${DETEKT_VERSION}-all.jar" \
                -o "$HOME/.local/share/detekt-cli.jar"
            cat > "$HOME/.local/bin/detekt" <<'WRAPPER'
#!/bin/sh
exec java -jar "$HOME/.local/share/detekt-cli.jar" "$@"
WRAPPER
            chmod +x "$HOME/.local/bin/detekt"
            success "detekt installed to ~/.local/bin/detekt"
        else
            success "detekt already installed"
        fi
    fi
}

install_ios_deps() {
    info "Installing iOS lint dependencies..."

    if [[ "$(uname)" == "Darwin" ]]; then
        if command -v brew &>/dev/null; then
            if ! command -v clang-format &>/dev/null; then
                info "Installing clang-format (via llvm)..."
                brew install llvm
            else
                success "clang-format already installed"
            fi

            if ! command -v cpplint &>/dev/null; then
                info "Installing cpplint..."
                $INSTALL_CMD cpplint
            else
                success "cpplint already installed"
            fi

            if ! command -v swiftlint &>/dev/null; then
                info "Installing swiftlint..."
                brew install swiftlint
            else
                success "swiftlint already installed"
            fi
        else
            warn "Homebrew not found. Please install clang-format, cpplint, swiftlint manually."
        fi
    else
        if ! command -v clang-format &>/dev/null; then
            info "Installing clang-format..."
            if command -v apt-get &>/dev/null; then
                sudo apt-get install -y clang-format clang-tidy
            elif command -v yum &>/dev/null; then
                sudo yum install -y clang-tools-extra
            elif command -v dnf &>/dev/null; then
                sudo dnf install -y clang-tools-extra
            fi
        else
            success "clang-format already installed"
        fi

        if ! command -v cpplint &>/dev/null; then
            info "Installing cpplint..."
            $INSTALL_CMD cpplint
        else
            success "cpplint already installed"
        fi

        if ! command -v swiftlint &>/dev/null; then
            warn "swiftlint is primarily macOS-only. Compile from source on Linux:"
            warn "  https://github.com/realm/SwiftLint"
        else
            success "swiftlint already installed"
        fi
    fi
}

install_ohos_deps() {
    info "Installing Ohos/HarmonyOS (TypeScript/ArkTS) lint dependencies..."

    # Check Node.js / npm
    if ! command -v node &>/dev/null; then
        warn "Node.js not found. TypeScript lint tools require Node.js."
        if [[ "$(uname)" == "Darwin" ]]; then
            if command -v brew &>/dev/null; then
                info "Installing Node.js via Homebrew..."
                brew install node
            else
                warn "Please install Node.js from https://nodejs.org/"
            fi
        elif command -v apt-get &>/dev/null; then
            info "Installing Node.js via apt..."
            sudo apt-get install -y nodejs npm
        elif command -v yum &>/dev/null; then
            sudo yum install -y nodejs npm
        elif command -v dnf &>/dev/null; then
            sudo dnf install -y nodejs npm
        else
            warn "Please install Node.js from https://nodejs.org/"
        fi
    else
        success "Node.js already installed: $(node --version)"
    fi

    if ! command -v npm &>/dev/null; then
        warn "npm not found. Please install npm alongside Node.js."
        return
    fi

    # eslint
    if ! command -v eslint &>/dev/null; then
        info "Installing eslint globally..."
        npm install -g eslint
    else
        success "eslint already installed: $(eslint --version)"
    fi

    # typescript
    if ! command -v tsc &>/dev/null; then
        info "Installing typescript globally..."
        npm install -g typescript
    else
        success "typescript already installed: $(tsc --version)"
    fi

    # @typescript-eslint plugins
    info "Installing @typescript-eslint/parser and @typescript-eslint/eslint-plugin globally..."
    npm install -g @typescript-eslint/parser @typescript-eslint/eslint-plugin

    success "Ohos/HarmonyOS TypeScript lint dependencies installed"
    info "For HarmonyOS-specific rules, see: https://developer.huawei.com/consumer/en/doc/harmonyos-guides/ide-code-linter"
}

case "$dep_choice" in
    1) install_android_deps ;;
    2) install_ios_deps ;;
    3) install_ohos_deps ;;
    4) install_android_deps; echo ""; install_ios_deps; echo ""; install_ohos_deps ;;
    5|*) info "Skipping dependency installation" ;;
esac

echo ""
echo -e "${GREEN}============================================${NC}"
echo -e "${GREEN}   Installation Complete!${NC}"
echo -e "${GREEN}============================================${NC}"
echo ""
echo "Common commands:"
echo "  linthis -i <path>    lint a specific path"
echo "  linthis -s           lint git staged files"
echo "  linthis -m           lint git modified files"
echo "  linthis doctor       check dependency status"
echo ""
echo "More help: linthis --help"
echo ""
