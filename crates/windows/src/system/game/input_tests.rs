use std::time::Duration;

use acdesktop_core::{Deadline, TouchContact, TouchPhase};

use super::InputGuard;
use crate::input::touch::touch_send_fake_sink;

fn deadline() -> Deadline {
    Deadline::from_duration(Duration::from_millis(500)).expect("a test deadline")
}

fn contact(id: u32, x: i32, y: i32, phase: TouchPhase) -> TouchContact {
    TouchContact { id, x, y, phase }
}

#[test]
fn an_unseen_move_becomes_a_press() {
    touch_send_fake_sink::reset();
    let mut guard = InputGuard::new((0, 0));
    guard
        .send_touch(&[contact(5, 10, 10, TouchPhase::Move)], deadline())
        .expect("a touch frame reaches the recording sink");
    let recorded = touch_send_fake_sink::recorded();
    assert_eq!(recorded.len(), 1);
    assert!(matches!(recorded[0].phase, TouchPhase::Down));
    assert_eq!(recorded[0].id, 5);
}

#[test]
fn a_move_on_a_held_contact_stays_a_move() {
    touch_send_fake_sink::reset();
    let mut guard = InputGuard::new((0, 0));
    guard
        .send_touch(&[contact(5, 10, 10, TouchPhase::Down)], deadline())
        .expect("a touch down reaches the recording sink");
    guard
        .send_touch(&[contact(5, 12, 10, TouchPhase::Move)], deadline())
        .expect("a touch move reaches the recording sink");
    let recorded = touch_send_fake_sink::recorded();
    assert_eq!(recorded.len(), 2);
    assert!(matches!(recorded[0].phase, TouchPhase::Down));
    assert!(matches!(recorded[1].phase, TouchPhase::Move));
}

#[test]
fn a_point_is_translated_by_the_client_origin() {
    touch_send_fake_sink::reset();
    let mut guard = InputGuard::new((100, 50));
    guard
        .send_touch(&[contact(1, 10, 20, TouchPhase::Move)], deadline())
        .expect("a touch frame reaches the recording sink");
    let recorded = touch_send_fake_sink::recorded();
    assert_eq!(recorded[0].x, 110);
    assert_eq!(recorded[0].y, 70);
}

#[test]
fn a_released_contact_is_pressed_again() {
    touch_send_fake_sink::reset();
    let mut guard = InputGuard::new((0, 0));
    guard
        .send_touch(&[contact(2, 10, 10, TouchPhase::Down)], deadline())
        .expect("a touch down reaches the recording sink");
    guard
        .send_touch(&[contact(2, 10, 10, TouchPhase::Up)], deadline())
        .expect("a touch up reaches the recording sink");
    guard
        .send_touch(&[contact(2, 10, 10, TouchPhase::Move)], deadline())
        .expect("a touch move reaches the recording sink");
    let recorded = touch_send_fake_sink::recorded();
    assert!(matches!(recorded[2].phase, TouchPhase::Down));
}
