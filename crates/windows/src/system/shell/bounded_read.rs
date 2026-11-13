use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
    mpsc,
};

use acdesktop_core::{AdapterError, Deadline, DeliverySemantics, ErrorCode};

const MAX_ACTIVE_READS: usize = 4;

#[derive(Clone, Default)]
pub(super) struct ReadLimiter {
    active: Arc<AtomicUsize>,
}

struct Ticket(Arc<AtomicUsize>);

impl Drop for Ticket {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}

impl ReadLimiter {
    pub(super) fn run<T: Send + 'static>(
        &self,
        deadline: Deadline,
        read: impl FnOnce() -> Result<T, AdapterError> + Send + 'static,
    ) -> Result<T, AdapterError> {
        deadline.ensure_remaining()?;
        self.active
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |active| {
                (active < MAX_ACTIVE_READS).then_some(active + 1)
            })
            .map_err(|_| {
                AdapterError::new(
                    ErrorCode::AppUnresponsive,
                    "Too many shell accessibility reads are still outstanding",
                )
                .with_disposition(DeliverySemantics::not_delivered())
            })?;
        let ticket = Ticket(Arc::clone(&self.active));
        let (sender, receiver) = mpsc::sync_channel(1);
        std::thread::Builder::new()
            .name("acdesktop-shell-read".into())
            .spawn(move || {
                let _ticket = ticket;
                let result = deadline.ensure_remaining().and_then(|()| read());
                let _ = sender.send(result);
            })
            .map_err(|error| {
                AdapterError::internal(format!("Cannot start shell accessibility read: {error}"))
                    .with_disposition(DeliverySemantics::not_delivered())
            })?;
        match receiver.recv_timeout(deadline.remaining()) {
            Ok(result) => {
                deadline.ensure_remaining()?;
                result
            }
            Err(mpsc::RecvTimeoutError::Timeout) => Err(deadline
                .timeout_error()
                .with_disposition(DeliverySemantics::not_delivered())),
            Err(mpsc::RecvTimeoutError::Disconnected) => Err(AdapterError::internal(
                "Shell accessibility read worker exited without a result",
            )
            .with_disposition(DeliverySemantics::not_delivered())),
        }
    }
}

pub(super) fn read<T: Send + 'static>(
    deadline: Deadline,
    operation: impl FnOnce() -> Result<T, AdapterError> + Send + 'static,
) -> Result<T, AdapterError> {
    static LIMITER: std::sync::OnceLock<ReadLimiter> = std::sync::OnceLock::new();
    LIMITER
        .get_or_init(ReadLimiter::default)
        .run(deadline, move || {
            crate::system::runtime::com_runtime::ensure_hosted_library_mta_and_dpi()?;
            deadline.ensure_remaining()?;
            operation()
        })
}

#[cfg(test)]
#[path = "bounded_read_tests.rs"]
mod tests;
