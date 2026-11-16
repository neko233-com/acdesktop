use acdesktop_core::{
    Action, AdapterError, ControlIndicatorEvent, DeliveryDisposition, DeliverySemantics,
    PlatformAdapter, Point,
};

use crate::AdAdapter;

#[cfg(target_os = "windows")]
type TransientLease = acdesktop_windows::HostedIndicatorLease;
#[cfg(not(target_os = "windows"))]
type TransientLease = ();

pub(crate) struct IndicatorGuard<'a> {
    adapter: &'a dyn PlatformAdapter,
    owner: String,
    lease: Option<TransientLease>,
}

impl IndicatorGuard<'_> {
    pub(crate) fn owner(&self) -> &str {
        &self.owner
    }

    pub(crate) fn click(&self, point: &Point, disposition: DeliverySemantics) {
        if matches!(
            disposition.delivery(),
            DeliveryDisposition::DeliveredUnverified | DeliveryDisposition::DeliveredVerified
        ) {
            let event = ControlIndicatorEvent::Click {
                owner: &self.owner,
                point,
            };
            if let Err(error) = self.adapter.present_control_indicator(&event) {
                tracing::warn!(code = %error.code.as_str(), "control click effect was skipped");
            }
        }
    }

    pub(crate) fn click_on_element(
        &self,
        handle: &acdesktop_core::NativeHandle,
        deadline: acdesktop_core::Deadline,
        disposition: DeliverySemantics,
    ) {
        if !matches!(
            disposition.delivery(),
            DeliveryDisposition::DeliveredUnverified | DeliveryDisposition::DeliveredVerified
        ) {
            return;
        }
        let deadline = deadline.capped(std::time::Duration::from_millis(100));
        if let Ok(Some(bounds)) = self.adapter.get_element_bounds(handle, deadline)
            && bounds.validate().is_ok()
            && bounds.width > 0.0
            && bounds.height > 0.0
        {
            let point = Point {
                x: bounds.x + bounds.width / 2.0,
                y: bounds.y + bounds.height / 2.0,
            };
            self.click(&point, disposition);
        }
    }
}

impl Drop for IndicatorGuard<'_> {
    fn drop(&mut self) {
        if self.lease.is_some() {
            let _ = self.adapter.hide_control_indicator(&self.owner);
        }
    }
}

pub(crate) fn begin<'a>(
    adapter: &'a AdAdapter,
    description: &str,
    action: Option<&Action>,
) -> Result<Option<IndicatorGuard<'a>>, AdapterError> {
    if !adapter.inner.requires_control_indicator() {
        return Ok(None);
    }
    let session = match adapter.session_id.as_deref() {
        Some(id) => acdesktop_core::session::read_manifest(id)
            .map_err(crate::commands::app_error_to_adapter)?
            .filter(|manifest| manifest.ended_at.is_none())
            .map(|_| id),
        None => None,
    };
    let (owner, lease) = match session {
        Some(id) => (id.to_owned(), None),
        None => {
            let (owner, lease) = transient_owner()?;
            (owner, Some(lease))
        }
    };
    adapter
        .inner
        .show_control_indicator(&owner, session.is_some(), description)
        .map_err(|error| error.with_disposition(DeliverySemantics::not_delivered()))?;
    let guard = IndicatorGuard {
        adapter: adapter.inner.as_ref(),
        owner,
        lease,
    };
    if let Some(display) = action.and_then(keyboard_cue) {
        guard
            .adapter
            .present_control_indicator(&ControlIndicatorEvent::Keys {
                owner: &guard.owner,
                display: &display,
            })
            .map_err(|error| error.with_disposition(DeliverySemantics::not_delivered()))?;
    }
    Ok(Some(guard))
}

#[cfg(target_os = "windows")]
fn transient_owner() -> Result<(String, TransientLease), AdapterError> {
    let lease = acdesktop_windows::HostedIndicatorLease::new()?;
    Ok((lease.owner().to_owned(), lease))
}

#[cfg(not(target_os = "windows"))]
fn transient_owner() -> Result<(String, TransientLease), AdapterError> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(1);
    Ok((
        format!(
            "hosted-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ),
        (),
    ))
}

fn keyboard_cue(action: &Action) -> Option<String> {
    match action {
        Action::TypeText(_) => Some("Text input requested".into()),
        Action::PressKey(combo) | Action::KeyDown(combo) | Action::KeyUp(combo) => {
            let mut parts: Vec<String> = combo
                .modifiers
                .iter()
                .map(|modifier| format!("{modifier:?}"))
                .collect();
            parts.push(combo.key.clone());
            Some(acdesktop_core::control_key_label(&parts.join("+")))
        }
        _ => None,
    }
}

macro_rules! begin_control {
    ($adapter:expr, $description:expr, $action:expr) => {{
        match $crate::indicator::begin(&$adapter, $description, $action) {
            Ok(guard) => guard,
            Err(error) => {
                $crate::error::set_last_error(&error);
                return $crate::error::last_error_code();
            }
        }
    }};
}

pub(crate) use begin_control;

#[cfg(test)]
#[path = "indicator_tests.rs"]
mod tests;
