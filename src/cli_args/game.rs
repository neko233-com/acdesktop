use std::path::PathBuf;

use clap::Parser;
use serde::Deserialize;

/// Run a game session: capture the target window, play a scheduled input
/// plan, and report capture timing and visual changes.
///
/// The plan is JSON; see the Windows platform skill for its shape. Inputs are injected
/// as a timeline (`at_ms` offsets), so a session measures how the game reacts
/// rather than racing a fixed sleep between commands.
#[derive(Parser, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct GameArgs {
    /// App name to drive. Ignored when --window-id is given.
    #[arg(long)]
    pub(crate) app: Option<String>,
    /// Exact window id (`w-<handle>`) to drive.
    #[arg(long)]
    pub(crate) window_id: Option<String>,
    /// Path to the session plan JSON.
    #[arg(long)]
    pub(crate) plan: PathBuf,
    /// Directory that receives `report.json` (and `frames/` with --save-frames).
    #[arg(long)]
    pub(crate) out: PathBuf,
    /// Override the requested duration.
    #[arg(long)]
    pub(crate) duration_ms: Option<u64>,
    /// Wall-clock budget for the whole session; defaults to the requested
    /// duration plus setup slack.
    #[arg(long)]
    pub(crate) timeout_ms: Option<u64>,
    /// Write every sampled frame as a PNG under `out/frames`.
    #[arg(long, default_value_t = false)]
    pub(crate) save_frames: bool,
}

impl GameArgs {
    pub(crate) fn into_core(self) -> acdesktop_core::commands::game::GameArgs {
        acdesktop_core::commands::game::GameArgs {
            app: self.app,
            window_id: self.window_id,
            plan_path: self.plan,
            out_dir: self.out,
            duration_ms: self.duration_ms,
            timeout_ms: self.timeout_ms,
            save_frames: self.save_frames,
        }
    }
}
