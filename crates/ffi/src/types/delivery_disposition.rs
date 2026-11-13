#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdDeliveryDisposition {
    Unknown = 0,
    NotDelivered = 1,
    DeliveryUncertain = 2,
    DeliveredUnverified = 3,
    DeliveredVerified = 4,
}

impl From<acdesktop_core::DeliveryDisposition> for AdDeliveryDisposition {
    fn from(value: acdesktop_core::DeliveryDisposition) -> Self {
        match value {
            acdesktop_core::DeliveryDisposition::Unknown => Self::Unknown,
            acdesktop_core::DeliveryDisposition::NotDelivered => Self::NotDelivered,
            acdesktop_core::DeliveryDisposition::DeliveryUncertain => Self::DeliveryUncertain,
            acdesktop_core::DeliveryDisposition::DeliveredUnverified => Self::DeliveredUnverified,
            acdesktop_core::DeliveryDisposition::DeliveredVerified => Self::DeliveredVerified,
        }
    }
}
