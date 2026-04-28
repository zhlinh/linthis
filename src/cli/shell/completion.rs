//! `linthis shell completion <shell>` — emits clap-generated completion script.

#![allow(dead_code)]

use crate::cli::commands::Cli;
use clap::CommandFactory;
use clap_complete::Shell as CcShell;
use std::io::Write;

use super::state::Shell;

fn map(shell: Shell) -> CcShell {
    match shell {
        Shell::Bash => CcShell::Bash,
        Shell::Zsh => CcShell::Zsh,
        Shell::Fish => CcShell::Fish,
        Shell::PowerShell => CcShell::PowerShell,
    }
}

/// Write the completion script for `shell` to `out`. Returns bytes written.
pub fn write_completion(shell: Shell, out: &mut dyn Write) -> std::io::Result<()> {
    let mut cmd = Cli::command();
    clap_complete::generate(map(shell), &mut cmd, "linthis", out);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render_to_string(shell: Shell) -> String {
        let mut buf = Vec::new();
        write_completion(shell, &mut buf).unwrap();
        String::from_utf8(buf).expect("completion output is utf-8")
    }

    #[test]
    fn bash_output_is_nonempty_and_mentions_linthis() {
        let s = render_to_string(Shell::Bash);
        assert!(s.len() > 100, "bash completion looks empty: {s:?}");
        assert!(s.contains("linthis"));
    }

    #[test]
    fn zsh_output_starts_with_compdef() {
        let s = render_to_string(Shell::Zsh);
        assert!(
            s.starts_with("#compdef linthis"),
            "zsh script missing #compdef: {}",
            &s[..80.min(s.len())]
        );
    }

    #[test]
    fn fish_output_uses_complete_command() {
        let s = render_to_string(Shell::Fish);
        assert!(s.contains("complete -c linthis"));
    }

    #[test]
    fn powershell_output_uses_register_argument_completer() {
        let s = render_to_string(Shell::PowerShell);
        assert!(s.contains("Register-ArgumentCompleter"));
    }
}
