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
    format!(
        "  if [ $LINTHIS_EXIT -ne 0 ]; then\n\
         \x20\x20\x20 {agent_check}\
         \x20\x20\x20 if [ \"$_LINTHIS_AGENT_OK\" = \"1\" ]; then\n\
         \x20\x20\x20\x20\x20 echo \"[linthis] {error_msg}. Invoking {provider} to fix...\" >&2\n\
         \x20\x20\x20\x20\x20 start_timer \"Fixing with {provider}\"\n\
         \x20\x20\x20\x20\x20 {agent}\n\
         \x20\x20\x20\x20\x20 stop_timer\n\
         \x20\x20\x20\x20\x20 echo \"[linthis] Re-verifying...\" >&2\n\
         \x20\x20\x20\x20\x20 $LINTHIS_CMD \"$@\"\n\
         \x20\x20\x20\x20\x20 LINTHIS_EXIT=$?\n\
         \x20\x20\x20 fi\n\
         {new_msg_print}\
         \x20 fi\n",
        provider = provider,
        agent = agent_cmd,
        agent_check = agent_check,
        error_msg = error_msg,
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
         \x20\x20\x20 # Local hook exists but has no linthis — run linthis first, then delegate\n\
         \x20\x20\x20 $LINTHIS_CMD \"$@\"\n\
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
         \x20 # No local hook — run linthis directly\n\
         \x20 $LINTHIS_CMD \"$@\"\n\
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
pub(crate) fn agent_fix_bin(provider: &AgentFixProvider) -> &'static str {
    match provider {
        AgentFixProvider::Claude => "claude",
        AgentFixProvider::Codex => "codex",
        AgentFixProvider::Gemini => "gemini",
        AgentFixProvider::Cursor => "cursor-agent",
        AgentFixProvider::Droid => "droid",
        AgentFixProvider::Auggie => "auggie",
        AgentFixProvider::Codebuddy => "codebuddy",
        AgentFixProvider::Openclaw => "openclaw",
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
    match provider {
        AgentFixProvider::Claude => format!(
            "claude -p{extra} --dangerously-skip-permissions '{}'",
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
            "codebuddy -p{extra} --dangerously-skip-permissions '{}'",
            escaped
        ),
        AgentFixProvider::Openclaw => format!("openclaw agent{extra} --message '{}'", escaped),
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
pub(crate) fn agent_fix_show_fixed_cmsg(indent: &str) -> String {
    format!(
        "{i}if [ $LINTHIS_EXIT -eq 0 ] && [ -n \"$_MSG_FILE\" ]; then\n\
         {i}  printf '\\033[0;32m[linthis] ✓ New message: %s\\033[0m\\n' \"$(cat \"$_MSG_FILE\")\" >&2\n\
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
    let prompt = "Commit message validation failed (not in Conventional Commits format). \
        Fix the commit message file at $_MSG_FILE: \
        (1) run 'git diff --cached --stat' to understand what actually changed, \
        (2) run 'git log -n 5 --oneline' to check recent commit style AND the language used \
        (Chinese or English) — match that language for the description, \
        (3) choose the correct type (feat/fix/refactor/perf/docs/style/test/build/ci/chore/revert) \
        based on the diff, \
        (4) rewrite to: type(scope)?: description — lowercase type, ≤72 chars, no trailing period. \
        Overwrite $_MSG_FILE directly without asking. \
        Verify with 'linthis cmsg $_MSG_FILE' until it passes.";
    // Escape backslashes and double quotes for use in double-quoted shell string
    let escaped = prompt.replace('\\', "\\\\").replace('"', "\\\"");
    let extra = provider_args
        .filter(|a| !a.is_empty())
        .map(|a| format!(" {a}"))
        .unwrap_or_default();
    let bin_cmd = match provider {
        AgentFixProvider::Claude => format!(
            "claude -p{extra} --dangerously-skip-permissions \"{}\"",
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
            "codebuddy -p{extra} --dangerously-skip-permissions \"{}\"",
            escaped
        ),
        AgentFixProvider::Openclaw => format!("openclaw agent{extra} --message \"{}\"", escaped),
    };
    // Prepend variable capture so $_MSG_FILE is available in the double-quoted prompt
    format!("_MSG_FILE=\"$1\"; {}", bin_cmd)
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

/// Shell snippet: a background elapsed-time spinner.
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
    format!(
        "\x20 # Check if agent provider is available before attempting fix\n\
         \x20 {agent_check}\
         \x20 if [ \"$_LINTHIS_AGENT_OK\" = \"1\" ]; then\n\
         \x20\x20\x20 echo \"[linthis] {error_msg}. Invoking {provider} to fix...\" >&2\n\
         \x20\x20\x20 # Backup staged files (safety net for linthis undo hook)\n\
         \x20\x20\x20 if [ -n \"$_STAGED_FILES\" ]; then\n\
         \x20\x20\x20\x20\x20 echo \"$_STAGED_FILES\" | tr '\\n' '\\0' | xargs -0 {linthis} backup create -d \"hook-agent-fix\" 2>/dev/null\n\
         \x20\x20\x20 fi\n\
         \x20\x20\x20 # Agent fixes directly in main working tree (backup provides safety net)\n\
         \x20\x20\x20 start_timer \"Fixing with {provider}\"\n\
         \x20\x20\x20 {agent}\n\
         \x20\x20\x20 stop_timer\n\
         \x20\x20\x20 # Re-stage files modified by agent\n\
         \x20\x20\x20 if [ -n \"$_STAGED_FILES\" ]; then\n\
         \x20\x20\x20\x20\x20 echo \"$_STAGED_FILES\" | xargs git add\n\
         \x20\x20\x20 fi\n\
         \x20\x20\x20 # Re-verify after agent fix\n\
         \x20\x20\x20 echo \"[linthis] Re-verifying...\" >&2\n\
         \x20\x20\x20 $LINTHIS_CMD\n\
         \x20\x20\x20 LINTHIS_EXIT=$?\n\
         \x20 fi\n",
        agent_check = agent_check,
        linthis = linthis_cmd,
        provider = fix_provider,
        agent = agent_cmd,
        error_msg = error_msg,
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
         $LINTHIS_CMD\n\
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
         \x20 {linthis_check_only}\n\
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
         $LINTHIS_CMD\n\
         LINTHIS_EXIT=$?\n\
         \n\
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
         \x20\x20\x20 echo \"  Revert:  linthis undo\" >&2\n\
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
    )
}

/// Generate the pre-push fix_commit_mode handler shell snippet.
fn shell_prepush_fix_commit_mode_handler(linthis_cmd: &str) -> String {
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
         \x20\x20\x20\x20\x20 echo \"$_CHANGED\" | xargs git add\n\
         \x20\x20\x20\x20\x20 git commit --no-verify -m \"fix(linthis): auto-fix lint issues\"\n\
         \x20\x20\x20\x20\x20 echo \"[linthis] Created fixup commit. Review with 'git log --oneline -2', then 'git push' again.\" >&2\n\
         \x20\x20\x20\x20\x20 exit 1\n\
         \x20\x20\x20 fi\n\
         \x20 fi\n",
        linthis = linthis_cmd,
    )
}

/// Generate shell snippet to handle agent review fixes based on fix_commit_mode.
fn shell_agent_review_fix_commit_handler() -> String {
    "# Handle agent's file changes based on fix_commit_mode\n\
         _AGENT_CHANGED=$(git diff --name-only)\n\
         if [ -n \"$_AGENT_CHANGED\" ]; then\n\
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
         \x20\x20\x20 exit 1\n\
         \x20 else\n\
         \x20\x20\x20 echo \"[linthis] Agent fixes left in working tree (dirty mode). Review: git diff, Revert: linthis undo\" >&2\n\
         \x20\x20\x20 exit 1\n\
         \x20 fi\n\
         fi\n\
         \n"
    .to_string()
}

/// Generate shell snippet for `git` type fix_commit_mode handling.
/// For pre-commit: re-stage formatted files based on mode.
/// For pre-push: handle format changes based on mode.
fn shell_git_fix_commit_mode_handler(hook_event: &HookEvent) -> String {
    if matches!(hook_event, HookEvent::PreCommit) {
        // Pre-commit: handle staged files after format
        "\x20\x20\x20 _STAGED_FILES=$(git diff --cached --name-only)\n\
         \x20\x20\x20 _DIRTY=$(git diff --name-only)\n\
         \x20\x20\x20 if [ \"$_FIX_MODE\" = \"squash\" ] && [ -n \"$_DIRTY\" ]; then\n\
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
         \x20\x20\x20\x20\x20\x20\x20 echo \"[linthis] Files formatted but not staged (dirty mode).\" >&2\n\
         \x20\x20\x20\x20\x20\x20\x20 echo \"  Review:  git diff\" >&2\n\
         \x20\x20\x20\x20\x20\x20\x20 echo \"  Accept:  git add -u && git commit\" >&2\n\
         \x20\x20\x20\x20\x20\x20\x20 echo \"  Revert:  linthis undo\" >&2\n\
         \x20\x20\x20\x20\x20\x20\x20 exit 1\n\
         \x20\x20\x20\x20\x20 fi\n\
         \x20\x20\x20 elif [ \"$_FIX_MODE\" = \"fixup\" ]; then\n\
         \x20\x20\x20\x20\x20 # fixup for git type: check only, no format (post-commit handles it)\n\
         \x20\x20\x20\x20\x20 true\n\
         \x20\x20\x20 fi\n"
            .to_string()
    } else if matches!(hook_event, HookEvent::PrePush) {
        // Pre-push: same as git-with-agent's prepush handler
        "".to_string() // Pre-push git type doesn't format, just checks
    } else {
        String::new()
    }
}

/// Generate shell snippet to read fix_commit_mode from linthis config.
/// `config_section` is "pre_commit" or "pre_push".
fn shell_read_fix_commit_mode(config_section: &str) -> String {
    let default = if config_section == "pre_commit" {
        "squash"
    } else {
        "dirty"
    };
    format!(
        "_FIX_MODE=$(linthis config get hook.{section}.fix_commit_mode 2>/dev/null || echo \"{default}\")\n",
        section = config_section,
        default = default,
    )
}

/// Build a post-commit hook script for fixup fix mode.
pub(crate) fn build_post_commit_script(linthis_cmd: &str) -> String {
    let timer_fns = shell_timer_functions();
    let fix_commit_mode_section = shell_read_fix_commit_mode("pre_commit");
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
         # Check + format committed files (includes lint fix, not just formatting)\n\
         echo \"$_FILES\" | tr '\\n' '\\0' | xargs -0 -I{{}} {linthis} -i {{}} --hook-event=post-commit\n\
         \n\
         # If any files changed, create fixup commit\n\
         _CHANGED=$(git diff --name-only)\n\
         if [ -n \"$_CHANGED\" ]; then\n\
         \x20 echo \"$_CHANGED\" | xargs git add\n\
         \x20 git commit --no-verify -m \"fix(linthis): auto-fix lint issues\"\n\
         \x20 echo \"[linthis] Created fixup commit with format changes\" >&2\n\
         fi\n",
        timer = timer_fns,
        fix_commit_mode = fix_commit_mode_section,
        linthis = linthis_cmd,
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
            // For post-commit: format files from the last commit (fixup mode)
            let extra = args.as_deref().unwrap_or("-f");
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
        .filter(|p| super::is_command_available(agent_fix_bin(p)))
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
        let parsed = match p.to_lowercase().as_str() {
            "claude" => Some(AgentFixProvider::Claude),
            "codex" => Some(AgentFixProvider::Codex),
            "gemini" => Some(AgentFixProvider::Gemini),
            "cursor" => Some(AgentFixProvider::Cursor),
            "droid" => Some(AgentFixProvider::Droid),
            "auggie" | "aug" | "augment" => Some(AgentFixProvider::Auggie),
            "codebuddy" => Some(AgentFixProvider::Codebuddy),
            "openclaw" => Some(AgentFixProvider::Openclaw),
            _ => None,
        };
        return parsed.ok_or_else(|| {
            eprintln!(
                "{}: Unknown agent fix provider '{}'. Valid: claude, codex, gemini, cursor, droid, auggie, codebuddy, openclaw",
                "Error".red(), p
            );
            ExitCode::from(1)
        });
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
        let available = super::is_command_available(agent_fix_bin(p));
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
