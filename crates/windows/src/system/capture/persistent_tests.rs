use super::frame_from_abi;
use windows::core::HRESULT;

#[test]
fn an_empty_frame_pool_is_distinct_from_a_failed_pool() {
    let empty = frame_from_abi(HRESULT(0), std::ptr::null_mut()).expect("successful null frame");
    assert!(empty.is_none());
    let failed = frame_from_abi(HRESULT(0x80004005u32 as i32), std::ptr::null_mut());
    assert!(failed.is_err());
}
