use acdesktop_core::{AppError, PlatformAdapter, commands};
use serde_json::Value;

use crate::cli_args::game::GameArgs;

pub(crate) fn run(
    args: GameArgs,
    adapter: &dyn PlatformAdapter,
    context: &acdesktop_core::context::CommandContext,
) -> Result<Value, AppError> {
    commands::game::execute(args.into_core(), adapter, context)
}
