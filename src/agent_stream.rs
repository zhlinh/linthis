// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Pretty-print Claude Code / Codebuddy stream-json output.
//!
//! `claude -p --verbose --output-format stream-json` emits newline-delimited
//! JSON events as the model works (each assistant message, each tool_use, each
//! tool_result). Piping that through `linthis agent-stream` converts those
//! events into human-readable text as they arrive — finally giving users live
//! visibility during long auto-fix runs that previously looked like 100+
//! seconds of silence followed by one wall of output.
//!
//! Non-JSON lines pass through unchanged so plain error messages from the
//! agent binary (auth failure, network issue, etc.) stay visible.

use std::io::{BufRead, Write};
use std::process::ExitCode;

use serde_json::Value;

/// Read stream-json events from stdin and emit a human-readable stream to
/// stdout. Returns success always — errors in individual lines are tolerated
/// so the pipeline keeps flowing.
pub fn handle_agent_stream() -> ExitCode {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut out = stdout.lock();

    for line in stdin.lock().lines() {
        let Ok(line) = line else { continue };
        if line.is_empty() {
            continue;
        }
        let trimmed = line.trim();
        // Fast path: non-JSON lines (provider CLI errors, etc.) pass through.
        if !trimmed.starts_with('{') {
            let _ = writeln!(out, "{line}");
            let _ = out.flush();
            continue;
        }
        let Ok(val) = serde_json::from_str::<Value>(trimmed) else {
            let _ = writeln!(out, "{line}");
            let _ = out.flush();
            continue;
        };
        render_event(&val, &mut out);
        let _ = out.flush();
    }
    ExitCode::SUCCESS
}

/// Dispatch a single JSON event to the appropriate renderer.
/// Unicode glyphs borrowed from claude-code's conventions (see
/// constants/figures.ts and tools/*/UI.tsx in the claude-code source).
/// - TOOL_MARKER: leading circle on each tool_use line. macOS uses `⏺`.
/// - RESULT_MARKER: `⎿` prefix on tool_result lines, visually nested under
///   the preceding tool_use block.
/// - INDENT_CONTENT: aligns preview lines (stats, diff previews) under the
///   tool name, matching MessageResponse's default padding.
/// - INDENT_RESULT: `  ⎿  ` — two-space lead, marker, two-space tail.
const TOOL_MARKER: &str = "⏺";
const RESULT_PREFIX: &str = "  ⎿  ";
const INDENT_CONTENT: &str = "    ";

fn render_event<W: Write>(val: &Value, out: &mut W) {
    let event_type = val.get("type").and_then(Value::as_str).unwrap_or("");
    match event_type {
        "system" => render_system(val, out),
        "assistant" => render_assistant(val, out),
        "user" => render_user_tool_result(val, out),
        "result" => render_result(val, out),
        // Partial/delta events only appear when --include-partial-messages is
        // set; when present we render them incrementally for token-level flow.
        "stream_event" => render_stream_event(val, out),
        _ => {
            // Unknown event type — don't spam the user; just swallow.
        }
    }
}

fn render_system<W: Write>(val: &Value, out: &mut W) {
    if val.get("subtype").and_then(Value::as_str) == Some("init") {
        let model = val
            .get("model")
            .and_then(Value::as_str)
            .unwrap_or("unknown-model");
        // Mirrors claude-code's session header — terse, one line.
        let _ = writeln!(out, "{TOOL_MARKER} session started · model: {model}");
    }
}

fn render_assistant<W: Write>(val: &Value, out: &mut W) {
    let Some(content) = val
        .get("message")
        .and_then(|m| m.get("content"))
        .and_then(Value::as_array)
    else {
        return;
    };
    for block in content {
        let block_type = block.get("type").and_then(Value::as_str).unwrap_or("");
        match block_type {
            "text" => {
                if let Some(text) = block.get("text").and_then(Value::as_str) {
                    if !text.is_empty() {
                        write_text_with_diff_coloring(text, out);
                    }
                }
            }
            "tool_use" => {
                render_tool_use_block(block, out);
            }
            "thinking" => {
                // Thinking blocks tend to be verbose and repetitive; surface
                // only a one-line marker so users know progress is happening.
                let _ = writeln!(out, "{TOOL_MARKER} (thinking…)");
            }
            _ => {}
        }
    }
}

/// Render a single tool_use block using claude-code's native style:
///
/// ```text
/// ⏺ Update(src/foo.rs)
///     Added 5 lines, removed 2 lines
///     <preview lines>
/// ```
///
/// Tool-name mapping follows `claude-code/src/tools/*/UI.tsx::userFacingName`.
/// File-touching tools put the path in parens; generic tools show the arg
/// they care about (command for Bash, pattern for Glob/Grep, etc.).
fn render_tool_use_block<W: Write>(block: &Value, out: &mut W) {
    let name = block.get("name").and_then(Value::as_str).unwrap_or("?");
    let input = block.get("input");
    match name {
        "Edit" => render_file_edit(input, out),
        "MultiEdit" => render_multi_edit(input, out),
        "Write" => render_file_write(input, out),
        "Read" => render_single_path(input, "Read", out),
        "NotebookEdit" => render_single_path(input, "Edit Notebook", out),
        "Bash" => render_bash(input, out),
        "Glob" | "Grep" => render_search(input, out),
        "WebFetch" => render_single_arg(input, "Fetch", "url", out),
        "WebSearch" => render_single_arg(input, "Web Search", "query", out),
        "LSPTool" | "LSP" => render_single_arg(input, "LSP", "operation", out),
        "Skill" => render_skill(input, out),
        "Task" | "Agent" => render_task_agent(input, out),
        "TodoWrite" => render_todowrite(input, out),
        // linthis-specific Claude Code SDK task tools. These fire often
        // during agent work and previously rendered as bare `⏺ TaskCreate`
        // with no context — show the subject/ID so users know what changed.
        "TaskCreate" => render_task_create(input, out),
        "TaskUpdate" => render_task_update(input, out),
        "TaskGet" | "TaskStop" | "TaskOutput" => render_single_arg(input, name, "taskId", out),
        "TaskList" => {
            let _ = writeln!(out, "{TOOL_MARKER} {name}");
        }
        other => {
            let summary = summarize_tool_input(other, input);
            let _ = writeln!(out, "{TOOL_MARKER} {other}{summary}");
        }
    }
}

/// Emit the `⏺ <Verb>(<path>)` header, or just `⏺ <Verb>` if path missing.
fn write_header_path<W: Write>(verb: &str, path: &str, out: &mut W) {
    if path.is_empty() {
        let _ = writeln!(out, "{TOOL_MARKER} {verb}");
    } else {
        let _ = writeln!(out, "{TOOL_MARKER} {verb}({path})");
    }
}

/// File tools that only carry a path (Read, NotebookEdit).
fn render_single_path<W: Write>(input: Option<&Value>, verb: &str, out: &mut W) {
    let path = input
        .and_then(|i| i.get("file_path"))
        .and_then(Value::as_str)
        .map(to_display_path)
        .unwrap_or_default();
    write_header_path(verb, &path, out);
}

/// Tools with a single string arg like query/url/operation.
fn render_single_arg<W: Write>(input: Option<&Value>, verb: &str, field: &str, out: &mut W) {
    let arg = input
        .and_then(|i| i.get(field))
        .and_then(Value::as_str)
        .map(truncate_one_line)
        .unwrap_or_default();
    write_header_path(verb, &arg, out);
}

/// Bash: wrap the command in `Bash(...)` for clarity. Claude-code's TUI can
/// drop the label because the round-bullet + syntax highlighting makes the
/// command stand out, but in plain-text streaming a bare `⏺ git diff ...`
/// reads like "git" is a tool name. Adding the `Bash(...)` label removes
/// that ambiguity. Command is still subject to BashTool's own 2-line /
/// 160-char truncation limits.
fn render_bash<W: Write>(input: Option<&Value>, out: &mut W) {
    let cmd = input
        .and_then(|i| i.get("command"))
        .and_then(Value::as_str)
        .unwrap_or("");
    let preview = truncate_bash_command(cmd);
    write_header_path("Bash", &preview, out);
}

/// TaskCreate: show the task subject — that's the human-readable summary
/// of what's being tracked. Fall back to `TaskCreate` with no args if
/// subject is missing.
fn render_task_create<W: Write>(input: Option<&Value>, out: &mut W) {
    let subject = input
        .and_then(|i| i.get("subject"))
        .and_then(Value::as_str)
        .map(truncate_one_line)
        .unwrap_or_default();
    write_header_path("TaskCreate", &subject, out);
}

/// TaskUpdate: show `<taskId> → <new status>` when both are present so the
/// user sees the transition (in_progress / completed / etc.). Otherwise
/// just the taskId, or the field that changed.
fn render_task_update<W: Write>(input: Option<&Value>, out: &mut W) {
    let Some(input) = input else {
        let _ = writeln!(out, "{TOOL_MARKER} TaskUpdate");
        return;
    };
    let task_id = input.get("taskId").and_then(Value::as_str).unwrap_or("");
    let status = input.get("status").and_then(Value::as_str);
    let owner = input.get("owner").and_then(Value::as_str);
    let subject = input.get("subject").and_then(Value::as_str);
    let detail = match (status, owner, subject) {
        (Some(s), _, _) if !s.is_empty() => format!("{task_id} → {s}"),
        (_, Some(o), _) if !o.is_empty() => format!("{task_id} owner={o}"),
        (_, _, Some(sub)) if !sub.is_empty() => format!("{task_id}: {sub}"),
        _ => task_id.to_string(),
    };
    write_header_path("TaskUpdate", &truncate_one_line(detail), out);
}

/// Glob/Grep both present as `Search(pattern)` per claude-code.
fn render_search<W: Write>(input: Option<&Value>, out: &mut W) {
    let pattern = input
        .and_then(|i| i.get("pattern"))
        .and_then(Value::as_str)
        .map(truncate_one_line)
        .unwrap_or_default();
    write_header_path("Search", &pattern, out);
}

/// Skill(name) — or Skill(name args) when args are present.
fn render_skill<W: Write>(input: Option<&Value>, out: &mut W) {
    let name = input
        .and_then(|i| i.get("skill"))
        .and_then(Value::as_str)
        .unwrap_or("");
    let args = input
        .and_then(|i| i.get("args"))
        .and_then(Value::as_str)
        .unwrap_or("");
    let label = match (name.is_empty(), args.is_empty()) {
        (true, _) => String::new(),
        (false, true) => name.to_string(),
        (false, false) => truncate_one_line(format!("{name} {args}")),
    };
    write_header_path("Skill", &label, out);
}

/// Task/Agent: `⏺ <Subagent type> · <description>`.
/// Matches claude-code's AgentTool/UI.tsx where the userFacingName IS the
/// subagent_type (general-purpose, statusline-setup, etc.).
fn render_task_agent<W: Write>(input: Option<&Value>, out: &mut W) {
    let desc = input
        .and_then(|i| i.get("description"))
        .and_then(Value::as_str)
        .map(truncate_one_line)
        .unwrap_or_default();
    let subtype = input
        .and_then(|i| i.get("subagent_type"))
        .and_then(Value::as_str)
        .unwrap_or("Agent");
    if desc.is_empty() {
        let _ = writeln!(out, "{TOOL_MARKER} {subtype}");
    } else {
        let _ = writeln!(out, "{TOOL_MARKER} {subtype} · {desc}");
    }
}

/// TodoWrite: show the first in-progress or pending item so the user sees
/// the current plan step. Claude-code's userFacingName is empty and it
/// renders the full list via its own component; we keep the essence.
fn render_todowrite<W: Write>(input: Option<&Value>, out: &mut W) {
    let first = input
        .and_then(|i| i.get("todos"))
        .and_then(Value::as_array)
        .and_then(|v| v.first())
        .and_then(|t| t.get("content"))
        .and_then(Value::as_str)
        .map(truncate_one_line)
        .unwrap_or_default();
    if first.is_empty() {
        let _ = writeln!(out, "{TOOL_MARKER} TodoWrite");
    } else {
        let _ = writeln!(out, "{TOOL_MARKER} TodoWrite · {first}");
    }
}

/// `⏺ Update(path)` / `Create(path)` + stats + colored diff preview.
/// "Create" when old_string is empty (FileEditTool UI.tsx convention).
fn render_file_edit<W: Write>(input: Option<&Value>, out: &mut W) {
    let Some(input) = input else {
        let _ = writeln!(out, "{TOOL_MARKER} Update");
        return;
    };
    let path = input
        .get("file_path")
        .and_then(Value::as_str)
        .map(to_display_path)
        .unwrap_or_default();
    let old_s = input
        .get("old_string")
        .and_then(Value::as_str)
        .unwrap_or("");
    let new_s = input
        .get("new_string")
        .and_then(Value::as_str)
        .unwrap_or("");
    let verb = if old_s.is_empty() { "Create" } else { "Update" };
    let added = line_count(new_s);
    let removed = line_count(old_s);
    write_header_path(verb, &path, out);
    write_diff_stats_line(added, removed, out);
    let shown = write_colored_diff(old_s, new_s, MAX_PREVIEW_LINES, out);
    let total = added + removed;
    if total > shown {
        let remaining = total - shown;
        let word = if remaining == 1 { "line" } else { "lines" };
        let _ = writeln!(
            out,
            "{INDENT_CONTENT}{ANSI_DIM}… +{remaining} {word}{ANSI_RESET}"
        );
    }
}

/// MultiEdit: aggregate adds/removes across ALL edits, then render diff
/// from each edit sequentially (up to the combined MAX_PREVIEW_LINES cap)
/// so the preview matches the stats.
fn render_multi_edit<W: Write>(input: Option<&Value>, out: &mut W) {
    let Some(input) = input else {
        let _ = writeln!(out, "{TOOL_MARKER} Update");
        return;
    };
    let path = input
        .get("file_path")
        .and_then(Value::as_str)
        .map(to_display_path)
        .unwrap_or_default();
    let edits = input.get("edits").and_then(Value::as_array);
    let mut added = 0usize;
    let mut removed = 0usize;
    let mut all_creates = true;
    if let Some(edits) = edits {
        for edit in edits {
            let old_s = edit.get("old_string").and_then(Value::as_str).unwrap_or("");
            let new_s = edit.get("new_string").and_then(Value::as_str).unwrap_or("");
            if !old_s.is_empty() {
                all_creates = false;
            }
            added += line_count(new_s);
            removed += line_count(old_s);
        }
    }
    let verb = if all_creates { "Create" } else { "Update" };
    write_header_path(verb, &path, out);
    write_diff_stats_line(added, removed, out);
    // Render diffs from each edit, sharing one combined line budget.
    if let Some(edits) = edits {
        let mut budget = MAX_PREVIEW_LINES;
        for edit in edits {
            if budget == 0 {
                break;
            }
            let old_s = edit.get("old_string").and_then(Value::as_str).unwrap_or("");
            let new_s = edit.get("new_string").and_then(Value::as_str).unwrap_or("");
            let used = write_colored_diff(old_s, new_s, budget, out);
            budget = budget.saturating_sub(used);
        }
        // If we ran out of budget, show remaining from total.
        let total = added + removed;
        let shown = MAX_PREVIEW_LINES - budget;
        if total > shown {
            let remaining = total - shown;
            let word = if remaining == 1 { "line" } else { "lines" };
            let _ = writeln!(
                out,
                "{INDENT_CONTENT}{ANSI_DIM}… +{remaining} {word}{ANSI_RESET}"
            );
        }
    }
}

/// Write: claude-code says "Wrote N lines to <path>" (FileWriteToolCreatedMessage).
/// Content preview: up to MAX_PREVIEW_LINES lines, then `… +N lines`.
fn render_file_write<W: Write>(input: Option<&Value>, out: &mut W) {
    let Some(input) = input else {
        let _ = writeln!(out, "{TOOL_MARKER} Write");
        return;
    };
    let path = input
        .get("file_path")
        .and_then(Value::as_str)
        .map(to_display_path)
        .unwrap_or_default();
    let content = input.get("content").and_then(Value::as_str).unwrap_or("");
    let total = line_count(content);
    write_header_path("Write", &path, out);
    if total > 0 {
        let word = if total == 1 { "line" } else { "lines" };
        let _ = writeln!(out, "{INDENT_CONTENT}Wrote {total} {word} to {path}");
    }
    write_content_preview(content, MAX_PREVIEW_LINES, out);
}

/// ANSI cyan for diff hunk headers (`@@ ... @@`).
const ANSI_CYAN: &str = "\x1b[36m";
/// ANSI bold for diff file headers (`diff --git`, `---`, `+++`).
const ANSI_BOLD: &str = "\x1b[1m";

/// Max content/diff lines before showing `… +N lines` (matches claude-code's
/// MAX_LINES_TO_RENDER = 10 in FileWriteTool/UI.tsx).
const MAX_PREVIEW_LINES: usize = 10;

/// Write assistant text, colorizing unified-diff blocks inline.
/// Detects `diff --git` or `@@` markers to enter diff mode; once inside
/// a fenced code block (` ```diff ` ... ` ``` `), every `+`/`-` line gets
/// colored green/red just like `git diff --color`.
fn write_text_with_diff_coloring<W: Write>(text: &str, out: &mut W) {
    let has_diff = text.contains("diff --git") || text.contains("@@") || text.contains("```diff");
    if !has_diff {
        let _ = writeln!(out, "{text}");
        return;
    }

    let mut in_diff_block = false;
    for line in text.lines() {
        let trimmed = line.trim();
        // Track fenced code blocks with `diff` language hint.
        if trimmed == "```diff" {
            in_diff_block = true;
            let _ = writeln!(out, "{line}");
            continue;
        }
        if trimmed == "```" && in_diff_block {
            in_diff_block = false;
            let _ = writeln!(out, "{line}");
            continue;
        }
        // Also treat bare `diff --git` outside fences as diff content.
        if line.starts_with("diff --git") {
            in_diff_block = true;
        }

        if in_diff_block {
            write_colored_diff_line(line, out);
        } else {
            let _ = writeln!(out, "{line}");
        }
    }
}

/// Colorize a single unified-diff line based on its prefix character.
fn write_colored_diff_line<W: Write>(line: &str, out: &mut W) {
    if line.starts_with("diff --git")
        || line.starts_with("index ")
        || line.starts_with("--- ")
        || line.starts_with("+++ ")
    {
        let _ = writeln!(out, "{ANSI_BOLD}{line}{ANSI_RESET}");
    } else if line.starts_with("@@") {
        let _ = writeln!(out, "{ANSI_CYAN}{line}{ANSI_RESET}");
    } else if line.starts_with('+') {
        let _ = writeln!(out, "{ANSI_GREEN}{line}{ANSI_RESET}");
    } else if line.starts_with('-') {
        let _ = writeln!(out, "{ANSI_RED}{line}{ANSI_RESET}");
    } else {
        // Context lines, "new file mode", etc. — no coloring.
        let _ = writeln!(out, "{line}");
    }
}

/// ANSI escape sequences for colored diff output — matches claude-code's
/// StructuredDiff green/red/dim styling for +/- lines and line numbers.
const ANSI_GREEN: &str = "\x1b[32m";
const ANSI_RED: &str = "\x1b[31m";
const ANSI_DIM: &str = "\x1b[2m";
const ANSI_RESET: &str = "\x1b[0m";

/// Claude-code: "Added N line(s), removed M line(s)" (note: "line" vs "lines").
/// Silent when both are zero — no point announcing a no-op edit.
fn write_diff_stats_line<W: Write>(added: usize, removed: usize, out: &mut W) {
    let pluralize = |n: usize| if n == 1 { "line" } else { "lines" };
    match (added, removed) {
        (0, 0) => {}
        (a, 0) => {
            let _ = writeln!(out, "{INDENT_CONTENT}Added {a} {}", pluralize(a));
        }
        (0, r) => {
            let _ = writeln!(out, "{INDENT_CONTENT}Removed {r} {}", pluralize(r));
        }
        (a, r) => {
            let _ = writeln!(
                out,
                "{INDENT_CONTENT}Added {a} {}, removed {r} {}",
                pluralize(a),
                pluralize(r)
            );
        }
    }
}

/// Show content preview for Write / new-file Create — plain text (all additions),
/// up to `max_lines`, then a dimmed `… +N lines` truncation indicator.
fn write_content_preview<W: Write>(text: &str, max_lines: usize, out: &mut W) {
    let lines: Vec<&str> = text.lines().collect();
    let total = lines.len();
    let visible = total.min(max_lines);
    for line in &lines[..visible] {
        let display = truncate_char_width(line.trim_end(), 120);
        let _ = writeln!(out, "{INDENT_CONTENT}{display}");
    }
    if total > visible {
        let remaining = total - visible;
        let word = if remaining == 1 { "line" } else { "lines" };
        let _ = writeln!(
            out,
            "{INDENT_CONTENT}{ANSI_DIM}… +{remaining} {word}{ANSI_RESET}"
        );
    }
}

/// Colored unified diff for Edit / MultiEdit — removed lines in red with `- `
/// prefix, added lines in green with `+ ` prefix. Caps at `max_lines` total
/// diff lines. Returns the number of lines actually rendered so the caller
/// can track a shared budget (for MultiEdit with multiple edits).
///
/// For single Edit calls, the caller prints a truncation indicator when
/// `returned < total_diff`. For MultiEdit, the outer loop handles it.
fn write_colored_diff<W: Write>(old: &str, new: &str, max_lines: usize, out: &mut W) -> usize {
    let old_lines: Vec<&str> = if old.is_empty() {
        Vec::new()
    } else {
        old.lines().collect()
    };
    let new_lines: Vec<&str> = if new.is_empty() {
        Vec::new()
    } else {
        new.lines().collect()
    };
    let total_diff = old_lines.len() + new_lines.len();
    if total_diff == 0 {
        return 0;
    }

    // Calculate gutter width based on max line number in the diff.
    let max_lineno = old_lines.len().max(new_lines.len());
    let gutter_width = if max_lineno >= 100 {
        3
    } else if max_lineno >= 10 {
        2
    } else {
        1
    };

    let mut shown = 0;

    // Removed lines (old_string) — red
    for (i, line) in old_lines.iter().enumerate() {
        if shown >= max_lines {
            break;
        }
        let lineno = i + 1;
        let display = truncate_char_width(line.trim_end(), 120);
        let _ = writeln!(
            out,
            "{INDENT_CONTENT}{ANSI_DIM}{lineno:>gutter_width$}{ANSI_RESET} {ANSI_RED}- {display}{ANSI_RESET}",
            gutter_width = gutter_width
        );
        shown += 1;
    }

    // Added lines (new_string) — green
    for (i, line) in new_lines.iter().enumerate() {
        if shown >= max_lines {
            break;
        }
        let lineno = i + 1;
        let display = truncate_char_width(line.trim_end(), 120);
        let _ = writeln!(
            out,
            "{INDENT_CONTENT}{ANSI_DIM}{lineno:>gutter_width$}{ANSI_RESET} {ANSI_GREEN}+ {display}{ANSI_RESET}",
            gutter_width = gutter_width
        );
        shown += 1;
    }

    shown
}

/// Truncate `s` to at most `max_chars` user-visible characters, appending `…`
/// if we had to cut anything off.
fn truncate_char_width(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        return s.to_string();
    }
    let cut: String = s.chars().take(max_chars.saturating_sub(1)).collect();
    format!("{cut}…")
}

/// Single-line, bounded-width display of a string. Mirrors
/// BashTool/UI.tsx's MAX_COMMAND_DISPLAY_CHARS (160) for header args.
fn truncate_one_line<S: AsRef<str>>(s: S) -> String {
    let first: String = s.as_ref().lines().next().unwrap_or("").trim().to_string();
    truncate_char_width(&first, 160)
}

/// BashTool truncation: first line only (multi-line commands are common),
/// capped at 100 chars so the `⏺ Bash(...)` wrapper fits within most
/// terminal widths (~120 cols). Claude-code's TUI uses 160 chars but has
/// horizontal scroll; our plain-text stream doesn't.
fn truncate_bash_command(cmd: &str) -> String {
    let max_chars = 100;
    let first = cmd.lines().next().unwrap_or("").trim();
    if first.chars().count() > max_chars {
        let cut: String = first.chars().take(max_chars.saturating_sub(1)).collect();
        format!("{cut}…")
    } else if cmd.lines().count() > 1 {
        format!("{first}…")
    } else {
        first.to_string()
    }
}

fn render_user_tool_result<W: Write>(val: &Value, out: &mut W) {
    let Some(content) = val
        .get("message")
        .and_then(|m| m.get("content"))
        .and_then(Value::as_array)
    else {
        return;
    };
    for block in content {
        if block.get("type").and_then(Value::as_str) != Some("tool_result") {
            continue;
        }
        // Stay silent on success — the tool_use header already told the user
        // what happened. Only surface failures, which otherwise disappear
        // into a sea of green ticks.
        let is_error = block
            .get("is_error")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        if is_error {
            // First line of the error content, if parseable, so users see
            // the actual failure reason (not just a generic ✗).
            let first = block
                .get("content")
                .and_then(Value::as_str)
                .or_else(|| {
                    block
                        .get("content")
                        .and_then(Value::as_array)
                        .and_then(|arr| arr.first())
                        .and_then(|v| v.get("text"))
                        .and_then(Value::as_str)
                })
                .map(truncate_one_line)
                .unwrap_or_default();
            if first.is_empty() {
                let _ = writeln!(out, "{RESULT_PREFIX}tool error");
            } else {
                let _ = writeln!(out, "{RESULT_PREFIX}tool error: {first}");
            }
        }
    }
}

fn render_result<W: Write>(val: &Value, out: &mut W) {
    let subtype = val.get("subtype").and_then(Value::as_str).unwrap_or("");
    // The agent's final assistant message is already rendered via
    // render_assistant; here we just emit a terminator on failure.
    if subtype == "error_max_turns"
        || subtype == "error_during_execution"
        || val.get("is_error").and_then(Value::as_bool) == Some(true)
    {
        let _ = writeln!(out, "{RESULT_PREFIX}run ended with error ({subtype})");
    }
}

fn render_stream_event<W: Write>(val: &Value, out: &mut W) {
    let Some(event) = val.get("event") else {
        return;
    };
    let ev_type = event.get("type").and_then(Value::as_str).unwrap_or("");
    match ev_type {
        "content_block_delta" => {
            let delta = event.get("delta");
            if let Some(delta) = delta {
                if delta.get("type").and_then(Value::as_str) == Some("text_delta") {
                    if let Some(text) = delta.get("text").and_then(Value::as_str) {
                        let _ = write!(out, "{text}");
                    }
                }
            }
        }
        "content_block_start" => {
            if let Some(block) = event.get("content_block") {
                if block.get("type").and_then(Value::as_str) == Some("tool_use") {
                    let name = block.get("name").and_then(Value::as_str).unwrap_or("?");
                    let _ = writeln!(out, "\n{TOOL_MARKER} {name}");
                }
            }
        }
        "content_block_stop" | "message_stop" => {
            let _ = writeln!(out);
        }
        _ => {}
    }
}

/// Convert a path to display form, matching claude-code/src/utils/file.ts
/// `getDisplayPath`:
///   1. If under cwd → relative path.
///   2. Else if under $HOME → "~/..." notation.
///   3. Else → absolute path unchanged.
fn to_display_path(path: &str) -> String {
    if let Ok(cwd) = std::env::current_dir() {
        if let Some(cwd_str) = cwd.to_str() {
            let prefix = format!("{cwd_str}/");
            if let Some(rest) = path.strip_prefix(&prefix) {
                return rest.to_string();
            }
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        let prefix = format!("{home}/");
        if let Some(rest) = path.strip_prefix(&prefix) {
            return format!("~/{rest}");
        }
    }
    path.to_string()
}

/// Count lines for diff-stats display. `str::lines()` already treats a
/// trailing newline as a terminator (not a new empty line), matching how
/// diff tools count — so `"a"` and `"a\n"` both count as 1 line.
fn line_count(s: &str) -> usize {
    s.lines().count()
}

/// Fallback summary for tools that don't have a bespoke renderer above.
/// Most first-party tools are handled directly in `render_tool_use_block`;
/// this covers MCP tools and anything else we haven't classified.
fn summarize_tool_input(_tool: &str, _input: Option<&Value>) -> String {
    // Intentionally minimal — a bare `⏺ ToolName` line still tells the
    // user a tool fired. If particular MCP tools become common we can add
    // bespoke renderers.
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render_to_string(line: &str) -> String {
        let val: Value = serde_json::from_str(line).unwrap();
        let mut buf = Vec::new();
        render_event(&val, &mut buf);
        String::from_utf8(buf).unwrap()
    }

    #[test]
    fn renders_system_init_with_model() {
        let line = r#"{"type":"system","subtype":"init","model":"claude-opus-4-6"}"#;
        let out = render_to_string(line);
        assert!(out.contains("claude-opus-4-6"), "got: {out}");
        assert!(out.contains("session started"), "got: {out}");
    }

    #[test]
    fn renders_assistant_text_block() {
        let line =
            r#"{"type":"assistant","message":{"content":[{"type":"text","text":"Hello world"}]}}"#;
        assert!(render_to_string(line).contains("Hello world"));
    }

    #[test]
    fn renders_read_tool_as_read_with_path() {
        let line = r#"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"Read","input":{"file_path":"/tmp/foo.rs"}}]}}"#;
        let out = render_to_string(line);
        assert!(out.contains(TOOL_MARKER), "missing ⏺ marker; got: {out}");
        assert!(out.contains("Read(/tmp/foo.rs)"), "got: {out}");
    }

    #[test]
    fn notebook_edit_uses_edit_notebook_verb() {
        // claude-code's NotebookEditTool userFacingName = "Edit Notebook".
        let line = r#"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"NotebookEdit","input":{"file_path":"/tmp/nb.ipynb"}}]}}"#;
        let out = render_to_string(line);
        assert!(out.contains("Edit Notebook(/tmp/nb.ipynb)"), "got: {out}");
    }

    #[test]
    fn edit_shows_update_header_with_native_stats_and_preview() {
        // Edit from 1 line to 3 lines — expect `Update(path)` header,
        // "Added 3 lines, removed 1 line" stats, and a preview of the new
        // content. Matches claude-code's FileEditToolUpdatedMessage output.
        let line = r#"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"Edit","input":{"file_path":"/tmp/foo.rs","old_string":"fn bar() {}","new_string":"fn bar() {\n    println!(\"hi\");\n}"}}]}}"#;
        let out = render_to_string(line);
        assert!(
            out.contains("Update(/tmp/foo.rs)"),
            "header missing; got: {out}"
        );
        assert!(
            out.contains("Added 3 lines, removed 1 line"),
            "native stats missing; got: {out}"
        );
        assert!(out.contains("fn bar()"), "preview missing; got: {out}");
    }

    #[test]
    fn edit_with_empty_old_string_shows_create() {
        // When old_string is empty, claude-code shows "Create" instead of "Update".
        let line = r#"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"Edit","input":{"file_path":"/tmp/new.rs","old_string":"","new_string":"fn main() {}"}}]}}"#;
        let out = render_to_string(line);
        assert!(out.contains("Create(/tmp/new.rs)"), "got: {out}");
        assert!(out.contains("Added 1 line"), "got: {out}");
        assert!(
            !out.contains("removed"),
            "no removed for create; got: {out}"
        );
    }

    #[test]
    fn multi_edit_aggregates_stats_and_uses_update_header() {
        let line = r#"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"MultiEdit","input":{"file_path":"/tmp/x.rs","edits":[{"old_string":"a","new_string":"a\nb"},{"old_string":"c","new_string":"c\nd\ne"}]}}]}}"#;
        let out = render_to_string(line);
        assert!(out.contains("Update(/tmp/x.rs)"), "got: {out}");
        // First edit: +2 -1, second: +3 -1 → total +5 -2
        assert!(out.contains("Added 5 lines, removed 2 lines"), "got: {out}");
    }

    #[test]
    fn write_uses_write_header_and_shows_line_count() {
        // FileWriteTool's userFacingName is literally "Write" — it does NOT
        // switch to "Create" based on file existence (that's FileEditTool).
        // Its result message says "Wrote N lines to path"; we surface that
        // up-front from the input content.
        let line = r#"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"Write","input":{"file_path":"/tmp/new.rs","content":"line 1\nline 2\nline 3\n"}}]}}"#;
        let out = render_to_string(line);
        assert!(out.contains("Write(/tmp/new.rs)"), "got: {out}");
        assert!(out.contains("Wrote 3 lines"), "got: {out}");
        assert!(out.contains("line 1"), "preview missing; got: {out}");
    }

    #[test]
    fn singular_vs_plural_line_phrasing() {
        // One line → "1 line" (singular); multiple → "N lines" (plural).
        // Mirrors claude-code's `numAdditions > 1 ? 'lines' : 'line'` logic.
        let line = r#"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"Edit","input":{"file_path":"/tmp/x.rs","old_string":"a","new_string":"a\nb"}}]}}"#;
        let out = render_to_string(line);
        assert!(out.contains("Added 2 lines, removed 1 line"), "got: {out}");
    }

    #[test]
    fn preview_truncates_long_lines() {
        let long_line = "a".repeat(200);
        let body = format!("fn f() {{\n    {long_line}\n}}");
        let input = serde_json::json!({
            "type": "assistant",
            "message": {
                "content": [{
                    "type": "tool_use",
                    "name": "Edit",
                    "input": {
                        "file_path": "/tmp/x.rs",
                        "old_string": "fn f() {}",
                        "new_string": body,
                    }
                }]
            }
        });
        let out = render_to_string(&input.to_string());
        assert!(out.contains("…"), "long line must be truncated; got: {out}");
        assert!(
            !out.contains(&"a".repeat(150)),
            "truncation must cap well below 200 chars; got: {out}"
        );
    }

    #[test]
    fn to_display_path_converts_when_under_cwd() {
        if let Ok(cwd) = std::env::current_dir() {
            let absolute = cwd.join("sub/path.rs");
            let rel = super::to_display_path(absolute.to_str().unwrap());
            assert_eq!(rel, "sub/path.rs");
        }
    }

    #[test]
    fn to_display_path_uses_tilde_for_home() {
        // Mirrors claude-code/src/utils/file.ts::getDisplayPath.
        if let Ok(home) = std::env::var("HOME") {
            let path = format!("{home}/Documents/project");
            assert_eq!(super::to_display_path(&path), "~/Documents/project");
        }
    }

    #[test]
    fn to_display_path_passes_through_when_outside_cwd_and_home() {
        let out = super::to_display_path("/tmp/unrelated/foo.rs");
        assert!(
            out == "/tmp/unrelated/foo.rs" || out.starts_with('~'),
            "got: {out}"
        );
    }

    #[test]
    fn renders_bash_with_bash_parens_prefix() {
        // Plain-text streaming benefits from the `Bash(...)` wrapper —
        // without it, `⏺ git diff ...` reads like "git" is a tool name.
        let line = r#"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"Bash","input":{"command":"cargo test"}}]}}"#;
        let out = render_to_string(line);
        assert!(out.contains(TOOL_MARKER), "got: {out}");
        assert!(out.contains("Bash(cargo test)"), "got: {out}");
    }

    #[test]
    fn renders_task_create_with_subject() {
        let line = r#"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"TaskCreate","input":{"subject":"Fix login bug","description":"..."}}]}}"#;
        let out = render_to_string(line);
        assert!(out.contains("TaskCreate(Fix login bug)"), "got: {out}");
    }

    #[test]
    fn renders_task_update_with_status() {
        let line = r#"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"TaskUpdate","input":{"taskId":"42","status":"completed"}}]}}"#;
        let out = render_to_string(line);
        assert!(out.contains("TaskUpdate(42 → completed)"), "got: {out}");
    }

    #[test]
    fn renders_task_update_owner_when_no_status() {
        let line = r#"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"TaskUpdate","input":{"taskId":"7","owner":"reviewer"}}]}}"#;
        let out = render_to_string(line);
        assert!(out.contains("TaskUpdate(7 owner=reviewer)"), "got: {out}");
    }

    #[test]
    fn renders_task_list_header_only() {
        let line = r#"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"TaskList","input":{}}]}}"#;
        let out = render_to_string(line);
        assert!(out.contains("TaskList"), "got: {out}");
        assert!(
            !out.contains("TaskList("),
            "no args for TaskList; got: {out}"
        );
    }

    #[test]
    fn renders_task_stop_with_id() {
        let line = r#"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"TaskStop","input":{"taskId":"bg123"}}]}}"#;
        let out = render_to_string(line);
        assert!(out.contains("TaskStop(bg123)"), "got: {out}");
    }

    #[test]
    fn renders_glob_and_grep_as_search() {
        // GlobTool and GrepTool both have userFacingName = "Search".
        let glob = r#"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"Glob","input":{"pattern":"**/*.rs"}}]}}"#;
        assert!(render_to_string(glob).contains("Search(**/*.rs)"));
        let grep = r#"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"Grep","input":{"pattern":"TODO"}}]}}"#;
        assert!(render_to_string(grep).contains("Search(TODO)"));
    }

    #[test]
    fn renders_webfetch_as_fetch() {
        let line = r#"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"WebFetch","input":{"url":"https://example.com"}}]}}"#;
        assert!(render_to_string(line).contains("Fetch(https://example.com)"));
    }

    #[test]
    fn renders_skill_tool_with_name() {
        let line = r#"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"Skill","input":{"skill":"lt.review"}}]}}"#;
        let out = render_to_string(line);
        assert!(out.contains("Skill(lt.review)"), "got: {out}");
    }

    #[test]
    fn renders_skill_tool_with_name_and_args() {
        let line = r#"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"Skill","input":{"skill":"pdf","args":"report.pdf"}}]}}"#;
        let out = render_to_string(line);
        assert!(out.contains("Skill(pdf report.pdf)"), "got: {out}");
    }

    #[test]
    fn renders_task_agent_with_description_and_subtype() {
        // claude-code's AgentTool uses subagent_type AS the tool name.
        let line = r#"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"Task","input":{"description":"Audit auth module","subagent_type":"security-reviewer"}}]}}"#;
        let out = render_to_string(line);
        assert!(out.contains("security-reviewer"), "got: {out}");
        assert!(out.contains("Audit auth module"), "got: {out}");
    }

    #[test]
    fn renders_todowrite_first_item() {
        let line = r#"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"TodoWrite","input":{"todos":[{"content":"Fix bug in auth handler","status":"pending"}]}}]}}"#;
        let out = render_to_string(line);
        assert!(out.contains("TodoWrite"), "got: {out}");
        assert!(out.contains("Fix bug in auth handler"), "got: {out}");
    }

    #[test]
    fn truncates_long_bash_command() {
        let long = "a".repeat(200);
        let line = format!(
            r#"{{"type":"assistant","message":{{"content":[{{"type":"tool_use","name":"Bash","input":{{"command":"{long}"}}}}]}}}}"#
        );
        let out = render_to_string(&line);
        assert!(out.contains("…"), "got: {out}");
        // Matches BashTool/UI.tsx's MAX_COMMAND_DISPLAY_CHARS = 160.
        assert!(!out.contains(&"a".repeat(170)), "got: {out}");
    }

    #[test]
    fn tool_result_silent_on_success_shows_reason_on_error() {
        // claude-code doesn't render a success tick — the tool_use header
        // already said what happened. Only failures deserve a new line
        // with the ⎿ marker and a truncated reason.
        let ok = r#"{"type":"user","message":{"content":[{"type":"tool_result","content":"file saved"}]}}"#;
        assert_eq!(render_to_string(ok), "");

        let err = r#"{"type":"user","message":{"content":[{"type":"tool_result","is_error":true,"content":"permission denied"}]}}"#;
        let out = render_to_string(err);
        assert!(out.contains("⎿"), "must use ⎿ marker; got: {out}");
        assert!(out.contains("permission denied"), "got: {out}");
    }

    #[test]
    fn thinking_block_shows_marker_not_content() {
        let line = r#"{"type":"assistant","message":{"content":[{"type":"thinking","thinking":"secret internal reasoning ..."}]}}"#;
        let out = render_to_string(line);
        assert!(out.contains("(thinking…)"), "got: {out}");
        // Must not leak the internal reasoning text
        assert!(!out.contains("secret internal reasoning"), "got: {out}");
    }

    #[test]
    fn stream_event_text_delta_is_appended_inline() {
        let line = r#"{"type":"stream_event","event":{"type":"content_block_delta","delta":{"type":"text_delta","text":"Hel"}}}"#;
        let out = render_to_string(line);
        // Delta events must NOT add a trailing newline — they get concatenated
        // to form streaming text.
        assert_eq!(out, "Hel");
    }

    #[test]
    fn result_error_event_shows_failure() {
        let line = r#"{"type":"result","subtype":"error_max_turns","is_error":true}"#;
        let out = render_to_string(line);
        assert!(out.contains("error"), "got: {out}");
        assert!(out.contains("max_turns"), "got: {out}");
    }

    #[test]
    fn successful_result_is_silent() {
        let line = r#"{"type":"result","subtype":"success","result":"done"}"#;
        // No noise on success — final text already rendered via assistant events.
        assert_eq!(render_to_string(line), "");
    }
}
