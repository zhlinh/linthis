# Report and Analysis

The `linthis report` command provides tools for generating reports, viewing results, analyzing trends, and checking code consistency across your codebase.

## Subcommands Overview

| Command | Description |
|---------|-------------|
| `show` | Display saved lint results in human-readable format |
| `html` | Generate an HTML report with charts and statistics |
| `stats` | Show statistics from lint results |
| `trends` | Analyze code quality trends over time |
| `consistency` | Analyze team code style consistency |

## Show Results

Display saved lint results (JSON) in the same human-readable format as `linthis -c` output. Useful for viewing full error details from hook output or reviewing previous runs.

### Basic Usage

```bash
# Show last result
linthis report show

# Show specific result file
linthis report show .linthis/result/result-20240101-120000.json

# Show with limit
linthis report show -n 10
```

### Options

| Option | Description |
|--------|-------------|
| `-n, --limit` | Limit number of issues to display (0 = unlimited) |
| `--compact` | Compact format: show only file:line message without code context |
| `--errors-only` | Only show errors (skip warnings and info) |
| `--warnings-only` | Only show warnings (skip errors and info) |
| `-f, --format` | Output format: `human` (default), `json` |

### Examples

```bash
# Compact format (no code context)
linthis report show --compact

# Only show errors
linthis report show --errors-only

# Limit to 5 issues
linthis report show -n 5

# JSON output
linthis report show --format json
```

### Typical Workflow

After seeing truncated messages in git hook output:

```bash
# Hook shows truncated output like:
# ╭──────────────────────────────────────╮
# │  E dt_head.h:11 inclusion of deprec... │
# ╰──────────────────────────────────────╯

# View full details with:
linthis report show
```

## HTML Report

Generate a self-contained HTML file with charts, statistics, and detailed issue listings.

### Basic Usage

```bash
# Generate HTML report from last result
linthis report html

# Include trend analysis
linthis report html --with-trends

# Specify output file
linthis report html -o my-report.html
```

### Options

| Option | Description |
|--------|-------------|
| `-o, --output` | Output file path (default: `.linthis/reports/report-{timestamp}.html`) |
| `--with-trends` | Include historical trend analysis in the report |
| `-n, --trend-count` | Number of historical runs to include (default: 10) |

## Statistics

Display issue counts by severity, language, tool, and rule. Also shows top problematic files and summary metrics.

### Basic Usage

```bash
# Show stats from last result
linthis report stats

# JSON format
linthis report stats --format json
```

### Options

| Option | Description |
|--------|-------------|
| `-f, --format` | Output format: `human` (default), `json` |

## Trends Analysis

Examine historical lint results to identify trends in code quality. Shows whether issues are improving, stable, or degrading.

### Basic Usage

```bash
# Analyze last 10 runs
linthis report trends

# Analyze last 20 runs
linthis report trends -n 20

# JSON format
linthis report trends --format json
```

### Options

| Option | Description |
|--------|-------------|
| `-n, --count` | Number of historical runs to analyze (default: 10) |
| `-f, --format` | Output format: `human` (default), `json` |

## Consistency Analysis

Identify repeated patterns, outlier files, and systematic issues to help improve code consistency across the codebase.

### Basic Usage

```bash
# Analyze code consistency
linthis report consistency

# JSON format
linthis report consistency --format json
```

### Options

| Option | Description |
|--------|-------------|
| `-f, --format` | Output format: `human` (default), `json` |

## Result Storage

By default, linthis saves lint results to `.linthis/result/` directory:

- Results are saved as JSON files with timestamp: `result-YYYYMMDD-HHMMSS.json`
- Use `--no-save-result` to disable auto-saving
- Use `--output-file` to specify a custom output path
- Use `--keep-results` to limit the number of result files kept (default: 10)

## Hook Output Width

The git hook output box width is now adaptive to your terminal width:

- Auto-detects terminal width (50-120 characters)
- Can be configured via `hook.output_width` in config file

```toml
# .linthis/config.toml
[hook]
output_width = 100  # 0 = auto-detect (default)
```
