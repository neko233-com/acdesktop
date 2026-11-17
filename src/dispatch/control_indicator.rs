use acdesktop_core::{
    AppError, CommandContext, ControlIndicatorEvent, ControlIndicatorGuard, DeliverySemantics,
    PlatformAdapter,
};

use crate::cli::Commands;

#[cfg(test)]
#[path = "control_indicator_tests.rs"]
mod tests;

pub(super) fn begin<'a>(
    command: &Commands,
    adapter: &'a dyn PlatformAdapter,
    context: &CommandContext,
) -> Result<Option<ControlIndicatorGuard<'a>>, AppError> {
    if !command.is_mutating()
        || matches!(
            command,
            Commands::Session(_)
                | Commands::CursorOverlay(_)
                | Commands::Batch(_)
                | Commands::Screenshot(_)
                | Commands::Game(_)
        )
    {
        return Ok(None);
    }
    let description = context.description().unwrap_or_else(|| command.name());
    let Some(guard) = ControlIndicatorGuard::begin(adapter, context, description)? else {
        return Ok(None);
    };
    if let Some(display) = keyboard_cue(command) {
        let event = ControlIndicatorEvent::Keys {
            owner: guard.owner(),
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
