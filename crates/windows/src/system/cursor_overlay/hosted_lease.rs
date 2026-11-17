use acdesktop_core::AdapterError;
use windows_sys::Win32::Foundation::{
    CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HANDLE, SetLastError,
};
use windows_sys::Win32::System::Com::CoCreateGuid;
use windows_sys::Win32::System::Threading::{CreateEventW, SetEvent};

use super::wide::wide;

const PREFIX: &str = "hosted-";

pub struct HostedIndicatorLease {
    handle: HANDLE,
    owner: String,
}

impl HostedIndicatorLease {
    pub fn new() -> Result<Self, AdapterError> {
        let mut id = windows_sys::core::GUID::default();
        if unsafe { CoCreateGuid(&mut id) } < 0 {
            return Err(AdapterError::internal(
                "The hosted control indicator identity could not be created",
            ));
        }
        let tail: String = id.data4.iter().map(|byte| format!("{byte:02x}")).collect();
        let owner = format!(
            "{PREFIX}{:08x}{:04x}{:04x}{tail}",
            id.data1, id.data2, id.data3
        );
        let name = event_name(&owner);
        let handle = unsafe {
            SetLastError(0);
            CreateEventW(std::ptr::null(), 1, 0, wide(&name).as_ptr())
        };
        if handle.is_null() {
            return Err(AdapterError::internal(
                "The hosted control indicator lease could not be created",
            ));
        }
        if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
            unsafe { CloseHandle(handle) };
            return Err(AdapterError::internal(
                "The hosted control indicator lease already exists",
            ));
        }
        Ok(Self { handle, owner })
    }

    pub fn owner(&self) -> &str {
        &self.owner
    }
}

impl Drop for HostedIndicatorLease {
    fn drop(&mut self) {
        unsafe {
            SetEvent(self.handle);
            CloseHandle(self.handle);
        }
    }
}

pub(super) fn event_name(owner: &str) -> String {
    format!("Local\\acdesktop-control-{owner}")
}

pub(super) fn is_hosted_owner(owner: &str) -> bool {
    owner.starts_with(PREFIX)
}

#[cfg(test)]
#[path = "hosted_lease_tests.rs"]
mod tests;
