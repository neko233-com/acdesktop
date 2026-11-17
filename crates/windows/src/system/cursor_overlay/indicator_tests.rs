use super::*;

#[test]
fn missing_or_ended_session_stops_the_indicator_but_read_errors_do_not() {
    assert!(classify_session(Err(std::io::ErrorKind::NotFound.into())));
    assert!(classify_session(Ok(br#"{"ended_at":42}"#.to_vec())));
    assert!(!classify_session(Ok(br#"{"ended_at":null}"#.to_vec())));
    assert!(!classify_session(Ok(b"invalid json".to_vec())));
    assert!(!classify_session(Err(
        std::io::ErrorKind::PermissionDenied.into()
    )));
}
