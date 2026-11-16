use std::sync::atomic::{AtomicU64, Ordering};

use acdesktop_core::{
    AppError, CommandContext, ControlIndicatorEvent, DeliverySemantics, PlatformAdapter,
};

use crate::cli::Commands;

static NEXT_OWNER: AtomicU64 = AtomicU64::new(1);

pub(super) struct IndicatorGuard<'a> {
    adapter: &'a dyn PlatformAdapter,
    owner: String,
    transient: bool,
}

impl IndicatorGuard<'_> {
    pub(super) fn owner(&self) -> &str {
        &self.owner
    }
}

#[cfg(test)]
#[path = "control_indicator_tests.rs"]
mod tests;

impl Drop for IndicatorGuard<'_> {
    fn drop(&mut self) {
        if self.transient {
            let _ = self.adapter.hide_control_indicator(&self.owner);
        }
    }
}

pub(super) fn begin<'a>(
    command: &Commands,
    adapter: &'a dyn PlatformAdapter,
    context: &CommandContext,
) -> Result<Option<IndicatorGuard<'a>>, AppError> {
    if !command.is_mutating()
        || matches!(
            command,
            Commands::Session(_)
                | Commands::CursorOverlay(_)
                | Commands::Batch(_)
                | Commands::Screenshot(_)
        )
    {
        return Ok(None);
    }
    let session = match context.session_id() {
        Some(id) if acdesktop_core::session::read_manifest(id)?.is_some() => Some(id),
        _ => None,
    };
    let transient = session.is_none();
    let owner = session.map(str::to_owned).unwrap_or_else(|| {
        format!(
            "once-{}-{}",
            std::process::id(),
            NEXT_OWNER.fetch_add(1, Ordering::Relaxed)
        )
    });
    let description = context.description().unwrap_or_else(|| command.name());
    if let Err(error) = adapter.show_control_indicator(&owner, !transient, description) {
        if adapter.requires_control_indicator() {
            return Err(error
                .with_disposition(DeliverySemantics::not_delivered())
                .into());
        }
        return Ok(None);
    }
    let guard = IndicatorGuard {
        adapter,
        owner,
        transient,
    };
    if let Some(display) = keyboard_cue(command) {
        let event = ControlIndicatorEvent::Keys {
            owner: &guard.owner,
            display: &display,
        };
        if let Err(error) = adapter.present_control_indicator(&event)
            && adapter.requires_control_indicator()
        {
            return Err(error
                .with_disposition(DeliverySemantics::not_delivered())
                .into());
        }
    }
    Ok(Some(guard))
}

fn keyboard_cue(command: &Commands) -> Option<String> {
    match command {
        Commands::Type(_) => Some("Text input requested".into()),
        Commands::Press(args) => Some(key_display(&args.combo)),
        Commands::KeyDown(args) | Commands::KeyUp(args) => Some(key_display(&args.combo)),
        _ => None,
    }
}

fn key_display(combo: &str) -> String {
    acdesktop_core::control_key_label(combo)
}
