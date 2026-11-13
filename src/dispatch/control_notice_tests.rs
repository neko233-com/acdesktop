use super::needs_notice;
use crate::cli::Cli;
use clap::Parser;

#[test]
fn distinguishes_control_from_observation_and_game_owns_its_notice() {
    for args in [
        vec!["acdesktop", "mouse-move", "--xy", "1,2"],
        vec!["acdesktop", "click", "@s1234567:e1"],
        vec!["acdesktop", "press", "a"],
    ] {
        let cli = Cli::try_parse_from(args).unwrap();
        assert!(needs_notice(cli.command.as_ref().unwrap()));
    }
    let cli = Cli::try_parse_from(["acdesktop", "list-apps"]).unwrap();
    assert!(!needs_notice(cli.command.as_ref().unwrap()));
}
