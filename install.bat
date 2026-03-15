@echo off
chcp 65001 >nul 2>&1
setlocal EnableDelayedExpansion

:: install.bat - linthis installation script (Windows)
::
:: Usage (CMD):
::   curl -fsSL -o %TEMP%\install.bat https://raw.githubusercontent.com/zhlinh/linthis/main/install.bat && %TEMP%\install.bat
::   %TEMP%\install.bat -y      :: -y: use defaults for all prompts, no interaction (CI-friendly)
::
:: Usage (PowerShell):
::   curl -fsSL -o $env:TEMP\install.bat https://raw.githubusercontent.com/zhlinh/linthis/main/install.bat; & $env:TEMP\install.bat
::   & $env:TEMP\install.bat -y

:: -y / --yes: use defaults for all prompts, no interaction
set "YES_ALL=0"
if /i "%~1"=="-y"    set "YES_ALL=1"
if /i "%~1"=="--yes" set "YES_ALL=1"

echo.
echo ============================================
echo    linthis Installation Script
echo ============================================
echo.

:: ============================================================
:: Step 1: Check and install Python environment and linthis
:: ============================================================
echo [INFO] Step 1/5: Checking Python environment and installing linthis...

set "HAS_UV=0"
set "HAS_PIP=0"
set "INSTALL_CMD="

where uv >nul 2>&1
if %errorlevel% equ 0 (
    set "HAS_UV=1"
    echo [OK] Detected uv
)

where pip >nul 2>&1
if %errorlevel% equ 0 (
    set "HAS_PIP=1"
    echo [OK] Detected pip
) else (
    where pip3 >nul 2>&1
    if %errorlevel% equ 0 (
        set "HAS_PIP=1"
        echo [OK] Detected pip3
    )
)

if "!HAS_UV!"=="0" if "!HAS_PIP!"=="0" (
    echo [WARN] pip or uv not found.
    echo.
    echo Please install Python first:
    echo   Option 1: https://www.python.org/downloads/  ^(check "Add Python to PATH"^)
    echo   Option 2: winget install Python.Python.3.12
    echo   Option 3: choco install python3
    echo.
    if "!YES_ALL!"=="0" pause
    exit /b 1
)

:: Select install tool
if "!HAS_UV!"=="1" (
    set "INSTALL_CMD=uv pip install --upgrade"
) else (
    set "INSTALL_CMD=pip install --upgrade"
)

:: Install linthis
where linthis >nul 2>&1
if %errorlevel% equ 0 (
    for /f "tokens=*" %%v in ('linthis --version 2^>nul') do set "_LINTHIS_VER=%%v"
    echo [OK] linthis already installed: !_LINTHIS_VER!
    echo [INFO] Checking for latest version on PyPI...
    for /f "tokens=*" %%v in ('curl -fsSL --max-time 5 https://pypi.org/pypi/linthis/json 2^>nul ^| python -c "import sys,json; print(json.load(sys.stdin)[\"info\"][\"version\"])" 2^>nul') do set "_LATEST_VER=%%v"
    if "!_LATEST_VER!"=="" (
        if "!YES_ALL!"=="1" (
            set "REINSTALL=y"
            echo [AUTO] Upgrade linthis? -^> y
        ) else (
            set /p "REINSTALL=Upgrade linthis? [Y/n]: "
            if "!REINSTALL!"=="" set "REINSTALL=y"
        )
        if /i "!REINSTALL!"=="y" (
            echo [INFO] Running: !INSTALL_CMD! linthis...
            !INSTALL_CMD! linthis
        )
    ) else if "!_LINTHIS_VER!"=="!_LATEST_VER!" (
        echo [OK] Already at latest version !_LATEST_VER!, skipping upgrade
    ) else (
        echo [INFO] New version available: !_LINTHIS_VER! -^> !_LATEST_VER!
        if "!YES_ALL!"=="1" (
            set "REINSTALL=y"
            echo [AUTO] Upgrade linthis? -^> y
        ) else (
            set /p "REINSTALL=Upgrade linthis? [Y/n]: "
            if "!REINSTALL!"=="" set "REINSTALL=y"
        )
        if /i "!REINSTALL!"=="y" (
            echo [INFO] Running: !INSTALL_CMD! linthis...
            !INSTALL_CMD! linthis
        )
    )
) else (
    echo [INFO] Running: !INSTALL_CMD! linthis...
    !INSTALL_CMD! linthis
)

:: Verify installation
where linthis >nul 2>&1
if %errorlevel% neq 0 (
    echo [ERROR] linthis installation failed. Check your PATH or run manually: !INSTALL_CMD! linthis
    if "!YES_ALL!"=="0" pause
    exit /b 1
)
echo [OK] linthis installed successfully
echo.

:: ============================================================
:: Step 2: Install linthis plugin
:: ============================================================
echo [INFO] Step 2/5: Install linthis plugin...

set "DEFAULT_PLUGIN_URL=https://github.com/zhlinh/linthis-plugin-template"
set "DEFAULT_PLUGIN_NAME=template"

echo   Default plugin: %DEFAULT_PLUGIN_URL%
echo.
echo   1. Install default plugin ^(recommended^)
echo   2. Install a custom plugin ^(enter URL^)
echo   0. Skip
echo.

if "!YES_ALL!"=="1" (
    set "PLUGIN_CHOICE=1"
    echo [AUTO] Plugin install: 1 ^(default plugin^)
) else (
    set /p "PLUGIN_CHOICE=Select plugin option [1/2/0, default 1]: "
    if "!PLUGIN_CHOICE!"=="" set "PLUGIN_CHOICE=1"
)

set "PLUGIN_URL="
set "PLUGIN_NAME="

if "!PLUGIN_CHOICE!"=="1" (
    set "PLUGIN_URL=!DEFAULT_PLUGIN_URL!"
    set "PLUGIN_NAME=!DEFAULT_PLUGIN_NAME!"
) else if "!PLUGIN_CHOICE!"=="2" (
    if "!YES_ALL!"=="1" (
        set "PLUGIN_URL=!DEFAULT_PLUGIN_URL!"
        set "PLUGIN_NAME=!DEFAULT_PLUGIN_NAME!"
        echo [AUTO] Using default plugin URL
    ) else (
        set /p "PLUGIN_URL=Enter plugin Git URL: "
        set /p "PLUGIN_NAME=Enter plugin name ^(alias^): "
        if "!PLUGIN_NAME!"=="" set "PLUGIN_NAME=custom"
    )
) else (
    echo [INFO] Skipping plugin installation
    echo [INFO] You can install later with: linthis plugin add -g ^<name^> ^<git-url^>
)

if not "!PLUGIN_URL!"=="" (
    linthis plugin list -g 2>nul | findstr /c:"!PLUGIN_NAME!" >nul 2>&1
    if %errorlevel% equ 0 (
        echo [INFO] Plugin '!PLUGIN_NAME!' already exists, syncing updates...
        linthis plugin sync -g
        if %errorlevel% equ 0 (
            echo [OK] Plugin '!PLUGIN_NAME!' updated ^(global^)
        ) else (
            echo [WARN] Plugin update failed. Run manually: linthis plugin sync -g
        )
    ) else (
        linthis plugin add -g !PLUGIN_NAME! !PLUGIN_URL!
        if %errorlevel% equ 0 (
            echo [OK] Plugin '!PLUGIN_NAME!' installed ^(global^)
        ) else (
            echo [WARN] Plugin installation failed. Check network or run manually:
            echo        linthis plugin add -g !PLUGIN_NAME! !PLUGIN_URL!
        )
    )
)
echo.

:: ============================================================
:: Step 3: Install Git Hooks
:: ============================================================
echo [INFO] Step 3/5: Install Git Hooks...

:: Uninstall all previous global hooks first to ensure a clean state
echo [INFO] Uninstalling all previous global hooks for a clean state...
linthis hook uninstall -g --all -y 2>nul
echo.

echo   1. git            - lint check only
echo   2. git-with-agent - lint check + AI auto-fix ^(default^)
echo   0. Skip
echo.
if "!YES_ALL!"=="1" (
    set "HOOK_TYPE_CHOICE=2"
    echo [AUTO] Git Hooks type: 2 ^(git-with-agent^)
) else (
    set /p "HOOK_TYPE_CHOICE=Select Git Hooks type [1/2/0, default 2]: "
    if "!HOOK_TYPE_CHOICE!"=="" set "HOOK_TYPE_CHOICE=2"
)

set "HOOK_TYPE="
if "!HOOK_TYPE_CHOICE!"=="1" set "HOOK_TYPE=git"
if "!HOOK_TYPE_CHOICE!"=="2" set "HOOK_TYPE=git-with-agent"

set "HOOK_EVENTS="

if not "!HOOK_TYPE!"=="" (
    echo.
    echo   1. pre-commit  - auto lint on every git commit
    echo   2. commit-msg  - check commit message format ^(Conventional Commits^)
    echo   3. pre-push    - auto lint before git push
    echo.
    if "!YES_ALL!"=="1" (
        set "HOOK_CHOICE=1 2 3"
        echo [AUTO] Hook events: 1 2 3 ^(pre-commit commit-msg pre-push^)
    ) else (
        set /p "HOOK_CHOICE=Select hook events [1/2/3, default 1 2 3]: "
        if "!HOOK_CHOICE!"=="" set "HOOK_CHOICE=1 2 3"
    )

    echo !HOOK_CHOICE! | findstr /c:"1" >nul && set "HOOK_EVENTS=!HOOK_EVENTS! --event pre-commit"
    echo !HOOK_CHOICE! | findstr /c:"2" >nul && set "HOOK_EVENTS=!HOOK_EVENTS! --event commit-msg"
    echo !HOOK_CHOICE! | findstr /c:"3" >nul && set "HOOK_EVENTS=!HOOK_EVENTS! --event pre-push"

    if not "!HOOK_EVENTS!"=="" (
        linthis hook install -g --type !HOOK_TYPE! !HOOK_EVENTS! -y
        if %errorlevel% equ 0 (
            echo [OK] Global Git Hooks installed ^(--type !HOOK_TYPE!^)
        ) else (
            echo [WARN] Git Hooks installation failed
        )
    ) else (
        echo [INFO] Skipping Git Hooks installation
        echo [INFO] You can run later: linthis hook install -g --type !HOOK_TYPE! --event pre-commit --event commit-msg --event pre-push
    )
) else (
    echo [INFO] Skipping Git Hooks installation
    echo [INFO] You can run later: linthis hook install -g --type git-with-agent --event pre-commit --event commit-msg --event pre-push
)
echo.

:: ============================================================
:: Agent hook: detect available providers and let user choose
:: ============================================================

:: Detect available AI agent providers
set "AVAILABLE_PROVIDERS="
set "_PROV_COUNT=0"
set "_HAS_CLAUDE=0"

where claude >nul 2>&1
if %errorlevel% equ 0 (
    set /a "_PROV_COUNT+=1"
    set "_PROV_!_PROV_COUNT!=claude"
    set "_HAS_CLAUDE=1"
)

where codebuddy >nul 2>&1
if %errorlevel% equ 0 (
    set /a "_PROV_COUNT+=1"
    set "_PROV_!_PROV_COUNT!=codebuddy"
)

where cursor >nul 2>&1
if %errorlevel% equ 0 (
    set /a "_PROV_COUNT+=1"
    set "_PROV_!_PROV_COUNT!=cursor"
)

where copilot >nul 2>&1
if %errorlevel% equ 0 (
    set /a "_PROV_COUNT+=1"
    set "_PROV_!_PROV_COUNT!=copilot"
)

:: Fallback: always include claude as option
if "!_PROV_COUNT!"=="0" (
    set /a "_PROV_COUNT+=1"
    set "_PROV_!_PROV_COUNT!=claude"
    set "_HAS_CLAUDE=1"
)

if "!YES_ALL!"=="1" (
    set "INSTALL_AGENT=y"
    echo [AUTO] Install agent hook? -^> y
) else (
    set /p "INSTALL_AGENT=Install global agent hook for AI code review? [Y/n]: "
    if "!INSTALL_AGENT!"=="" set "INSTALL_AGENT=y"
)

if /i "!INSTALL_AGENT!"=="y" (

    echo.
    echo   Detected providers:
    set "_DEFAULT_PROV_IDX=1"
    for /l %%i in (1,1,!_PROV_COUNT!) do (
        if "!_PROV_%%i!"=="claude" (
            echo   %%i. claude     ^(Claude Code - default^)
            set "_DEFAULT_PROV_IDX=%%i"
        ) else (
            echo   %%i. !_PROV_%%i!
        )
    )
    set /a "_OTHER_IDX=_PROV_COUNT+1"
    echo   !_OTHER_IDX!. Other ^(enter manually^)
    echo.

    set "SELECTED_PROVIDER="
    if "!YES_ALL!"=="1" (
        set "SELECTED_PROVIDER=claude"
        echo [AUTO] Agent provider: claude
    ) else (
        set /p "_PROV_CHOICE=Select provider [default !_DEFAULT_PROV_IDX!]: "
        if "!_PROV_CHOICE!"=="" set "_PROV_CHOICE=!_DEFAULT_PROV_IDX!"

        if "!_PROV_CHOICE!"=="!_OTHER_IDX!" (
            set /p "SELECTED_PROVIDER=Enter provider name: "
        ) else (
            for /l %%i in (1,1,!_PROV_COUNT!) do (
                if "!_PROV_CHOICE!"=="%%i" set "SELECTED_PROVIDER=!_PROV_%%i!"
            )
        )
    )

    if "!SELECTED_PROVIDER!"=="" (
        echo [WARN] No provider selected, defaulting to claude
        set "SELECTED_PROVIDER=claude"
    )

    :: Model selection
    set "SELECTED_MODEL="
    echo.
    if /i "!SELECTED_PROVIDER!"=="claude" (
        echo   Available Claude models:
        echo   1. claude-sonnet-4-5  ^(recommended, balanced^)
        echo   2. claude-opus-4-5    ^(most capable^)
        echo   3. claude-haiku-4-5   ^(fastest^)
        echo   0. Skip ^(use provider default^)
        echo.
        if "!YES_ALL!"=="1" (
            set "SELECTED_MODEL=claude-sonnet-4-5"
            echo [AUTO] Model: claude-sonnet-4-5
        ) else (
            set /p "_MODEL_CHOICE=Select model [1/2/3/0, default 1]: "
            if "!_MODEL_CHOICE!"=="" set "_MODEL_CHOICE=1"
            if "!_MODEL_CHOICE!"=="1" set "SELECTED_MODEL=claude-sonnet-4-5"
            if "!_MODEL_CHOICE!"=="2" set "SELECTED_MODEL=claude-opus-4-5"
            if "!_MODEL_CHOICE!"=="3" set "SELECTED_MODEL=claude-haiku-4-5"
            if "!_MODEL_CHOICE!"=="0" set "SELECTED_MODEL="
        )
    ) else (
        echo   Enter model name for provider '!SELECTED_PROVIDER!' ^(leave blank for default^):
        if "!YES_ALL!"=="1" (
            set "SELECTED_MODEL="
            echo [AUTO] Model: ^(provider default^)
        ) else (
            set /p "SELECTED_MODEL=Model name [leave blank for default]: "
        )
    )

    :: Select hook events for agent
    set "_AGENT_DEFAULT="
    echo !HOOK_EVENTS! | findstr /c:"pre-commit" >nul && set "_AGENT_DEFAULT=!_AGENT_DEFAULT!1 "
    echo !HOOK_EVENTS! | findstr /c:"commit-msg" >nul && set "_AGENT_DEFAULT=!_AGENT_DEFAULT!2 "
    echo !HOOK_EVENTS! | findstr /c:"pre-push"   >nul && set "_AGENT_DEFAULT=!_AGENT_DEFAULT!3 "
    if "!_AGENT_DEFAULT!"=="" set "_AGENT_DEFAULT=1 2 3"

    echo.
    echo   1. pre-commit  - AI code review on every git commit
    echo   2. commit-msg  - check commit message format
    echo   3. pre-push    - AI code review before git push
    echo.
    if "!YES_ALL!"=="1" (
        set "AGENT_CHOICE=!_AGENT_DEFAULT!"
        echo [AUTO] Agent hook events: !_AGENT_DEFAULT!
    ) else (
        set /p "AGENT_CHOICE=Select hook events [1/2/3, default !_AGENT_DEFAULT!]: "
        if "!AGENT_CHOICE!"=="" set "AGENT_CHOICE=!_AGENT_DEFAULT!"
    )

    set "AGENT_EVENTS="
    echo !AGENT_CHOICE! | findstr /c:"1" >nul && set "AGENT_EVENTS=!AGENT_EVENTS! --event pre-commit"
    echo !AGENT_CHOICE! | findstr /c:"2" >nul && set "AGENT_EVENTS=!AGENT_EVENTS! --event commit-msg"
    echo !AGENT_CHOICE! | findstr /c:"3" >nul && set "AGENT_EVENTS=!AGENT_EVENTS! --event pre-push"

    if not "!AGENT_EVENTS!"=="" (
        set "MODEL_FLAG="
        if not "!SELECTED_MODEL!"=="" set "MODEL_FLAG=--model !SELECTED_MODEL!"
        linthis hook install -g --type agent --provider !SELECTED_PROVIDER! !MODEL_FLAG! !AGENT_EVENTS! -y
        if %errorlevel% equ 0 (
            echo [OK] Global agent hook installed ^(provider: !SELECTED_PROVIDER!^)
        ) else (
            echo [WARN] Agent hook installation failed
        )
    ) else (
        echo [INFO] Skipping agent hook installation
    )
) else (
    echo [INFO] Skipping agent hook installation
    echo [INFO] You can run later: linthis hook install -g --type agent --provider claude --event pre-commit --event commit-msg --event pre-push
)
echo.

:: ============================================================
:: Step 4: Configure aliases (doskey)
:: ============================================================
echo [INFO] Step 4/5: Configure command aliases...

if "!YES_ALL!"=="1" (
    set "ADD_ALIAS=y"
    echo [AUTO] Add aliases? -^> y
) else (
    set /p "ADD_ALIAS=Add common aliases? (lt=linthis, lts=linthis -s, ltm=linthis -m) [Y/n]: "
    if "!ADD_ALIAS!"=="" set "ADD_ALIAS=y"
)
if /i "!ADD_ALIAS!"=="y" (
    set "ALIAS_DIR=%USERPROFILE%\.linthis"
    set "ALIAS_FILE=!ALIAS_DIR!\aliases.cmd"

    if not exist "!ALIAS_DIR!" mkdir "!ALIAS_DIR!"

    (
        echo @echo off
        echo doskey lt=linthis $*
        echo doskey lts=linthis -s $*
        echo doskey ltm=linthis -m $*
    ) > "!ALIAS_FILE!"

    set "REG_KEY=HKCU\Software\Microsoft\Command Processor"
    reg query "!REG_KEY!" /v AutoRun >nul 2>&1
    if %errorlevel% equ 0 (
        for /f "tokens=2,*" %%a in ('reg query "!REG_KEY!" /v AutoRun 2^>nul ^| find "AutoRun"') do set "CURRENT_AUTORUN=%%b"
        echo !CURRENT_AUTORUN! | find /i "linthis" >nul 2>&1
        if %errorlevel% neq 0 (
            if "!CURRENT_AUTORUN!"=="" (
                reg add "!REG_KEY!" /v AutoRun /t REG_SZ /d "\"!ALIAS_FILE!\"" /f >nul
            ) else (
                reg add "!REG_KEY!" /v AutoRun /t REG_SZ /d "!CURRENT_AUTORUN! & \"!ALIAS_FILE!\"" /f >nul
            )
            echo [OK] Aliases configured. Reopen Command Prompt to activate.
        ) else (
            echo [OK] Aliases already exist
        )
    ) else (
        reg add "!REG_KEY!" /v AutoRun /t REG_SZ /d "\"!ALIAS_FILE!\"" /f >nul
        echo [OK] Aliases configured. Reopen Command Prompt to activate.
    )

    echo.
    echo [INFO] PowerShell users: add these to $PROFILE manually:
    echo   Set-Alias -Name lt -Value linthis
    echo   function lts { linthis -s @args }
    echo   function ltm { linthis -m @args }
) else (
    echo [INFO] Skipping alias configuration
    echo [INFO] You can configure manually:
    echo   doskey lt=linthis $*
    echo   doskey lts=linthis -s $*
    echo   doskey ltm=linthis -m $*
)
echo.

:: ============================================================
:: Step 5: Dependency check
:: ============================================================
echo [INFO] Step 5/5: Running linthis doctor to check dependencies...

linthis doctor 2>nul
echo.

echo [INFO] Depending on your project type, you may need additional lint tool dependencies:
echo.
echo   [1] Android (Java/Kotlin)        - checkstyle, detekt
echo   [2] iOS (OC/C++/Swift)           - clang-format, clang-tidy, cpplint, swiftlint
echo   [3] Ohos/HarmonyOS (TypeScript)  - node, eslint, typescript, @typescript-eslint
echo   [4] All three
echo   [5] Skip
echo.
if "!YES_ALL!"=="1" (
    set "DEP_CHOICE=5"
    echo [AUTO] Dependency install: 5 ^(skip^)
) else (
    set /p "DEP_CHOICE=Select [1/2/3/4/5, default 5 skip]: "
)

if "!DEP_CHOICE!"=="1" goto :install_android
if "!DEP_CHOICE!"=="2" goto :install_ios
if "!DEP_CHOICE!"=="3" goto :install_ohos
if "!DEP_CHOICE!"=="4" goto :install_all
goto :install_done

:install_android
echo [INFO] Installing Android lint dependencies...

where java >nul 2>&1
if %errorlevel% neq 0 (
    echo [WARN] Java not found. checkstyle and detekt require a Java runtime.
    echo [INFO] Install JDK: winget install Microsoft.OpenJDK.17
    echo        or visit: https://adoptium.net/
)

where checkstyle >nul 2>&1
if %errorlevel% neq 0 (
    echo [INFO] Installing checkstyle...
    where scoop >nul 2>&1
    if %errorlevel% equ 0 (
        scoop install checkstyle
    ) else (
        where choco >nul 2>&1
        if %errorlevel% equ 0 (
            choco install checkstyle -y
        ) else (
            echo [WARN] Please install checkstyle manually:
            echo        scoop install checkstyle
            echo        or choco install checkstyle
            echo        or download from https://checkstyle.org/
        )
    )
) else (
    echo [OK] checkstyle already installed
)

where detekt >nul 2>&1
if %errorlevel% neq 0 (
    echo [INFO] Installing detekt...
    where scoop >nul 2>&1
    if %errorlevel% equ 0 (
        scoop install detekt
    ) else (
        echo [WARN] Please install detekt manually:
        echo        scoop install detekt
        echo        or download from https://detekt.dev/
    )
) else (
    echo [OK] detekt already installed
)

if "!DEP_CHOICE!"=="4" goto :install_ios
goto :install_done

:install_ios
echo [INFO] Installing iOS lint dependencies...

where clang-format >nul 2>&1
if %errorlevel% neq 0 (
    echo [INFO] Installing clang-format...
    where scoop >nul 2>&1
    if %errorlevel% equ 0 (
        scoop install llvm
    ) else (
        where choco >nul 2>&1
        if %errorlevel% equ 0 (
            choco install llvm -y
        ) else (
            echo [WARN] Please install LLVM ^(includes clang-format^) manually:
            echo        scoop install llvm
            echo        or choco install llvm
            echo        or download from https://releases.llvm.org/
        )
    )
) else (
    echo [OK] clang-format already installed
)

where cpplint >nul 2>&1
if %errorlevel% neq 0 (
    echo [INFO] Installing cpplint...
    !INSTALL_CMD! cpplint
) else (
    echo [OK] cpplint already installed
)

echo [WARN] swiftlint is primarily macOS-only and is not available on Windows.
echo [INFO] For Swift linting, use macOS.

if "!DEP_CHOICE!"=="4" (
    echo.
    goto :install_ohos
)
goto :install_done

:install_ohos
echo [INFO] Installing Ohos/HarmonyOS (TypeScript/ArkTS) lint dependencies...

:: Check Node.js
where node >nul 2>&1
if %errorlevel% neq 0 (
    echo [WARN] Node.js not found. TypeScript lint tools require Node.js.
    where winget >nul 2>&1
    if %errorlevel% equ 0 (
        echo [INFO] Installing Node.js via winget...
        winget install OpenJS.NodeJS.LTS -e
    ) else (
        where choco >nul 2>&1
        if %errorlevel% equ 0 (
            echo [INFO] Installing Node.js via choco...
            choco install nodejs-lts -y
        ) else (
            echo [WARN] Please install Node.js manually from https://nodejs.org/
        )
    )
) else (
    for /f "tokens=*" %%v in ('node --version 2^>nul') do echo [OK] Node.js already installed: %%v
)

where npm >nul 2>&1
if %errorlevel% neq 0 (
    echo [WARN] npm not found. Please install npm alongside Node.js.
    goto :ohos_done
)

:: eslint
where eslint >nul 2>&1
if %errorlevel% neq 0 (
    echo [INFO] Installing eslint globally...
    npm install -g eslint
) else (
    for /f "tokens=*" %%v in ('eslint --version 2^>nul') do echo [OK] eslint already installed: %%v
)

:: typescript
where tsc >nul 2>&1
if %errorlevel% neq 0 (
    echo [INFO] Installing typescript globally...
    npm install -g typescript
) else (
    for /f "tokens=*" %%v in ('tsc --version 2^>nul') do echo [OK] typescript already installed: %%v
)

:: @typescript-eslint
echo [INFO] Installing @typescript-eslint/parser and @typescript-eslint/eslint-plugin globally...
npm install -g @typescript-eslint/parser @typescript-eslint/eslint-plugin

echo [OK] Ohos/HarmonyOS TypeScript lint dependencies installed
echo [INFO] For HarmonyOS-specific rules, see: https://developer.huawei.com/consumer/en/doc/harmonyos-guides/ide-code-linter

:ohos_done
goto :install_done

:install_all
set "DEP_CHOICE=4"
goto :install_android

:install_done
echo.
echo ============================================
echo    Installation Complete!
echo ============================================
echo.
echo Common commands:
echo   linthis -i ^<path^>    lint a specific path
echo   linthis -s           lint git staged files
echo   linthis -m           lint git modified files
echo   linthis doctor       check dependency status
echo.
echo More help: linthis --help
echo.

if "!YES_ALL!"=="0" pause
endlocal
