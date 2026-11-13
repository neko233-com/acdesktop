use super::{changed_ratio, grid_step};

#[test]
fn an_identical_frame_changes_nothing() {
    let frame = vec![0u8; 4 * 16 * 16];
    assert_eq!(changed_ratio(Some(&frame), &frame, 16, 16), 0.0);
}

#[test]
fn the_first_frame_is_the_baseline() {
    let frame = vec![255u8; 4 * 16 * 16];
    assert_eq!(changed_ratio(None, &frame, 16, 16), 0.0);
}

#[test]
fn a_different_frame_is_fully_changed() {
    let previous = vec![0u8; 4 * 16 * 16];
    let current = vec![255u8; 4 * 16 * 16];
    assert_eq!(changed_ratio(Some(&previous), &current, 16, 16), 1.0);
}

#[test]
fn a_small_change_reports_a_partial_ratio() {
    let previous = vec![0u8; 4 * 64 * 64];
    let mut current = previous.clone();
    current[0] = 255;
    let ratio = changed_ratio(Some(&previous), &current, 64, 64);
    assert!(ratio > 0.0);
    assert!(ratio < 0.05);
}

#[test]
fn a_mismatched_buffer_is_ignored() {
    assert_eq!(changed_ratio(Some(&[0; 4]), &[0; 8], 1, 1), 0.0);
    assert_eq!(changed_ratio(Some(&[0; 4]), &[0; 4], 2, 2), 0.0);
}

#[test]
fn the_grid_step_keeps_the_sample_count_small() {
    assert_eq!(grid_step(16, 16), 1);
    assert!(grid_step(1920, 1080) > 1);
}
