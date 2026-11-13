#[cfg(target_os = "windows")]
mod bounded_read;
#[cfg(all(test, target_os = "windows"))]
pub(crate) mod raise_oracle;
pub(crate) mod shell_surface;
mod shell_surface_errors;
#[cfg(target_os = "windows")]
pub(crate) mod shell_surface_immersive;
pub(crate) mod shell_surface_kinds;
pub(crate) mod shell_surface_open;
mod shell_surface_poll;
pub(crate) mod shell_surface_raise;
