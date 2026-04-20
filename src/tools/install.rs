// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style license that can be
// found at https://opensource.org/license/MIT

//! Data-driven tool install matrix.
//!
//! Each `ToolInstallSpec` entry in `TOOL_INSTALLS` lists per-platform
//! candidate commands tried in order until one succeeds. A missing OS
//! entry means the tool is not supported on that platform; the resolver
//! returns an empty `Vec` in that case so callers can explicitly skip.

use crate::Language;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Os {
    MacOs,
    Linux,
    Windows,
}

impl Os {
    /// The OS this binary was compiled for.
    pub fn current() -> Self {
        if cfg!(target_os = "macos") {
            Os::MacOs
        } else if cfg!(target_os = "windows") {
            Os::Windows
        } else {
            Os::Linux
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ToolRole {
    Checker,
    Formatter,
}

#[derive(Debug)]
pub struct PlatformCmds {
    pub os: Os,
    /// Candidate commands tried in order (first success wins).
    pub cmds: &'static [&'static [&'static str]],
}

#[derive(Debug)]
pub struct ToolInstallSpec {
    /// Canonical tool name (e.g. "stylua", "golangci-lint").
    pub tool: &'static str,
    pub language: Language,
    pub role: ToolRole,
    /// Only lists platforms where the tool is supported.
    /// A missing OS entry = "not supported on this platform".
    pub platforms: &'static [PlatformCmds],
    /// User-facing hint shown when auto-install is declined/disabled.
    pub hint: &'static str,
}

pub static TOOL_INSTALLS: &[ToolInstallSpec] = &[
    // ─────────────── Python ───────────────
    // ruff is both checker and formatter; register once per role.
    ToolInstallSpec {
        tool: "ruff",
        language: Language::Python,
        role: ToolRole::Checker,
        platforms: &[
            PlatformCmds {
                os: Os::MacOs,
                cmds: &[
                    &["brew", "install", "ruff"],
                    &["uv", "tool", "install", "ruff"],
                    &["pipx", "install", "ruff"],
                    &["uv", "pip", "install", "--system", "ruff"],
                    &["pip3", "install", "ruff"],
                    &["pip", "install", "ruff"],
                ],
            },
            PlatformCmds {
                os: Os::Linux,
                cmds: &[
                    &["uv", "tool", "install", "ruff"],
                    &["pipx", "install", "ruff"],
                    &["uv", "pip", "install", "--system", "ruff"],
                    &["pip3", "install", "ruff"],
                    &["pip", "install", "ruff"],
                ],
            },
            PlatformCmds {
                os: Os::Windows,
                cmds: &[
                    &["uv", "tool", "install", "ruff"],
                    &["pipx", "install", "ruff"],
                    &["pip3", "install", "ruff"],
                    &["pip", "install", "ruff"],
                ],
            },
        ],
        hint: "Install: brew install ruff (macOS) / uv tool install ruff / pipx install ruff / pip install ruff",
    },
    ToolInstallSpec {
        tool: "ruff",
        language: Language::Python,
        role: ToolRole::Formatter,
        platforms: &[
            PlatformCmds {
                os: Os::MacOs,
                cmds: &[
                    &["brew", "install", "ruff"],
                    &["uv", "tool", "install", "ruff"],
                    &["pipx", "install", "ruff"],
                    &["uv", "pip", "install", "--system", "ruff"],
                    &["pip3", "install", "ruff"],
                    &["pip", "install", "ruff"],
                ],
            },
            PlatformCmds {
                os: Os::Linux,
                cmds: &[
                    &["uv", "tool", "install", "ruff"],
                    &["pipx", "install", "ruff"],
                    &["uv", "pip", "install", "--system", "ruff"],
                    &["pip3", "install", "ruff"],
                    &["pip", "install", "ruff"],
                ],
            },
            PlatformCmds {
                os: Os::Windows,
                cmds: &[
                    &["uv", "tool", "install", "ruff"],
                    &["pipx", "install", "ruff"],
                    &["pip3", "install", "ruff"],
                    &["pip", "install", "ruff"],
                ],
            },
        ],
        hint: "Install: brew install ruff (macOS) / uv tool install ruff / pipx install ruff / pip install ruff",
    },

    // ─────────────── Rust ───────────────
    ToolInstallSpec {
        tool: "clippy",
        language: Language::Rust,
        role: ToolRole::Checker,
        platforms: &[
            PlatformCmds {
                os: Os::MacOs,
                cmds: &[&["rustup", "component", "add", "clippy"]],
            },
            PlatformCmds {
                os: Os::Linux,
                cmds: &[&["rustup", "component", "add", "clippy"]],
            },
            PlatformCmds {
                os: Os::Windows,
                cmds: &[&["rustup", "component", "add", "clippy"]],
            },
        ],
        hint: "Install: rustup component add clippy",
    },
    ToolInstallSpec {
        tool: "rustfmt",
        language: Language::Rust,
        role: ToolRole::Formatter,
        platforms: &[
            PlatformCmds {
                os: Os::MacOs,
                cmds: &[&["rustup", "component", "add", "rustfmt"]],
            },
            PlatformCmds {
                os: Os::Linux,
                cmds: &[&["rustup", "component", "add", "rustfmt"]],
            },
            PlatformCmds {
                os: Os::Windows,
                cmds: &[&["rustup", "component", "add", "rustfmt"]],
            },
        ],
        hint: "Install: rustup component add rustfmt",
    },

    // ─────────────── Go ───────────────
    ToolInstallSpec {
        tool: "golangci-lint",
        language: Language::Go,
        role: ToolRole::Checker,
        platforms: &[
            PlatformCmds {
                os: Os::MacOs,
                cmds: &[
                    &["brew", "install", "golangci-lint"],
                    &["go", "install", "github.com/golangci/golangci-lint/cmd/golangci-lint@latest"],
                ],
            },
            PlatformCmds {
                os: Os::Linux,
                cmds: &[
                    &["go", "install", "github.com/golangci/golangci-lint/cmd/golangci-lint@latest"],
                ],
            },
            PlatformCmds {
                os: Os::Windows,
                cmds: &[
                    &["choco", "install", "golangci-lint"],
                    &["scoop", "install", "golangci-lint"],
                    &["go", "install", "github.com/golangci/golangci-lint/cmd/golangci-lint@latest"],
                ],
            },
        ],
        hint: "Install: brew install golangci-lint (macOS) / go install github.com/golangci/golangci-lint/cmd/golangci-lint@latest",
    },
    // gofmt: ships with the Go toolchain; no separate install needed.

    // ─────────────── TypeScript / JavaScript ───────────────
    ToolInstallSpec {
        tool: "eslint",
        language: Language::TypeScript,
        role: ToolRole::Checker,
        platforms: &[
            PlatformCmds {
                os: Os::MacOs,
                cmds: &[
                    &["pnpm", "add", "-g", "eslint"],
                    &["yarn", "global", "add", "eslint"],
                    &["npm", "install", "-g", "eslint"],
                ],
            },
            PlatformCmds {
                os: Os::Linux,
                cmds: &[
                    &["pnpm", "add", "-g", "eslint"],
                    &["yarn", "global", "add", "eslint"],
                    &["npm", "install", "-g", "eslint"],
                ],
            },
            PlatformCmds {
                os: Os::Windows,
                cmds: &[
                    &["pnpm", "add", "-g", "eslint"],
                    &["yarn", "global", "add", "eslint"],
                    &["npm", "install", "-g", "eslint"],
                ],
            },
        ],
        hint: "Install: pnpm add -g eslint / yarn global add eslint / npm install -g eslint",
    },
    ToolInstallSpec {
        tool: "prettier",
        language: Language::TypeScript,
        role: ToolRole::Formatter,
        platforms: &[
            PlatformCmds {
                os: Os::MacOs,
                cmds: &[
                    &["pnpm", "add", "-g", "prettier"],
                    &["yarn", "global", "add", "prettier"],
                    &["npm", "install", "-g", "prettier"],
                ],
            },
            PlatformCmds {
                os: Os::Linux,
                cmds: &[
                    &["pnpm", "add", "-g", "prettier"],
                    &["yarn", "global", "add", "prettier"],
                    &["npm", "install", "-g", "prettier"],
                ],
            },
            PlatformCmds {
                os: Os::Windows,
                cmds: &[
                    &["pnpm", "add", "-g", "prettier"],
                    &["yarn", "global", "add", "prettier"],
                    &["npm", "install", "-g", "prettier"],
                ],
            },
        ],
        hint: "Install: pnpm add -g prettier / yarn global add prettier / npm install -g prettier",
    },
    ToolInstallSpec {
        tool: "eslint",
        language: Language::JavaScript,
        role: ToolRole::Checker,
        platforms: &[
            PlatformCmds {
                os: Os::MacOs,
                cmds: &[
                    &["pnpm", "add", "-g", "eslint"],
                    &["yarn", "global", "add", "eslint"],
                    &["npm", "install", "-g", "eslint"],
                ],
            },
            PlatformCmds {
                os: Os::Linux,
                cmds: &[
                    &["pnpm", "add", "-g", "eslint"],
                    &["yarn", "global", "add", "eslint"],
                    &["npm", "install", "-g", "eslint"],
                ],
            },
            PlatformCmds {
                os: Os::Windows,
                cmds: &[
                    &["pnpm", "add", "-g", "eslint"],
                    &["yarn", "global", "add", "eslint"],
                    &["npm", "install", "-g", "eslint"],
                ],
            },
        ],
        hint: "Install: pnpm add -g eslint / yarn global add eslint / npm install -g eslint",
    },
    ToolInstallSpec {
        tool: "prettier",
        language: Language::JavaScript,
        role: ToolRole::Formatter,
        platforms: &[
            PlatformCmds {
                os: Os::MacOs,
                cmds: &[
                    &["pnpm", "add", "-g", "prettier"],
                    &["yarn", "global", "add", "prettier"],
                    &["npm", "install", "-g", "prettier"],
                ],
            },
            PlatformCmds {
                os: Os::Linux,
                cmds: &[
                    &["pnpm", "add", "-g", "prettier"],
                    &["yarn", "global", "add", "prettier"],
                    &["npm", "install", "-g", "prettier"],
                ],
            },
            PlatformCmds {
                os: Os::Windows,
                cmds: &[
                    &["pnpm", "add", "-g", "prettier"],
                    &["yarn", "global", "add", "prettier"],
                    &["npm", "install", "-g", "prettier"],
                ],
            },
        ],
        hint: "Install: pnpm add -g prettier / yarn global add prettier / npm install -g prettier",
    },

    // ─────────────── C++ (shared for ObjectiveC via lib.rs thunk) ───────────────
    ToolInstallSpec {
        tool: "cpplint",
        language: Language::Cpp,
        role: ToolRole::Checker,
        platforms: &[
            PlatformCmds {
                os: Os::MacOs,
                cmds: &[
                    &["brew", "install", "cpplint"],
                    &["uv", "tool", "install", "cpplint"],
                    &["pipx", "install", "cpplint"],
                    &["uv", "pip", "install", "--system", "cpplint"],
                    &["pip3", "install", "cpplint"],
                    &["pip", "install", "cpplint"],
                ],
            },
            PlatformCmds {
                os: Os::Linux,
                cmds: &[
                    &["uv", "tool", "install", "cpplint"],
                    &["pipx", "install", "cpplint"],
                    &["uv", "pip", "install", "--system", "cpplint"],
                    &["pip3", "install", "cpplint"],
                    &["pip", "install", "cpplint"],
                ],
            },
            PlatformCmds {
                os: Os::Windows,
                cmds: &[
                    &["uv", "tool", "install", "cpplint"],
                    &["pipx", "install", "cpplint"],
                    &["pip3", "install", "cpplint"],
                    &["pip", "install", "cpplint"],
                ],
            },
        ],
        hint: "Install: brew install cpplint (macOS) / uv tool install cpplint / pipx install cpplint / pip install cpplint",
    },
    ToolInstallSpec {
        tool: "clang-format",
        language: Language::Cpp,
        role: ToolRole::Formatter,
        platforms: &[
            PlatformCmds {
                os: Os::MacOs,
                cmds: &[
                    &["brew", "install", "clang-format"],
                    &["brew", "install", "llvm"],
                ],
            },
            PlatformCmds {
                os: Os::Linux,
                cmds: &[
                    &["sudo", "apt-get", "install", "-y", "clang-format"],
                    &["sudo", "dnf", "install", "-y", "clang-tools-extra"],
                    &["sudo", "pacman", "-S", "--noconfirm", "clang"],
                ],
            },
            PlatformCmds {
                os: Os::Windows,
                cmds: &[
                    &["choco", "install", "llvm"],
                    &["scoop", "install", "llvm"],
                ],
            },
        ],
        hint: "Install: brew install clang-format (macOS) / apt install clang-format / choco install llvm",
    },

    // ─────────────── Java ───────────────
    ToolInstallSpec {
        tool: "checkstyle",
        language: Language::Java,
        role: ToolRole::Checker,
        platforms: &[
            PlatformCmds {
                os: Os::MacOs,
                cmds: &[&["brew", "install", "checkstyle"]],
            },
            PlatformCmds {
                os: Os::Linux,
                cmds: &[&["sudo", "apt-get", "install", "-y", "checkstyle"]],
            },
            PlatformCmds {
                os: Os::Windows,
                cmds: &[&["choco", "install", "checkstyle"]],
            },
        ],
        hint: "Install: brew install checkstyle (macOS) / apt install checkstyle / choco install checkstyle",
    },
    ToolInstallSpec {
        tool: "google-java-format",
        language: Language::Java,
        role: ToolRole::Formatter,
        platforms: &[
            PlatformCmds {
                os: Os::MacOs,
                cmds: &[&["brew", "install", "google-java-format"]],
            },
            // Linux / Windows: no stable package-manager route; user must
            // download the jar. We expose an empty platform list to signal
            // "not auto-installable here"; hint directs them to the release page.
        ],
        hint: "Install: brew install google-java-format (macOS) / download jar from https://github.com/google/google-java-format/releases",
    },

    // ─────────────── Shell ───────────────
    ToolInstallSpec {
        tool: "shellcheck",
        language: Language::Shell,
        role: ToolRole::Checker,
        platforms: &[
            PlatformCmds {
                os: Os::MacOs,
                cmds: &[&["brew", "install", "shellcheck"]],
            },
            PlatformCmds {
                os: Os::Linux,
                cmds: &[
                    &["sudo", "apt-get", "install", "-y", "shellcheck"],
                    &["sudo", "dnf", "install", "-y", "ShellCheck"],
                    &["sudo", "pacman", "-S", "--noconfirm", "shellcheck"],
                ],
            },
            PlatformCmds {
                os: Os::Windows,
                cmds: &[
                    &["scoop", "install", "shellcheck"],
                    &["choco", "install", "shellcheck"],
                ],
            },
        ],
        hint: "Install: brew install shellcheck / apt install shellcheck / scoop install shellcheck",
    },
    ToolInstallSpec {
        tool: "shfmt",
        language: Language::Shell,
        role: ToolRole::Formatter,
        platforms: &[
            PlatformCmds {
                os: Os::MacOs,
                cmds: &[&["brew", "install", "shfmt"]],
            },
            PlatformCmds {
                os: Os::Linux,
                cmds: &[
                    &["sudo", "apt-get", "install", "-y", "shfmt"],
                    &["sudo", "dnf", "install", "-y", "shfmt"],
                    &["go", "install", "mvdan.cc/sh/v3/cmd/shfmt@latest"],
                ],
            },
            PlatformCmds {
                os: Os::Windows,
                cmds: &[
                    &["scoop", "install", "shfmt"],
                    &["choco", "install", "shfmt"],
                    &["go", "install", "mvdan.cc/sh/v3/cmd/shfmt@latest"],
                ],
            },
        ],
        hint: "Install: brew install shfmt / apt install shfmt / scoop install shfmt / go install mvdan.cc/sh/v3/cmd/shfmt@latest",
    },

    // ─────────────── Lua ───────────────
    ToolInstallSpec {
        tool: "luacheck",
        language: Language::Lua,
        role: ToolRole::Checker,
        platforms: &[
            PlatformCmds {
                os: Os::MacOs,
                cmds: &[&["brew", "install", "luacheck"]],
            },
            PlatformCmds {
                os: Os::Linux,
                cmds: &[&["luarocks", "install", "luacheck"]],
            },
            PlatformCmds {
                os: Os::Windows,
                cmds: &[&["luarocks", "install", "luacheck"]],
            },
        ],
        hint: "Install: brew install luacheck (macOS) / luarocks install luacheck",
    },
    ToolInstallSpec {
        tool: "stylua",
        language: Language::Lua,
        role: ToolRole::Formatter,
        platforms: &[
            PlatformCmds {
                os: Os::MacOs,
                cmds: &[
                    &["brew", "install", "stylua"],
                    &["cargo", "install", "stylua"],
                ],
            },
            PlatformCmds {
                os: Os::Linux,
                cmds: &[&["cargo", "install", "stylua"]],
            },
            PlatformCmds {
                os: Os::Windows,
                cmds: &[
                    &["scoop", "install", "stylua"],
                    &["choco", "install", "stylua"],
                    &["cargo", "install", "stylua"],
                ],
            },
        ],
        hint: "Install: brew install stylua (macOS) / scoop install stylua (Windows) / cargo install stylua",
    },

    // ─────────────── Swift (macOS only) ───────────────
    ToolInstallSpec {
        tool: "swiftlint",
        language: Language::Swift,
        role: ToolRole::Checker,
        platforms: &[
            PlatformCmds {
                os: Os::MacOs,
                cmds: &[&["brew", "install", "swiftlint"]],
            },
        ],
        hint: "Install: brew install swiftlint (macOS only)",
    },
    ToolInstallSpec {
        tool: "swift-format",
        language: Language::Swift,
        role: ToolRole::Formatter,
        platforms: &[
            PlatformCmds {
                os: Os::MacOs,
                cmds: &[&["brew", "install", "swift-format"]],
            },
        ],
        hint: "Install: brew install swift-format (macOS only)",
    },

    // ─────────────── Kotlin ───────────────
    // ktlint plays both roles; register once per role.
    ToolInstallSpec {
        tool: "ktlint",
        language: Language::Kotlin,
        role: ToolRole::Checker,
        platforms: &[
            PlatformCmds {
                os: Os::MacOs,
                cmds: &[&["brew", "install", "ktlint"]],
            },
            PlatformCmds {
                os: Os::Windows,
                cmds: &[
                    &["scoop", "install", "ktlint"],
                    &["choco", "install", "ktlint"],
                ],
            },
            // Linux: no package-manager formula; downloading the binary
            // is scripted separately. Leave empty so users get the hint.
        ],
        hint: "Install: brew install ktlint (macOS) / scoop install ktlint (Windows) / download from https://github.com/pinterest/ktlint/releases",
    },
    ToolInstallSpec {
        tool: "ktlint",
        language: Language::Kotlin,
        role: ToolRole::Formatter,
        platforms: &[
            PlatformCmds {
                os: Os::MacOs,
                cmds: &[&["brew", "install", "ktlint"]],
            },
            PlatformCmds {
                os: Os::Windows,
                cmds: &[
                    &["scoop", "install", "ktlint"],
                    &["choco", "install", "ktlint"],
                ],
            },
        ],
        hint: "Install: brew install ktlint (macOS) / scoop install ktlint (Windows) / download from https://github.com/pinterest/ktlint/releases",
    },

    // ─────────────── Ruby ───────────────
    ToolInstallSpec {
        tool: "rubocop",
        language: Language::Ruby,
        role: ToolRole::Checker,
        platforms: &[
            PlatformCmds {
                os: Os::MacOs,
                cmds: &[&["gem", "install", "rubocop"]],
            },
            PlatformCmds {
                os: Os::Linux,
                cmds: &[&["gem", "install", "rubocop"]],
            },
            PlatformCmds {
                os: Os::Windows,
                cmds: &[&["gem", "install", "rubocop"]],
            },
        ],
        hint: "Install: gem install rubocop (requires Ruby)",
    },
    ToolInstallSpec {
        tool: "rubocop",
        language: Language::Ruby,
        role: ToolRole::Formatter,
        platforms: &[
            PlatformCmds {
                os: Os::MacOs,
                cmds: &[&["gem", "install", "rubocop"]],
            },
            PlatformCmds {
                os: Os::Linux,
                cmds: &[&["gem", "install", "rubocop"]],
            },
            PlatformCmds {
                os: Os::Windows,
                cmds: &[&["gem", "install", "rubocop"]],
            },
        ],
        hint: "Install: gem install rubocop (requires Ruby)",
    },

    // ─────────────── PHP ───────────────
    ToolInstallSpec {
        tool: "phpcs",
        language: Language::Php,
        role: ToolRole::Checker,
        platforms: &[
            PlatformCmds {
                os: Os::MacOs,
                cmds: &[&["composer", "global", "require", "squizlabs/php_codesniffer"]],
            },
            PlatformCmds {
                os: Os::Linux,
                cmds: &[&["composer", "global", "require", "squizlabs/php_codesniffer"]],
            },
            PlatformCmds {
                os: Os::Windows,
                cmds: &[&["composer", "global", "require", "squizlabs/php_codesniffer"]],
            },
        ],
        hint: "Install: composer global require squizlabs/php_codesniffer",
    },
    ToolInstallSpec {
        tool: "php-cs-fixer",
        language: Language::Php,
        role: ToolRole::Formatter,
        platforms: &[
            PlatformCmds {
                os: Os::MacOs,
                cmds: &[&["composer", "global", "require", "friendsofphp/php-cs-fixer"]],
            },
            PlatformCmds {
                os: Os::Linux,
                cmds: &[&["composer", "global", "require", "friendsofphp/php-cs-fixer"]],
            },
            PlatformCmds {
                os: Os::Windows,
                cmds: &[&["composer", "global", "require", "friendsofphp/php-cs-fixer"]],
            },
        ],
        hint: "Install: composer global require friendsofphp/php-cs-fixer",
    },

    // ─────────────── Scala ───────────────
    ToolInstallSpec {
        tool: "scalafix",
        language: Language::Scala,
        role: ToolRole::Checker,
        platforms: &[
            PlatformCmds {
                os: Os::MacOs,
                cmds: &[
                    &["brew", "install", "scalafix"],
                    &["cs", "install", "scalafix"],
                ],
            },
            PlatformCmds {
                os: Os::Linux,
                cmds: &[&["cs", "install", "scalafix"]],
            },
            PlatformCmds {
                os: Os::Windows,
                cmds: &[&["cs", "install", "scalafix"]],
            },
        ],
        hint: "Install: brew install scalafix (macOS) / cs install scalafix (requires coursier)",
    },
    ToolInstallSpec {
        tool: "scalafmt",
        language: Language::Scala,
        role: ToolRole::Formatter,
        platforms: &[
            PlatformCmds {
                os: Os::MacOs,
                cmds: &[
                    &["brew", "install", "scalafmt"],
                    &["cs", "install", "scalafmt"],
                ],
            },
            PlatformCmds {
                os: Os::Linux,
                cmds: &[&["cs", "install", "scalafmt"]],
            },
            PlatformCmds {
                os: Os::Windows,
                cmds: &[&["cs", "install", "scalafmt"]],
            },
        ],
        hint: "Install: brew install scalafmt (macOS) / cs install scalafmt (requires coursier)",
    },

    // ─────────────── C# ───────────────
    ToolInstallSpec {
        tool: "dotnet-format",
        language: Language::CSharp,
        role: ToolRole::Checker,
        platforms: &[
            PlatformCmds {
                os: Os::MacOs,
                cmds: &[&["dotnet", "tool", "install", "-g", "dotnet-format"]],
            },
            PlatformCmds {
                os: Os::Linux,
                cmds: &[&["dotnet", "tool", "install", "-g", "dotnet-format"]],
            },
            PlatformCmds {
                os: Os::Windows,
                cmds: &[&["dotnet", "tool", "install", "-g", "dotnet-format"]],
            },
        ],
        hint: "Install: dotnet tool install -g dotnet-format (requires .NET SDK)",
    },
    ToolInstallSpec {
        tool: "dotnet-format",
        language: Language::CSharp,
        role: ToolRole::Formatter,
        platforms: &[
            PlatformCmds {
                os: Os::MacOs,
                cmds: &[&["dotnet", "tool", "install", "-g", "dotnet-format"]],
            },
            PlatformCmds {
                os: Os::Linux,
                cmds: &[&["dotnet", "tool", "install", "-g", "dotnet-format"]],
            },
            PlatformCmds {
                os: Os::Windows,
                cmds: &[&["dotnet", "tool", "install", "-g", "dotnet-format"]],
            },
        ],
        hint: "Install: dotnet tool install -g dotnet-format (requires .NET SDK)",
    },

    // Dart: no auto-install (SDK required). Explicitly absent from TOOL_INSTALLS.
];

/// Find the spec for (lang, role); returns `None` if no entry.
fn find_spec(lang: Language, role: ToolRole) -> Option<&'static ToolInstallSpec> {
    TOOL_INSTALLS
        .iter()
        .find(|s| s.language == lang && s.role == role)
}

/// Resolve install candidate commands for (lang, role) on the current platform.
/// Returns `vec![]` if the tool is not supported on this platform.
pub fn resolve_install_cmds(lang: Language, role: ToolRole) -> Vec<Vec<String>> {
    let spec = match find_spec(lang, role) {
        Some(s) => s,
        None => return Vec::new(),
    };
    let current = Os::current();
    spec.platforms
        .iter()
        .find(|p| p.os == current)
        .map(|p| {
            p.cmds
                .iter()
                .map(|c| c.iter().map(|s| s.to_string()).collect())
                .collect()
        })
        .unwrap_or_default()
}

/// Is this tool installable on the current platform?
pub fn is_tool_supported_on_current_platform(tool: &str) -> bool {
    let current = Os::current();
    TOOL_INSTALLS
        .iter()
        .find(|s| s.tool == tool)
        .map(|s| s.platforms.iter().any(|p| p.os == current))
        .unwrap_or(false)
}

/// Which platforms does this tool support?
pub fn supported_platforms(tool: &str) -> Vec<Os> {
    TOOL_INSTALLS
        .iter()
        .find(|s| s.tool == tool)
        .map(|s| s.platforms.iter().map(|p| p.os).collect())
        .unwrap_or_default()
}

/// Hint string for a (lang, role) — used when auto-install is off/declined.
pub fn install_hint(lang: Language, role: ToolRole) -> String {
    find_spec(lang, role)
        .map(|s| s.hint.to_string())
        .unwrap_or_else(|| format!("No install hint available for {:?} {:?}", lang, role))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unsupported_language_resolves_to_empty_vec() {
        // Dart has no entry in TOOL_INSTALLS; lookup must be a miss.
        assert!(resolve_install_cmds(Language::Dart, ToolRole::Formatter).is_empty());
    }

    #[test]
    fn unsupported_tool_is_not_supported_on_current_platform() {
        assert!(!is_tool_supported_on_current_platform("nonexistent-tool"));
    }

    #[test]
    fn os_current_matches_compile_target() {
        #[cfg(target_os = "macos")]
        assert_eq!(Os::current(), Os::MacOs);
        #[cfg(target_os = "linux")]
        assert_eq!(Os::current(), Os::Linux);
        #[cfg(target_os = "windows")]
        assert_eq!(Os::current(), Os::Windows);
    }

    #[test]
    fn install_hint_for_unknown_has_fallback() {
        let hint = install_hint(Language::Dart, ToolRole::Formatter);
        assert!(hint.contains("No install hint"));
    }

    #[test]
    fn table_contains_all_expected_tools() {
        let expected: &[&str] = &[
            "ruff",
            "clippy",
            "rustfmt",
            "golangci-lint",
            "eslint",
            "prettier",
            "cpplint",
            "clang-format",
            "checkstyle",
            "google-java-format",
            "shellcheck",
            "shfmt",
            "luacheck",
            "stylua",
            "swiftlint",
            "swift-format",
            "ktlint",
            "rubocop",
            "phpcs",
            "php-cs-fixer",
            "scalafix",
            "scalafmt",
            "dotnet-format",
        ];
        let present: std::collections::HashSet<&str> =
            TOOL_INSTALLS.iter().map(|s| s.tool).collect();
        for tool in expected {
            assert!(
                present.contains(tool),
                "TOOL_INSTALLS missing entry for {tool}"
            );
        }
    }

    #[test]
    fn stylua_macos_first_candidate_is_brew() {
        let spec = TOOL_INSTALLS
            .iter()
            .find(|s| s.tool == "stylua")
            .expect("stylua spec present");
        let macos = spec
            .platforms
            .iter()
            .find(|p| p.os == Os::MacOs)
            .expect("stylua supports macOS");
        assert_eq!(macos.cmds[0], &["brew", "install", "stylua"]);
    }

    #[test]
    fn swiftlint_is_macos_only() {
        let spec = TOOL_INSTALLS
            .iter()
            .find(|s| s.tool == "swiftlint")
            .expect("swiftlint spec present");
        let oses: Vec<Os> = spec.platforms.iter().map(|p| p.os).collect();
        assert_eq!(oses, vec![Os::MacOs]);
    }

    #[test]
    fn all_platform_cmds_have_at_least_one_candidate() {
        for spec in TOOL_INSTALLS {
            for platform in spec.platforms {
                assert!(
                    !platform.cmds.is_empty(),
                    "{} on {:?} has no commands",
                    spec.tool,
                    platform.os
                );
                for cmd in platform.cmds {
                    assert!(
                        !cmd.is_empty(),
                        "{} on {:?} has an empty command",
                        spec.tool,
                        platform.os
                    );
                }
            }
        }
    }

    #[test]
    fn no_duplicate_tool_role_pairs() {
        let mut seen = std::collections::HashSet::new();
        for spec in TOOL_INSTALLS {
            let key = (spec.language, spec.role);
            assert!(
                seen.insert(key),
                "duplicate (language, role) entry for tool '{}': {:?} {:?}",
                spec.tool,
                spec.language,
                spec.role
            );
        }
    }
}
