#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdRetryDisposition {
    Unknown = 0,
    Safe = 1,
    Unsafe = 2,
}

impl From<acdesktop_core::RetryDisposition> for AdRetryDisposition {
    fn from(value: acdesktop_core::RetryDisposition) -> Self {
        match value {
            acdesktop_core::RetryDisposition::Unknown => Self::Unknown,
            acdesktop_core::RetryDisposition::Safe => Self::Safe,
            acdesktop_core::RetryDisposition::Unsafe => Self::Unsafe,
        }
    }
}
