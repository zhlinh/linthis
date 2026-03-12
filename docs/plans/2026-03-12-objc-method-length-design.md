# ObjC Method Length Check Design

**Goal:** Add a native Objective-C method length check to linthis that counts SLOC (non-blank, non-comment lines) per method and reports violations via the existing `readability/fn_size` rule code.

**Architecture:** Extend `CppChecker` with a pure-Rust ObjC method scanner. Threshold is loaded via the existing config priority chain (`[oc]` in `config.toml` > plugin config > default 80). No external tool dependency.

**Tech Stack:** Rust, existing `CppLanguageConfig` / `CppChecker` in `src/checkers/cpp.rs` and `src/config/mod.rs`.

---

## Components

### 1. Config (`src/config/mod.rs`)

Add `fn_length: Option<u32>` to `CppLanguageConfig`:

```rust
/// Max ObjC method SLOC (non-blank, non-comment lines). Default: 80.
#[serde(default)]
pub fn_length: Option<u32>,
```

User config example:
```toml
[oc]
fn_length = 100
```

### 2. CppChecker (`src/checkers/cpp.rs`)

**Struct field:**
```rust
oc_fn_length: u32,   // default 80
```

**Config loading** — extend `load_cpplint_configs()` with the same priority chain already used for `linelength`:
1. Default: `80`
2. Plugin config: `.linthis/configs/oc/linthis.cfg` → `fn_length = N`
3. `config.toml` `[oc] fn_length` (highest, overrides plugin)

**New function `run_objc_method_length(path) -> Result<Vec<LintIssue>>`:**

- Scan lines, detect method start with regex: `^\s*[+-]\s*\(`
- For each method: count SLOC using a state machine that skips blank lines and pure-comment lines (`//…`, `/* … */` blocks)
- When next method start or EOF reached: if SLOC > threshold → emit `LintIssue`

**Issue format:**
- code: `readability/fn_size`
- severity: `Warning`
- line: method signature start line
- message: `Method 'foo' has 97 lines of code (limit is 80) [readability/fn_size]`

**`check()` call site:**
```rust
if is_oc {
    match self.run_objc_method_length(path) {
        Ok(issues) => all_issues.extend(issues),
        Err(e) => log::warn!("objc method length check failed: {}", e),
    }
}
```

### 3. SLOC Counting Rules

State machine per method body:
- `in_block_comment = false`
- Blank line → skip
- Pure `// comment` line → skip
- `/* ... */` on one line → skip
- `/*` starts block → `in_block_comment = true`, skip
- Inside block comment → skip until `*/` found
- Everything else → count += 1

Line-trailing comments (`code; // comment`) count as code (line has substantive content).

---

## Testing

- Unit test: method under threshold → no issue
- Unit test: method over threshold → issue at correct line
- Unit test: blank lines and comments not counted
- Unit test: multiple methods in one file → each checked independently
- Unit test: block comment spanning multiple lines not counted
- Integration test: verify `checkCodeStyleRight2` (112 total, ~97 SLOC) is flagged in AppDelegate.mm
