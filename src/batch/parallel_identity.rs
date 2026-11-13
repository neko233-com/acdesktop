use acdesktop_core::RefStore;

use super::preparation::PreparedCommand;
use crate::cli::Commands;

pub(super) fn process_key(item: &PreparedCommand) -> Option<String> {
    key_with_lookup(item, |reference, session| {
        RefStore::for_session(session)
            .ok()?
            .load_ref(reference)
            .ok()?
            .process
            .identity()
    })
}

fn key_with_lookup(
    item: &PreparedCommand,
    lookup: impl FnOnce(&str, Option<&str>) -> Option<acdesktop_core::ProcessIdentity>,
) -> Option<String> {
    if item.context.is_headed() {
        return None;
    }
    let reference = match &item.command {
        Commands::Click(args)
        | Commands::Clear(args)
        | Commands::Toggle(args)
        | Commands::Check(args)
        | Commands::Uncheck(args)
        | Commands::Expand(args)
        | Commands::Collapse(args)
        | Commands::ScrollTo(args) => &args.ref_id,
        Commands::SetValue(args) => &args.ref_id,
        Commands::Select(args) => &args.ref_id,
        _ => return None,
    };
    let identity = lookup(reference, item.context.session_id())?;
    Some(format!("{}:{}", identity.pid, identity.instance))
}

#[cfg(test)]
#[path = "parallel_identity_tests.rs"]
mod tests;
