use crate::{AdapterError, DeliverySemantics, InteractionPolicy, WindowInfo};

pub(super) fn refusal(window: &WindowInfo, policy: InteractionPolicy) -> AdapterError {
    AdapterError::stale_ref_because(
        "The named window's exact focus could not be confirmed, so no key was sent",
    )
    .with_suggestion(if policy.is_headed() {
        "Ensure the named window remains active before retrying; a sibling window or another application's menu can hold the keyboard"
    } else {
        "Retry with --headed so the named window is brought forward first, or name the active window"
    })
    .with_details(serde_json::json!({
        "check": "window_active",
        "cause": "exact_window_focus_unconfirmed",
        "physical_delivery_started": false,
        "window_id": window.id,
    }))
    .with_disposition(DeliverySemantics::not_delivered())
}
