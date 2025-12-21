# Implementation Plan: Ruff Integration for Python Linting & Formatting

## Overview

Replace `flake8` (checker) and `black` (formatter) with `ruff` for Python file processing in linthis. Ruff is an extremely fast Python linter and formatter written in Rust, offering 10-100x speed improvements.

## Research Summary

### Ruff Capabilities

| Feature | flake8+black | ruff |
|---------|--------------|------|
| Linting | flake8 | `ruff check` |
| Formatting | black | `ruff format` |
| Speed | Baseline | 10-100x faster |
| Language | Python | Rust |
| Rule Coverage | flake8 + plugins | 800+ built-in rules |
| Auto-fix | Limited | Comprehensive |
| Config | .flake8, pyproject.toml | pyproject.toml, ruff.toml |

### Ruff CLI Commands

**Linting:**
```bash
ruff check path/to/file.py              # Check file
ruff check --output-format json         # JSON output
ruff check --fix                        # Auto-fix
```

**Formatting:**
```bash
ruff format path/to/file.py             # Format file
ruff format --check                     # Check without modifying
ruff format --diff                      # Show diff
```

### Ruff JSON Output Structure

```json
{
  "cell": null,
  "code": "F401",
  "end_location": {"column": 10, "row": 1},
  "filename": "test.py",
  "fix": {
    "applicability": "safe",
    "edits": [{"content": "", "end_location": {...}, "location": {...}}],
    "message": "Remove unused import: `os`"
  },
  "location": {"column": 8, "row": 1},
  "message": "`os` imported but unused",
  "noqa_row": 1,
  "url": "https://docs.astral.sh/ruff/rules/unused-import"
}
```

## Technical Design

### Phase 1: Replace Python Checker (flake8 -> ruff check)

**File:** `src/checkers/python.rs`

Changes:
1. Replace `flake8` command with `ruff check --output-format json`
2. Parse JSON output instead of text parsing
3. Map ruff error codes to `Severity` (same prefix logic works)
4. Update `name()` to return `"ruff"`
5. Update `is_available()` to check for `ruff`

**New parsing logic:**
```rust
fn parse_ruff_json_output(&self, output: &str, file_path: &Path) -> Vec<LintIssue> {
    // Parse JSON array of issues
    // Extract: filename, code, message, location.row, location.column
    // Map code prefix to severity (E/F -> Error, W -> Warning, etc.)
}
```

### Phase 2: Replace Python Formatter (black -> ruff format)

**File:** `src/formatters/python.rs`

Changes:
1. Replace `black` command with `ruff format`
2. Replace `black --check` with `ruff format --check`
3. Update `name()` to return `"ruff"`
4. Update `is_available()` to check for `ruff`
5. Remove hard-coded `--line-length 120` (use ruff config)

### Phase 3: Configuration Updates

**File:** `defaults/config.toml`

Add ruff-specific configuration section:
```toml
[python]
linter = "ruff"       # or "flake8" for backwards compatibility
formatter = "ruff"    # or "black" for backwards compatibility
line_length = 120
```

**File:** `src/main.rs` (--init-configs)

Update to generate `ruff.toml` or `pyproject.toml` with ruff config:
```toml
[tool.ruff]
line-length = 120

[tool.ruff.lint]
select = ["E", "F", "W"]
```

### Phase 4: Speed Comparison Feature

Add optional benchmark mode to compare linter/formatter performance:

**CLI:**
```bash
linthis --benchmark --lang python    # Compare ruff vs flake8+black
```

**Implementation:**
1. Add `--benchmark` flag to CLI
2. When enabled, run both tool sets and measure time
3. Output comparison table

**Output example:**
```
Python Linting/Formatting Benchmark (100 files)
┌──────────────┬─────────────┬─────────────┬──────────┐
│ Tool         │ Lint (ms)   │ Format (ms) │ Total    │
├──────────────┼─────────────┼─────────────┼──────────┤
│ flake8+black │ 5,234       │ 3,421       │ 8,655ms  │
│ ruff         │ 312         │ 198         │ 510ms    │
├──────────────┼─────────────┼─────────────┼──────────┤
│ Speedup      │ 16.8x       │ 17.3x       │ 17.0x    │
└──────────────┴─────────────┴─────────────┴──────────┘
```

## Implementation Tasks

### Task 1: Update Python Checker for Ruff
- [ ] Modify `src/checkers/python.rs`
- [ ] Add JSON parsing with serde
- [ ] Update command invocation
- [ ] Add tests

### Task 2: Update Python Formatter for Ruff
- [ ] Modify `src/formatters/python.rs`
- [ ] Update command invocation
- [ ] Add tests

### Task 3: Update Configuration
- [ ] Update `defaults/config.toml`
- [ ] Update `--init-configs` to generate ruff config
- [ ] Document new options

### Task 4: Add Benchmark Mode (Optional)
- [ ] Add `--benchmark` CLI flag
- [ ] Implement dual-tool timing
- [ ] Format comparison output

### Task 5: Testing & Documentation
- [ ] Integration tests with ruff
- [ ] Update README
- [ ] Test backwards compatibility

## Data Model

### Ruff JSON Issue Structure

```rust
#[derive(Debug, Deserialize)]
struct RuffIssue {
    filename: String,
    code: String,
    message: String,
    location: RuffLocation,
    end_location: RuffLocation,
    fix: Option<RuffFix>,
    url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RuffLocation {
    row: usize,
    column: usize,
}

#[derive(Debug, Deserialize)]
struct RuffFix {
    message: String,
    applicability: String,
    edits: Vec<RuffEdit>,
}

#[derive(Debug, Deserialize)]
struct RuffEdit {
    content: String,
    location: RuffLocation,
    end_location: RuffLocation,
}
```

### Severity Mapping

| Ruff Code Prefix | Severity |
|------------------|----------|
| E (Error) | Error |
| F (Pyflakes) | Error |
| W (Warning) | Warning |
| C (Convention) | Info |
| R (Refactor) | Info |
| I (Import) | Info |
| N (Naming) | Warning |
| D (Docstring) | Info |
| UP (pyupgrade) | Info |
| B (bugbear) | Warning |
| S (bandit/security) | Warning |
| A (builtins) | Warning |
| Others | Info |

## API Contracts

### Checker Trait (unchanged)

```rust
pub trait Checker: Send + Sync {
    fn name(&self) -> &str;                              // Returns "ruff"
    fn supported_languages(&self) -> &[Language];        // [Language::Python]
    fn check(&self, path: &Path) -> Result<Vec<LintIssue>>;
    fn is_available(&self) -> bool;
}
```

### Formatter Trait (unchanged)

```rust
pub trait Formatter: Send + Sync {
    fn name(&self) -> &str;                              // Returns "ruff"
    fn supported_languages(&self) -> &[Language];        // [Language::Python]
    fn format(&self, path: &Path) -> Result<FormatResult>;
    fn check(&self, path: &Path) -> Result<bool>;
    fn is_available(&self) -> bool;
}
```

## Quickstart

### Prerequisites
```bash
# Install ruff
pip install ruff
# or
uv pip install ruff
# or
brew install ruff
```

### Verify Installation
```bash
ruff --version
# ruff 0.8.x
```

### Basic Usage (after implementation)
```bash
# Run linthis on Python files (uses ruff)
linthis --lang python

# Benchmark comparison
linthis --benchmark --lang python
```

## Risk Assessment

| Risk | Mitigation |
|------|------------|
| Ruff not installed | Fall back to flake8+black if ruff unavailable |
| Different rule sets | Document rule mapping, provide migration guide |
| Config format change | Support both ruff.toml and legacy .flake8 |

## Timeline Estimate

This plan covers all implementation details. The actual implementation can proceed task-by-task.

## References

- [Ruff Documentation](https://docs.astral.sh/ruff/)
- [Ruff GitHub](https://github.com/astral-sh/ruff)
- [Ruff vs Flake8 FAQ](https://docs.astral.sh/ruff/faq/)
