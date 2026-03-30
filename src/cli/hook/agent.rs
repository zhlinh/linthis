// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Agent (AI Coding Agent) multi-provider integration:
//! handle_agent_hook_install/uninstall, agent skill management, plugin installation.

use colored::Colorize;
use std::path::PathBuf;
use std::process::ExitCode;

use super::dirs;
use super::metadata::{add_skill_provider_to_hook, remove_skill_provider_from_hook};
use super::{find_git_root, is_command_available};
use crate::cli::commands::{AgentProvider, HookEvent};

/// All supported agent providers (in display order)
pub(crate) const ALL_AGENT_PROVIDERS: &[AgentProvider] = &[
    AgentProvider::Claude,
    AgentProvider::Codex,
    AgentProvider::Gemini,
    AgentProvider::Cursor,
    AgentProvider::Droid,
    AgentProvider::Auggie,
    AgentProvider::Codebuddy,
    AgentProvider::Openclaw,
];

/// The section marker used in append-style files (CLAUDE.md, AGENTS.md)
const AGENT_SECTION_MARKER: &str = "## Linthis Lint Rule";
/// Legacy section marker for backward compatibility (uninstall/detection)
const AGENT_SECTION_MARKER_LEGACY: &str = "## Linthis Agent Rules";

/// Generic skill body for a given event (used by all providers).
/// Test-only re-export of agent_event_content_generic.
#[cfg(test)]
pub(crate) fn agent_event_content_generic_test(event: &HookEvent) -> String {
    agent_event_content_generic(event)
}

fn agent_event_content_generic(event: &HookEvent) -> String {
    match event {
        HookEvent::PreCommit => agent_lint_body(),
        HookEvent::CommitMsg => agent_cmsg_body(),
        HookEvent::PrePush => agent_review_body(),
    }
}

fn agent_lint_body() -> String {
    r#"## Companion Skills

When the user asks to commit, if the **lt-cmsg** skill is also available, both lt-lint and lt-cmsg should be invoked together. Run lt-lint first (to fix code issues), then lt-cmsg (to validate the commit message).

## Goal

Catch and fix code quality issues before they enter the repository. Running lint at pre-commit ensures every committed file meets project coding standards, preventing style debt from accumulating over time.

## When to Skip

If no code files were modified in this session, approve immediately.

## Key Commands

| Scope | Command | Description |
|-------|---------|-------------|
| Staged files | `linthis -s` | Check & format all files in the git staging area (`git add`ed) |
| Modified files | `linthis -m` | Check & format all locally modified files (staged + unstaged) |
| Specific files | `linthis -i <f1> -i <f2>` | Check & format listed files — one `-i` per file |
| Check only | append `-c` | Lint only, no formatting (e.g. `linthis -s -c`) |

## Steps

1. Identify modified code files in this session (files written or edited via Write/Edit tools, or via Bash)
2. Run lint + format on those files:
   - `linthis -m` to cover all modified files at once, or
   - `linthis -i <file1> -i <file2>` to target specific files
   - **Note**: linthis may auto-format files (whitespace, trailing newlines, etc.) in addition to reporting lint errors
3. Read the lint output carefully — each issue includes file path, line number, and rule name
4. If issues are found, fix them by editing the code directly
   - Do **NOT** use `linthis --fix` or `linthis fix` — fixing manually ensures you understand the issue and don't introduce regressions from blind automated transforms
5. Re-run linthis to confirm all issues are resolved
6. **Re-stage**: if any files were already staged before step 2, linting/formatting may have changed them on disk. You must re-stage those files so the index matches the working tree:
   ```
   git add <formatted or fixed files>
   ```
7. Final check: run `linthis -s -c` (check-only on staged files) to verify the staging area is clean
8. Only approve the commit once lint passes with zero errors

## Example

```
$ linthis -i src/handler.go

src/handler.go:15:1: exported function HandleRequest should have comment (golint)
src/handler.go:23:4: error return value not checked (errcheck)

2 issues found
```

Fix line 15 by adding a doc comment, and line 23 by handling the error return value. Then re-run to confirm zero errors. If files were staged, re-stage: `git add src/handler.go`."#
        .to_string()
}

fn agent_cmsg_body() -> String {
    r#"## Companion Skills

When the user asks to commit, if the **lt-lint** skill is also available, both lt-lint and lt-cmsg should be invoked together. Run lt-lint first (to fix code issues), then lt-cmsg (to validate the commit message).

## Goal

Ensure every commit message follows Conventional Commits format and accurately reflects the actual code changes. A well-structured commit history makes code review, changelog generation, and git bisect much easier.

## When to Skip

If `linthis cmsg .git/COMMIT_EDITMSG` passes on the first run, approve immediately with `✅ Commit message OK`.

## Configuration

The validation pattern can be configured in `.linthis/config.toml` (project-level) or the global linthis config. If no config is present, `linthis cmsg` defaults to Conventional Commits format:

```toml
[cmsg]
commit_msg_pattern = "^(feat|fix|docs|...)\\(\\S+\\)?: .{1,72}"
require_ticket = false          # require ticket reference e.g. [JIRA-123]
ticket_pattern = "\\[\\w+-\\d+\\]"  # custom ticket regex
```

`linthis cmsg` resolves config automatically (project → global → built-in default). To check the effective pattern quickly:

```bash
linthis config get cmsg.commit_msg_pattern      # project-level
linthis config get cmsg.commit_msg_pattern -g   # global
# "not found" means built-in default (Conventional Commits) applies
```

## Steps

1. Run `linthis cmsg .git/COMMIT_EDITMSG` — this is the **authoritative validator** and reads `.linthis/config.toml` automatically
2. If linthis cmsg **passes** → output `✅ Commit message OK` and approve immediately
3. If linthis cmsg **fails** → read the error output to understand what rule was violated, then:
   - Run `git diff --cached --stat` to understand what files actually changed — the type prefix must match the actual diff
   - Run `git log -n 5 --oneline` to check the recent commit style **and language** (Chinese or English) — match that language for consistency
   - **Automatically rewrite** `.git/COMMIT_EDITMSG` based on the linthis error hints + diff analysis — do NOT ask for confirmation
4. Re-run `linthis cmsg .git/COMMIT_EDITMSG` to confirm the rewrite passes

## Type Selection Guide

Select the type by examining the staged diff, not by guessing from the message:

| Type | When to use |
|------|-------------|
| **feat** | New feature or functionality |
| **fix** | Bug fix |
| **refactor** | Code restructured, no behavior change |
| **docs** | Documentation only |
| **style** | Formatting, whitespace, lint fixes |
| **test** | Adding or updating tests |
| **build** | Build scripts, deps, CI config |
| **chore** | Maintenance, tooling |

## Examples

**Good:**
```
feat: add user authentication module
fix(parser): handle nil pointer when input is empty
docs: update README with setup instructions
refactor(core): extract common utility functions
```

**Bad → Fixed:**
```
# Bad: wrong type (diff shows bug fix)
feat: fix login crash on empty password
# Fixed:
fix(auth): handle empty password input gracefully

# Bad: vague, no type
update code
# Fixed (based on diff):
refactor(utils): extract shared validation logic
```"#
        .to_string()
}

fn agent_review_body() -> String {
    r#"## Goal

Catch issues that lint can't — logic errors, security vulnerabilities, architectural problems, and missing test coverage. This is the last automated quality gate before code reaches the remote, so focus on issues that would be costly to fix after pushing.

## When to Skip

If there are no outgoing commits (local is up-to-date with remote), approve immediately.

## Steps

### Step 1 — Gather diff

```bash
BASE_SHA=$(git merge-base HEAD origin/main 2>/dev/null || git rev-parse HEAD~1)
HEAD_SHA=$(git rev-parse HEAD)
git diff "$BASE_SHA".."$HEAD_SHA" --stat
git diff "$BASE_SHA".."$HEAD_SHA" --name-status
git diff "$BASE_SHA".."$HEAD_SHA"
```

### Step 2 — Print diff stats

```
📊 Diff Stats
  Base:  <BASE_SHA>
  Head:  <HEAD_SHA>
  Files: N changed, +X insertions, -Y deletions

📁 Changed Files
  ✅ M  src/foo.rs
  ⚠️ A  src/bar.rs   (new file — review carefully)
  ⏭️ D  src/old.rs   (deleted)
```

### Step 3 — Review by category

| Category | What to look for | Severity |
|---|---|---|
| **Critical** | Security vulnerabilities (injection, hardcoded secrets), data loss risk, logic errors, broken API | Blocking |
| **Important** | Missing error handling, untested edge cases, performance issues, missing test coverage | Should fix |
| **Minor** | Style inconsistencies, redundant code, missing comments | Optional |

Focus on the diff, not the whole file — only review what changed. Explain **why** something is a problem and suggest concrete fixes.

### Step 4 — Write structured review

Output to terminal AND write to `.linthis/review/result/review-<YYYYMMDD-HHMMSS>.md`:

```markdown
# Code Review — <HEAD_SHA>
Date: <timestamp>
Base: <BASE_SHA> → Head: <HEAD_SHA>
Files: N changed, +X -Y

## Summary
<1-3 sentence overall assessment>

## Critical Issues
- [ ] <file>:<line> — <description>

## Important Issues
- [ ] <file>:<line> — <description>

## Minor Issues
- [ ] <file>:<line> — <description>

## Assessment
BLOCK / PROCEED WITH FIXES / APPROVED
```

Create `.linthis/review/result/` directory if it doesn't exist.

### Step 5 — Gate the push

- **Critical issues** → output `❌ Push blocked — fix Critical issues first`; do not proceed
- **Important issues only** → output `⚠️ Push with caution`; ask user to confirm
- **Minor or none** → output `✅ Review passed`; proceed

## Review Principles

- **Don't nitpick style** — that's what the lint skill handles. Focus on logic, security, and architecture
- **Explain why** — "SQL injection lets attackers execute arbitrary queries" is more actionable than just "SQL injection found"
- **Suggest concrete fixes** — show corrected code when possible, not just "fix this""#
        .to_string()
}

/// Generate the Stop hook JSON content for .claude/settings.json
const AGENT_STOP_HOOK_JSON: &str = r#"{
  "hooks": {
    "Stop": [
      {
        "hooks": [
          {
            "type": "prompt",
            "prompt": "Before finishing, check if any code files were modified during this session (Write/Edit/Bash tools). If code was modified:\n1. Run `linthis -i <file1> -i <file2>` on all modified files to check for lint issues\n2. If issues are found, fix them yourself by editing the code directly (do NOT use `linthis --fix` or `linthis fix`)\n3. Re-run `linthis -i <files>` to confirm all issues are resolved\n4. Only approve stopping once lint passes with no errors\n\nIf no code files were modified, approve stopping immediately.\n\nYou MUST respond with valid JSON: {\"ok\": true} to approve stopping, or {\"ok\": false, \"reason\": \"description of remaining lint issues\"} to block."
          }
        ]
      }
    ]
  }
}"#;

fn agent_stop_hook_json_ref() -> &'static str {
    AGENT_STOP_HOOK_JSON
}

/// Get the short event name used for skill file naming.
fn event_short_name(event: &HookEvent) -> &'static str {
    match event {
        HookEvent::PreCommit => "lint",
        HookEvent::CommitMsg => "cmsg",
        HookEvent::PrePush => "review",
    }
}

/// Resolve the custom or default skill name for an event.
fn resolve_skill_name(
    event: &HookEvent,
    skill_names: Option<&linthis::config::AgentSkillNamesConfig>,
    prefix: &str,
) -> String {
    let custom: Option<&str> = skill_names.and_then(|sn| match event {
        HookEvent::PreCommit => sn.pre_commit.as_deref(),
        HookEvent::CommitMsg => sn.commit_msg.as_deref(),
        HookEvent::PrePush => sn.pre_push.as_deref(),
    });
    custom.map_or_else(
        || format!("{}{}", prefix, event_short_name(event)),
        |n| n.to_string(),
    )
}

/// Get the skill file path for a given agent provider and hook event.
pub(crate) fn agent_skill_path(
    base: &std::path::Path,
    provider: &AgentProvider,
    global: bool,
    event: &HookEvent,
    skill_names: Option<&linthis::config::AgentSkillNamesConfig>,
) -> PathBuf {
    match provider {
        AgentProvider::Claude => {
            let dir_name = resolve_skill_name(event, skill_names, "lt-");
            base.join(".claude/skills").join(dir_name).join("SKILL.md")
        }
        AgentProvider::Codex => {
            if global {
                base.join(".codex/AGENTS.md")
            } else {
                base.join("AGENTS.md")
            }
        }
        AgentProvider::Gemini => {
            let name = resolve_skill_name(event, skill_names, "linthis-");
            base.join(".gemini").join(format!("{}.md", name))
        }
        AgentProvider::Cursor => {
            let name = resolve_skill_name(event, skill_names, "linthis-");
            base.join(".cursor/rules").join(format!("{}.mdc", name))
        }
        AgentProvider::Droid => {
            let name = resolve_skill_name(event, skill_names, "linthis-");
            base.join(".droid/rules").join(format!("{}.md", name))
        }
        AgentProvider::Auggie => {
            let name = resolve_skill_name(event, skill_names, "linthis-");
            base.join(".augment/rules").join(format!("{}.md", name))
        }
        AgentProvider::Codebuddy => {
            let dir_name = resolve_skill_name(event, skill_names, "lt-");
            base.join(".codebuddy/skills")
                .join(dir_name)
                .join("SKILL.md")
        }
        AgentProvider::Openclaw => {
            let dir_name = resolve_skill_name(event, skill_names, "lt-");
            base.join(".openclaw/skills")
                .join(dir_name)
                .join("SKILL.md")
        }
    }
}

/// Get the Stop Hook settings file path for providers that support it.
pub(crate) fn agent_stop_hook_settings_path(
    base: &std::path::Path,
    provider: &AgentProvider,
) -> Option<PathBuf> {
    match provider {
        AgentProvider::Claude => Some(base.join(".claude/settings.json")),
        AgentProvider::Codebuddy => Some(base.join(".codebuddy/settings.json")),
        _ => None,
    }
}

/// Print extra installed file messages (Stop Hook)
fn print_extra_installed(base: &std::path::Path, provider: &AgentProvider) {
    if let Some(settings_path) = agent_stop_hook_settings_path(base, provider) {
        println!(
            "{} Installed Stop Hook → {}",
            "✓".green(),
            settings_path.display()
        );
    }
}

/// Print info about an already-installed agent provider (file path + content)
fn print_agent_installed_info(
    base: &std::path::Path,
    provider: &AgentProvider,
    global: bool,
    skill_names: Option<&linthis::config::AgentSkillNamesConfig>,
) {
    let events = [
        HookEvent::PreCommit,
        HookEvent::CommitMsg,
        HookEvent::PrePush,
    ];
    for event in &events {
        let path = agent_skill_path(base, provider, global, event, skill_names);
        if path.exists() {
            let event_name = match event {
                HookEvent::PreCommit => "pre-commit",
                HookEvent::CommitMsg => "commit-msg",
                HookEvent::PrePush => "pre-push",
            };
            println!(
                "       {} {} ({})",
                "File:".dimmed(),
                path.display(),
                event_name
            );
        }
    }
    if let Some(settings_path) = agent_stop_hook_settings_path(base, provider) {
        if settings_path.exists() {
            println!("       {} {}", "File:".dimmed(), settings_path.display());
        }
    }
}

/// Check if agent integration is installed for a given provider
pub(crate) fn agent_is_installed(
    base: &std::path::Path,
    provider: &AgentProvider,
    global: bool,
    skill_names: Option<&linthis::config::AgentSkillNamesConfig>,
) -> bool {
    let events = [
        HookEvent::PreCommit,
        HookEvent::CommitMsg,
        HookEvent::PrePush,
    ];
    match provider {
        AgentProvider::Codex => {
            let path = agent_skill_path(base, provider, global, &HookEvent::PreCommit, skill_names);
            path.exists()
                && std::fs::read_to_string(&path)
                    .map(|c| {
                        c.contains(AGENT_SECTION_MARKER)
                            || c.contains("## Linthis Commit Message Rule")
                            || c.contains("## Linthis Review Rule")
                            || c.contains(AGENT_SECTION_MARKER_LEGACY)
                    })
                    .unwrap_or(false)
        }
        AgentProvider::Claude | AgentProvider::Codebuddy | AgentProvider::Openclaw => events
            .iter()
            .any(|e| agent_skill_path(base, provider, global, e, skill_names).exists()),
        _ => events
            .iter()
            .any(|e| agent_skill_path(base, provider, global, e, skill_names).exists()),
    }
}

/// Detect which agent providers are likely in use (by checking for their directories).
fn detect_agent_providers(base: &std::path::Path) -> Vec<AgentProvider> {
    let mut detected = Vec::new();
    if base.join(".claude").exists() {
        detected.push(AgentProvider::Claude);
    }
    if base.join("AGENTS.md").exists() || base.join(".codex").exists() {
        detected.push(AgentProvider::Codex);
    }
    if base.join(".gemini").exists() {
        detected.push(AgentProvider::Gemini);
    }
    if base.join(".cursor").exists() {
        detected.push(AgentProvider::Cursor);
    }
    if base.join(".droid").exists() {
        detected.push(AgentProvider::Droid);
    }
    if base.join(".augment").exists() {
        detected.push(AgentProvider::Auggie);
    }
    if base.join("CODEBUDDY.md").exists() || base.join(".codebuddy").exists() {
        detected.push(AgentProvider::Codebuddy);
    }
    if base.join(".openclaw").exists() {
        detected.push(AgentProvider::Openclaw);
    }
    detected
}

/// Lightweight agent provider detection for dynamic help text.
pub fn detect_agent_providers_lightweight() -> Vec<(&'static str, bool)> {
    let root = linthis::utils::get_project_root();
    ALL_AGENT_PROVIDERS
        .iter()
        .map(|p| {
            let name = match p {
                AgentProvider::Claude => "Claude Code",
                AgentProvider::Codex => "Codex",
                AgentProvider::Gemini => "Gemini",
                AgentProvider::Cursor => "Cursor",
                AgentProvider::Droid => "Droid",
                AgentProvider::Auggie => "Auggie",
                AgentProvider::Codebuddy => "CodeBuddy",
                AgentProvider::Openclaw => "OpenClaw",
            };
            let detected = match p {
                AgentProvider::Claude => root.join(".claude").exists(),
                AgentProvider::Codex => root.join("AGENTS.md").exists(),
                AgentProvider::Gemini => root.join(".gemini").exists(),
                AgentProvider::Cursor => root.join(".cursor").exists(),
                AgentProvider::Droid => root.join(".droid").exists(),
                AgentProvider::Auggie => root.join(".augment").exists(),
                AgentProvider::Codebuddy => {
                    root.join("CODEBUDDY.md").exists() || root.join(".codebuddy").exists()
                }
                AgentProvider::Openclaw => root.join(".openclaw").exists(),
            };
            (name, detected)
        })
        .collect()
}

/// Install a dedicated skill file
fn install_agent_dedicated_file(path: &std::path::Path, content: &str) -> Result<(), String> {
    use std::fs;
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create directory {}: {}", parent.display(), e))?;
        }
    }
    fs::write(path, content).map_err(|e| format!("Failed to write {}: {}", path.display(), e))?;
    Ok(())
}

/// Recursively copy a directory tree from `src` to `dst`.
pub(crate) fn copy_dir_recursive(
    src: &std::path::Path,
    dst: &std::path::Path,
) -> Result<(), String> {
    use std::fs;
    if !dst.exists() {
        fs::create_dir_all(dst)
            .map_err(|e| format!("Failed to create directory {}: {}", dst.display(), e))?;
    }
    for entry in fs::read_dir(src)
        .map_err(|e| format!("Failed to read directory {}: {}", src.display(), e))?
    {
        let entry = entry.map_err(|e| format!("Failed to read dir entry: {}", e))?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path).map_err(|e| {
                format!(
                    "Failed to copy {} → {}: {}",
                    src_path.display(),
                    dst_path.display(),
                    e
                )
            })?;
        }
    }
    Ok(())
}

/// Map an event to its built-in agent plugin ID.
fn agent_plugin_id(_event: &HookEvent) -> &'static str {
    "lt"
}

/// Target directory for agent slash commands per provider.
fn agent_command_dir(
    base: &std::path::Path,
    provider: &AgentProvider,
) -> Option<std::path::PathBuf> {
    match provider {
        AgentProvider::Claude => Some(base.join(".claude/commands/linthis")),
        AgentProvider::Codebuddy => Some(base.join(".codebuddy/commands/linthis")),
        AgentProvider::Gemini => Some(base.join(".gemini/commands")),
        AgentProvider::Cursor => Some(base.join(".cursor/commands")),
        AgentProvider::Droid => Some(base.join(".droid/commands")),
        AgentProvider::Auggie => Some(base.join(".augment/commands")),
        AgentProvider::Codex => None,
        AgentProvider::Openclaw => Some(base.join(".openclaw/commands")),
    }
}

/// Install skill component from a plugin directory.
fn install_plugin_skill(
    plugin_dir: &std::path::Path,
    base: &std::path::Path,
    provider: &AgentProvider,
    event: &HookEvent,
    skill_names: Option<&linthis::config::AgentSkillNamesConfig>,
    target: Option<&linthis::config::AgentTargetConfig>,
) -> Result<(), String> {
    use std::fs;
    let (skill_name, _) = agent_event_skill_metadata(event, skill_names);
    let skill_src_dir = plugin_dir.join("skills").join(&skill_name);
    let skill_src = skill_src_dir.join("SKILL.md");
    if !skill_src.is_file() {
        return Ok(());
    }
    if let Some(target_skills) = target.and_then(|t| t.skills.as_deref()) {
        let custom_skill_dir = base.join(target_skills).join(&skill_name);
        return copy_dir_recursive(&skill_src_dir, &custom_skill_dir);
    }
    let skill_path = agent_skill_path(base, provider, false, event, skill_names);
    match provider {
        AgentProvider::Codex => {
            let content = fs::read_to_string(&skill_src).map_err(|e| {
                format!("Failed to read skill file '{}': {}", skill_src.display(), e)
            })?;
            install_agent_append_section(
                &skill_path,
                &content,
                agent_event_section_marker(event),
                "# Agent Instructions\n",
            )
        }
        AgentProvider::Claude | AgentProvider::Codebuddy | AgentProvider::Openclaw => {
            let target_dir = skill_path.parent().unwrap();
            copy_dir_recursive(&skill_src_dir, target_dir)?;
            if matches!(provider, AgentProvider::Openclaw) {
                openclaw_post_install_skill(target_dir);
            }
            Ok(())
        }
        _ => {
            let content = fs::read_to_string(&skill_src).map_err(|e| {
                format!("Failed to read skill file '{}': {}", skill_src.display(), e)
            })?;
            install_agent_dedicated_file(&skill_path, &content)
        }
    }
}

/// Install command files from a plugin directory.
fn install_plugin_commands(
    plugin_dir: &std::path::Path,
    base: &std::path::Path,
    provider: &AgentProvider,
    target: Option<&linthis::config::AgentTargetConfig>,
) -> Result<(), String> {
    use std::fs;
    let cmd_src_dir = plugin_dir.join("commands");
    if !cmd_src_dir.is_dir() {
        return Ok(());
    }
    let cmd_dir = if let Some(target_commands) = target.and_then(|t| t.commands.as_deref()) {
        Some(base.join(target_commands))
    } else {
        agent_command_dir(base, provider)
    };
    if let Some(cmd_dir) = cmd_dir {
        if let Ok(entries) = fs::read_dir(&cmd_src_dir) {
            for entry in entries.flatten() {
                if entry.path().is_file() {
                    let cmd_target = cmd_dir.join(entry.file_name());
                    let content = fs::read_to_string(entry.path()).map_err(|e| {
                        format!(
                            "Failed to read command file '{}': {}",
                            entry.path().display(),
                            e
                        )
                    })?;
                    install_agent_dedicated_file(&cmd_target, &content)?;
                }
            }
        }
    }
    Ok(())
}

/// Get the default memory file path for a provider.
fn agent_memory_path(base: &std::path::Path, provider: &AgentProvider) -> Option<PathBuf> {
    match provider {
        AgentProvider::Claude => Some(base.join("CLAUDE.md")),
        AgentProvider::Codebuddy => Some(base.join("CODEBUDDY.md")),
        AgentProvider::Gemini => Some(base.join(".gemini/GEMINI.md")),
        AgentProvider::Cursor => Some(base.join(".cursor/CURSOR.md")),
        AgentProvider::Droid => Some(base.join(".droid/DROID.md")),
        AgentProvider::Auggie => Some(base.join(".augment/AUGMENT.md")),
        AgentProvider::Codex => None,
        AgentProvider::Openclaw => Some(base.join("AGENTS.md")),
    }
}

/// Install memory component from a plugin directory.
fn install_plugin_memory(
    plugin_dir: &std::path::Path,
    base: &std::path::Path,
    provider: &AgentProvider,
    event: &HookEvent,
    target: Option<&linthis::config::AgentTargetConfig>,
) -> Result<(), String> {
    let mem_src = plugin_dir.join("memories").join("TOPLEVEL.md");
    if !mem_src.is_file() {
        return Ok(());
    }
    let memory_target = if let Some(target_memory) = target.and_then(|t| t.memory.as_deref()) {
        Some(base.join(target_memory))
    } else {
        agent_memory_path(base, provider)
    };
    if let Some(mem_target) = memory_target {
        let content = std::fs::read_to_string(&mem_src)
            .map_err(|e| format!("Failed to read memory file '{}': {}", mem_src.display(), e))?;
        let plugin_id = agent_plugin_id(event);
        let section_marker = &format!("linthis-memory-{}", plugin_id);
        install_agent_append_section(&mem_target, &content, section_marker, "")?;
    }
    Ok(())
}

/// Install stop hook from a plugin's hooks/hooks.json.
fn install_plugin_stop_hook(
    plugin_dir: &std::path::Path,
    base: &std::path::Path,
    provider: &AgentProvider,
    target: Option<&linthis::config::AgentTargetConfig>,
) -> Result<(), String> {
    let hooks_json_src = plugin_dir.join("hooks").join("hooks.json");
    if !hooks_json_src.is_file() || matches!(provider, AgentProvider::Openclaw) {
        return Ok(());
    }
    let settings_path = if let Some(target_settings) = target.and_then(|t| t.settings.as_deref()) {
        Some(base.join(target_settings))
    } else {
        agent_stop_hook_settings_path(base, provider)
    };
    if let Some(settings_path) = settings_path {
        let override_json = std::fs::read_to_string(&hooks_json_src).map_err(|e| {
            format!(
                "Failed to read hooks.json '{}': {}",
                hooks_json_src.display(),
                e
            )
        })?;
        install_agent_stop_hook_from_json(base, &settings_path, &override_json)?;
    }
    Ok(())
}

/// Resolve and install agent plugin components from a plugin directory.
pub(crate) fn install_agent_plugin_from_dir(
    plugin_dir: &std::path::Path,
    base: &std::path::Path,
    provider: &AgentProvider,
    event: &HookEvent,
    skill_names: Option<&linthis::config::AgentSkillNamesConfig>,
    target: Option<&linthis::config::AgentTargetConfig>,
) -> Result<(), String> {
    install_plugin_skill(plugin_dir, base, provider, event, skill_names, target)?;
    install_plugin_commands(plugin_dir, base, provider, target)?;
    install_plugin_memory(plugin_dir, base, provider, event, target)?;
    install_plugin_stop_hook(plugin_dir, base, provider, target)?;
    Ok(())
}

/// Resolve an agent plugin entry from the nested `[hook.agent.plugins]` config.
fn resolve_agent_plugin<'a>(
    hook_config: &'a linthis::config::HookConfig,
    plugin_id: &str,
    provider: &str,
) -> Option<&'a linthis::config::HookSourceEntry> {
    hook_config
        .agent
        .plugins
        .get(provider)
        .and_then(|m| m.get(plugin_id))
        .or_else(|| {
            hook_config
                .agent
                .plugins
                .get("_default")
                .and_then(|m| m.get(plugin_id))
        })
}

/// Tier-1/2 override check for an agent plugin.
fn resolve_and_install_agent_plugin_override(
    base: &std::path::Path,
    provider: &AgentProvider,
    event: &HookEvent,
    global: bool,
    skill_names: Option<&linthis::config::AgentSkillNamesConfig>,
) -> Result<bool, String> {
    use linthis::config::Config;
    use linthis::hooks::resolver;

    let plugin_id = agent_plugin_id(event);
    let provider_name = format!("{:?}", provider).to_lowercase();
    let project_root = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));

    // Tier 1: fixed-path plugin directory (project-local only)
    if !global {
        if let Some(plugin_dir) =
            resolver::fixed_agent_plugin_dir(&project_root, &provider_name, plugin_id)
        {
            install_agent_plugin_from_dir(&plugin_dir, base, provider, event, skill_names, None)?;
            return Ok(true);
        }
    }

    // Tier 2: TOML agent plugin entry with provider fallback
    let config = Config::load_merged(&project_root);
    if let Some(entry) = resolve_agent_plugin(&config.hook, plugin_id, &provider_name) {
        let resolved =
            resolver::resolve_to_dir(&entry.source, &project_root, &config.hook.marketplaces)
                .map_err(|e| format!("Failed to resolve agent plugin '{}': {}", plugin_id, e))?;
        install_agent_plugin_from_dir(
            resolved.path(),
            base,
            provider,
            event,
            skill_names,
            entry.target.as_ref(),
        )?;
        return Ok(true);
    }

    // Tier 2.5: Scan cached plugin directories
    {
        use linthis::plugin::{PluginCache, PluginConfigManager};

        let managers: Vec<_> = if global {
            [PluginConfigManager::global()]
                .into_iter()
                .filter_map(|r| r.ok())
                .collect()
        } else {
            [PluginConfigManager::project()]
                .into_iter()
                .filter_map(|r| r.ok())
                .collect()
        };

        if let Ok(cache) = PluginCache::new() {
            for mgr in &managers {
                if let Ok(plugins) = mgr.list_plugins() {
                    for (_name, url, _ref) in &plugins {
                        let cache_path = cache.url_to_cache_path(url);
                        let provider_dir = cache_path
                            .join("hooks/agent/plugins")
                            .join(&provider_name)
                            .join(plugin_id);
                        if provider_dir.is_dir() {
                            install_agent_plugin_from_dir(
                                &provider_dir,
                                base,
                                provider,
                                event,
                                skill_names,
                                None,
                            )?;
                            return Ok(true);
                        }
                        let default_dir = cache_path
                            .join("hooks/agent/plugins/_default")
                            .join(plugin_id);
                        if default_dir.is_dir() {
                            install_agent_plugin_from_dir(
                                &default_dir,
                                base,
                                provider,
                                event,
                                skill_names,
                                None,
                            )?;
                            return Ok(true);
                        }
                    }
                }
            }
        }
    }

    Ok(false)
}

/// Resolve the OpenClaw global skills directory.
fn resolve_openclaw_global_skills_dir() -> Option<PathBuf> {
    if let Some(home) = dirs::home_dir() {
        let candidate = home.join(".openclaw/skills");
        if candidate.is_dir() {
            return Some(candidate);
        }
    }
    let win_candidate = PathBuf::from(r"C:\openclaw\openclaw\source\node_modules\openclaw\skills");
    if win_candidate.is_dir() {
        return Some(win_candidate);
    }
    None
}

/// Register an OpenClaw skill via CLI after files are written.
fn openclaw_post_install_skill(skill_dir: &std::path::Path) {
    use std::process::Command;

    if is_command_available("openclaw") {
        match Command::new("openclaw")
            .args(["skills", "install", &skill_dir.to_string_lossy()])
            .output()
        {
            Ok(output) if output.status.success() => {
                println!(
                    "  {} Registered skill via 'openclaw skills install'",
                    "✓".green()
                );
                println!("  {} Verify with 'openclaw skills list'", "→".dimmed());
                return;
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                println!(
                    "  {} 'openclaw skills install' exited with {}: {}",
                    "Warning".yellow(),
                    output.status,
                    stderr.trim()
                );
            }
            Err(e) => {
                println!(
                    "  {} Failed to run 'openclaw skills install': {}",
                    "Warning".yellow(),
                    e
                );
            }
        }
    }

    let skill_name = match skill_dir.file_name() {
        Some(name) => name,
        None => {
            println!(
                "  {} Could not determine skill name from path '{}'",
                "Warning".yellow(),
                skill_dir.display()
            );
            return;
        }
    };

    if let Some(global_skills) = resolve_openclaw_global_skills_dir() {
        let target = global_skills.join(skill_name);
        match copy_dir_recursive(skill_dir, &target) {
            Ok(()) => {
                println!(
                    "  {} Copied skill to {} (CLI unavailable, direct copy fallback)",
                    "✓".green(),
                    target.display()
                );
                println!(
                    "  {} When openclaw CLI is available, verify with 'openclaw skills list'",
                    "→".dimmed()
                );
            }
            Err(e) => {
                println!(
                    "  {} Failed to copy skill to {}: {}",
                    "Warning".yellow(),
                    target.display(),
                    e
                );
            }
        }
    } else {
        println!("  {} 'openclaw' CLI not found and no known skills directory (~/.openclaw/skills/) detected.", "Notice".cyan());
        println!(
            "  {} Run 'openclaw skills install {}' manually after installing OpenClaw.",
            "→".dimmed(),
            skill_dir.display()
        );
    }
}

/// Unregister an OpenClaw skill via CLI before files are removed.
fn openclaw_post_uninstall_skill(skill_dir: &std::path::Path) {
    use std::process::Command;

    if is_command_available("openclaw") {
        match Command::new("openclaw")
            .args(["skills", "uninstall", &skill_dir.to_string_lossy()])
            .output()
        {
            Ok(output) if output.status.success() => {
                println!(
                    "  {} Unregistered skill via 'openclaw skills uninstall'",
                    "✓".green()
                );
                return;
            }
            _ => {}
        }
    }

    let skill_name = match skill_dir.file_name() {
        Some(name) => name,
        None => return,
    };
    if let Some(global_skills) = resolve_openclaw_global_skills_dir() {
        let target = global_skills.join(skill_name);
        if target.is_dir() {
            if let Err(e) = std::fs::remove_dir_all(&target) {
                println!(
                    "  {} Failed to remove skill dir {}: {}",
                    "Warning".yellow(),
                    target.display(),
                    e
                );
            } else {
                println!(
                    "  {} Removed skill from {} (direct removal fallback)",
                    "✓".green(),
                    target.display()
                );
            }
        }
    }
}

/// Install a single agent skill for a given provider and event.
pub(crate) fn install_agent_skill(
    base: &std::path::Path,
    provider: &AgentProvider,
    global: bool,
    event: &HookEvent,
    skill_names: Option<&linthis::config::AgentSkillNamesConfig>,
) -> Result<(), String> {
    match resolve_and_install_agent_plugin_override(base, provider, event, global, skill_names) {
        Ok(true) => return Ok(()),
        Ok(false) => {}
        Err(e) => return Err(e),
    }

    let skill_path = agent_skill_path(base, provider, global, event, skill_names);
    let content = agent_event_content_for_provider(provider, event, skill_names);

    match provider {
        AgentProvider::Codex => {
            let section_marker = agent_event_section_marker(event);
            install_agent_append_section(
                &skill_path,
                &content,
                section_marker,
                "# Agent Instructions\n",
            )?;
        }
        AgentProvider::Claude | AgentProvider::Codebuddy | AgentProvider::Openclaw => {
            install_agent_dedicated_file(&skill_path, &content)?;
            if matches!(event, HookEvent::PreCommit) {
                if let Some(settings_path) = agent_stop_hook_settings_path(base, provider) {
                    install_agent_stop_hook(base, provider, &settings_path)?;
                }
            }
            if matches!(provider, AgentProvider::Openclaw) {
                if let Some(skill_dir) = skill_path.parent() {
                    openclaw_post_install_skill(skill_dir);
                }
            }
        }
        _ => {
            install_agent_dedicated_file(&skill_path, &content)?;
        }
    }
    Ok(())
}

/// Uninstall a single agent skill for a given provider and event.
fn uninstall_agent_skill(
    base: &std::path::Path,
    provider: &AgentProvider,
    global: bool,
    event: &HookEvent,
    skill_names: Option<&linthis::config::AgentSkillNamesConfig>,
) -> Result<(), String> {
    let skill_path = agent_skill_path(base, provider, global, event, skill_names);

    match provider {
        AgentProvider::Codex => {
            if skill_path.exists() {
                let section_marker = agent_event_section_marker(event);
                remove_agent_section_by_marker(&skill_path, section_marker)?;
            }
        }
        AgentProvider::Claude | AgentProvider::Codebuddy | AgentProvider::Openclaw => {
            if skill_path.exists() {
                if matches!(provider, AgentProvider::Openclaw) {
                    if let Some(skill_dir) = skill_path.parent() {
                        openclaw_post_uninstall_skill(skill_dir);
                    }
                }
                remove_agent_dedicated_file(&skill_path)?;
            }
            if matches!(event, HookEvent::PreCommit) {
                if let Some(settings_path) = agent_stop_hook_settings_path(base, provider) {
                    if settings_path.exists() {
                        remove_agent_stop_hook(&settings_path)?;
                    }
                }
            }
        }
        _ => {
            if skill_path.exists() {
                remove_agent_dedicated_file(&skill_path)?;
            }
        }
    }
    Ok(())
}

/// Format skill content for a provider + event.
fn agent_event_content_for_provider(
    provider: &AgentProvider,
    event: &HookEvent,
    skill_names: Option<&linthis::config::AgentSkillNamesConfig>,
) -> String {
    let body = agent_event_content_generic(event);
    let (name, desc) = agent_event_skill_metadata(event, skill_names);
    match provider {
        AgentProvider::Codex => body,
        AgentProvider::Claude | AgentProvider::Codebuddy | AgentProvider::Openclaw => {
            format!(
                "---\nname: {}\ndescription: {}\n---\n\n# {}\n\n{}\n",
                name, desc, name, body
            )
        }
        AgentProvider::Gemini | AgentProvider::Droid | AgentProvider::Auggie => {
            format!(
                "---\nname: {}\ndescription: {}\n---\n\n# {}\n\n{}\n",
                name, desc, name, body
            )
        }
        AgentProvider::Cursor => {
            format!(
                "---\ndescription: {}\nalwaysApply: true\n---\n\n# {}\n\n{}\n",
                desc, name, body
            )
        }
    }
}

pub(crate) fn agent_event_skill_metadata(
    event: &HookEvent,
    skill_names: Option<&linthis::config::AgentSkillNamesConfig>,
) -> (String, &'static str) {
    let custom_name: Option<&str> = skill_names.and_then(|sn| match event {
        HookEvent::PreCommit => sn.pre_commit.as_deref(),
        HookEvent::CommitMsg => sn.commit_msg.as_deref(),
        HookEvent::PrePush => sn.pre_push.as_deref(),
    });
    match event {
        HookEvent::PreCommit => (
            custom_name.unwrap_or("lt-lint").to_string(),
            "对暂存/修改的代码文件运行 linthis 代码检查，提交前修复所有问题。使用 `linthis -i <file>` 按项目编码规范检查，必须手动编辑修复（不能用 linthis --fix）。提交时若 lt-cmsg skill 也存在应一起触发。Run linthis lint checks on staged/modified code files and fix all issues before committing. Uses `linthis -i <file>`. Issues must be fixed by editing code directly. If the lt-cmsg skill also exists, both should be invoked together when committing.",
        ),
        HookEvent::CommitMsg => (
            custom_name.unwrap_or("lt-cmsg").to_string(),
            "验证并自动修复 git 提交信息，使其符合 Conventional Commits 规范。分析暂存区 diff 选择正确的 type 前缀（feat/fix/refactor 等），检查标题格式，自动改写不合规的提交信息。提交时若 lt-lint skill 也存在应一起触发。Validate and auto-fix git commit messages to comply with Conventional Commits. Analyzes staged diff to select correct type prefix, checks format, auto-rewrites malformed messages. If the lt-lint skill also exists, both should be invoked together when committing.",
        ),
        HookEvent::PrePush => (
            custom_name.unwrap_or("lt-review").to_string(),
            "推送前审查待推送的提交，检查代码质量、安全性和正确性问题。检查完整 diff 发现逻辑错误、安全漏洞（注入、硬编码密钥）、代码质量问题及测试覆盖缺失。由 pre-push hook 触发。Review outgoing commits for quality, security, and correctness before pushing. Catches logic errors, security vulnerabilities, code quality issues. Triggered by pre-push hook.",
        ),
    }
}

fn agent_event_section_marker(event: &HookEvent) -> &'static str {
    match event {
        HookEvent::PreCommit => "## Linthis Lint Rule",
        HookEvent::CommitMsg => "## Linthis Commit Message Rule",
        HookEvent::PrePush => "## Linthis Review Rule",
    }
}

/// Append or replace a section (identified by marker) in a file.
fn install_agent_append_section(
    path: &std::path::Path,
    content: &str,
    section_marker: &str,
    file_header: &str,
) -> Result<(), String> {
    use std::fs;
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent).map_err(|e| format!("create dir: {}", e))?;
        }
    }
    let section = format!("\n{}\n\n{}\n", section_marker, content);
    if path.exists() {
        let existing =
            fs::read_to_string(path).map_err(|e| format!("read {}: {}", path.display(), e))?;
        if existing.contains(section_marker) {
            let start = existing.find(section_marker).unwrap();
            let after = &existing[start + section_marker.len()..];
            let end = after
                .find("\n## ")
                .map(|i| start + section_marker.len() + i)
                .unwrap_or(existing.len());
            let updated = format!(
                "{}{}{}",
                &existing[..start],
                &section[1..],
                &existing[end..]
            );
            fs::write(path, updated).map_err(|e| format!("write {}: {}", path.display(), e))?;
        } else {
            let mut f = std::fs::OpenOptions::new()
                .append(true)
                .open(path)
                .map_err(|e| format!("open {}: {}", path.display(), e))?;
            use std::io::Write;
            f.write_all(section.as_bytes())
                .map_err(|e| format!("write {}: {}", path.display(), e))?;
        }
    } else {
        fs::write(path, format!("{}{}", file_header, section))
            .map_err(|e| format!("write {}: {}", path.display(), e))?;
    }
    Ok(())
}

/// Remove a specific section (by marker) from a file.
fn remove_agent_section_by_marker(path: &std::path::Path, marker: &str) -> Result<(), String> {
    let content =
        std::fs::read_to_string(path).map_err(|e| format!("read {}: {}", path.display(), e))?;
    if !content.contains(marker) {
        return Ok(());
    }
    let start = content.find(marker).unwrap();
    let after = &content[start + marker.len()..];
    let end = after
        .find("\n## ")
        .map(|i| start + marker.len() + i)
        .unwrap_or(content.len());
    let trim_start = if start > 0 && content.as_bytes()[start - 1] == b'\n' {
        start - 1
    } else {
        start
    };
    let updated = format!("{}{}", &content[..trim_start], &content[end..]);
    if updated.trim().is_empty() {
        std::fs::remove_file(path).map_err(|e| format!("remove {}: {}", path.display(), e))?;
    } else {
        std::fs::write(path, updated).map_err(|e| format!("write {}: {}", path.display(), e))?;
    }
    Ok(())
}

/// Remove a dedicated skill file and clean up empty parent directories
fn remove_agent_dedicated_file(path: &std::path::Path) -> Result<(), String> {
    use std::fs;
    if path.exists() {
        fs::remove_file(path).map_err(|e| format!("Failed to remove {}: {}", path.display(), e))?;
        if let Some(parent) = path.parent() {
            let _ = fs::remove_dir(parent);
            if let Some(grandparent) = parent.parent() {
                if grandparent
                    .file_name()
                    .map(|n| n.to_string_lossy().starts_with('.'))
                    .unwrap_or(false)
                {
                    let _ = fs::remove_dir(grandparent);
                }
            }
        }
    }
    Ok(())
}

/// Remove the linthis section from a file (CLAUDE.md, AGENTS.md)
fn remove_agent_section_from_file(path: &std::path::Path) -> Result<(), String> {
    use std::fs;
    let existing = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
    let (start, marker_len) = if let Some(s) = existing.find(AGENT_SECTION_MARKER) {
        (s, AGENT_SECTION_MARKER.len())
    } else if let Some(s) = existing.find(AGENT_SECTION_MARKER_LEGACY) {
        (s, AGENT_SECTION_MARKER_LEGACY.len())
    } else {
        return Ok(());
    };
    let after_marker = &existing[start + marker_len..];
    let section_end = after_marker
        .find("\n## ")
        .map(|pos| start + marker_len + pos)
        .unwrap_or(existing.len());
    let mut result = existing[..start].trim_end().to_string();
    let remaining = existing[section_end..].trim_start();
    if !remaining.is_empty() {
        result.push_str("\n\n");
        result.push_str(remaining);
    }
    if !result.ends_with('\n') {
        result.push('\n');
    }
    fs::write(path, result).map_err(|e| format!("Failed to write {}: {}", path.display(), e))?;
    Ok(())
}

/// Shallow-merge a stop hook JSON string into a settings file.
fn install_agent_stop_hook_from_json(
    _git_root: &std::path::Path,
    settings_path: &std::path::Path,
    override_json: &str,
) -> Result<(), String> {
    use std::fs;
    if let Some(parent) = settings_path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create directory {}: {}", parent.display(), e))?;
        }
    }
    if settings_path.exists() {
        let existing = fs::read_to_string(settings_path)
            .map_err(|e| format!("Failed to read {}: {}", settings_path.display(), e))?;
        let mut json: serde_json::Value = serde_json::from_str(&existing)
            .map_err(|e| format!("Failed to parse {}: {}", settings_path.display(), e))?;
        let override_val: serde_json::Value = serde_json::from_str(override_json)
            .map_err(|e| format!("Failed to parse stop hook JSON: {}", e))?;
        if let (Some(root), Some(override_obj)) = (json.as_object_mut(), override_val.as_object()) {
            for (k, v) in override_obj {
                root.insert(k.clone(), v.clone());
            }
        }
        let output = serde_json::to_string_pretty(&json)
            .map_err(|e| format!("Failed to serialize JSON: {}", e))?;
        fs::write(settings_path, output + "\n")
            .map_err(|e| format!("Failed to write {}: {}", settings_path.display(), e))?;
    } else {
        fs::write(settings_path, override_json.to_string() + "\n")
            .map_err(|e| format!("Failed to write {}: {}", settings_path.display(), e))?;
    }
    Ok(())
}

/// Install the Stop Hook into a settings JSON file.
fn install_agent_stop_hook(
    git_root: &std::path::Path,
    _provider: &AgentProvider,
    settings_path: &std::path::Path,
) -> Result<(), String> {
    let override_json_str: Option<String> = None;
    let override_json = override_json_str
        .as_deref()
        .unwrap_or_else(|| agent_stop_hook_json_ref());
    install_agent_stop_hook_from_json(git_root, settings_path, override_json)
}

/// Remove the Stop Hook from settings.json
fn remove_agent_stop_hook(settings_path: &std::path::Path) -> Result<(), String> {
    use std::fs;
    let existing = fs::read_to_string(settings_path)
        .map_err(|e| format!("Failed to read {}: {}", settings_path.display(), e))?;
    let mut json: serde_json::Value = serde_json::from_str(&existing)
        .map_err(|e| format!("Failed to parse {}: {}", settings_path.display(), e))?;
    if let Some(hooks) = json.get_mut("hooks").and_then(|h| h.as_object_mut()) {
        hooks.remove("Stop");
        if hooks.is_empty() {
            json.as_object_mut().unwrap().remove("hooks");
        }
    }
    if json.as_object().map(|o| o.is_empty()).unwrap_or(false) {
        fs::remove_file(settings_path)
            .map_err(|e| format!("Failed to remove {}: {}", settings_path.display(), e))?;
        if let Some(parent) = settings_path.parent() {
            let _ = fs::remove_dir(parent);
        }
    } else {
        let output = serde_json::to_string_pretty(&json)
            .map_err(|e| format!("Failed to serialize JSON: {}", e))?;
        fs::write(settings_path, output + "\n")
            .map_err(|e| format!("Failed to write {}: {}", settings_path.display(), e))?;
    }
    Ok(())
}

fn warn_legacy_if_present(base: &std::path::Path, provider: &AgentProvider) {
    match provider {
        AgentProvider::Claude => {
            let legacy = base.join("CLAUDE.md");
            if legacy.exists()
                && std::fs::read_to_string(&legacy)
                    .map(|c| c.contains("## Linthis"))
                    .unwrap_or(false)
            {
                println!(
                    "{}: Legacy linthis section detected in {} — you may remove it manually.",
                    "Notice".cyan(),
                    legacy.display()
                );
            }
        }
        AgentProvider::Codebuddy => {
            let legacy_md = base.join("CODEBUDDY.md");
            let legacy_skill = base.join(".codebuddy/skills/linthis/SKILL.md");
            if (legacy_md.exists()
                && std::fs::read_to_string(&legacy_md)
                    .map(|c| c.contains("## Linthis"))
                    .unwrap_or(false))
                || legacy_skill.exists()
            {
                println!("{}: Legacy linthis files detected (CODEBUDDY.md section / SKILL.md) — you may remove them manually.", "Notice".cyan());
            }
        }
        _ => {}
    }
}

fn uninstall_agent_legacy(base: &std::path::Path, provider: &AgentProvider) {
    match provider {
        AgentProvider::Claude => {
            let legacy = base.join("CLAUDE.md");
            if legacy.exists() {
                let _ = remove_agent_section_from_file(&legacy);
            }
        }
        AgentProvider::Codebuddy => {
            let legacy_md = base.join("CODEBUDDY.md");
            if legacy_md.exists() {
                let _ = remove_agent_section_from_file(&legacy_md);
            }
            let legacy_skill = base.join(".codebuddy/skills/linthis/SKILL.md");
            if legacy_skill.exists() {
                let _ = remove_agent_dedicated_file(&legacy_skill);
            }
        }
        _ => {}
    }
}

/// Resolve the base directory for agent hook installation.
pub(crate) fn resolve_agent_base(global: bool) -> Result<PathBuf, ExitCode> {
    if global {
        dirs::home_dir().ok_or_else(|| {
            eprintln!("{}: Could not determine home directory", "Error".red());
            ExitCode::from(1)
        })
    } else {
        find_git_root().ok_or_else(|| {
            eprintln!("{}: Not in a git repository", "Error".red());
            eprintln!("  Run this command from within a git repository, or use --global / -g to install user-level skills");
            ExitCode::from(1)
        })
    }
}

/// Parameters for batch agent provider installation.
struct AgentInstallBatchParams<'a> {
    providers: &'a [&'a AgentProvider],
    base: &'a std::path::Path,
    events: &'a [HookEvent],
    force: bool,
    global: bool,
    scope: &'a str,
    project_str: &'a str,
    skill_names: Option<&'a linthis::config::AgentSkillNamesConfig>,
}

/// Install agent skills for a list of providers across all events.
fn install_agent_providers_batch(params: &AgentInstallBatchParams<'_>) -> bool {
    let mut any_installed = false;
    for p in params.providers {
        if agent_is_installed(params.base, p, params.global, params.skill_names) && !params.force {
            println!("{}: {} already installed", "Info".cyan(), p);
            print_agent_installed_info(params.base, p, params.global, params.skill_names);
            continue;
        }
        warn_legacy_if_present(params.base, p);
        let provider_name = format!("{}", p).to_lowercase();
        let mut provider_ok = true;
        for event in params.events {
            match install_agent_skill(params.base, p, params.global, event, params.skill_names) {
                Ok(_) => {
                    let path =
                        agent_skill_path(params.base, p, params.global, event, params.skill_names);
                    println!(
                        "{} Installed {} ({}) → {}",
                        "✓".green(),
                        p,
                        event.hook_filename(),
                        path.display()
                    );
                    add_skill_provider_to_hook(
                        params.scope,
                        params.project_str,
                        event,
                        &provider_name,
                    );
                }
                Err(e) => {
                    eprintln!(
                        "{}: Failed to install {} ({}): {}",
                        "Error".red(),
                        p,
                        event.hook_filename(),
                        e
                    );
                    provider_ok = false;
                }
            }
        }
        if provider_ok {
            print_extra_installed(params.base, p);
            any_installed = true;
        }
    }
    any_installed
}

/// Build an ordered list of providers: detected/installed first, then others.
fn build_ordered_provider_list<'a>(
    base: &std::path::Path,
    detected: &[AgentProvider],
    global: bool,
    skill_names: Option<&linthis::config::AgentSkillNamesConfig>,
) -> Vec<&'a AgentProvider> {
    let mut ordered: Vec<&AgentProvider> = Vec::new();
    for p in ALL_AGENT_PROVIDERS {
        if detected
            .iter()
            .any(|d| std::mem::discriminant(d) == std::mem::discriminant(p))
            || agent_is_installed(base, p, global, skill_names)
        {
            ordered.push(p);
        }
    }
    for p in ALL_AGENT_PROVIDERS {
        if !ordered
            .iter()
            .any(|o| std::mem::discriminant(*o) == std::mem::discriminant(p))
        {
            ordered.push(p);
        }
    }
    ordered
}

/// Prompt user to select agents from an interactive menu.
fn prompt_agent_selection<'a>(
    ordered: &[&'a AgentProvider],
    detected: &'a [AgentProvider],
    base: &std::path::Path,
    global: bool,
    skill_names: Option<&linthis::config::AgentSkillNamesConfig>,
) -> Option<Vec<&'a AgentProvider>> {
    use std::io::{self, Write};

    let provider_count = ordered.len();
    println!("Select agent(s) to integrate with linthis:");
    println!();

    for (i, p) in ordered.iter().enumerate() {
        let is_installed = agent_is_installed(base, p, global, skill_names);
        let is_detected = detected
            .iter()
            .any(|d| std::mem::discriminant(d) == std::mem::discriminant(p));
        let status = match (is_installed, is_detected) {
            (true, _) => format!(" {}", "(installed)".yellow()),
            (false, true) => format!(" {}", "(detected)".cyan()),
            _ => String::new(),
        };
        println!("  {}. {}{}", i + 1, p, status);
    }

    println!();
    println!("  {}. All detected agents", provider_count + 1);
    println!("  {}. All agents", provider_count + 2);
    println!("  {}. Cancel", provider_count + 3);
    println!();
    print!("Choose (comma-separated for multiple, e.g. 1,2): ");
    io::stdout().flush().unwrap();

    let mut choice = String::new();
    io::stdin().read_line(&mut choice).ok();
    let choice = choice.trim();

    if choice == (provider_count + 3).to_string() || choice.is_empty() {
        return None;
    }
    if choice == (provider_count + 1).to_string() {
        if detected.is_empty() {
            println!("{}: No agents detected, installing all", "Info".cyan());
            return Some(ordered.to_vec());
        }
        return Some(detected.iter().collect());
    }
    if choice == (provider_count + 2).to_string() {
        return Some(ordered.to_vec());
    }

    let selected: Vec<&AgentProvider> = choice
        .split(',')
        .filter_map(|s| s.trim().parse::<usize>().ok())
        .filter(|&n| n >= 1 && n <= provider_count)
        .map(|n| ordered[n - 1])
        .collect();

    if selected.is_empty() {
        None
    } else {
        Some(selected)
    }
}

pub(crate) fn handle_agent_hook_install(
    provider: Option<AgentProvider>,
    events: &[HookEvent],
    force: bool,
    yes: bool,
    global: bool,
) -> ExitCode {
    let base = match resolve_agent_base(global) {
        Ok(b) => b,
        Err(code) => return code,
    };

    let scope = if global { "global" } else { "local" };
    let project_str = if global {
        String::new()
    } else {
        base.to_str().unwrap_or("").to_string()
    };

    let project_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let skill_names_cfg = linthis::config::Config::load_merged(&project_root)
        .hook
        .agent
        .skill_names;
    let skill_names = Some(&skill_names_cfg);

    println!("{}", "🤖 AI Coding Agent Integration".bold());
    if global {
        println!(
            "  {} Installing user-level skills in {}",
            "→".dimmed(),
            base.display()
        );
    }
    println!();

    if let Some(ref p) = provider {
        if agent_is_installed(&base, p, global, skill_names) && !force {
            println!("{}: {} is already installed", "Info".cyan(), p);
            print_agent_installed_info(&base, p, global, skill_names);
            return ExitCode::SUCCESS;
        }
        let providers = vec![p];
        install_agent_providers_batch(&AgentInstallBatchParams {
            providers: &providers,
            base: &base,
            events,
            force,
            global,
            scope,
            project_str: &project_str,
            skill_names,
        });
        return ExitCode::SUCCESS;
    }

    if yes {
        let detected = detect_agent_providers(&base);
        let targets: Vec<&AgentProvider> = if detected.is_empty() {
            ALL_AGENT_PROVIDERS.iter().collect()
        } else {
            detected.iter().collect()
        };
        let any = install_agent_providers_batch(&AgentInstallBatchParams {
            providers: &targets,
            base: &base,
            events,
            force,
            global,
            scope,
            project_str: &project_str,
            skill_names,
        });
        if any {
            println!();
            println!(
                "{}",
                "Agents will auto-check code quality after edits.".bold()
            );
        }
        return ExitCode::SUCCESS;
    }

    let detected = detect_agent_providers(&base);
    let ordered = build_ordered_provider_list(&base, &detected, global, skill_names);

    let selected = match prompt_agent_selection(&ordered, &detected, &base, global, skill_names) {
        Some(s) => s,
        None => {
            println!("Installation cancelled");
            return ExitCode::SUCCESS;
        }
    };

    println!();
    let any = install_agent_providers_batch(&AgentInstallBatchParams {
        providers: &selected,
        base: &base,
        events,
        force,
        global,
        scope,
        project_str: &project_str,
        skill_names,
    });
    if any {
        println!();
        println!(
            "{}",
            "Agents will auto-check code quality after edits.".bold()
        );
    }
    ExitCode::SUCCESS
}

/// Uninstall agent hooks for all installed providers.
pub(crate) fn handle_agent_hook_uninstall(
    yes: bool,
    global: bool,
    events: &[HookEvent],
) -> ExitCode {
    use std::io::{self, Write};

    let base = match resolve_agent_base(global) {
        Ok(b) => b,
        Err(code) => return code,
    };

    let project_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let skill_names_cfg = linthis::config::Config::load_merged(&project_root)
        .hook
        .agent
        .skill_names;
    let skill_names = Some(&skill_names_cfg);

    let installed: Vec<&AgentProvider> = ALL_AGENT_PROVIDERS
        .iter()
        .filter(|p| agent_is_installed(&base, p, global, skill_names))
        .collect();

    if installed.is_empty() {
        return ExitCode::from(1);
    }

    if !yes {
        println!("{}", "Agent Integration:".bold());
        for p in &installed {
            let path = agent_skill_path(&base, p, global, &HookEvent::PreCommit, skill_names);
            println!("  {} {} ({})", "✓".green(), p, path.display());
        }
        println!();
        print!("Remove agent integration? [y/N]: ");
        io::stdout().flush().unwrap();
        let mut answer = String::new();
        io::stdin().read_line(&mut answer).ok();
        if !answer.trim().eq_ignore_ascii_case("y") {
            println!("Uninstall cancelled");
            return ExitCode::SUCCESS;
        }
    }

    let scope = if global { "global" } else { "local" };
    let project_str = if global {
        String::new()
    } else {
        base.to_str().unwrap_or("").to_string()
    };

    let mut any_removed = false;
    for p in &installed {
        let provider_name = format!("{}", p).to_lowercase();
        let ok = uninstall_provider_events(&AgentUninstallParams {
            base: &base,
            provider: p,
            events,
            global,
            skill_names,
            scope,
            project_str: &project_str,
            provider_name: &provider_name,
        });
        if ok {
            uninstall_agent_legacy(&base, p);
            any_removed = true;
        }
    }

    if any_removed {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

/// Parameters for uninstalling event skills for a single provider.
struct AgentUninstallParams<'a> {
    base: &'a std::path::Path,
    provider: &'a AgentProvider,
    events: &'a [HookEvent],
    global: bool,
    skill_names: Option<&'a linthis::config::AgentSkillNamesConfig>,
    scope: &'a str,
    project_str: &'a str,
    provider_name: &'a str,
}

/// Uninstall all event skills for a single provider.
fn uninstall_provider_events(params: &AgentUninstallParams<'_>) -> bool {
    let mut ok = true;
    for event in params.events {
        match uninstall_agent_skill(
            params.base,
            params.provider,
            params.global,
            event,
            params.skill_names,
        ) {
            Ok(_) => {
                println!(
                    "{} Uninstalled {} ({}) skill",
                    "✓".green(),
                    params.provider,
                    event.hook_filename()
                );
                remove_skill_provider_from_hook(
                    params.scope,
                    params.project_str,
                    event,
                    params.provider_name,
                );
            }
            Err(e) => {
                eprintln!(
                    "{}: Failed to uninstall {} ({}): {}",
                    "Error".red(),
                    params.provider,
                    event.hook_filename(),
                    e
                );
                ok = false;
            }
        }
    }
    ok
}
