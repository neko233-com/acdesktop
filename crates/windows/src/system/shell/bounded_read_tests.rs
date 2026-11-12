use super::*;
use std::time::{Duration, Instant};

#[test]
fn returns_fast_results_and_preserves_errors() {
    let limiter = ReadLimiter::default();
    assert_eq!(
        limiter
            .run(Deadline::after(1_000).unwrap(), || Ok(42))
            .unwrap(),
        42
    );
    let error = limiter
        .run::<()>(Deadline::after(1_000).unwrap(), || {
            Err(AdapterError::internal("fixture"))
        })
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::Internal);
}

#[test]
fn a_stalled_read_obeys_the_deadline_and_is_counted_until_it_finishes() {
    let limiter = ReadLimiter::default();
    let (release_tx, release_rx) = mpsc::channel::<()>();
    let started = Instant::now();
    let error = limiter
        .run(Deadline::after(80).unwrap(), move || {
            let _ = release_rx.recv();
            Ok(())
        })
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::Timeout);
    assert!(started.elapsed() < Duration::from_secs(1));
    assert_eq!(limiter.active.load(Ordering::Acquire), 1);
    release_tx.send(()).unwrap();
    let until = Instant::now() + Duration::from_secs(2);
    while limiter.active.load(Ordering::Acquire) != 0 && Instant::now() < until {
        std::thread::yield_now();
    }
    assert_eq!(limiter.active.load(Ordering::Acquire), 0);
}

#[test]
fn abandoned_reads_cannot_spawn_unbounded_workers() {
    let limiter = ReadLimiter::default();
    let mut releases = Vec::new();
    for _ in 0..MAX_ACTIVE_READS {
        let (send, receive) = mpsc::channel::<()>();
        releases.push(send);
        let error = limiter
            .run(Deadline::after(30).unwrap(), move || {
                let _ = receive.recv();
                Ok(())
            })
            .unwrap_err();
        assert_eq!(error.code, ErrorCode::Timeout);
    }
    let error = limiter
        .run::<()>(Deadline::after(1_000).unwrap(), || Ok(()))
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::AppUnresponsive);
    for release in releases {
        release.send(()).unwrap();
    }
}
