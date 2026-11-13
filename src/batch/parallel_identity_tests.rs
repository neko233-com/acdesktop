use super::*;
use acdesktop_core::{CommandContext, ProcessIdentity, commands::batch::BatchCommand};
use serde_json::json;

fn item(command: &str, args: serde_json::Value, headed: bool) -> PreparedCommand {
    PreparedCommand {
        name: command.into(),
        command: crate::batch::parse_command(BatchCommand {
            command: command.into(),
            args,
            session: None,
        })
        .unwrap(),
        context: CommandContext::default().with_headed(headed),
    }
}

#[test]
fn only_semantic_headless_ref_actions_with_verified_identity_get_a_key() {
    let click = item("click", json!({"ref_id":"@s12345678:e1"}), false);
    let identity = || ProcessIdentity::new(7, "generation");
    assert_eq!(
        key_with_lookup(&click, |_, _| Some(identity())).as_deref(),
        Some("7:generation")
    );
    assert_eq!(key_with_lookup(&click, |_, _| None), None);
    let headed = item("click", json!({"ref_id":"@s12345678:e1"}), true);
    assert_eq!(key_with_lookup(&headed, |_, _| Some(identity())), None);
    let launch = item("launch", json!({"app":"fixture"}), false);
    assert_eq!(key_with_lookup(&launch, |_, _| Some(identity())), None);
}

#[test]
fn process_keys_distinguish_generation_and_pid() {
    let click = item("click", json!({"ref_id":"@s12345678:e1"}), false);
    let key = |pid, generation| {
        key_with_lookup(&click, |_, _| Some(ProcessIdentity::new(pid, generation)))
    };
    assert_eq!(key(7_u32, "one"), key(7_u32, "one"));
    assert_ne!(key(7_u32, "one"), key(7_u32, "two"));
    assert_ne!(key(7_u32, "one"), key(8_u32, "one"));
}
