// Copyright 2024 zhlinh and linthis Project Authors. All rights reserved.
// Use of this source code is governed by a MIT-style
// license that can be found at
//
// https://opensource.org/license/MIT
//
// The above copyright notice and this permission
// notice shall be included in all copies or
// substantial portions of the Software.

//! Linthis CLI - A fast, cross-platform multi-language linter and formatter.

use clap::Parser;
use colored::Colorize;
use std::path::PathBuf;
use std::process::ExitCode;

use linthis::utils::output::{format_result, OutputFormat};
use linthis::{run, Language, RunMode, RunOptions};

#[derive(Parser, Debug)]
#[command(name = "linthis")]
#[command(
    author,
    version,
    about = "A fast, cross-platform multi-language linter and formatter"
)]
struct Cli {
    /// Files or directories to check
    #[arg(default_value = ".")]
    paths: Vec<PathBuf>,

    /// Only run lint checks, no formatting
    #[arg(short = 'c', long)]
    check_only: bool,

    /// Only format files, no lint checking
    #[arg(short = 'f', long)]
    format_only: bool,

    /// Check only staged files (git cached)
    #[arg(short = 's', long)]
    staged: bool,

    /// Specify languages to check (comma-separated: rust,python,typescript)
    #[arg(short, long, value_delimiter = ',')]
    lang: Option<Vec<String>>,

    /// Exclude patterns (glob patterns)
    #[arg(short, long)]
    exclude: Option<Vec<String>>,

    /// Disable default exclusions (.git, node_modules, target, etc.)
    #[arg(long)]
    no_default_excludes: bool,

    /// Disable .gitignore pattern exclusions
    #[arg(long)]
    no_gitignore: bool,

    /// Path to configuration file
    #[arg(long)]
    config: Option<std::path::PathBuf>,

    /// Initialize a new .linthis.toml configuration file
    #[arg(long)]
    init: bool,

    /// Generate default config files for all linters/formatters
    #[arg(long)]
    init_configs: bool,

    /// Format preset (google, standard, airbnb)
    #[arg(short, long)]
    preset: Option<String>,

    /// Output format: human, json, github-actions
    #[arg(short, long, default_value = "human")]
    output: String,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,

    /// Suppress non-error output
    #[arg(short, long)]
    quiet: bool,

    /// Run benchmark comparing ruff vs flake8+black for Python
    #[arg(long)]
    benchmark: bool,
}

/// Default config file contents for each linter/formatter
fn get_default_configs() -> Vec<(&'static str, &'static str)> {
    vec![
        // Python - ruff (standalone config)
        (
            "ruff.toml",
            r#"# Linthis default ruff config
# Ruff is an extremely fast Python linter and formatter, written in Rust

line-length = 120
target-version = "py38"

[lint]
# Enable recommended rules
select = ["E", "F", "W", "I", "UP", "B", "C4", "SIM"]
ignore = ["E203", "W503"]

# Allow unused variables when underscore-prefixed
dummy-variable-rgx = "^(_+|(_+[a-zA-Z0-9_]*[a-zA-Z0-9]+?))$"

[lint.per-file-ignores]
"__init__.py" = ["F401"]

[format]
# Use double quotes for strings
quote-style = "double"
# Indent with spaces
indent-style = "space"
"#,
        ),
        // Python - ruff (in pyproject.toml for projects that prefer it)
        (
            "pyproject.toml",
            r#"[tool.ruff]
line-length = 120
target-version = "py38"

[tool.ruff.lint]
# Enable recommended rules
select = ["E", "F", "W", "I", "UP", "B", "C4", "SIM"]
ignore = ["E203", "W503"]

[tool.ruff.lint.per-file-ignores]
"__init__.py" = ["F401"]

[tool.ruff.format]
quote-style = "double"
indent-style = "space"
"#,
        ),
        // C/C++ - clang-format
        (
            ".clang-format",
            r#"# Lintis default clang-format config
BasedOnStyle: Google
IndentWidth: 4
ColumnLimit: 120
AllowShortFunctionsOnASingleLine: None
AllowShortIfStatementsOnASingleLine: false
AllowShortLoopsOnASingleLine: false
BreakBeforeBraces: Attach
PointerAlignment: Left
SpaceAfterCStyleCast: false
"#,
        ),
        // C/C++ - cpplint
        (
            "CPPLINT.cfg",
            r#"# Lintis default cpplint config
set noparent
linelength=120
"#,
        ),
        // TypeScript/JavaScript - prettier
        (
            ".prettierrc",
            r#"{
  "semi": true,
  "singleQuote": true,
  "tabWidth": 2,
  "printWidth": 120,
  "trailingComma": "es5",
  "bracketSpacing": true,
  "arrowParens": "avoid"
}
"#,
        ),
        // TypeScript/JavaScript - eslint
        (
            ".eslintrc.json",
            r#"{
  "env": {
    "browser": true,
    "es2021": true,
    "node": true
  },
  "extends": ["eslint:recommended"],
  "parserOptions": {
    "ecmaVersion": "latest",
    "sourceType": "module"
  },
  "rules": {
    "no-unused-vars": "warn",
    "no-console": "off",
    "semi": ["error", "always"],
    "quotes": ["error", "single"]
  }
}
"#,
        ),
        // Rust - rustfmt
        (
            "rustfmt.toml",
            r#"# Lintis default rustfmt config
max_width = 120
tab_spaces = 4
edition = "2021"
use_small_heuristics = "Default"
"#,
        ),
    ]
}

/// Initialize default config files for all linters/formatters
fn init_linter_configs() -> ExitCode {
    use std::fs;
    use std::path::Path;

    let configs = get_default_configs();
    let mut created = 0;
    let mut skipped = 0;

    println!("{}", "Generating default linter/formatter configs...".cyan());

    for (filename, content) in configs {
        let path = Path::new(filename);
        if path.exists() {
            println!("  {} {} (already exists)", "⊘".yellow(), filename);
            skipped += 1;
        } else {
            match fs::write(path, content) {
                Ok(_) => {
                    println!("  {} {}", "✓".green(), filename);
                    created += 1;
                }
                Err(e) => {
                    eprintln!("  {} {} ({})", "✗".red(), filename, e);
                }
            }
        }
    }

    println!();
    println!(
        "Created {} config file{}, skipped {} existing",
        created,
        if created == 1 { "" } else { "s" },
        skipped
    );

    ExitCode::SUCCESS
}

/// Run benchmark comparing ruff vs flake8+black for Python
fn run_benchmark(cli: &Cli) -> ExitCode {
    use linthis::benchmark::{format_benchmark_table, run_python_benchmark};
    use linthis::utils::walker::{walk_paths, WalkerConfig};

    println!(
        "{}",
        "Running Python linting/formatting benchmark...".cyan()
    );
    println!("Comparing ruff vs flake8+black\n");

    // Get paths to scan
    let paths = cli.paths.clone();

    // Configure walker for Python files only
    let walker_config = WalkerConfig {
        exclude_patterns: cli.exclude.clone().unwrap_or_default(),
        languages: vec![Language::Python],
        ..Default::default()
    };

    // Collect Python files
    let files = walk_paths(&paths, &walker_config);

    if files.is_empty() {
        println!("{}", "No Python files found to benchmark.".yellow());
        return ExitCode::SUCCESS;
    }

    println!("Found {} Python files", files.len());

    // Convert to Path references
    let file_refs: Vec<&std::path::Path> = files.iter().map(|p| p.as_path()).collect();

    // Run benchmark
    let comparison = run_python_benchmark(&file_refs);

    // Output results
    println!("{}", format_benchmark_table(&comparison));

    ExitCode::SUCCESS
}

fn main() -> ExitCode {
    env_logger::init();

    let cli = Cli::parse();

    // Handle --init flag
    if cli.init {
        let config_path = linthis::config::Config::project_config_path(
            &std::env::current_dir().unwrap_or_default(),
        );
        if config_path.exists() {
            eprintln!(
                "{}: {} already exists",
                "Warning".yellow(),
                config_path.display()
            );
            return ExitCode::from(1);
        }

        let content = linthis::config::Config::generate_default_toml();
        match std::fs::write(&config_path, content) {
            Ok(_) => {
                println!("{} Created {}", "✓".green(), config_path.display());
                return ExitCode::SUCCESS;
            }
            Err(e) => {
                eprintln!("{}: Failed to create config: {}", "Error".red(), e);
                return ExitCode::from(2);
            }
        }
    }

    // Handle --init-configs flag
    if cli.init_configs {
        return init_linter_configs();
    }

    // Handle --benchmark flag
    if cli.benchmark {
        return run_benchmark(&cli);
    }

    // Determine run mode
    let mode = if cli.check_only {
        RunMode::CheckOnly
    } else if cli.format_only {
        RunMode::FormatOnly
    } else {
        RunMode::Both
    };

    // Parse languages
    let languages: Vec<Language> = cli
        .lang
        .unwrap_or_default()
        .iter()
        .filter_map(|s| Language::from_name(s))
        .collect();

    // Get paths (handle staged files)
    let paths = if cli.staged {
        match linthis::utils::get_staged_files() {
            Ok(files) => {
                if files.is_empty() {
                    if !cli.quiet {
                        println!("{}", "No staged files to check".yellow());
                    }
                    return ExitCode::SUCCESS;
                }
                files
            }
            Err(e) => {
                eprintln!("{}: {}", "Error getting staged files".red(), e);
                return ExitCode::from(2);
            }
        }
    } else {
        cli.paths
    };

    // Build exclusion patterns (defaults + gitignore + user-specified)
    let mut exclude_patterns: Vec<String> = if cli.no_default_excludes {
        Vec::new()
    } else {
        linthis::utils::DEFAULT_EXCLUDES
            .iter()
            .map(|s| s.to_string())
            .collect()
    };

    // Add .gitignore patterns if in a git repo and not disabled
    if !cli.no_gitignore && linthis::utils::is_git_repo() {
        let project_root = linthis::utils::get_project_root();
        let gitignore_patterns = linthis::utils::get_gitignore_patterns(&project_root);
        if cli.verbose && !gitignore_patterns.is_empty() {
            eprintln!(
                "Loaded {} patterns from .gitignore",
                gitignore_patterns.len()
            );
        }
        exclude_patterns.extend(gitignore_patterns);
    }

    exclude_patterns.extend(cli.exclude.unwrap_or_default());

    // Build options
    let options = RunOptions {
        paths,
        mode,
        languages,
        exclude_patterns,
        verbose: cli.verbose,
        quiet: cli.quiet,
    };

    // Parse output format
    let output_format = OutputFormat::parse(&cli.output).unwrap_or(OutputFormat::Human);

    if cli.verbose {
        eprintln!(
            "{}",
            "linthis - Multi-language Linter & Formatter".bold().cyan()
        );
        eprintln!("Mode: {:?}", mode);
        eprintln!("Paths: {:?}", options.paths);
    }

    // Run linthis
    match run(&options) {
        Ok(result) => {
            // Output results
            if !cli.quiet || result.exit_code != 0 {
                let output = format_result(&result, output_format);
                if !output.is_empty() {
                    println!("{}", output);
                }
            }

            ExitCode::from(result.exit_code as u8)
        }
        Err(e) => {
            eprintln!("{}: {}", "Error".red().bold(), e);
            ExitCode::from(2)
        }
    }
}
