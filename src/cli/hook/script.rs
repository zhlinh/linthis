// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Script generation for git hooks (thin wrappers, global scripts, agent fix blocks).

use crate::cli::commands::{AgentFixProvider, HookEvent, HookTool};

/// Build a thin wrapper script that delegates to `linthis hook run` at runtime.
///
/// The wrapper is 3 lines:
/// ```sh
/// #!/bin/sh
/// exec linthis hook run --event <event> --type <type> [--provider <p>] [--global] "$@"
/// ```
/// This means hook logic always comes from the installed linthis binary,
/// so upgrading linthis automatically updates hook behaviour without reinstallation.
pub(crate) fn build_thin_wrapper_script(
    event: &HookEvent,
    hook_type: &HookTool,
    provider: Option<&str>,
    global: bool,
    provider_args: Option<&str>,
) -> String {
    let provider_arg = provider
        .filter(|p| !p.is_empty())
        .map(|p| format!(" --provider {p}"))
        .unwrap_or_default();
    let provider_args_arg = provider_args
        .filter(|a| !a.is_empty())
        .map(|a| format!(" --provider-args '{}'", a.replace('\'', "'\\''")))
        .unwrap_or_default();
    let global_arg = if global { " --global" } else { "" };
    format!(
        "#!/bin/sh\nexec linthis hook run --event {} --type {}{}{}{} \"$@\"\n",
        event.as_str(),
        hook_type.as_str(),
        provider_arg,
        provider_args_arg,
        global_arg,
    )
}

/// Build the shell preamble and local-hook argument style for pre-push events.
/// Returns (preamble_script, local_hook_args_expression).
pub(crate) fn build_pre_push_preamble() -> (String, &'static str) {
    let preamble = "# For pre-push: save remote args, read stdin for push info\n\
         _REMOTE_NAME=\"$1\"\n\
         _REMOTE_URL=\"$2\"\n\
         # Read push info from stdin: <local_ref> <local_sha> <remote_ref> <remote_sha>\n\
         _IS_TAG=0\n\
         _LOCAL_SHA=\"\"\n\
         _REMOTE_SHA=\"\"\n\
         while read -r _LREF _LSHA _RREF _RSHA; do\n\
         \x20 # Skip tag pushes — no source code to check\n\
         \x20 case \"$_LREF\" in refs/tags/*) _IS_TAG=1 ;; esac\n\
         \x20 _LOCAL_SHA=\"$_LSHA\"\n\
         \x20 _REMOTE_SHA=\"$_RSHA\"\n\
         done\n\
         if [ \"$_IS_TAG\" = \"1\" ]; then\n\
         \x20 exit 0\n\
         fi\n\
         # Compute changed files between remote and local\n\
         _ZERO_SHA=\"0000000000000000000000000000000000000000\"\n\
         if [ \"$_REMOTE_SHA\" = \"$_ZERO_SHA\" ]; then\n\
         \x20 # New branch: diff against default branch\n\
         \x20 _BASE=$(git rev-parse 'HEAD~1' 2>/dev/null || echo \"$_LOCAL_SHA\")\n\
         else\n\
         \x20 _BASE=\"$_REMOTE_SHA\"\n\
         fi\n\
         _PUSHED_FILES=$(git diff --name-only \"$_BASE\"..\"$_LOCAL_SHA\" 2>/dev/null | grep -v '^$')\n\
         # No files to push = nothing to check\n\
         if [ -z \"$_PUSHED_FILES\" ]; then\n\
         \x20 exit 0\n\
         fi\n\
         set --\n\
         while IFS= read -r _F; do set -- \"$@\" -i \"$_F\"; done <<_EOF_\n\
         $_PUSHED_FILES\n\
         _EOF_\n\
         \n"
        .to_string();
    (preamble, "\"$_REMOTE_NAME\" \"$_REMOTE_URL\"")
}

/// Build the agent fix command for a given hook event.
pub(crate) fn agent_fix_cmd_for_event(
    provider: &AgentFixProvider,
    hook_event: &HookEvent,
) -> String {
    if matches!(hook_event, HookEvent::CommitMsg) {
        agent_fix_headless_cmd_commit_msg(provider, None)
    } else {
        let prompt = agent_fix_prompt_for_event(hook_event);
        agent_fix_headless_cmd(provider, &prompt, None)
    }
}

/// Shell snippet that gates an agent invocation behind the
/// `LINTHIS_AGENT_MAX_AUTO_FIX` threshold, prints a streaming header +
/// elapsed-time footer, and lets the agent's output flow straight to
/// stderr (no spinner — the spinner's `\033[1A\r\033[K` overwrites would
/// clobber the agent's per-tool-call output).
///
/// Exports `_AGENT_RAN=1` iff the agent actually ran (not skipped by
/// threshold), so callers can decide whether to re-verify.
///
/// - `indent` is the per-line prefix (spaces) matching the surrounding
///   if-block depth; pass `"     "` for two-nested, `"   "` for one.
/// - `error_msg` is the short verb used in the header ("Fix errors", etc.).
fn shell_agent_invoke_block(
    provider: &AgentFixProvider,
    agent_cmd: &str,
    error_msg: &str,
    indent: &str,
) -> String {
    // Thresholds: 0 disables the cap. Default picked to protect against
    // runaway sessions on huge error sets while still covering the common
    // case (a handful to a few dozen issues).
    // Important: parse `linthis report count` output without touching $@.
    // `set -- $counts` would clobber positional params, and the commit-msg
    // agent command relies on $1 to locate $_MSG_FILE — losing that made
    // Claude write its fix to a file literally named "0". Use parameter
    // expansion instead, which is POSIX-portable and side-effect-free.
    format!(
        "{i}_AGENT_RAN=0\n\
         {i}_AGENT_MAX=\"${{LINTHIS_AGENT_MAX_AUTO_FIX:-100}}\"\n\
         {i}_AGENT_COUNTS=$(linthis report count 2>/dev/null || echo \"0 0 0 0\")\n\
         {i}_AGENT_ERR=${{_AGENT_COUNTS%% *}}\n\
         {i}_AGENT_REM=${{_AGENT_COUNTS#* }}\n\
         {i}_AGENT_WARN=${{_AGENT_REM%% *}}\n\
         {i}_AGENT_FILES=${{_AGENT_COUNTS##* }}\n\
         {i}_AGENT_TOTAL=$((_AGENT_ERR + _AGENT_WARN))\n\
         {i}if [ \"$_AGENT_MAX\" != \"0\" ] && [ \"$_AGENT_TOTAL\" -gt \"$_AGENT_MAX\" ]; then\n\
         {i}  echo \"[linthis] ⚠ Too many issues ($_AGENT_TOTAL in $_AGENT_FILES files) — auto-fix skipped to avoid long blocking run\" >&2\n\
         {i}  echo \"[linthis]   Fix interactively (live progress): linthis fix --ai --provider {provider_cli}\" >&2\n\
         {i}  echo \"[linthis]   Raise the threshold: LINTHIS_AGENT_MAX_AUTO_FIX=$((_AGENT_TOTAL + 10)) git ...\" >&2\n\
         {i}  echo \"[linthis]   Disable the cap:    LINTHIS_AGENT_MAX_AUTO_FIX=0 git ...\" >&2\n\
         {i}else\n\
         {i}  printf \"${{_LINTHIS_W}}[linthis] {error_msg}. Found $_AGENT_TOTAL issues in $_AGENT_FILES files — invoking {provider}...${{_LINTHIS_R}}\\n\"\n\
         {i}  printf \"${{_LINTHIS_W}}[linthis] ─── {provider} output (streaming; Ctrl-C to cancel) ───${{_LINTHIS_R}}\\n\"\n\
         {i}  _AGENT_START=$(date +%s)\n\
         {i}  # Wrap the agent pipeline so each command's stderr (claude/codebuddy progress,\n\
         {i}  # agent-stream diagnostics) is merged into stdout. IDE terminals colour stderr\n\
         {i}  # red, and none of this output signals failure.\n\
         {i}  {{ {agent} ; }} 2>&1\n\
         {i}  _AGENT_ELAPSED=$(($(date +%s) - _AGENT_START))\n\
         {i}  printf \"${{_LINTHIS_W}}[linthis] ─── {provider} done in ${{_AGENT_ELAPSED}}s ───${{_LINTHIS_R}}\\n\"\n\
         {i}  _AGENT_RAN=1\n\
         {i}fi\n",
        i = indent,
        provider = provider,
        provider_cli = provider.as_str(),
        agent = agent_cmd,
        error_msg = error_msg,
    )
}

/// Shell snippet showing how to review/undo the agent's changes.
/// Printed after the agent finishes and before re-verify, so the user sees
/// the hint even if re-verify fails (and they need to undo).
///
/// Uses `git diff --cached` for viewing because linthis's own backup tracks
/// format changes (small), not the full agent rewrite. The staged diff
/// against HEAD shows exactly what the agent changed.
fn shell_agent_fix_hint(indent: &str) -> String {
    // stdout + IDE-safe colour wrapping (see shell_timer_functions for the
    // $_LINTHIS_W / $_LINTHIS_WR / $_LINTHIS_R convention).
    format!(
        "{i}printf \"${{_LINTHIS_W}}[linthis] \\033[0;32m✓${{_LINTHIS_WR}} Agent fix applied${{_LINTHIS_R}}\\n\"\n\
         {i}printf \"${{_LINTHIS_W}}[linthis]   View changes : \\033[0;36mgit diff --cached${{_LINTHIS_R}}\\n\"\n\
         {i}printf \"${{_LINTHIS_W}}[linthis]   Undo changes : \\033[0;33mlinthis backup undo${{_LINTHIS_R}}\\n\"\n",
        i = indent
    )
}

/// Shell snippet showing how to review/undo the agent's changes in post-commit fixup mode.
/// Printed after the fixup commit is created, so `HEAD~1` correctly refers to
/// the fixup commit and `git diff HEAD~1` shows the agent's changes.
fn shell_agent_fix_hint_post_commit(indent: &str) -> String {
    format!(
        "{i}printf \"${{_LINTHIS_W}}[linthis] \\033[0;32m✓${{_LINTHIS_WR}} Agent fix applied${{_LINTHIS_R}}\\n\"\n\
         {i}printf \"${{_LINTHIS_W}}[linthis]   View changes : \\033[0;36mgit diff HEAD~1${{_LINTHIS_R}}\\n\"\n\
         {i}printf \"${{_LINTHIS_W}}[linthis]   Undo changes : \\033[0;33mgit reset HEAD~1${{_LINTHIS_R}}\\n\"\n",
        i = indent
    )
}

/// Shell snippet hinting how to switch fix_commit_mode for pre-commit hooks.
fn shell_fix_commit_mode_hint_pre_commit(indent: &str) -> String {
    format!(
        "{i}printf \"${{_LINTHIS_W}}[linthis]   Tip : switch mode → \\033[0;36mlinthis config set hook.pre_commit.fix_commit_mode [dirty|squash|fixup] -g${{_LINTHIS_R}}\\n\"\n",
        i = indent
    )
}

/// Shell snippet hinting how to switch fix_commit_mode for pre-push hooks.
fn shell_fix_commit_mode_hint_pre_push(indent: &str) -> String {
    format!(
        "{i}printf \"${{_LINTHIS_W}}[linthis]   Tip : switch mode → \\033[0;36mlinthis config set hook.pre_push.fix_commit_mode [dirty|squash|fixup] -g${{_LINTHIS_R}}\\n\"\n",
        i = indent
    )
}

/// Save staged changes (format + agent) as a patch before the fixup commit.
/// Sets `_DIFF_FILE` to the saved path, or empty string if nothing to save.
/// Does NOT print anything — the caller prints `Diff Patch created:` after
/// the fixup commit so the message appears in the right order.
fn shell_save_diff_patch_for_post_commit(indent: &str) -> String {
    let keep = load_retention_diffs();
    format!(
        "{i}# Save staged changes as a patch before fixup commit\n\
         {i}_SLUG=$(git rev-parse --show-toplevel 2>/dev/null | tr '/' '-' | sed 's/^-//')\n\
         {i}_DIFF_DIR=\"$HOME/.linthis/projects/$_SLUG/diff\"\n\
         {i}mkdir -p \"$_DIFF_DIR\"\n\
         {i}_DIFF_FILE=\"$_DIFF_DIR/diff-$(date +%Y%m%d-%H%M%S).patch\"\n\
         {i}git diff --cached > \"$_DIFF_FILE\" 2>/dev/null\n\
         {i}if [ -s \"$_DIFF_FILE\" ]; then\n\
         {i}  ls -t \"$_DIFF_DIR\"/diff-*.patch 2>/dev/null | tail -n +{keep_plus_one} | xargs rm -f 2>/dev/null\n\
         {i}else\n\
         {i}  rm -f \"$_DIFF_FILE\"\n\
         {i}  _DIFF_FILE=\"\"\n\
         {i}fi\n",
        i = indent,
        keep_plus_one = keep + 1,
    )
}

/// Build the shell fix block that invokes an agent on lint failure.
pub(crate) fn build_agent_fix_block(provider: &AgentFixProvider, hook_event: &HookEvent) -> String {
    let agent_cmd = agent_fix_cmd_for_event(provider, hook_event);
    let agent_check = shell_agent_availability_check(provider);
    let error_msg = agent_fix_error_msg(hook_event);
    let new_msg_print = if matches!(hook_event, HookEvent::CommitMsg) {
        agent_fix_show_fixed_cmsg("   ")
    } else {
        String::new()
    };
    let agent_block = shell_agent_invoke_block(provider, &agent_cmd, error_msg, "     ");
    format!(
        "  if [ $LINTHIS_EXIT -ne 0 ]; then\n\
         \x20\x20\x20 {agent_check}\
         \x20\x20\x20 if [ \"$_LINTHIS_AGENT_OK\" = \"1\" ]; then\n\
         {agent_block}\
         \x20\x20\x20\x20\x20 if [ \"$_AGENT_RAN\" = \"1\" ]; then\n\
         {agent_hint}\
         \x20\x20\x20\x20\x20\x20\x20 printf \"${{_LINTHIS_W}}[linthis] Re-verifying...${{_LINTHIS_R}}\\n\"\n\
         \x20\x20\x20\x20\x20\x20\x20 _linthis_run_painted $LINTHIS_CMD \"$@\"\n\
         \x20\x20\x20\x20\x20\x20\x20 LINTHIS_EXIT=$?\n\
         \x20\x20\x20\x20\x20 fi\n\
         \x20\x20\x20 fi\n\
         {new_msg_print}\
         \x20 fi\n",
        agent_block = agent_block,
        agent_check = agent_check,
        agent_hint = shell_agent_fix_hint("       "),
        new_msg_print = new_msg_print,
    )
}

/// Build the linthis command variable for the global hook script.
/// For commit-msg, strips "$1" so it can be forwarded via "$@".
pub(crate) fn build_linthis_cmd_var(hook_event: &HookEvent, args: &Option<String>) -> String {
    let cmd = build_hook_command(hook_event, args);
    match hook_event {
        HookEvent::CommitMsg => cmd.trim_end_matches(" \"$1\"").to_string(),
        _ => cmd,
    }
}

/// Resolve preamble and local-hook argument style for a given event.
pub(crate) fn resolve_event_preamble(hook_event: &HookEvent) -> (String, &'static str) {
    if matches!(hook_event, HookEvent::PrePush) {
        build_pre_push_preamble()
    } else {
        (String::new(), "\"$@\"")
    }
}

/// Resolve the fix block, review block, and timer block for the global hook script.
pub(crate) fn resolve_global_hook_blocks(
    hook_event: &HookEvent,
    fix_provider: Option<&AgentFixProvider>,
) -> (String, String, &'static str, &'static str) {
    let fix_block = fix_provider
        .map(|p| build_agent_fix_block(p, hook_event))
        .unwrap_or_default();
    let fix_block_direct = fix_provider
        .map(|p| build_agent_fix_block(p, hook_event))
        .unwrap_or_default();
    let review_block = if matches!(hook_event, HookEvent::PrePush) {
        "\n# Trigger background AI code review (non-blocking)\n\
         linthis review --background 2>/dev/null &\n"
    } else {
        ""
    };
    let timer_block = if fix_provider.is_some() {
        shell_timer_functions()
    } else {
        ""
    };
    (fix_block, fix_block_direct, review_block, timer_block)
}

/// Build the global hook script with the hook event name substituted.
pub(crate) fn build_global_hook_script_for_event(
    hook_event: &HookEvent,
    args: &Option<String>,
    fix_provider: Option<&AgentFixProvider>,
) -> String {
    let linthis_cmd_var = build_linthis_cmd_var(hook_event, args);
    let (pre_push_preamble, local_hook_orig_args) = resolve_event_preamble(hook_event);
    let (fix_block, fix_block_direct, review_block, timer_block) =
        resolve_global_hook_blocks(hook_event, fix_provider);
    let event_name = hook_event.hook_filename();

    let fix_commit_mode_section = if matches!(hook_event, HookEvent::PreCommit) {
        shell_read_fix_commit_mode("pre_commit")
    } else if matches!(hook_event, HookEvent::PrePush) {
        shell_read_fix_commit_mode("pre_push")
    } else {
        String::new()
    };

    let git_fix_commit_mode_handler = shell_git_fix_commit_mode_handler(hook_event);

    format!(
        "#!/bin/sh\n\
         # linthis-hook\n\
         {timer}\
         LINTHIS_CMD=\"{linthis}\"\n\
         {fix_commit_mode}\
         # Snapshot pre-format state for stash (squash mode)\n\
         if [ \"$_FIX_MODE\" = \"squash\" ]; then\n\
         \x20 _STASH_REF=$(git stash create 2>/dev/null)\n\
         fi\n\
         {pre_push_preamble}\
         # Locate the local project hook (git-dir aware)\n\
         GIT_DIR=\"$(git rev-parse --git-dir 2>/dev/null)\"\n\
         LOCAL_HOOK=\"\"\n\
         if [ -n \"$GIT_DIR\" ]; then\n\
         \x20 LOCAL_HOOK=\"$GIT_DIR/hooks/{event}\"\n\
         fi\n\
         \n\
         if [ -f \"$LOCAL_HOOK\" ] && [ -x \"$LOCAL_HOOK\" ]; then\n\
         \x20 if grep -qE '^[^#]*linthis' \"$LOCAL_HOOK\" 2>/dev/null; then\n\
         \x20\x20\x20 # Local hook already calls linthis — delegate entirely\n\
         \x20\x20\x20 exec \"$LOCAL_HOOK\" {local_hook_orig_args}\n\
         \x20 else\n\
         \x20\x20\x20 # Local hook exists but has no linthis — run linthis first, then delegate.\n\
         \x20\x20\x20 # _linthis_run_painted folds stderr into stdout and pipes through the\n\
         \x20\x20\x20 # VCS-console white filter while preserving linthis's exit status.\n\
         \x20\x20\x20 _linthis_run_painted $LINTHIS_CMD \"$@\"\n\
         \x20\x20\x20 LINTHIS_EXIT=$?\n\
         {git_fix_handler}\
         {fix_local}\
         \x20\x20\x20 \"$LOCAL_HOOK\" {local_hook_orig_args}\n\
         \x20\x20\x20 LOCAL_EXIT=$?\n\
         {review}\
         \x20\x20\x20 [ $LINTHIS_EXIT -ne 0 ] && exit $LINTHIS_EXIT\n\
         \x20\x20\x20 exit $LOCAL_EXIT\n\
         \x20 fi\n\
         else\n\
         \x20 # No local hook — run linthis directly.\n\
         \x20 _linthis_run_painted $LINTHIS_CMD \"$@\"\n\
         \x20 LINTHIS_EXIT=$?\n\
         {git_fix_handler}\
         {fix_direct}\
         {review}\
         \x20 exit $LINTHIS_EXIT\n\
         fi\n",
        timer = timer_block,
        linthis = linthis_cmd_var,
        fix_commit_mode = fix_commit_mode_section,
        pre_push_preamble = pre_push_preamble,
        event = event_name,
        local_hook_orig_args = local_hook_orig_args,
        git_fix_handler = git_fix_commit_mode_handler,
        fix_local = fix_block,
        fix_direct = fix_block_direct,
        review = review_block,
    )
}

/// Return the binary name used to invoke the agent CLI headlessly.
/// Used for PATH detection via `which`.
pub(crate) fn agent_fix_bin(provider: &AgentFixProvider) -> std::borrow::Cow<'static, str> {
    match provider {
        AgentFixProvider::Claude => "claude".into(),
        AgentFixProvider::Codex => "codex".into(),
        AgentFixProvider::Gemini => "gemini".into(),
        AgentFixProvider::Cursor => "cursor-agent".into(),
        AgentFixProvider::Droid => "droid".into(),
        AgentFixProvider::Auggie => "auggie".into(),
        AgentFixProvider::Codebuddy => "codebuddy".into(),
        AgentFixProvider::Openclaw => "openclaw".into(),
        AgentFixProvider::Custom { bin, .. } => bin.clone().into(),
    }
}

/// Build the headless shell command that invokes the agent with a prompt.
///
/// Commands confirmed from official docs:
/// - Claude:    `claude -p '...'`             (claude -p / --print)
/// - Codex:     `codex exec '...'`            (codex exec subcommand for non-interactive)
/// - Gemini:    `gemini -p '...'`             (gemini -p / --prompt)
/// - Cursor:    `cursor-agent chat '...'`     (cursor-agent chat subcommand)
/// - Droid:     `droid exec --auto low '...'` (droid exec with --auto for edits)
/// - Auggie:    `auggie --print '...'`        (auggie --print for headless/non-interactive)
/// - Codebuddy: `codebuddy -p '...'`         (codebuddy -p / --prompt)
pub(crate) fn agent_fix_headless_cmd(
    provider: &AgentFixProvider,
    prompt: &str,
    provider_args: Option<&str>,
) -> String {
    // Escape single quotes in prompt for shell safety
    let escaped = prompt.replace('\'', "'\\''");
    let extra = provider_args
        .filter(|a| !a.is_empty())
        .map(|a| format!(" {a}"))
        .unwrap_or_default();
    // Streaming strategy per provider:
    //   claude / codebuddy: `-p` buffers text output; use `--output-format
    //     stream-json --verbose` and pipe through `linthis agent-stream`
    //     to pretty-print events (text, tool_use) as they arrive.
    //   codex / cursor / droid: already stream by default.
    //   gemini / auggie / openclaw: no official streaming flag known — kept
    //     as-is; output appears when the call completes.
    match provider {
        AgentFixProvider::Claude => format!(
            "claude -p{extra} --verbose --output-format stream-json --dangerously-skip-permissions '{}' | linthis agent-stream",
            escaped
        ),
        AgentFixProvider::Codex => {
            format!("codex exec{extra} --ask-for-approval never '{}'", escaped)
        }
        AgentFixProvider::Gemini => {
            format!("gemini -p{extra} --approval-mode=auto_edit '{}'", escaped)
        }
        AgentFixProvider::Cursor => format!("cursor-agent chat{extra} --force '{}'", escaped),
        AgentFixProvider::Droid => format!("droid exec{extra} --auto high '{}'", escaped),
        AgentFixProvider::Auggie => format!("auggie{extra} --print '{}'", escaped),
        AgentFixProvider::Codebuddy => format!(
            "codebuddy -p{extra} --verbose --output-format stream-json --dangerously-skip-permissions '{}' | linthis agent-stream",
            escaped
        ),
        AgentFixProvider::Openclaw => format!("openclaw agent{extra} --message '{}'", escaped),
        AgentFixProvider::Custom { bin, style } => match style.as_str() {
            "claude" | "claude-cli" | "codebuddy" | "codebuddy-cli" => format!(
                "{bin} -p{extra} --verbose --output-format stream-json --dangerously-skip-permissions '{}' | linthis agent-stream",
                escaped
            ),
            "codex" | "codex-cli" => format!("{bin} exec{extra} --ask-for-approval never '{}'", escaped),
            "gemini" | "gemini-cli" => format!("{bin} -p{extra} --approval-mode=auto_edit '{}'", escaped),
            "cursor" => format!("{bin} chat{extra} --force '{}'", escaped),
            "droid" => format!("{bin} exec{extra} --auto high '{}'", escaped),
            "auggie" => format!("{bin}{extra} --print '{}'", escaped),
            "openclaw" => format!("{bin} agent{extra} --message '{}'", escaped),
            _ => format!("{bin}{extra} '{}'", escaped),
        },
    }
}

/// Generate a shell snippet that checks whether the provider binary exists in PATH.
///
/// If the binary is not found, prints a friendly message suggesting installation
/// or provider change, then gracefully degrades (skips the agent invocation).
/// The snippet sets `_LINTHIS_AGENT_OK=1` if available, `_LINTHIS_AGENT_OK=0` otherwise.
pub(crate) fn shell_agent_availability_check(provider: &AgentFixProvider) -> String {
    let bin = agent_fix_bin(provider);
    format!(
        "if command -v {bin} >/dev/null 2>&1; then\n\
         \x20 _LINTHIS_AGENT_OK=1\n\
         else\n\
         \x20 _LINTHIS_AGENT_OK=0\n\
         \x20 echo \"[linthis] ⚠ '{bin}' not found in PATH — skipping AI auto-fix\" >&2\n\
         \x20 echo \"[linthis]   To install: https://docs.anthropic.com/en/docs/claude-code\" >&2\n\
         \x20 echo \"[linthis]   To change provider: linthis hook install -g --type git-with-agent --provider <name> --event <event> --force\" >&2\n\
         \x20 echo \"[linthis]   Please fix the issues manually and retry.\" >&2\n\
         fi\n",
        bin = bin,
    )
}

/// Agent fix prompt for post-commit: files are committed (not staged), use report show.
fn agent_fix_prompt_for_post_commit() -> String {
    "Lint issues remain in committed files after auto-formatting. A backup has been created. \
     Follow these steps: \
     (1) Run 'linthis report show' to inspect all remaining issues. \
     (2) Group the issues by file. For files with independent errors (no cross-file dependencies), \
     fix them in parallel using concurrent tool calls — each tool call fixes one file. \
     For files with cross-file dependencies (e.g. shared type renames, API signature changes), \
     fix them sequentially in dependency order. \
     Fix by editing the code directly (do NOT use linthis --fix). \
     (3) Re-run 'linthis report show' and verify remaining issues. \
     If none remain, you are done. Otherwise keep fixing. \
     (4) Display a Changes Summary showing each modified file, \
     what was changed, and why (e.g. which lint rule). Then show the full diff output."
        .to_string()
}

/// Build the agent fix prompt based on the hook event type.
/// Note: CommitMsg uses agent_fix_headless_cmd_commit_msg() instead (needs $1 expansion).
pub(crate) fn agent_fix_prompt_for_event(_hook_event: &HookEvent) -> String {
    "Lint issues were found in staged files. A backup has been created. \
     Follow these steps: \
     (1) Run 'linthis -s' to inspect all issues. \
     (2) Group the issues by file. For files with independent errors (no cross-file dependencies), \
     fix them in parallel using concurrent tool calls — each tool call fixes one file. \
     For files with cross-file dependencies (e.g. shared type renames, API signature changes), \
     fix them sequentially in dependency order. \
     Fix by editing the code directly (do NOT use linthis --fix). \
     (3) Re-run 'linthis -s' and check the exit code. \
     Exit code 0 means all checks passed — you are done. \
     Non-zero means issues remain — keep fixing until exit code is 0. \
     (4) Run the project build/test to ensure fixes don't break anything \
     (detect project type: cargo check && cargo test for Rust, \
     go build ./... && go test ./... for Go, \
     npx tsc --noEmit for TypeScript, python -m py_compile for Python). \
     If build/tests fail, revert the problematic fix and try again. \
     (5) Display a Changes Summary showing each modified file, \
     what was changed, and why (e.g. which lint rule). Then show the full diff output."
        .to_string()
}

/// Shell snippet printed after a successful agent commit-msg fix.
/// Shows the fixed message in green so it's visible in the terminal.
/// `indent` is the per-line prefix (spaces) matching the surrounding if-block depth.
///
/// Reads `$_REAL_MSG` (the actual `.git/COMMIT_EDITMSG`), not `$_MSG_FILE`,
/// because the latter points at a temp file the agent_cmd already deleted.
pub(crate) fn agent_fix_show_fixed_cmsg(indent: &str) -> String {
    format!(
        "{i}if [ $LINTHIS_EXIT -eq 0 ] && [ -n \"$_REAL_MSG\" ] && [ -f \"$_REAL_MSG\" ]; then\n\
         {i}  printf '\\033[0;32m[linthis] ✓ New message: %s\\033[0m\\n' \"$(cat \"$_REAL_MSG\")\" >&2\n\
         {i}fi\n",
        i = indent,
    )
}

/// Build the agent command for commit-msg hook: captures $1 in _MSG_FILE then invokes agent.
/// Uses double-quoted prompt string so $_MSG_FILE expands at shell runtime.
pub(crate) fn agent_fix_headless_cmd_commit_msg(
    provider: &AgentFixProvider,
    provider_args: Option<&str>,
) -> String {
    // We can't have the agent write directly to .git/COMMIT_EDITMSG: Claude
    // Code (and similar tools) flag .git/ as a sensitive path and refuse the
    // write even with --dangerously-skip-permissions. Workaround: stage the
    // current message into a temp file, point the agent at the temp file,
    // and copy the temp file back to .git/COMMIT_EDITMSG when the agent
    // returns. The agent never touches .git/ directly.
    let prompt = "Commit message validation failed (not in Conventional Commits format). \
        Fix the commit message in $_MSG_FILE (this is a TEMP FILE outside .git/ — \
        do NOT try to write to .git/COMMIT_EDITMSG, that path is blocked; \
        write to $_MSG_FILE only): \
        (1) read $_MSG_FILE for the current message, \
        (2) run 'git diff --cached --stat' to understand what actually changed, \
        (3) run 'git log -n 5 --oneline' to check recent commit style AND the language used \
        (Chinese or English) — match that language for the description, \
        (4) choose the correct type (feat/fix/refactor/perf/docs/style/test/build/ci/chore/revert) \
        based on the diff, \
        (5) rewrite to: type(scope)?: description — lowercase type, ≤72 chars, no trailing period. \
        Overwrite $_MSG_FILE in place. \
        Verify with 'linthis cmsg $_MSG_FILE' until it passes.";
    // Escape backslashes and double quotes for use in double-quoted shell string
    let escaped = prompt.replace('\\', "\\\\").replace('"', "\\\"");
    let bin_cmd = build_commit_msg_agent_bin_cmd(provider, &escaped, provider_args);
    // Capture the real .git/COMMIT_EDITMSG path in $_REAL_MSG, point
    // $_MSG_FILE at a writable temp file (the agent can't touch .git/),
    // and copy back when the agent finishes if it actually changed anything.
    format!(
        "_REAL_MSG=\"$1\"; \
         _MSG_FILE=$(mktemp -t linthis-cmsg.XXXXXX 2>/dev/null || echo \"/tmp/linthis-cmsg.$$\"); \
         cp \"$_REAL_MSG\" \"$_MSG_FILE\" 2>/dev/null; \
         {bin_cmd}; \
         if [ -s \"$_MSG_FILE\" ] && ! cmp -s \"$_REAL_MSG\" \"$_MSG_FILE\" 2>/dev/null; then \
           cp \"$_MSG_FILE\" \"$_REAL_MSG\"; \
         fi; \
         rm -f \"$_MSG_FILE\"",
        bin_cmd = bin_cmd
    )
}

/// Build the binary command portion for commit-msg agent fix.
/// Extracted to reduce cyclomatic complexity of `agent_fix_headless_cmd_commit_msg`.
fn build_commit_msg_agent_bin_cmd(
    provider: &AgentFixProvider,
    escaped: &str,
    provider_args: Option<&str>,
) -> String {
    let extra = provider_args
        .filter(|a| !a.is_empty())
        .map(|a| format!(" {a}"))
        .unwrap_or_default();
    match provider {
        AgentFixProvider::Claude => format!(
            "claude -p{extra} --verbose --output-format stream-json --dangerously-skip-permissions \"{}\" | linthis agent-stream",
            escaped
        ),
        AgentFixProvider::Codex => {
            format!("codex exec{extra} --ask-for-approval never \"{}\"", escaped)
        }
        AgentFixProvider::Gemini => {
            format!("gemini -p{extra} --approval-mode=auto_edit \"{}\"", escaped)
        }
        AgentFixProvider::Cursor => format!("cursor-agent chat{extra} --force \"{}\"", escaped),
        AgentFixProvider::Droid => format!("droid exec{extra} --auto high \"{}\"", escaped),
        AgentFixProvider::Auggie => format!("auggie{extra} --print \"{}\"", escaped),
        AgentFixProvider::Codebuddy => format!(
            "codebuddy -p{extra} --verbose --output-format stream-json --dangerously-skip-permissions \"{}\" | linthis agent-stream",
            escaped
        ),
        AgentFixProvider::Openclaw => format!("openclaw agent{extra} --message \"{}\"", escaped),
        AgentFixProvider::Custom { bin, style } => {
            build_custom_commit_msg_cmd(bin, style, &extra, escaped)
        }
    }
}

/// Build commit-msg command for custom provider based on CLI style.
fn build_custom_commit_msg_cmd(bin: &str, style: &str, extra: &str, escaped: &str) -> String {
    match style {
        "claude" | "claude-cli" | "codebuddy" | "codebuddy-cli" => format!(
            "{bin} -p{extra} --verbose --output-format stream-json --dangerously-skip-permissions \"{}\" | linthis agent-stream",
            escaped
        ),
        "codex" | "codex-cli" => format!("{bin} exec{extra} --ask-for-approval never \"{}\"", escaped),
        "gemini" | "gemini-cli" => format!("{bin} -p{extra} --approval-mode=auto_edit \"{}\"", escaped),
        "cursor" => format!("{bin} chat{extra} --force \"{}\"", escaped),
        "droid" => format!("{bin} exec{extra} --auto high \"{}\"", escaped),
        "auggie" => format!("{bin}{extra} --print \"{}\"", escaped),
        "openclaw" => format!("{bin} agent{extra} --message \"{}\"", escaped),
        _ => format!("{bin}{extra} \"{}\"", escaped),
    }
}

/// Error message for agent fix echo based on hook event type.
pub(crate) fn agent_fix_error_msg(hook_event: &HookEvent) -> &'static str {
    match hook_event {
        HookEvent::CommitMsg => "Commit message validation failed",
        _ => "Lint errors detected",
    }
}

/// Shell function to print a colored review summary box.
pub(crate) fn shell_review_box_fn() -> &'static str {
    r#"
_print_review_box() {
  if [ "$1" = "passed" ]; then
    _RH="✓ Linthis 📤 [Pre-push] Review Passed"
    _RC="\033[32m"
    _RHP="           "
  else
    _RH="✗ Linthis 📤 [Pre-push] Review Blocked"
    _RC="\033[31m"
    _RHP="          "
  fi
  _RN="\033[0m"
  _RM=$(printf "%-48s" "$2")
  printf "${_RC}╭──────────────────────────────────────────────────╮${_RN}\n" >&2
  printf "${_RC}│ ${_RH}${_RHP}│${_RN}\n" >&2
  printf "${_RC}├──────────────────────────────────────────────────┤${_RN}\n" >&2
  printf "${_RC}│ ${_RM} │${_RN}\n" >&2
  if [ "$1" != "passed" ]; then
    printf "${_RC}├──────────────────────────────────────────────────┤${_RN}\n" >&2
    printf "${_RC}│ To skip this check:                              │${_RN}\n" >&2
    printf "${_RC}│   git push --no-verify                           │${_RN}\n" >&2
  fi
  printf "${_RC}╰──────────────────────────────────────────────────╯${_RN}\n" >&2
}
"#
}

/// Shell snippet: a background elapsed-time spinner, plus "VCS console"
/// colour wrapping. The latter guards against hosts (e.g. JetBrains' Git tool
/// window) that render every unformatted line of hook output red. Detection:
///
///   * `LINTHIS_HOOK_COLOR=auto|off|white` overrides.
///   * Default (`auto`) wraps only when stdout is **not a TTY** AND the common
///     CI env vars are absent — i.e. IDE VCS consoles (pipe, interactive),
///     never real terminals and never CI logs.
///
/// Three shell variables are exported:
///   `$_LINTHIS_W`  — initial white ANSI prefix (or empty)
///   `$_LINTHIS_WR` — "resume white after an inner colour" (or reset)
///   `$_LINTHIS_R`  — trailing reset (always `\033[0m`)
/// Plus `_linthis_paint`, a filter that wraps every line read on stdin. Used
/// on `git commit` output so its `[master HASH] message` summary picks up the
/// wrapping too.
pub(crate) fn shell_timer_functions() -> &'static str {
    r#"
_linthis_timer_pid=""
start_timer() {
  _linthis_label="$1"
  printf "[linthis] ⠋ %s (0s)\n" "$_linthis_label" >&2
  (
    _i=0
    _s=0
    while true; do
      sleep 0.1
      _i=$((_i + 1))
      case $((_i % 10)) in
        0) _spin="⠋" ;;
        1) _spin="⠙" ;;
        2) _spin="⠹" ;;
        3) _spin="⠸" ;;
        4) _spin="⠼" ;;
        5) _spin="⠴" ;;
        6) _spin="⠦" ;;
        7) _spin="⠧" ;;
        8) _spin="⠇" ;;
        9) _spin="⠏" ;;
      esac
      if [ $((_i % 10)) -eq 0 ]; then
        _s=$((_s + 1))
      fi
      printf "\033[1A\r[linthis] %s %s (%ds)\033[K\n" "$_spin" "$_linthis_label" "$_s" >&2
    done
  ) &
  _linthis_timer_pid=$!
}
stop_timer() {
  if [ -n "$_linthis_timer_pid" ]; then
    kill "$_linthis_timer_pid" 2>/dev/null
    wait "$_linthis_timer_pid" 2>/dev/null
    _linthis_timer_pid=""
    printf "\r\033[K" >&2
  fi
}

# ---- IDE "VCS console" colour compensation ----
_LINTHIS_W=""
_LINTHIS_WR=$(printf '\033[0m')
_LINTHIS_R=$(printf '\033[0m')
case "${LINTHIS_HOOK_COLOR:-auto}" in
  off)
    ;;
  white)
    _LINTHIS_W=$(printf '\033[0;37m')
    _LINTHIS_WR=$(printf '\033[0;37m')
    ;;
  auto|*)
    if [ ! -t 1 ] \
        && [ -z "$CI" ] \
        && [ -z "$GITHUB_ACTIONS" ] \
        && [ -z "$GITLAB_CI" ] \
        && [ -z "$CIRCLECI" ] \
        && [ -z "$BUILDKITE" ] \
        && [ -z "$CONTINUOUS_INTEGRATION" ]; then
      _LINTHIS_W=$(printf '\033[0;37m')
      _LINTHIS_WR=$(printf '\033[0;37m')
    fi
    ;;
esac

# Expose paint state to child processes so the outer `linthis hook run`
# (invoked by git, above this script) and any sub-linthis can align without
# re-running the detection. CLICOLOR_FORCE makes `colored`-crate output still
# emit codes under a pipe.
if [ -n "$_LINTHIS_W" ]; then
  export LINTHIS_HOOK_PAINT=white
  export CLICOLOR_FORCE=1
fi

_linthis_paint() {
  if [ -z "$_LINTHIS_W" ]; then
    cat
  else
    # awk filter: replace every `ESC[0m` (reset) with `ESC[0;37m` so text
    # after an inner colour block continues in white, then wrap the whole
    # line with white + final reset. Existing coloured segments (cyan tips,
    # green ✓, red ✗, box borders) keep their colour; only unformatted
    # stretches gain the white tint that IDE VCS consoles need.
    # gsub takes a regex, so escape the `[` in the RESET pattern.
    awk 'BEGIN { ESC = sprintf("\033"); RESET = ESC "[0m"; RESET_RE = ESC "\\[0m"; WHITE = ESC "[0;37m" }
         { gsub(RESET_RE, WHITE); print WHITE $0 RESET; fflush() }'
  fi
}

# Run "$@" with stderr folded into stdout, pipe the combined output through
# _linthis_paint, and propagate the command's exit status. A bare pipe would
# report awk's (always 0) exit, which silently broke the caller's
# `if [ $LINTHIS_EXIT -ne 0 ]` flow. Tempfile path is POSIX-portable; no
# PIPESTATUS or `set -o pipefail` (both non-POSIX) required.
_linthis_run_painted() {
  if [ -z "$_LINTHIS_W" ]; then
    "$@" 2>&1
    return $?
  fi
  _lrp_xf=$(mktemp 2>/dev/null || printf '/tmp/linthis-exit.%d' "$$")
  { "$@" 2>&1; printf '%s\n' "$?" > "$_lrp_xf"; } | _linthis_paint
  _lrp_rc=$(cat "$_lrp_xf" 2>/dev/null || echo 1)
  rm -f "$_lrp_xf"
  return "$_lrp_rc"
}
"#
}

/// The review prompt for the pre-push agent code review.
pub(crate) fn prepush_review_prompt() -> &'static str {
    "Perform a structured pre-push code review using the lt.review skill. \
     Steps: \
     (1) Run: BASE_SHA=$(git merge-base HEAD origin/main 2>/dev/null || git rev-parse HEAD~1); \
     git diff $BASE_SHA..HEAD --stat; git diff $BASE_SHA..HEAD --name-status; git diff $BASE_SHA..HEAD. \
     (2) Review for Critical (security, data loss, broken API, logic errors), \
     Important (missing error handling, performance), and Minor issues. \
     (3) Get the review dir: _SLUG=$(git rev-parse --show-toplevel 2>/dev/null | tr '/' '-' | sed 's/^-//'); \
     _REVIEW_DIR=\"$HOME/.linthis/projects/$_SLUG/review/result\"; mkdir -p \"$_REVIEW_DIR\". \
     Write the review to $_REVIEW_DIR/review-$(date +%Y%m%d-%H%M%S).md. \
     (4) If Critical issues found: save a snapshot with 'git diff', auto-fix the issues, \
     then run build/test to verify fixes don't break anything \
     (detect project type: cargo check && cargo test for Rust, \
     go build ./... && go test ./... for Go, \
     npx tsc --noEmit for TypeScript, python -m py_compile for Python). \
     If build/tests fail, revert and retry with a different approach. \
     After fixing, display a Changes Summary showing each file, what changed, and why, \
     plus the full git diff. Then re-run the review. \
     Print '❌ Push blocked — fix Critical issues first' and exit 1 if issues remain. \
     If Important issues only: print '⚠️ Push with caution'. \
     If Minor or none: print '✅ Review passed'. \
     Exit 0 unless Critical issues were found."
}

/// Build the pre-push hook script that ALWAYS triggers an agent code review.
pub(crate) fn build_git_with_agent_prepush_script(
    linthis_cmd: &str,
    fix_provider: &AgentFixProvider,
    provider_args: Option<&str>,
) -> String {
    let agent_cmd = agent_fix_headless_cmd(fix_provider, prepush_review_prompt(), provider_args);
    let timer_fns = shell_timer_functions();
    let review_box = shell_review_box_fn();
    let fix_commit_mode_section = shell_read_fix_commit_mode("pre_push");
    format!(
        "#!/bin/sh\n\
         {timer}\
         {review_box}\
         \n\
         # Read fix_commit_mode from config\n\
         {fix_commit_mode}\
         \n\
         # Compute files changed in commits being pushed vs upstream (or HEAD~1 as fallback)\n\
         _BASE=$(git rev-parse '@{{u}}' 2>/dev/null || \\\n\
         \x20       git merge-base HEAD origin/main 2>/dev/null || \\\n\
         \x20       git rev-parse 'HEAD~1' 2>/dev/null)\n\
         _PUSHED_FILES=$(git diff --name-only \"$_BASE\"..HEAD 2>/dev/null | grep -v '^$')\n\
         \n\
         # Run lint check on pushed files only (skip if no file changes, e.g. empty commits)\n\
         # Build -i <file> args for each pushed file (linthis uses -i, not positional paths)\n\
         _LINTHIS_CHECKED=0\n\
         if [ -n \"$_PUSHED_FILES\" ]; then\n\
         \x20 set --\n\
         \x20 while IFS= read -r _F; do set -- \"$@\" -i \"$_F\"; done <<_EOF_\n\
         $_PUSHED_FILES\n\
         _EOF_\n\
         \x20 _LINT_OUT=$({linthis} \"$@\" 2>&1)\n\
         \x20 LINTHIS_EXIT=$?\n\
         \x20 printf \"%s\\n\" \"$_LINT_OUT\" >&2\n\
         \x20 # Extract actual number of files checked from linthis output\n\
         \x20 _LINTHIS_CHECKED=$(printf \"%s\" \"$_LINT_OUT\" | sed -n 's/.*Files checked:[[:space:]]*\\([0-9]*\\).*/\\1/p' | tail -1)\n\
         \x20 _LINTHIS_CHECKED=${{_LINTHIS_CHECKED:-0}}\n\
         {prepush_fix_commit_mode_handler}\\
         fi\n\
         \n\
         # Skip agent review if no files were actually checked\n\
         if [ -z \"$_PUSHED_FILES\" ] || [ \"$_LINTHIS_CHECKED\" = \"0\" ]; then\n\
         \x20 echo \"[linthis] No files to review — skipping code review\" >&2\n\
         \x20 exit 0\n\
         fi\n\
         \n\
         # Check if agent provider is available before review\n\
         {agent_check}\
         if [ \"$_LINTHIS_AGENT_OK\" = \"0\" ]; then\n\
         \x20 exit 0\n\
         fi\n\
         \n\
         # Invoke agent code review before push\n\
         echo \"[linthis] Invoking {provider} code review...\" >&2\n\
         start_timer \"Reviewing with {provider}\"\n\
         {agent}\n\
         REVIEW_EXIT=$?\n\
         stop_timer\n\
         \n\
         {agent_fix_commit_handler}\
         # Find the latest review report and check for critical issues\n\
         _SLUG=$(git rev-parse --show-toplevel 2>/dev/null | tr '/' '-' | sed 's/^-//')\n\
         _REVIEW_DIR=\"$HOME/.linthis/projects/$_SLUG/review/result\"\n\
         REVIEW_REPORT=$(ls -t \"$_REVIEW_DIR\"/review-*.md 2>/dev/null | head -1)\n\
         if [ -n \"$REVIEW_REPORT\" ]; then\n\
         \x20 # Check for actual critical issues (agent exit code is unreliable)\n\
         \x20 _CRITICAL=$(awk '/^## Critical Issues/{{found=1;next}} found && /^## /{{found=0}} found && /^- \\[/{{print}}' \"$REVIEW_REPORT\")\n\
         \x20 if [ -n \"$_CRITICAL\" ]; then\n\
         \x20\x20\x20 _print_review_box \"blocked\" \"Critical issues found — fix before pushing\"\n\
         \x20\x20\x20 echo \"[linthis] Review saved: $REVIEW_REPORT\" >&2\n\
         \x20\x20\x20 exit 1\n\
         \x20 else\n\
         \x20\x20\x20 _print_review_box \"passed\" \"No critical issues found\"\n\
         \x20\x20\x20 echo \"[linthis] Review saved: $REVIEW_REPORT\" >&2\n\
         \x20 fi\n\
         fi\n\
         \n\
         exit $REVIEW_EXIT\n",
        timer = timer_fns,
        review_box = review_box,
        fix_commit_mode = fix_commit_mode_section,
        prepush_fix_commit_mode_handler = shell_prepush_fix_commit_mode_handler(linthis_cmd),
        agent_fix_commit_handler = shell_agent_review_fix_commit_handler(),
        linthis = linthis_cmd,
        provider = fix_provider,
        agent = agent_cmd,
        agent_check = shell_agent_availability_check(fix_provider),
    )
}

/// Build the full git hook shell script with agent fix fallback.
/// Generate the worktree-based agent fix shell snippet.
fn shell_worktree_agent_fix(
    linthis_cmd: &str,
    fix_provider: &AgentFixProvider,
    agent_cmd: &str,
    error_msg: &str,
) -> String {
    let agent_check = shell_agent_availability_check(fix_provider);
    let agent_block = shell_agent_invoke_block(fix_provider, agent_cmd, error_msg, "   ");
    format!(
        "\x20 # Check if agent provider is available before attempting fix\n\
         \x20 {agent_check}\
         \x20 if [ \"$_LINTHIS_AGENT_OK\" = \"1\" ]; then\n\
         \x20\x20\x20 # Backup staged files (safety net for linthis backup undo hook)\n\
         \x20\x20\x20 if [ -n \"$_STAGED_FILES\" ]; then\n\
         \x20\x20\x20\x20\x20 echo \"$_STAGED_FILES\" | tr '\\n' '\\0' | xargs -0 {linthis} backup create -d \"hook-agent-fix\" 2>/dev/null\n\
         \x20\x20\x20 fi\n\
         \x20\x20\x20 # Agent fixes directly in main working tree (backup provides safety net)\n\
         {agent_block}\
         \x20\x20\x20 if [ \"$_AGENT_RAN\" = \"1\" ]; then\n\
         {agent_hint}\
         \x20\x20\x20\x20\x20 # Re-stage files modified by agent\n\
         \x20\x20\x20\x20\x20 if [ -n \"$_STAGED_FILES\" ]; then\n\
         \x20\x20\x20\x20\x20\x20\x20 echo \"$_STAGED_FILES\" | xargs git add\n\
         \x20\x20\x20\x20\x20 fi\n\
         \x20\x20\x20\x20\x20 # Re-verify after agent fix\n\
         \x20\x20\x20\x20\x20 printf \"${{_LINTHIS_W}}[linthis] Re-verifying...${{_LINTHIS_R}}\\n\"\n\
         \x20\x20\x20\x20\x20 _linthis_run_painted $LINTHIS_CMD\n\
         \x20\x20\x20\x20\x20 LINTHIS_EXIT=$?\n\
         \x20\x20\x20 fi\n\
         \x20 fi\n",
        agent_check = agent_check,
        agent_block = agent_block,
        agent_hint = shell_agent_fix_hint("     "),
        linthis = linthis_cmd,
    )
}

/// Build the git-with-agent script for commit-msg hooks (no fix_commit_mode branching).
fn build_git_with_agent_commitmsg_script(
    linthis_cmd: &str,
    fix_provider: &AgentFixProvider,
    provider_args: Option<&str>,
) -> String {
    let agent_cmd = agent_fix_headless_cmd_commit_msg(fix_provider, provider_args);
    let error_msg = agent_fix_error_msg(&HookEvent::CommitMsg);
    let timer_fns = shell_timer_functions();
    let new_msg_print = agent_fix_show_fixed_cmsg("  ");
    let worktree_fix = shell_worktree_agent_fix(linthis_cmd, fix_provider, &agent_cmd, error_msg);
    format!(
        "#!/bin/sh\n\
         {timer}\
         LINTHIS_CMD=\"{linthis}\"\n\
         _STAGED_FILES=$(git diff --cached --name-only)\n\
         \n\
         if [ -z \"$_STAGED_FILES\" ]; then\n\
         \x20 exit 0\n\
         fi\n\
         \n\
         _linthis_run_painted $LINTHIS_CMD\n\
         LINTHIS_EXIT=$?\n\
         if [ -n \"$_STAGED_FILES\" ]; then\n\
         \x20 echo \"$_STAGED_FILES\" | xargs git add\n\
         fi\n\
         \n\
         if [ $LINTHIS_EXIT -ne 0 ]; then\n\
         {worktree_fix}\
         {new_msg_print}\
         fi\n\
         \n\
         exit $LINTHIS_EXIT\n",
        timer = timer_fns,
        linthis = linthis_cmd,
        worktree_fix = worktree_fix,
        new_msg_print = new_msg_print,
    )
}

pub(crate) fn build_git_with_agent_hook_script(
    linthis_cmd: &str,
    fix_provider: &AgentFixProvider,
    hook_event: &HookEvent,
    provider_args: Option<&str>,
) -> String {
    if matches!(hook_event, HookEvent::PrePush) {
        return build_git_with_agent_prepush_script(linthis_cmd, fix_provider, provider_args);
    }
    if matches!(hook_event, HookEvent::CommitMsg) {
        return build_git_with_agent_commitmsg_script(linthis_cmd, fix_provider, provider_args);
    }

    let prompt = agent_fix_prompt_for_event(hook_event);
    let agent_cmd = agent_fix_headless_cmd(fix_provider, &prompt, provider_args);
    let error_msg = agent_fix_error_msg(hook_event);
    let timer_fns = shell_timer_functions();
    let worktree_fix = shell_worktree_agent_fix(linthis_cmd, fix_provider, &agent_cmd, error_msg);

    // For pre-commit (and post-commit), add fix_commit_mode branching
    let fix_commit_mode_section = shell_read_fix_commit_mode("pre_commit");
    let linthis_check_only = linthis_cmd.replace("-c -f", "-c");
    let save_diff = shell_save_diff_patch();

    format!(
        "#!/bin/sh\n\
         {timer}\
         _STAGED_FILES=$(git diff --cached --name-only)\n\
         \n\
         # Skip entirely if no staged files (empty commit)\n\
         if [ -z \"$_STAGED_FILES\" ]; then\n\
         \x20 exit 0\n\
         fi\n\
         \n\
         # Read fix_commit_mode from config\n\
         {fix_commit_mode}\
         \n\
         if [ \"$_FIX_MODE\" = \"fixup\" ]; then\n\
         \x20 # fixup: check only, let commit through, post-commit handles format\n\
         \x20 _linthis_run_painted {linthis_check_only}\n\
         \x20 exit 0\n\
         fi\n\
         \n\
         # squash / dirty: run check + format\n\
         # Snapshot pre-format state for stash (squash mode)\n\
         if [ \"$_FIX_MODE\" = \"squash\" ]; then\n\
         \x20 _STASH_REF=$(git stash create 2>/dev/null)\n\
         fi\n\
         \n\
         LINTHIS_CMD=\"{linthis}\"\n\
         _linthis_run_painted $LINTHIS_CMD\n\
         LINTHIS_EXIT=$?\n\
         \n\
         {save_diff}\
         if [ \"$_FIX_MODE\" = \"squash\" ]; then\n\
         \x20 # Re-stage files modified by linthis -f (auto-format)\n\
         \x20 if [ -n \"$_STAGED_FILES\" ]; then\n\
         \x20\x20\x20 echo \"$_STAGED_FILES\" | xargs git add\n\
         \x20 fi\n\
         \x20 # Save stash if files were formatted\n\
         \x20 if [ -n \"$_STASH_REF\" ]; then\n\
         \x20\x20\x20 git stash store -m \"linthis: pre-format snapshot\" \"$_STASH_REF\" 2>/dev/null\n\
         \x20 fi\n\
         elif [ \"$_FIX_MODE\" = \"dirty\" ]; then\n\
         \x20 # dirty: do NOT re-stage, block commit if files changed\n\
         \x20 _DIRTY=$(git diff --name-only)\n\
         \x20 if [ -n \"$_DIRTY\" ]; then\n\
         \x20\x20\x20 echo \"[linthis] Files formatted but not staged (dirty mode).\" >&2\n\
         \x20\x20\x20 echo \"  Review:  git diff\" >&2\n\
         \x20\x20\x20 echo \"  Accept:  git add -u && git commit\" >&2\n\
         \x20\x20\x20 echo \"  Revert:  linthis backup undo\" >&2\n\
         \x20\x20\x20 exit 1\n\
         \x20 fi\n\
         fi\n\
         \n\
         if [ $LINTHIS_EXIT -ne 0 ]; then\n\
         {worktree_fix}\
         fi\n\
         \n\
         exit $LINTHIS_EXIT\n",
        timer = timer_fns,
        fix_commit_mode = fix_commit_mode_section,
        linthis_check_only = linthis_check_only,
        linthis = linthis_cmd,
        worktree_fix = worktree_fix,
        save_diff = save_diff,
    )
}

/// Generate the pre-push fix_commit_mode handler shell snippet.
fn shell_prepush_fix_commit_mode_handler(linthis_cmd: &str) -> String {
    let save_diff = shell_save_diff_patch();
    format!(
        "\x20 # Handle fix_commit_mode for pre-push\n\
         \x20 if [ \"$LINTHIS_EXIT\" -ne 0 ] && [ \"$_FIX_MODE\" = \"dirty\" ]; then\n\
         \x20\x20\x20 exit $LINTHIS_EXIT\n\
         \x20 fi\n\
         \x20 if [ \"$LINTHIS_EXIT\" -ne 0 ] && [ \"$_FIX_MODE\" = \"squash\" ]; then\n\
         \x20\x20\x20 # Format + amend latest commit\n\
         \x20\x20\x20 _STASH_REF=$(git stash create 2>/dev/null)\n\
         \x20\x20\x20 {linthis} \"$@\" -f 2>&1\n\
         \x20\x20\x20 _CHANGED=$(git diff --name-only)\n\
         \x20\x20\x20 if [ -n \"$_CHANGED\" ]; then\n\
         \x20\x20\x20\x20\x20 {save_diff}\
         \x20\x20\x20\x20\x20 echo \"$_CHANGED\" | xargs git add\n\
         \x20\x20\x20\x20\x20 # Create fixup commit (preserved in reflog), then squash into previous\n\
         \x20\x20\x20\x20\x20 git commit --no-verify -m \"fix(linthis): auto-fix lint issues\"\n\
         \x20\x20\x20\x20\x20 git reset --soft HEAD~2\n\
         \x20\x20\x20\x20\x20 git commit --no-verify -C HEAD@{{2}}\n\
         \x20\x20\x20\x20\x20 [ -n \"$_STASH_REF\" ] && git stash store -m \"linthis: pre-format snapshot\" \"$_STASH_REF\" 2>/dev/null\n\
         \x20\x20\x20\x20\x20 echo \"[linthis] Lint fixes squashed into latest commit. Review with 'git diff HEAD~1', then 'git push' again.\" >&2\n\
         \x20\x20\x20\x20\x20 exit 1\n\
         \x20\x20\x20 fi\n\
         \x20 fi\n\
         \x20 if [ \"$LINTHIS_EXIT\" -ne 0 ] && [ \"$_FIX_MODE\" = \"fixup\" ]; then\n\
         \x20\x20\x20 # Format + create fixup commit, then block push for review\n\
         \x20\x20\x20 {linthis} \"$@\" -f 2>&1\n\
         \x20\x20\x20 _CHANGED=$(git diff --name-only)\n\
         \x20\x20\x20 if [ -n \"$_CHANGED\" ]; then\n\
         \x20\x20\x20\x20\x20 {save_diff}\
         \x20\x20\x20\x20\x20 echo \"$_CHANGED\" | xargs git add\n\
         \x20\x20\x20\x20\x20 git commit --no-verify -m \"fix(linthis): auto-fix lint issues\"\n\
         \x20\x20\x20\x20\x20 echo \"[linthis] Created fixup commit. Review with 'git log --oneline -2', then 'git push' again.\" >&2\n\
         {mode_hint_pre_push}\
         \x20\x20\x20\x20\x20 exit 1\n\
         \x20\x20\x20 fi\n\
         \x20 fi\n",
        linthis = linthis_cmd,
        save_diff = save_diff,
        mode_hint_pre_push = shell_fix_commit_mode_hint_pre_push("     "),
    )
}

/// Generate shell snippet to handle agent review fixes based on fix_commit_mode.
fn shell_agent_review_fix_commit_handler() -> String {
    let save_diff = shell_save_diff_patch();
    format!(
    "# Handle agent's file changes based on fix_commit_mode\n\
         _AGENT_CHANGED=$(git diff --name-only)\n\
         if [ -n \"$_AGENT_CHANGED\" ]; then\n\
         \x20 {save_diff}\
         \x20 if [ \"$_FIX_MODE\" = \"squash\" ]; then\n\
         \x20\x20\x20 echo \"$_AGENT_CHANGED\" | xargs git add\n\
         \x20\x20\x20 # Create fixup commit (preserved in reflog), then squash into previous\n\
         \x20\x20\x20 git commit --no-verify -m \"fix(linthis): auto-fix review issues\"\n\
         \x20\x20\x20 git reset --soft HEAD~2\n\
         \x20\x20\x20 git commit --no-verify -C HEAD@{{2}}\n\
         \x20\x20\x20 echo \"[linthis] Agent fixes squashed into latest commit. Review with 'git diff HEAD~1', then 'git push' again.\" >&2\n\
         \x20\x20\x20 exit 1\n\
         \x20 elif [ \"$_FIX_MODE\" = \"fixup\" ]; then\n\
         \x20\x20\x20 echo \"$_AGENT_CHANGED\" | xargs git add\n\
         \x20\x20\x20 git commit --no-verify -m \"fix(linthis): auto-fix review issues\"\n\
         \x20\x20\x20 echo \"[linthis] Created fixup commit with agent fixes. Review with 'git log --oneline -2', then 'git push' again.\" >&2\n\
         \x20\x20\x20 {mode_hint_pre_push}\
         \x20\x20\x20 exit 1\n\
         \x20 else\n\
         \x20\x20\x20 echo \"[linthis] Agent fixes left in working tree (dirty mode). Review: git diff, Revert: linthis backup undo\" >&2\n\
         \x20\x20\x20 exit 1\n\
         \x20 fi\n\
         fi\n\
         \n",
    save_diff = save_diff,
    mode_hint_pre_push = shell_fix_commit_mode_hint_pre_push("   "),
    )
}

/// Generate shell snippet for `git` type fix_commit_mode handling.
/// For pre-commit: re-stage formatted files based on mode.
/// For pre-push: handle format changes based on mode.
fn shell_git_fix_commit_mode_handler(hook_event: &HookEvent) -> String {
    if matches!(hook_event, HookEvent::PreCommit) {
        // Pre-commit: handle staged files after format
        let save_diff = shell_save_diff_patch();
        format!(
        "\x20\x20\x20 _STAGED_FILES=$(git diff --cached --name-only)\n\
         \x20\x20\x20 _DIRTY=$(git diff --name-only)\n\
         \x20\x20\x20 if [ \"$_FIX_MODE\" = \"squash\" ] && [ -n \"$_DIRTY\" ]; then\n\
         \x20\x20\x20\x20\x20 {save_diff}\
         \x20\x20\x20\x20\x20 echo \"$_STAGED_FILES\" | xargs git add\n\
         \x20\x20\x20\x20\x20 # Save stash snapshot\n\
         \x20\x20\x20\x20\x20 if [ -n \"$_STASH_REF\" ]; then\n\
         \x20\x20\x20\x20\x20\x20\x20 git stash store -m \"linthis: pre-format snapshot\" \"$_STASH_REF\" 2>/dev/null\n\
         \x20\x20\x20\x20\x20 fi\n\
         \x20\x20\x20 elif [ \"$_FIX_MODE\" = \"squash\" ] && [ -n \"$_STAGED_FILES\" ]; then\n\
         \x20\x20\x20\x20\x20 echo \"$_STAGED_FILES\" | xargs git add\n\
         \x20\x20\x20 elif [ \"$_FIX_MODE\" = \"dirty\" ]; then\n\
         \x20\x20\x20\x20\x20 _DIRTY=$(git diff --name-only)\n\
         \x20\x20\x20\x20\x20 if [ -n \"$_DIRTY\" ]; then\n\
         \x20\x20\x20\x20\x20\x20\x20 {save_diff}\
         \x20\x20\x20\x20\x20\x20\x20 echo \"[linthis] Files formatted but not staged (dirty mode).\" >&2\n\
         \x20\x20\x20\x20\x20\x20\x20 echo \"  Review:  git diff\" >&2\n\
         \x20\x20\x20\x20\x20\x20\x20 echo \"  Accept:  git add -u && git commit\" >&2\n\
         \x20\x20\x20\x20\x20\x20\x20 echo \"  Revert:  linthis backup undo\" >&2\n\
         \x20\x20\x20\x20\x20\x20\x20 exit 1\n\
         \x20\x20\x20\x20\x20 fi\n\
         \x20\x20\x20 elif [ \"$_FIX_MODE\" = \"fixup\" ]; then\n\
         \x20\x20\x20\x20\x20 # fixup for git type: check only, no format (post-commit handles it)\n\
         \x20\x20\x20\x20\x20 true\n\
         \x20\x20\x20 fi\n",
         save_diff = save_diff,
        )
    } else if matches!(hook_event, HookEvent::PrePush) {
        // Pre-push: same as git-with-agent's prepush handler
        "".to_string() // Pre-push git type doesn't format, just checks
    } else {
        String::new()
    }
}

/// Generate shell snippet to save linthis changes as a git patch file.
/// Must be called while changes are still unstaged (before `git add`).
/// Uses `_SLUG` variable which must be set earlier in the script.
fn shell_save_diff_patch() -> String {
    let keep = load_retention_diffs();
    format!(
        "# Save linthis changes as a git patch for review/revert\n\
         _SLUG=$(git rev-parse --show-toplevel 2>/dev/null | tr '/' '-' | sed 's/^-//')\n\
         _DIFF_DIR=\"$HOME/.linthis/projects/$_SLUG/diff\"\n\
         mkdir -p \"$_DIFF_DIR\"\n\
         _DIFF_FILE=\"$_DIFF_DIR/diff-$(date +%Y%m%d-%H%M%S).patch\"\n\
         git diff > \"$_DIFF_FILE\" 2>/dev/null\n\
         if [ -s \"$_DIFF_FILE\" ]; then\n\
         \x20 echo \"[linthis] Patch saved: $_DIFF_FILE\" >&2\n\
         \x20 echo \"  Revert: git apply -R $_DIFF_FILE\" >&2\n\
         \x20 ls -t \"$_DIFF_DIR\"/diff-*.patch 2>/dev/null | tail -n +{keep_plus_one} | xargs rm -f 2>/dev/null\n\
         else\n\
         \x20 rm -f \"$_DIFF_FILE\"\n\
         fi\n",
        keep_plus_one = keep + 1,
    )
}

/// Same as `shell_save_diff_patch` but for staged changes (after `git add`).
#[allow(dead_code)]
fn shell_save_diff_patch_cached() -> String {
    let keep = load_retention_diffs();
    format!(
        "# Save linthis changes as a git patch for review/revert\n\
         _SLUG=$(git rev-parse --show-toplevel 2>/dev/null | tr '/' '-' | sed 's/^-//')\n\
         _DIFF_DIR=\"$HOME/.linthis/projects/$_SLUG/diff\"\n\
         mkdir -p \"$_DIFF_DIR\"\n\
         _DIFF_FILE=\"$_DIFF_DIR/diff-$(date +%Y%m%d-%H%M%S).patch\"\n\
         git diff --cached > \"$_DIFF_FILE\" 2>/dev/null\n\
         if [ -s \"$_DIFF_FILE\" ]; then\n\
         \x20 echo \"[linthis] Patch saved: $_DIFF_FILE\" >&2\n\
         \x20 echo \"  Revert: git apply -R $_DIFF_FILE\" >&2\n\
         \x20 ls -t \"$_DIFF_DIR\"/diff-*.patch 2>/dev/null | tail -n +{keep_plus_one} | xargs rm -f 2>/dev/null\n\
         else\n\
         \x20 rm -f \"$_DIFF_FILE\"\n\
         fi\n",
        keep_plus_one = keep + 1,
    )
}

/// Load retention.diffs config value.
fn load_retention_diffs() -> usize {
    let project_root = linthis::utils::get_project_root();
    linthis::config::Config::load_merged(&project_root)
        .retention
        .diffs
}

/// Generate a POSIX shell snippet that re-stages ONLY working-tree changes to
/// files listed in `$_FILES` (the set produced by the post-commit script from
/// `git diff-tree ... HEAD`).
///
/// Without this restriction, `git diff --name-only | xargs git add` would also
/// pick up unrelated, previously-unstaged modifications to other files, and
/// they'd leak into the fixup commit. We iterate the committed scope instead
/// so a file is only staged if it was part of the original commit AND was
/// further modified by linthis/agent.
fn shell_restage_committed_scope(indent: &str) -> String {
    // Heredoc body and terminator MUST start at column 0 (unquoted `<<EOF`
    // form — `<<-` would only strip tabs, and shell syntax requires the
    // closing marker to match verbatim at the beginning of a line). If we
    // indented `$_FILES` or `_RESTAGE_EOF_`, the heredoc would never close
    // and the shell would fail with "unexpected end of file", and the first
    // filename would be prefixed with stray leading whitespace.
    format!(
        "{i}# Re-stage only files in the committed scope that were further modified.\n\
         {i}# (Prevents unrelated unstaged working-tree changes from leaking into the fixup commit.)\n\
         {i}while IFS= read -r _F; do\n\
         {i}  [ -z \"$_F\" ] && continue\n\
         {i}  git add -- \"$_F\" 2>/dev/null || true\n\
         {i}done <<_RESTAGE_EOF_\n\
         $_FILES\n\
         _RESTAGE_EOF_\n",
        i = indent,
    )
}

/// Generate shell snippet to read fix_commit_mode from linthis config.
fn shell_read_fix_commit_mode(config_section: &str) -> String {
    let default = if config_section == "pre_commit" {
        "squash"
    } else {
        "dirty"
    };
    // Try project config first, then global config, then built-in default.
    // `linthis config get` without `--global` errors out when no project
    // config exists (`.linthis/config.toml` missing), so we need to fall
    // through to `--global` explicitly to honour user's global setting.
    format!(
        "_FIX_MODE=$(linthis config get hook.{section}.fix_commit_mode 2>/dev/null || \
         linthis config get hook.{section}.fix_commit_mode --global 2>/dev/null || \
         echo \"{default}\")\n",
        section = config_section,
        default = default,
    )
}

/// Build a post-commit hook script for fixup fix mode.
pub(crate) fn build_post_commit_script(linthis_cmd: &str) -> String {
    let timer_fns = shell_timer_functions();
    let fix_commit_mode_section = shell_read_fix_commit_mode("pre_commit");
    let restage_scope = shell_restage_committed_scope(" ");
    format!(
        "#!/bin/sh\n\
         {timer}\
         # Read fix_commit_mode — only activate in fixup mode\n\
         {fix_commit_mode}\
         if [ \"$_FIX_MODE\" != \"fixup\" ]; then\n\
         \x20 exit 0\n\
         fi\n\
         \n\
         # Get files from the commit that was just created\n\
         _FILES=$(git diff-tree --no-commit-id --name-only -r HEAD)\n\
         [ -z \"$_FILES\" ] && exit 0\n\
         \n\
         # Build -i <file> args and run linthis once for all committed files\n\
         # (single invocation = single [Post-commit] output box)\n\
         set --\n\
         while IFS= read -r _F; do set -- \"$@\" -i \"$_F\"; done <<_EOF_\n\
         $_FILES\n\
         _EOF_\n\
         _linthis_run_painted {linthis} \"$@\"\n\
         \n\
         # Restrict staging to the committed scope, then make the fixup commit\n\
         # only if that scope actually changed.\n\
         {restage}\
         _STAGED=$(git diff --cached --name-only)\n\
         if [ -n \"$_STAGED\" ]; then\n\
         \x20 # Pipe git's commit summary through _linthis_paint so IDE VCS\n\
         \x20 # consoles don't render it red.\n\
         \x20 git commit --no-verify -m \"fix(linthis): auto-fix lint issues\" 2>&1 | _linthis_paint\n\
         \x20 printf \"${{_LINTHIS_W}}[linthis] Created fixup commit with format changes${{_LINTHIS_R}}\\n\"\n\
         {mode_hint}\
         fi\n",
        timer = timer_fns,
        fix_commit_mode = fix_commit_mode_section,
        linthis = linthis_cmd,
        restage = restage_scope,
        mode_hint = shell_fix_commit_mode_hint_pre_commit(" "),
    )
}

/// Build a post-commit hook script for git-with-agent fixup mode.
/// Formats committed files, then invokes the agent for any remaining issues,
/// and creates a single fixup commit with all changes.
pub(crate) fn build_post_commit_with_agent_script(
    linthis_fmt_cmd: &str,
    fix_provider: &AgentFixProvider,
    provider_args: Option<&str>,
) -> String {
    let timer_fns = shell_timer_functions();
    let fix_commit_mode_section = shell_read_fix_commit_mode("pre_commit");
    let prompt = agent_fix_prompt_for_post_commit();
    let agent_cmd = agent_fix_headless_cmd(fix_provider, &prompt, provider_args);
    let error_msg = "Lint issues remain after formatting";
    let agent_check = shell_agent_availability_check(fix_provider);
    let agent_block = shell_agent_invoke_block(fix_provider, &agent_cmd, error_msg, "  ");
    let save_diff = shell_save_diff_patch_for_post_commit("         ");
    let agent_hint = shell_agent_fix_hint_post_commit("  ");
    let restage_scope_top = shell_restage_committed_scope("");
    let restage_scope_agent = shell_restage_committed_scope("      ");
    format!(
        "#!/bin/sh\n\
         {timer}\
         # Read fix_commit_mode — only activate in fixup mode\n\
         {fix_commit_mode}\
         if [ \"$_FIX_MODE\" != \"fixup\" ]; then\n\
         \x20 exit 0\n\
         fi\n\
         \n\
         # Get files from the commit that was just created\n\
         _FILES=$(git diff-tree --no-commit-id --name-only -r HEAD)\n\
         [ -z \"$_FILES\" ] && exit 0\n\
         \n\
         # Build -i <file> args and run formatter on all committed files\n\
         set --\n\
         while IFS= read -r _F; do set -- \"$@\" -i \"$_F\"; done <<_EOF_\n\
         $_FILES\n\
         _EOF_\n\
         _linthis_run_painted {linthis_fmt} \"$@\"\n\
         _FMT_EXIT=$?\n\
         \n\
         # Stage formatting changes (scoped to the committed-files set)\n\
         {restage_top}\
         \n\
         # If formatter didn't fully fix, try agent\n\
         _DIFF_FILE=\"\"\n\
         if [ \"$_FMT_EXIT\" -ne 0 ]; then\n\
         \x20 {agent_check}\
         \x20 if [ \"$_LINTHIS_AGENT_OK\" = \"1\" ]; then\n\
         {agent_block}\
         \x20\x20\x20 if [ \"$_AGENT_RAN\" = \"1\" ]; then\n\
         \x20\x20\x20\x20\x20 # Stage agent's changes (scoped to the committed-files set)\n\
         {restage_agent}\
         \x20\x20\x20\x20\x20 # Save staged diff patch (format + agent) before fixup commit\n\
         {save_diff}\
         \x20\x20\x20\x20\x20 # Re-verify using the same format command (not check-only) so the\n\
         \x20\x20\x20\x20\x20 # result is consistent with the initial run and shows Passed when\n\
         \x20\x20\x20\x20\x20 # the agent fixed the remaining format/security issues.\n\
         \x20\x20\x20\x20\x20 printf \"${{_LINTHIS_W}}[linthis] Re-verifying...${{_LINTHIS_R}}\\n\"\n\
         \x20\x20\x20\x20\x20 _linthis_run_painted {linthis_fmt} \"$@\"\n\
         \x20\x20\x20 fi\n\
         \x20 fi\n\
         fi\n\
         \n\
         # Create fixup commit if anything was staged (format + agent changes)\n\
         _STAGED=$(git diff --cached --name-only)\n\
         if [ -n \"$_STAGED\" ]; then\n\
         \x20 # Pipe git's commit summary through _linthis_paint so IDE VCS\n\
         \x20 # consoles don't render it red.\n\
         \x20 git commit --no-verify -m \"fix(linthis): auto-fix lint issues\" 2>&1 | _linthis_paint\n\
         \x20 printf \"${{_LINTHIS_W}}[linthis] Created fixup commit with format changes${{_LINTHIS_R}}\\n\"\n\
         {mode_hint_pre_commit}\
         \x20 # Show hint and diff path after fixup commit so HEAD~1 is correct\n\
         \x20 if [ \"$_AGENT_RAN\" = \"1\" ]; then\n\
         {agent_hint}\
         \x20\x20\x20 [ -n \"$_DIFF_FILE\" ] && printf \"${{_LINTHIS_W}}[linthis]   Undo (by patch created) : \\033[0;33mgit apply -R $_DIFF_FILE${{_LINTHIS_R}}\\n\"\n\
         \x20 fi\n\
         fi\n",
        timer = timer_fns,
        fix_commit_mode = fix_commit_mode_section,
        linthis_fmt = linthis_fmt_cmd,
        agent_check = agent_check,
        agent_block = agent_block,
        save_diff = save_diff,
        agent_hint = agent_hint,
        restage_top = restage_scope_top,
        restage_agent = restage_scope_agent,
        mode_hint_pre_commit = shell_fix_commit_mode_hint_pre_commit(" "),
    )
}

/// Build the linthis command for a hook based on event type and extra args
pub(crate) fn build_hook_command(hook_event: &HookEvent, args: &Option<String>) -> String {
    match hook_event {
        HookEvent::PreCommit => {
            // For pre-commit: check + format staged files
            // Default "-c -f" = RunMode::Both (check AND format)
            let extra = args.as_deref().unwrap_or("-c -f");
            format!("linthis -s {} --hook-event=pre-commit", extra)
        }
        HookEvent::PrePush => {
            // For pre-push: check only (formatting should happen at pre-commit stage)
            // Default "-c" = RunMode::CheckOnly
            let extra = args.as_deref().unwrap_or("-c");
            format!("linthis {} --hook-event=pre-push", extra)
        }
        HookEvent::CommitMsg => {
            // For commit-msg: validate commit message using the msg file passed as $1
            "linthis cmsg \"$1\"".to_string()
        }
        HookEvent::PostCommit => {
            // For post-commit: check + format (same as pre-commit) so lint-only
            // rules like line-too-long are caught and passed to the agent.
            // Using only "-f" silently skips rules the formatter can't auto-fix.
            let extra = args.as_deref().unwrap_or("-c -f");
            format!("linthis {} --hook-event=post-commit", extra)
        }
    }
}

/// Get the git action for a hook event
pub(crate) fn hook_action(hook_event: &HookEvent) -> &'static str {
    match hook_event {
        HookEvent::PreCommit => "commit",
        HookEvent::PrePush => "push",
        HookEvent::CommitMsg => "commit",
        HookEvent::PostCommit => "commit",
    }
}

/// All AgentFixProvider variants in detection-priority order
pub(crate) const ALL_AGENT_FIX_PROVIDERS: &[AgentFixProvider] = &[
    AgentFixProvider::Claude,
    AgentFixProvider::Codex,
    AgentFixProvider::Gemini,
    AgentFixProvider::Cursor,
    AgentFixProvider::Droid,
    AgentFixProvider::Auggie,
    AgentFixProvider::Codebuddy,
    AgentFixProvider::Openclaw,
];

/// Split a `provider[/model]` string into (provider_name, Option<model>).
///
/// Examples:
///   "claude"       → ("claude", None)
///   "claude/opus"  → ("claude", Some("opus"))
///   "gemini/flash" → ("gemini", Some("flash"))
pub(crate) fn parse_provider_with_model(raw: &str) -> (&str, Option<&str>) {
    match raw.split_once('/') {
        Some((provider, model)) if !model.is_empty() => (provider, Some(model)),
        _ => (raw, None),
    }
}

/// Merge a model extracted from `provider/model` syntax into existing provider_args.
///
/// If `--provider-args` already contains `--model`, the `/model` part is ignored
/// and a warning is printed (explicit `--provider-args` takes precedence).
pub(crate) fn merge_model_into_provider_args(
    model: Option<&str>,
    existing: Option<&str>,
) -> Option<String> {
    use colored::Colorize;
    // Check if existing provider_args already specifies --model
    if let (Some(m), Some(pa)) = (model, existing) {
        if pa.contains("--model") {
            eprintln!(
                "{}: --provider-args already contains --model, ignoring '{}' from provider/model syntax",
                "Warning".yellow(), m
            );
            return Some(pa.to_string());
        }
    }
    match (model, existing) {
        (Some(m), Some(pa)) => Some(format!("--model {} {}", m, pa)),
        (Some(m), None) => Some(format!("--model {}", m)),
        (None, Some(pa)) => Some(pa.to_string()),
        (None, None) => None,
    }
}

/// Detect which AgentFixProvider CLIs are available in PATH
pub(crate) fn detect_agent_fix_providers() -> Vec<AgentFixProvider> {
    ALL_AGENT_FIX_PROVIDERS
        .iter()
        .filter(|p| super::is_command_available(agent_fix_bin(p).as_ref()))
        .cloned()
        .collect()
}

/// Resolve AgentFixProvider from an optional --provider string.
/// - If specified: parse and validate.
/// - If not specified + yes: auto-detect first available CLI.
/// - If not specified + interactive: show selection menu.
pub(crate) fn resolve_agent_fix_provider(
    provider: Option<&str>,
    yes: bool,
) -> Result<AgentFixProvider, std::process::ExitCode> {
    use colored::Colorize;
    use std::process::ExitCode;

    if let Some(p) = provider {
        if let Some(known) = parse_agent_fix_provider_name(p) {
            return Ok(known);
        }
        // Not a built-in provider — look up in [ai.custom_providers] config
        let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        let custom = linthis::config::Config::load_project_config(&cwd)
            .as_ref()
            .and_then(|c| c.ai.custom_providers.get(p).cloned())
            .or_else(|| {
                linthis::config::Config::load_user_config()
                    .as_ref()
                    .and_then(|c| c.ai.custom_providers.get(p).cloned())
            });
        if let Some(custom) = custom {
            let bin = match &custom.command {
                Some(c) => c.to_string(),
                None => {
                    eprintln!(
                        "{}: custom provider '{}' requires a 'command' field in [ai.custom_providers.{}]",
                        "Error".red(), p, p
                    );
                    return Err(ExitCode::from(1));
                }
            };
            let style = match &custom.cli_style {
                Some(s) => s.to_string(),
                None => {
                    eprintln!(
                        "{}: custom provider '{}' requires 'cli_style' in [ai.custom_providers.{}]\n  Valid styles: claude, codex, gemini, cursor, droid, auggie, codebuddy, openclaw",
                        "Error".red(), p, p
                    );
                    return Err(ExitCode::from(1));
                }
            };
            return Ok(AgentFixProvider::Custom { bin, style });
        }
        eprintln!(
            "{}: Unknown provider '{}'. Add it to config:\n  [ai.custom_providers.{}]\n  command = \"{}\"\n  cli_style = \"claude\"  # or codex, gemini, cursor, droid, auggie, codebuddy, openclaw",
            "Error".red(), p, p, p
        );
        return Err(ExitCode::from(1));
    }

    let detected = detect_agent_fix_providers();

    if yes {
        // Auto-detect: use first available, default to claude
        return Ok(detected
            .into_iter()
            .next()
            .unwrap_or(AgentFixProvider::Claude));
    }

    // Interactive menu
    use std::io::{self, Write};

    println!("{}", "Select AI agent for automatic fix:".bold());
    println!();

    for (i, p) in ALL_AGENT_FIX_PROVIDERS.iter().enumerate() {
        let available = super::is_command_available(agent_fix_bin(p).as_ref());
        let tag = if available {
            format!(" {}", "(detected)".cyan())
        } else {
            String::new()
        };
        println!("  {}. {}{}", i + 1, p, tag);
    }
    println!();
    print!("Choose [1-{}]: ", ALL_AGENT_FIX_PROVIDERS.len());
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).ok();
    let n: usize = input.trim().parse().unwrap_or(0);

    if n >= 1 && n <= ALL_AGENT_FIX_PROVIDERS.len() {
        Ok(ALL_AGENT_FIX_PROVIDERS[n - 1].clone())
    } else {
        println!("Installation cancelled");
        Err(ExitCode::SUCCESS)
    }
}

/// Parse a provider string into an AgentFixProvider.
pub(crate) fn parse_agent_fix_provider_name(name: &str) -> Option<AgentFixProvider> {
    match name.to_lowercase().as_str() {
        "claude" => Some(AgentFixProvider::Claude),
        "codex" => Some(AgentFixProvider::Codex),
        "gemini" => Some(AgentFixProvider::Gemini),
        "cursor" => Some(AgentFixProvider::Cursor),
        "droid" => Some(AgentFixProvider::Droid),
        "auggie" | "aug" | "augment" => Some(AgentFixProvider::Auggie),
        "codebuddy" => Some(AgentFixProvider::Codebuddy),
        "openclaw" => Some(AgentFixProvider::Openclaw),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn claude_headless_pipes_through_agent_stream() {
        // `claude -p` alone buffers output; real streaming requires
        // --output-format stream-json piped through `linthis agent-stream`
        // for human-readable rendering.
        let cmd = agent_fix_headless_cmd(&AgentFixProvider::Claude, "hi", None);
        assert!(cmd.contains("--verbose"), "got: {cmd}");
        assert!(cmd.contains("--output-format stream-json"), "got: {cmd}");
        assert!(cmd.contains("linthis agent-stream"), "got: {cmd}");
        assert!(cmd.contains(" | "), "must pipe; got: {cmd}");
    }

    #[test]
    fn codebuddy_headless_pipes_through_agent_stream() {
        let cmd = agent_fix_headless_cmd(&AgentFixProvider::Codebuddy, "hi", None);
        assert!(cmd.contains("--output-format stream-json"), "got: {cmd}");
        assert!(cmd.contains("linthis agent-stream"), "got: {cmd}");
    }

    #[test]
    fn commit_msg_claude_pipes_through_agent_stream() {
        let cmd = agent_fix_headless_cmd_commit_msg(&AgentFixProvider::Claude, None);
        assert!(cmd.contains("--output-format stream-json"), "got: {cmd}");
        assert!(cmd.contains("linthis agent-stream"), "got: {cmd}");
    }

    #[test]
    fn fix_block_has_threshold_guard_and_no_spinner() {
        let block = build_agent_fix_block(&AgentFixProvider::Claude, &HookEvent::PreCommit);
        // Threshold guard present and references the env var
        assert!(block.contains("LINTHIS_AGENT_MAX_AUTO_FIX"), "got: {block}");
        // Streaming banner restored now that we actually stream via agent-stream.
        assert!(block.contains("streaming"), "got: {block}");
        // Spinner removed around the agent invocation — start_timer and
        // stop_timer must no longer wrap the agent call in this block.
        assert!(
            !block.contains("start_timer \"Fixing"),
            "spinner should be gone; got: {block}"
        );
        assert!(block.contains("linthis report count"), "got: {block}");
    }

    #[test]
    fn fix_block_reverifies_only_when_agent_actually_ran() {
        let block = build_agent_fix_block(&AgentFixProvider::Claude, &HookEvent::PreCommit);
        // Re-verify branch must be gated on _AGENT_RAN — when the threshold
        // skips the agent, running linthis again is pointless and would
        // just re-print the same failure.
        assert!(block.contains("_AGENT_RAN"), "got: {block}");
        assert!(block.contains("Re-verifying"), "got: {block}");
    }

    #[test]
    fn threshold_parser_does_not_clobber_positional_params() {
        // Regression: `set -- $counts` used to overwrite $@, which broke the
        // commit-msg flow because its agent command depends on $1 holding
        // the commit-message file path. Claude ended up writing its fix
        // to a file literally named "0" (the zero-error count).
        let block = build_agent_fix_block(&AgentFixProvider::Claude, &HookEvent::PreCommit);
        assert!(
            !block.contains("set -- $_AGENT_COUNTS"),
            "parser must not clobber positional params; got: {block}"
        );
        // Positive: uses parameter expansion (%%, ##, #) to split the count string.
        assert!(
            block.contains("_AGENT_ERR=${_AGENT_COUNTS%% *}"),
            "got: {block}"
        );
        assert!(
            block.contains("_AGENT_FILES=${_AGENT_COUNTS##* }"),
            "got: {block}"
        );
    }

    #[test]
    fn commit_msg_block_preserves_msg_file_through_agent_fix() {
        // The cmsg agent command must capture $1 (the real .git/COMMIT_EDITMSG
        // path) before anything else — otherwise the threshold guard's
        // command substitution or $@-mutating shell idioms would clobber it.
        let agent_cmd = agent_fix_headless_cmd_commit_msg(&AgentFixProvider::Claude, None);
        assert!(
            agent_cmd.starts_with("_REAL_MSG=\"$1\";"),
            "cmsg agent command must capture $1 first; got: {agent_cmd}"
        );
        // And the outer fix block must not `set --` or otherwise reassign $@.
        let outer = build_agent_fix_block(&AgentFixProvider::Claude, &HookEvent::CommitMsg);
        assert!(
            !outer.contains("set --"),
            "no positional-param mutation allowed around cmsg agent; got: {outer}"
        );
    }

    #[test]
    fn commit_msg_uses_temp_file_to_bypass_dotgit_block() {
        // Claude (and similar tools) refuse writes to .git/ even with
        // --dangerously-skip-permissions. Workaround: agent edits a temp
        // file, the shell copies it back to $_REAL_MSG. Verify that
        // structure is in place so a future refactor can't quietly break it.
        let cmd = agent_fix_headless_cmd_commit_msg(&AgentFixProvider::Claude, None);

        // Temp file allocation
        assert!(
            cmd.contains("mktemp"),
            "must allocate temp file; got: {cmd}"
        );
        // Seed temp with current message so the agent can read it
        assert!(
            cmd.contains("cp \"$_REAL_MSG\" \"$_MSG_FILE\""),
            "must seed temp file with current message; got: {cmd}"
        );
        // Copy-back conditional on actual change
        assert!(
            cmd.contains("cp \"$_MSG_FILE\" \"$_REAL_MSG\""),
            "must copy fixed message back to .git/; got: {cmd}"
        );
        assert!(cmd.contains("cmp -s"), "must check for changes; got: {cmd}");
        // Cleanup
        assert!(
            cmd.contains("rm -f \"$_MSG_FILE\""),
            "must clean temp; got: {cmd}"
        );
        // Prompt warns the agent off .git/
        assert!(
            cmd.contains("TEMP FILE outside .git/"),
            "prompt must steer agent away from .git/; got: {cmd}"
        );
    }

    #[test]
    fn post_commit_script_restages_only_committed_scope() {
        let s = build_post_commit_script("linthis -c -f --hook-event=post-commit");
        // Must iterate committed files instead of staging every dirty path.
        assert!(
            s.contains("while IFS= read -r _F; do"),
            "restage loop missing: {s}"
        );
        assert!(
            s.contains("git add -- \"$_F\""),
            "per-file scoped add missing: {s}"
        );
        assert!(
            s.contains("<<_RESTAGE_EOF_"),
            "restage heredoc missing: {s}"
        );
        // Old unrestricted-scope staging must be gone so unstaged unrelated
        // changes can no longer leak into the fixup commit.
        assert!(
            !s.contains("echo \"$_CHANGED\" | xargs git add"),
            "unrestricted xargs git add should be removed: {s}"
        );
    }

    #[test]
    fn post_commit_with_agent_script_restages_only_committed_scope() {
        let s = build_post_commit_with_agent_script(
            "linthis -c -f --hook-event=post-commit",
            &AgentFixProvider::Claude,
            None,
        );
        // Two scoped restage blocks expected: after formatter, after agent.
        let count = s.matches("<<_RESTAGE_EOF_").count();
        assert_eq!(
            count, 2,
            "expected 2 scoped restage blocks (fmt + agent), got {count}: {s}"
        );
        // Neither phase may do the old unrestricted staging.
        assert!(
            !s.contains("echo \"$_FMT_CHANGED\" | xargs git add"),
            "unrestricted FMT xargs git add should be removed: {s}"
        );
        assert!(
            !s.contains("echo \"$_AGENT_CHANGED\" | xargs git add"),
            "unrestricted AGENT xargs git add should be removed: {s}"
        );
    }

    /// Heredoc terminator MUST live at column 0 (unquoted `<<EOF` form) or the
    /// shell swallows the rest of the script and errors with "unexpected end
    /// of file" — exactly the regression that hit the agent post-commit script
    /// when the terminator was indented alongside the enclosing `if` block.
    #[test]
    fn restage_heredoc_terminator_is_at_column_zero() {
        for indent in ["", " ", "      "] {
            let snippet = shell_restage_committed_scope(indent);
            for marker in ["$_FILES", "_RESTAGE_EOF_"] {
                let expected = format!("\n{marker}\n");
                assert!(
                    snippet.contains(&expected),
                    "`{marker}` must be on its own line with no leading whitespace \
                     (indent={indent:?}); got:\n{snippet}"
                );
            }
        }
    }

    /// IDE terminals (VS Code, JetBrains) colour stderr red. The post-commit
    /// informational output — git's commit summary, "Created fixup commit…",
    /// the Agent-fix-applied hint, the fix-commit-mode tip, and the "Undo (by
    /// patch created)" line — must therefore go through stdout, not stderr.
    /// Real errors/warnings in this file still use `>&2` and are not checked
    /// here.
    #[test]
    fn post_commit_informational_output_is_stdout() {
        let plain = build_post_commit_script("linthis -c -f --hook-event=post-commit");
        let agent = build_post_commit_with_agent_script(
            "linthis -c -f --hook-event=post-commit",
            &AgentFixProvider::Claude,
            None,
        );

        for (name, script) in [("plain", &plain), ("agent", &agent)] {
            for offender in [
                "Created fixup commit with format changes\" >&2",
                "Agent fix applied\\n\" >&2",
                "switch mode → \\033[0;36mlinthis config set hook.pre_commit.fix_commit_mode",
            ] {
                // The tip line is always present; assert the whole indented
                // printf does not end with `>&2` by checking the contextual
                // substring for its stderr variant.
                if offender.starts_with("switch mode") {
                    let stderr_form = format!(
                        "{offender} [dirty|squash|fixup] -g\\033[0m\\n\" >&2"
                    );
                    assert!(
                        !script.contains(&stderr_form),
                        "{name}: fix-commit-mode tip must not end with >&2"
                    );
                } else {
                    assert!(
                        !script.contains(offender),
                        "{name}: `{offender}` found — informational output must not be stderr"
                    );
                }
            }
            // git's own commit summary must be merged into stdout AND piped
            // through _linthis_paint for the IDE VCS-console white wrap.
            assert!(
                script.contains(
                    "git commit --no-verify -m \"fix(linthis): auto-fix lint issues\" 2>&1 | _linthis_paint"
                ),
                "{name}: git commit summary must be merged + painted: {script}"
            );
        }

        // Agent-specific lines: Re-verifying + Undo-by-patch printf.
        assert!(
            !agent.contains("echo \"[linthis] Re-verifying...\" >&2"),
            "Re-verifying must be stdout"
        );
        assert!(
            !agent
                .contains("[linthis]   Undo (by patch created) : \\033[0;33mgit apply -R $_DIFF_FILE\\033[0m\\n\" >&2"),
            "Undo-by-patch hint must be stdout"
        );

        // Every linthis child-process invocation in the post-commit scripts
        // must route through _linthis_run_painted (which folds stderr in and
        // pipes through the VCS-console white filter while preserving exit
        // status) — or, if still inline, at least carry 2>&1.
        for (name, script) in [("plain", &plain), ("agent", &agent)] {
            let linthis_invocations = script.lines().filter(|l| {
                let trimmed = l.trim_start();
                (trimmed.contains("linthis ")
                    || trimmed.contains("$LINTHIS_CMD")
                    || trimmed.contains("hook-event=post-commit"))
                    && !trimmed.starts_with("#")
                    && !trimmed.starts_with("LINTHIS_CMD=")
                    && trimmed.contains("\"$@\"")
            });
            for line in linthis_invocations {
                let ok = line.contains("_linthis_run_painted") || line.contains("2>&1");
                assert!(
                    ok,
                    "{name}: linthis invocation missing _linthis_run_painted/2>&1 (would leak red stderr to IDE):\n  {line}\n---script---\n{script}"
                );
            }
        }
    }

    #[test]
    #[ignore = "diagnostic dump — run with: cargo test --release --bin linthis -- --ignored dump_pre_commit"]
    fn dump_pre_commit() {
        use crate::cli::commands::HookEvent;
        let linthis_cmd = super::build_hook_command(&HookEvent::PreCommit, &None);
        let script = build_git_with_agent_hook_script(
            &linthis_cmd,
            &AgentFixProvider::Codebuddy,
            &HookEvent::PreCommit,
            Some("--model glm-5.0-ioa"),
        );
        println!("---BEGIN PRE-COMMIT SCRIPT---");
        println!("{script}");
        println!("---END PRE-COMMIT SCRIPT---");
    }

    /// The IDE-VCS-console colour compensation must:
    ///   * wrap our own informational lines with `${_LINTHIS_W}…${_LINTHIS_R}`
    ///   * pipe `git commit`'s summary through `_linthis_paint`
    ///   * honour LINTHIS_HOOK_COLOR via the shell `case` so a real TTY gets
    ///     empty wrap variables (i.e. no visible change).
    /// Runtime behaviour is exercised by piping the script to `sh` with the
    /// env var controlling the mode; we assert stdout is painted in
    /// `force-white` mode and left plain in `off` mode.
    #[test]
    fn ide_color_wrapping_toggles_by_env() {
        use std::io::Write;
        use std::process::{Command, Stdio};

        // Tiny standalone snippet: just the color setup + a sample printf.
        let script = format!(
            "{}\nprintf \"${{_LINTHIS_W}}[linthis] hello${{_LINTHIS_R}}\\n\"\n\
             echo '[plain]' | _linthis_paint\n",
            shell_timer_functions()
        );

        let run = |env_value: Option<&str>| -> String {
            let mut cmd = Command::new("sh");
            cmd.arg("-c")
                .arg(&script)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                // Force "non-TTY, non-CI" so auto mode engages.
                .env_remove("CI")
                .env_remove("GITHUB_ACTIONS")
                .env_remove("GITLAB_CI")
                .env_remove("CIRCLECI")
                .env_remove("BUILDKITE")
                .env_remove("CONTINUOUS_INTEGRATION");
            match env_value {
                Some(v) => {
                    cmd.env("LINTHIS_HOOK_COLOR", v);
                }
                None => {
                    cmd.env_remove("LINTHIS_HOOK_COLOR");
                }
            }
            let out = cmd.output().expect("spawn sh");
            assert!(out.status.success(), "sh failed: {:?}", out);
            String::from_utf8_lossy(&out.stdout).into_owned()
        };

        let forced = run(Some("white"));
        assert!(
            forced.contains("\x1b[0;37m[linthis] hello\x1b[0m"),
            "white must wrap printf; got: {forced:?}"
        );
        assert!(
            forced.contains("\x1b[0;37m[plain]\x1b[0m"),
            "white must wrap _linthis_paint input; got: {forced:?}"
        );

        let off = run(Some("off"));
        // In "off" mode, `_LINTHIS_W` is empty so no white prefix is emitted.
        // `_LINTHIS_R` stays at `\033[0m` so an inner colour (e.g. green ✓)
        // in a line like `printf "${_LINTHIS_W}[linthis] \033[0;32m✓…${_LINTHIS_R}"`
        // still resets properly. A trailing bare reset is visually neutral.
        assert!(
            !off.contains("\x1b[0;37m"),
            "off must not emit white ANSI; got: {off:?}"
        );
        assert!(
            off.contains("[linthis] hello"),
            "off must emit the line text; got: {off:?}"
        );
        // _linthis_paint in off mode is `cat`, no wrapping at all.
        assert!(
            off.contains("[plain]\n"),
            "off must pass _linthis_paint through cat; got: {off:?}"
        );
        let paint_line = off.lines().find(|l| l.contains("[plain]")).unwrap();
        assert!(
            !paint_line.contains("\x1b["),
            "off's _linthis_paint must not add ANSI; got: {paint_line:?}"
        );
    }

    /// `sh -n` parse-check the generated post-commit scripts so any future
    /// refactor that breaks heredoc termination fails CI, not production.
    #[test]
    fn generated_post_commit_scripts_pass_sh_parse_check() {
        use std::io::Write;
        use std::process::{Command, Stdio};

        let scripts = [
            (
                "post_commit_script",
                build_post_commit_script("linthis -c -f --hook-event=post-commit"),
            ),
            (
                "post_commit_with_agent_script",
                build_post_commit_with_agent_script(
                    "linthis -c -f --hook-event=post-commit",
                    &AgentFixProvider::Claude,
                    None,
                ),
            ),
        ];

        for (name, body) in scripts {
            let mut child = Command::new("sh")
                .arg("-n")
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect("spawn sh -n");
            child
                .stdin
                .as_mut()
                .expect("stdin")
                .write_all(body.as_bytes())
                .expect("write script");
            let out = child.wait_with_output().expect("wait sh -n");
            assert!(
                out.status.success(),
                "`sh -n` rejected generated {name}:\nstderr: {}\n---script---\n{}",
                String::from_utf8_lossy(&out.stderr),
                body,
            );
        }
    }
}
