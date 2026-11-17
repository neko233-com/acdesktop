use acdesktop_core::AdapterError;
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, WAIT_TIMEOUT};
use windows_sys::Win32::System::Threading::{
    OpenEventW, SYNCHRONIZATION_SYNCHRONIZE, WaitForSingleObject,
};

use super::{hosted_lease::event_name, wide::wide};

pub(super) struct HostedIndicatorWatch(HANDLE);

impl HostedIndicatorWatch {
    pub(super) fn open(owner: &str) -> Result<Self, AdapterError> {
        let name = event_name(owner);
        let handle = unsafe { OpenEventW(SYNCHRONIZATION_SYNCHRONIZE, 0, wide(&name).as_ptr()) };
        if handle.is_null() {
            return Err(AdapterError::internal(
                "The hosted control indicator lease is unavailable",
            ));
        }
        Ok(Self(handle))
    }

    pub(super) fn finished(&self) -> bool {
        unsafe { WaitForSingleObject(self.0, 0) != WAIT_TIMEOUT }
    }
}

impl Drop for HostedIndicatorWatch {
    fn drop(&mut self) {
        unsafe { CloseHandle(self.0) };
    }
}
