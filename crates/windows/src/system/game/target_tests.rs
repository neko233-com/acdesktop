use super::inside;

#[test]
fn input_points_must_be_inside_the_client_rectangle() {
    assert!(inside(640, 480, 0, 0));
    assert!(inside(640, 480, 639, 479));
    assert!(!inside(640, 480, 640, 0));
    assert!(!inside(640, 480, -1, 0));
    assert!(!inside(640, 480, 0, 480));
}
