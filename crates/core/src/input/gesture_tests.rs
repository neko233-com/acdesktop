use super::*;

fn spec(name: GestureName) -> GestureSpec {
    GestureSpec {
        name,
        x: 100,
        y: 200,
        duration_ms: 160,
        distance: 120,
    }
}

#[test]
fn a_tap_presses_and_lifts_one_contact() {
    let frames = gesture_frames(&spec(GestureName::Tap), 0);
    assert_eq!(frames.len(), 2);
    assert_eq!(frames[0].contacts.len(), 1);
    assert_eq!(frames[0].contacts[0].phase, TouchPhase::Down);
    assert_eq!(frames[1].contacts[0].phase, TouchPhase::Up);
    assert!(frames[1].at_ms > frames[0].at_ms);
}

#[test]
fn a_double_tap_has_two_presses() {
    let frames = gesture_frames(&spec(GestureName::DoubleTap), 0);
    let presses = frames
        .iter()
        .filter(|frame| frame.contacts[0].phase == TouchPhase::Down)
        .count();
    assert_eq!(presses, 2);
}

#[test]
fn a_pinch_out_separates_two_contacts() {
    let frames = gesture_frames(&spec(GestureName::PinchOut), 0);
    let first = &frames[0].contacts;
    let last = &frames[frames.len() - 1].contacts;
    assert_eq!(first.len(), 2);
    assert_eq!(last.len(), 2);
    let first_span = (first[1].x - first[0].x).abs();
    let last_span = (last[1].x - last[0].x).abs();
    assert!(last_span > first_span);
    assert_eq!(last[0].phase, TouchPhase::Up);
}

#[test]
fn a_three_finger_swipe_moves_three_contacts() {
    let frames = gesture_frames(&spec(GestureName::ThreeFingerSwipeLeft), 0);
    assert_eq!(frames[0].contacts.len(), 3);
    let last = &frames[frames.len() - 1];
    assert_eq!(last.contacts.len(), 3);
    assert_eq!(last.contacts[0].phase, TouchPhase::Up);
    assert!(last.contacts[0].x < frames[0].contacts[0].x);
}

#[test]
fn a_long_press_holds_the_contact() {
    let frames = gesture_frames(&spec(GestureName::LongPress), 0);
    assert!(frames.len() > 2);
    let last = &frames[frames.len() - 1];
    assert_eq!(last.contacts[0].phase, TouchPhase::Up);
    assert!(last.at_ms >= LONG_PRESS_MS);
}

#[test]
fn a_rotation_keeps_both_contacts_opposed() {
    let frames = gesture_frames(&spec(GestureName::RotateCw), 0);
    let mid = &frames[frames.len() / 2];
    assert_eq!(mid.contacts.len(), 2);
    assert_eq!(mid.contacts[0].id, 0);
    assert_eq!(mid.contacts[1].id, 1);
}

#[test]
fn a_moving_gesture_lifts_without_changing_position() {
    for name in [
        GestureName::SwipeLeft,
        GestureName::SwipeDown,
        GestureName::PinchOut,
        GestureName::RotateCw,
        GestureName::ThreeFingerSwipeRight,
        GestureName::DrawS,
        GestureName::CircleCw,
        GestureName::CircleCcw,
    ] {
        let frames = gesture_frames(&spec(name), 0);
        assert!(frames.len() >= 2);
        let lifted = &frames[frames.len() - 1];
        let settled = &frames[frames.len() - 2];
        for contact in &lifted.contacts {
            assert_eq!(contact.phase, TouchPhase::Up);
            let before = settled
                .contacts
                .iter()
                .find(|held| held.id == contact.id)
                .expect("a contact that lifts also moved in the previous frame");
            assert_eq!(
                (contact.x, contact.y),
                (before.x, before.y),
                "{name:?} lifted a contact while moving it"
            );
        }
    }
}

#[test]
fn s_and_circle_follow_curves_and_release() {
    let s = gesture_frames(&spec(GestureName::DrawS), 0);
    assert!(s.iter().any(|f| f.contacts[0].x < 100));
    assert!(s.iter().any(|f| f.contacts[0].x > 100));
    assert!(s.first().unwrap().contacts[0].y < s.last().unwrap().contacts[0].y);
    for name in [GestureName::CircleCw, GestureName::CircleCcw] {
        let circle = gesture_frames(&spec(name), 0);
        let first = &circle.first().unwrap().contacts[0];
        let last = &circle.last().unwrap().contacts[0];
        assert_eq!((first.x, first.y), (last.x, last.y));
        assert_eq!(last.phase, TouchPhase::Up);
        assert!(circle.iter().any(|f| f.contacts[0].x < 100));
    }
}

#[test]
fn every_gesture_has_ordered_complete_contact_lifecycles() {
    use std::collections::{BTreeMap, BTreeSet};
    for name in [
        GestureName::Tap,
        GestureName::DoubleTap,
        GestureName::LongPress,
        GestureName::SwipeLeft,
        GestureName::SwipeRight,
        GestureName::SwipeUp,
        GestureName::SwipeDown,
        GestureName::PinchIn,
        GestureName::PinchOut,
        GestureName::RotateCw,
        GestureName::RotateCcw,
        GestureName::TwoFingerTap,
        GestureName::DrawS,
        GestureName::CircleCw,
        GestureName::CircleCcw,
        GestureName::ThreeFingerSwipeLeft,
        GestureName::ThreeFingerSwipeRight,
        GestureName::ThreeFingerSwipeUp,
        GestureName::ThreeFingerSwipeDown,
    ] {
        let frames = gesture_frames(&spec(name), 17);
        assert_eq!(frames[0].at_ms, 17, "{name:?}");
        assert!(
            frames.windows(2).all(|pair| pair[0].at_ms < pair[1].at_ms),
            "{name:?}"
        );
        let mut held = BTreeMap::new();
        for frame in &frames {
            let ids: BTreeSet<_> = frame.contacts.iter().map(|contact| contact.id).collect();
            assert_eq!(ids.len(), frame.contacts.len(), "{name:?}");
            assert!(held.keys().all(|id| ids.contains(id)), "{name:?}");
            for contact in &frame.contacts {
                let position = (contact.x, contact.y);
                match contact.phase {
                    TouchPhase::Down => {
                        assert!(held.insert(contact.id, position).is_none(), "{name:?}")
                    }
                    TouchPhase::Move => {
                        assert!(held.insert(contact.id, position).is_some(), "{name:?}")
                    }
                    TouchPhase::Up => {
                        assert_eq!(held.remove(&contact.id), Some(position), "{name:?}")
                    }
                }
            }
        }
        assert!(held.is_empty(), "{name:?} left a contact held");
    }
}

#[test]
fn directional_gestures_move_in_the_named_screen_direction() {
    for (name, dx, dy) in [
        (GestureName::SwipeLeft, -1, 0),
        (GestureName::SwipeRight, 1, 0),
        (GestureName::SwipeUp, 0, -1),
        (GestureName::SwipeDown, 0, 1),
        (GestureName::ThreeFingerSwipeLeft, -1, 0),
        (GestureName::ThreeFingerSwipeRight, 1, 0),
        (GestureName::ThreeFingerSwipeUp, 0, -1),
        (GestureName::ThreeFingerSwipeDown, 0, 1),
    ] {
        let frames = gesture_frames(&spec(name), 0);
        let first = &frames[0].contacts[0];
        let last = &frames[frames.len() - 1].contacts[0];
        assert_eq!((last.x - first.x).signum(), dx, "{name:?}");
        assert_eq!((last.y - first.y).signum(), dy, "{name:?}");
    }
    for (name, sign) in [
        (GestureName::RotateCw, 1),
        (GestureName::RotateCcw, -1),
        (GestureName::CircleCw, 1),
        (GestureName::CircleCcw, -1),
    ] {
        let frames = gesture_frames(&spec(name), 0);
        assert_eq!(
            (frames[1].contacts[0].y - frames[0].contacts[0].y).signum(),
            sign,
            "{name:?}"
        );
    }
    let pinch = gesture_frames(&spec(GestureName::PinchIn), 0);
    let span = |frame: &TouchFrame| (frame.contacts[1].x - frame.contacts[0].x).abs();
    assert!(span(pinch.last().expect("final frame")) < span(&pinch[0]));
}
