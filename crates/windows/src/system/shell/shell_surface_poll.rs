use acdesktop_core::{AdapterError, Deadline, WindowInfo};

use super::shell_surface::{SurfaceFamily, SurfaceKindRow};
use super::shell_surface_errors::timeout_error;
use super::shell_surface_open::resolved_surface_presented;

/// The open is never reported from the fact that the accelerator was sent or
/// the chevron invoked - the shell can simply decline - so every open ends in
/// an observed, presented resolve or a `TIMEOUT`, polled at a bounded
/// interval that grows the way the launch observer's does rather than
/// sleeping a fixed guess. The resolve alone is not enough: it matches the
/// overflow's window while the surface is still hidden, so the presented
/// read rides alongside it. A raise that presents a same-class, same-host
/// surface matching none of the kind's landmarks resolves as the named
/// foreign-shape refusal instead of burning the deadline - the raise
/// presented something, and it is not the measured shape.
pub(super) fn poll_until_observed(
    row: &SurfaceKindRow,
    deadline: Deadline,
    pre_raise_children: &[isize],
) -> Result<WindowInfo, AdapterError> {
    let mut interval = std::time::Duration::from_millis(50);
    let mut foreign_since: Option<std::time::Instant> = None;
    loop {
        if deadline.remaining().is_zero() {
            return Err(timeout_error(
                row,
                "did not open within the deadline",
                "shell_surface_not_opened",
            ));
        }
        deadline.ensure_remaining()?;
        let (observed, foreign) = match &row.family {
            SurfaceFamily::Immersive {
                expected_class,
                host_images,
                landmarks,
            } => super::shell_surface_immersive::resolve_after_raise(
                expected_class,
                host_images,
                landmarks,
                pre_raise_children.to_vec(),
                deadline,
            )?,
            SurfaceFamily::Win32Class(_) => {
                (super::shell_surface::resolve_row(row, deadline)?, false)
            }
        };
        if let Some(observed) = observed
            && resolved_surface_presented(row, deadline)?
        {
            return Ok(observed);
        }
        if let SurfaceFamily::Immersive { landmarks, .. } = &row.family
            && foreign_shape_settled(&mut foreign_since, foreign, std::time::Instant::now())
        {
            return Err(super::shell_surface_immersive::foreign_shape_error(
                landmarks,
            ));
        }
        let remaining = deadline.remaining();
        std::thread::sleep(interval.min(remaining));
        interval = (interval * 3 / 2).min(std::time::Duration::from_millis(250));
    }
}

/// How long a raise-presented window must stay landmark-free before it is
/// called a foreign shape. A surface that is still loading uncloaks before
/// its tree carries a landmark, and a toast arriving during the raise is a
/// landmark-free window that is not the surface, so a single poll's answer
/// would refuse an open that one more poll resolves.
const FOREIGN_SHAPE_SETTLE: std::time::Duration = std::time::Duration::from_millis(500);

fn foreign_shape_settled(
    since: &mut Option<std::time::Instant>,
    foreign_now: bool,
    now: std::time::Instant,
) -> bool {
    if !foreign_now {
        *since = None;
        return false;
    }
    let first = *since.get_or_insert(now);
    now.duration_since(first) >= FOREIGN_SHAPE_SETTLE
}

#[cfg(test)]
#[path = "shell_surface_settle_tests.rs"]
mod settle_tests;
