use acdesktop_core::AdapterError;

pub(crate) fn headed_requirement(
    request: &acdesktop_core::ActionRequest,
    classify: impl FnOnce() -> Result<bool, AdapterError>,
) -> Result<acdesktop_core::HeadedRequirement, AdapterError> {
    use acdesktop_core::{Action, HeadedRequirement};
    if request.policy.is_headed()
        && matches!(
            request.action,
            Action::Click | Action::RightClick | Action::DoubleClick | Action::TripleClick
        )
        && classify()?
    {
        return Ok(if matches!(request.action, Action::Click) {
            HeadedRequirement::FocusedApplication
        } else {
            HeadedRequirement::FocusedApplicationAndCursor
        });
    }
    Ok(request.headed_requirement())
}

#[cfg(test)]
#[path = "native_menu_tests.rs"]
mod tests;
