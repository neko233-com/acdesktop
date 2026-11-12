pub(crate) struct ChainContext<'a> {
    pub(crate) dynamic_value: Option<&'a str>,
    pub(crate) verified_point: Option<&'a acdesktop_core::Point>,
    pub(crate) deadline: acdesktop_core::Deadline,
}

impl ChainContext<'_> {
    pub(crate) fn remaining(
        &self,
    ) -> Result<std::time::Duration, acdesktop_core::AdapterError> {
        let remaining = self.deadline.remaining();
        if remaining.is_zero() {
            Err(self.deadline.timeout_error())
        } else {
            Ok(remaining)
        }
    }

    pub(crate) fn ensure_budget(&self) -> Result<(), acdesktop_core::AdapterError> {
        self.remaining().map(|_| ())
    }
}
