use super::{Feedback, feedback_for};
use crate::system::game::steps::StepAction;

#[test]
fn keyboard_feedback_hides_character_keys() {
    let action = StepAction::KeyDown { key: "a".into() };
    let Some(Feedback::Keys(label)) = feedback_for(&action, (0, 0)) else {
        panic!("missing keyboard feedback");
    };
    assert_eq!(label, "Character key requested");
}

#[test]
fn click_feedback_uses_screen_coordinates_and_only_follows_press() {
    let action = StepAction::MouseButton {
        x: 10,
        y: 20,
        button: "left".into(),
        down: true,
    };
    let Some(Feedback::Click(point)) = feedback_for(&action, (-100, 200)) else {
        panic!("missing click feedback");
    };
    assert_eq!((point.x, point.y), (-90.0, 220.0));
    let release = StepAction::MouseButton {
        x: 10,
        y: 20,
        button: "left".into(),
        down: false,
    };
    assert!(feedback_for(&release, (0, 0)).is_none());
}
