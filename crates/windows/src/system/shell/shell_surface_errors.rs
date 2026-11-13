use acdesktop_core::{AdapterError, DeliverySemantics};

use super::shell_surface::{SurfaceKindRow, kebab};

pub(super) fn timeout_error(
    row: &SurfaceKindRow,
    outcome: &str,
    kind: &'static str,
) -> AdapterError {
    AdapterError::timeout(format!("the '{}' shell surface {outcome}", kebab(row.kind)))
        .with_suggestion(
            "Retry: the shell can decline an accelerator or an invoke without reporting it; \
         a build that lacks the surface refuses instead of timing out",
        )
        .with_details(serde_json::json!({ "kind": kind, "retryable": true }))
        .with_disposition(DeliverySemantics::not_delivered())
}
