#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdIdentifierKind {
    AxIdentifier = 0,
    AxDomIdentifier = 1,
    AutomationId = 2,
    RuntimeId = 3,
    AtspiObjectPath = 4,
}

impl AdIdentifierKind {
    pub(crate) fn from_c(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::AxIdentifier),
            1 => Some(Self::AxDomIdentifier),
            2 => Some(Self::AutomationId),
            3 => Some(Self::RuntimeId),
            4 => Some(Self::AtspiObjectPath),
            _ => None,
        }
    }

    pub(crate) fn to_core(self) -> acdesktop_core::IdentifierKind {
        match self {
            Self::AxIdentifier => acdesktop_core::IdentifierKind::AxIdentifier,
            Self::AxDomIdentifier => acdesktop_core::IdentifierKind::AxDomIdentifier,
            Self::AutomationId => acdesktop_core::IdentifierKind::AutomationId,
            Self::RuntimeId => acdesktop_core::IdentifierKind::RuntimeId,
            Self::AtspiObjectPath => acdesktop_core::IdentifierKind::AtspiObjectPath,
        }
    }
}
