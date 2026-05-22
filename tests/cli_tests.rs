//! Integration tests for linthis CLI.

mod integration;

#[test]
fn test_hook_install_multi_type_parses() {
    use clap::Parser;
    use linthis::cli::commands::{Cli, Commands, HookCommands};
    let cli = Cli::try_parse_from([
        "linthis",
        "hook",
        "install",
        "--type",
        "git,agent",
        "--event",
        "pre-commit,pre-push",
    ])
    .expect("parse failed");
    if let Some(Commands::Hook {
        action:
            HookCommands::Add {
                hook_types,
                hook_events,
                ..
            },
    }) = cli.command
    {
        assert_eq!(hook_types.len(), 2);
        assert_eq!(hook_events.len(), 2);
    } else {
        panic!("wrong command");
    }
}

#[test]
fn test_hook_install_repeated_event_parses() {
    use clap::Parser;
    use linthis::cli::commands::{Cli, Commands, HookCommands};
    let cli = Cli::try_parse_from([
        "linthis",
        "hook",
        "install",
        "--type",
        "git",
        "--event",
        "pre-commit",
        "--event",
        "commit-msg",
    ])
    .expect("parse failed");
    if let Some(Commands::Hook {
        action: HookCommands::Add { hook_events, .. },
    }) = cli.command
    {
        assert_eq!(hook_events.len(), 2);
    } else {
        panic!("wrong command");
    }
}
