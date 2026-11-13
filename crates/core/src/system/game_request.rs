use std::path::Path;

use crate::{InteractionLease, WindowInfo};

use super::game_plan::GameSessionPlan;

/// Everything a platform needs to play one [`GameSessionPlan`] against a
/// window: the window to observe, the timeline to inject, and the lease that
/// authorizes the physical input.
///
/// When `frames_dir` is set the adapter writes each sampled frame there as a
/// PNG instead of keeping frames in memory, so a long session stays flat.
pub struct GameSessionRequest<'a> {
    pub window: &'a WindowInfo,
    pub plan: &'a GameSessionPlan,
    pub lease: &'a InteractionLease,
    pub frames_dir: Option<&'a Path>,
}
