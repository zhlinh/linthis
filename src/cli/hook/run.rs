// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! `linthis hook run` — execute full hook logic at runtime from thin wrapper.

use colored::Colorize;

use super::config::describe_hook_source;
use super::script::{
    build_git_with_agent_hook_script, build_global_hook_script_for_event, build_hook_command,
    build_post_commit_script, merge_model_into_provider_args, parse_agent_fix_provider_name,
    parse_provider_with_model,
};
use crate::cli::commands::{AgentFixProvider, HookEvent, HookTool};

/// Environment variable injected by `handle_hook_run` to detect re-entrant hook calls.
const LINTHIS_HOOK_RUNNING_PREFIX: &str = "LINTHIS_HOOK_RUNNING_";

/// Build the re-entrant (direct) script for a git hook.
pub(crate) fn build_reentrant_git_script(event: &HookEvent) -> String {
    let linthis_cmd = build_hook_command(event, &None);
    if matches!(event, HookEvent::PrePush) {
        format!(
            "#!/bin/sh\n\
             _BASE=$(git rev-parse '@{{u}}' 2>/dev/null || \\\n\
             \x20       git rev-parse 'HEAD~1' 2>/dev/null)\n\
             _PUSHED_FILES=$(git diff --name-only \"$_BASE\"..HEAD 2>/dev/null | grep -v '^$')\n\
             if [ -n \"$_PUSHED_FILES\" ]; then\n\
             \x20 set --\n\
             \x20 while IFS= read -r _F; do set -- \"$@\" -i \"$_F\"; done <<_EOF_\n\
             $_PUSHED_FILES\n\
             _EOF_\n\
             \x20 {linthis} \"$@\"\n\
             fi\n",
            linthis = linthis_cmd
        )
    } else {
        format!("#!/bin/sh\n{linthis_cmd} \"$@\"\n")
    }
}

pub(crate) fn handle_hook_run(
    event: &HookEvent,
    hook_type: &HookTool,
    raw_provider: Option<&str>,
    raw_provider_args: Option<&str>,
    _global: bool,
    hook_args: &[String],
) -> i32 {
    // LINTHIS_SKIP=<event|alias> lets users bypass a specific hook without
    // using git --no-verify (which kills every hook at once).
    if super::skip::should_skip(event) {
        return 0;
    }

    let (provider_name, merged_pa) = if let Some(raw) = raw_provider {
        let (name, model) = parse_provider_with_model(raw);
        (
            Some(name),
            merge_model_into_provider_args(model, raw_provider_args),
        )
    } else {
        (None, raw_provider_args.map(|s| s.to_string()))
    };
    let provider: Option<&str> = provider_name;
    let provider_args: Option<&str> = merged_pa.as_deref();

    let already_running = std::env::vars().any(|(k, _)| k.starts_with(LINTHIS_HOOK_RUNNING_PREFIX));

    // PostCommit uses a dedicated script regardless of hook type
    let script = if matches!(event, HookEvent::PostCommit) {
        let linthis_cmd = build_hook_command(event, &None);
        build_post_commit_script(&linthis_cmd)
    } else {
        match hook_type {
            HookTool::Git => {
                if already_running {
                    build_reentrant_git_script(event)
                } else {
                    build_global_hook_script_for_event(event, &None, None)
                }
            }
            HookTool::GitWithAgent => {
                let fix_provider = provider
                    .and_then(parse_agent_fix_provider_name)
                    .unwrap_or(AgentFixProvider::Claude);
                let linthis_cmd = build_hook_command(event, &None);
                build_git_with_agent_hook_script(
                    &linthis_cmd,
                    &fix_provider,
                    event,
                    provider_args,
                )
            }
            _ => {
                eprintln!(
                    "{}: hook run: unsupported hook type '{}' (supported: git, git-with-agent)",
                    "Error".red(),
                    hook_type.as_str()
                );
                return 1;
            }
        }
    };

    // Skip hook entirely during rebase/merge/cherry-pick
    if is_rebase_in_progress() {
        return 0;
    }

    // For pre-push: if nothing to push, skip silently (avoid noisy "Config:" line)
    // Must capture stdin first because git passes refs via stdin and we need to
    // re-feed it to the inner script.
    let captured_stdin: Option<Vec<u8>> = if matches!(event, HookEvent::PrePush) {
        let (is_empty, buf) = read_pre_push_stdin();
        if is_empty {
            return 0;
        }
        Some(buf)
    } else {
        None
    };

    {
        let description = describe_hook_source(hook_type, event);
        eprintln!("{}", format!("📄 Config: {}", description).dimmed());
    }

    let pid = std::process::id().to_string();
    let env_key = format!("{}{}", LINTHIS_HOOK_RUNNING_PREFIX, pid);

    let status = run_hook_script(&script, hook_args, &env_key, captured_stdin);

    match status {
        Ok(s) => s.code().unwrap_or(1),
        Err(e) => {
            eprintln!(
                "{}: hook run: failed to execute script: {}",
                "Error".red(),
                e
            );
            1
        }
    }
}

/// Execute the generated hook script, optionally piping captured stdin.
fn run_hook_script(
    script: &str,
    hook_args: &[String],
    env_key: &str,
    captured_stdin: Option<Vec<u8>>,
) -> std::io::Result<std::process::ExitStatus> {
    let mut sh_cmd = std::process::Command::new("sh");
    sh_cmd
        .arg("-c")
        .arg(script)
        .arg("--")
        .args(hook_args)
        .env(env_key, "1");

    if let Some(stdin_buf) = captured_stdin {
        use std::io::Write;
        let mut child = sh_cmd.stdin(std::process::Stdio::piped()).spawn()?;
        if let Some(mut child_stdin) = child.stdin.take() {
            let _ = child_stdin.write_all(&stdin_buf);
        }
        child.wait()
    } else {
        sh_cmd.status()
    }
}

/// Check if there's nothing to push.
/// Git passes refs via stdin to pre-push; each line is
/// "<local ref> <local sha> <remote ref> <remote sha>".
/// If local SHA == remote SHA for all lines, nothing to push.
/// Also returns true if stdin is empty.
/// Captures stdin and returns (is_empty, captured_stdin) so caller can
/// re-feed it to the inner script.
fn read_pre_push_stdin() -> (bool, Vec<u8>) {
    use std::io::Read;
    let mut buf = Vec::new();
    if std::io::stdin().read_to_end(&mut buf).is_err() {
        return (false, buf);
    }

    let content = String::from_utf8_lossy(&buf);
    let mut has_push = false;
    for line in content.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        // Expected: <local_ref> <local_sha> <remote_ref> <remote_sha>
        if parts.len() >= 4 {
            // If local SHA differs from remote SHA, there's something to push
            // (delete operation has local_sha = 0000...)
            if parts[1] != parts[3] {
                has_push = true;
                break;
            }
        } else if !parts.is_empty() {
            // Non-standard line — be conservative, treat as has_push
            has_push = true;
            break;
        }
    }

    (!has_push, buf)
}

/// Check if a rebase, merge, or cherry-pick is in progress.
/// Used to skip hooks during these operations to avoid noise.
fn is_rebase_in_progress() -> bool {
    if let Ok(output) = std::process::Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .output()
    {
        if output.status.success() {
            let git_dir = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let git_path = std::path::Path::new(&git_dir);
            return git_path.join("rebase-merge").exists()
                || git_path.join("rebase-apply").exists()
                || git_path.join("MERGE_HEAD").exists()
                || git_path.join("CHERRY_PICK_HEAD").exists();
        }
    }
    false
}
