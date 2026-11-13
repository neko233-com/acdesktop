use crate::cli::Commands;

#[cfg(not(test))]
pub(super) fn begin(
    cmd: &Commands,
) -> Result<Option<acdesktop_windows::ControlNotice>, acdesktop_core::AppError> {
    if needs_notice(cmd) {
        let notice = acdesktop_windows::ControlNotice::start().map_err(|error| {
            error.with_disposition(acdesktop_core::DeliverySemantics::not_delivered())
        })?;
        Ok(Some(notice))
    } else {
        Ok(None)
    }
}

fn needs_notice(cmd: &Commands) -> bool {
    matches!(
        cmd,
        Commands::Click(_)
            | Commands::DoubleClick(_)
            | Commands::TripleClick(_)
            | Commands::RightClick(_)
            | Commands::Type(_)
            | Commands::SetValue(_)
            | Commands::Clear(_)
            | Commands::Focus(_)
            | Commands::Select(_)
            | Commands::Toggle(_)
            | Commands::Check(_)
            | Commands::Uncheck(_)
            | Commands::Expand(_)
            | Commands::Collapse(_)
            | Commands::Scroll(_)
            | Commands::ScrollTo(_)
            | Commands::Press(_)
            | Commands::KeyDown(_)
            | Commands::KeyUp(_)
            | Commands::Hover(_)
            | Commands::Drag(_)
            | Commands::MouseMove(_)
            | Commands::MouseClick(_)
            | Commands::MouseDown(_)
            | Commands::MouseUp(_)
            | Commands::MouseWheel(_)
            | Commands::Launch(_)
            | Commands::CloseApp(_)
            | Commands::FocusWindow(_)
            | Commands::ResizeWindow(_)
            | Commands::MoveWindow(_)
            | Commands::Minimize(_)
            | Commands::Maximize(_)
            | Commands::Restore(_)
            | Commands::OpenSystemSurface(_)
            | Commands::DismissNotification(_)
            | Commands::DismissAllNotifications(_)
            | Commands::NotificationAction(_)
    )
}

#[cfg(test)]
#[path = "control_notice_tests.rs"]
mod tests;
