use std::sync::atomic::{AtomicU64, Ordering};

use crate::{AppError, CommandContext, DeliverySemantics, PlatformAdapter};

static NEXT_OWNER: AtomicU64 = AtomicU64::new(1);

pub struct ControlIndicatorGuard<'a> {
    adapter: &'a dyn PlatformAdapter,
    owner: String,
    transient: bool,
}

impl<'a> ControlIndicatorGuard<'a> {
    pub fn begin(
        adapter: &'a dyn PlatformAdapter,
        context: &CommandContext,
        description: &str,
    ) -> Result<Option<Self>, AppError> {
        let session = match context.session_id() {
            Some(id) if crate::session::read_manifest(id)?.is_some() => Some(id),
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
        if let Err(error) = adapter.show_control_indicator(&owner, !transient, description) {
            if adapter.requires_control_indicator() {
                return Err(error
                    .with_disposition(DeliverySemantics::not_delivered())
                    .into());
            }
            return Ok(None);
        }
        Ok(Some(Self {
            adapter,
            owner,
            transient,
        }))
    }

    pub fn owner(&self) -> &str {
        &self.owner
    }
}

impl Drop for ControlIndicatorGuard<'_> {
    fn drop(&mut self) {
        if self.transient {
            let _ = self.adapter.hide_control_indicator(&self.owner);
        }
    }
}
