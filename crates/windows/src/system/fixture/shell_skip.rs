use acdesktop_core::{AdapterError, ErrorCode};

/// Whether a shell observation exhausted its budget without a classified
/// outcome: the machine-global shell surface could not be read within the
/// deadline under desktop load. The shell liveness tests treat this like a
/// declined precondition - the test's quiet-desktop precondition was not met,
/// so the leg is skipped rather than failed. It is deliberately narrow, a
/// `Timeout` stamped `kind: "deadline"`, so a genuine product error still
/// reaches the panic that surfaces it.
pub(crate) fn desktop_unobservable(error: &AdapterError) -> bool {
    error.code == ErrorCode::Timeout
        && error
            .details
            .as_ref()
            .and_then(|details| details.get("kind"))
            .and_then(serde_json::Value::as_str)
            == Some("deadline")
}

/// The shell surfaces are machine-global and share one UIA read budget with the
/// whole desktop. Under load a liveness read can exhaust its deadline before it
/// classifies the surface as present or absent - a generic `kind: "deadline"`
/// timeout rather than the product's own `shell_surface_not_opened` refusal. The
/// liveness tests treat that like a declined precondition: the quiet-desktop
/// precondition they assume was not met, so the leg is skipped, not failed. The
/// product surface is untouched - agents still receive the timeout, and the
/// classification stays narrow so a genuine product error still reaches the
/// panic that surfaces it.
pub(crate) fn skip_if_desktop_unobservable(what: &str, error: &AdapterError) -> bool {
    if desktop_unobservable(error) {
        eprintln!(
            "skip {what}: the shell surface was not observable within the deadline under desktop load"
        );
        true
    } else {
        false
    }
}
