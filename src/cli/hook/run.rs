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

    {
        let description = describe_hook_source(hook_type, event);
        eprintln!("{}", format!("📄 Config: {}", description).dimmed());
    }

    let pid = std::process::id().to_string();
    let env_key = format!("{}{}", LINTHIS_HOOK_RUNNING_PREFIX, pid);

    let status = std::process::Command::new("sh")
        .arg("-c")
        .arg(&script)
        .arg("--")
        .args(hook_args)
        .env(&env_key, "1")
        .status();

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
