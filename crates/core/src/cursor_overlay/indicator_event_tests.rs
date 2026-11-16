use std::sync::Mutex;

use super::*;
use crate::{ActionOps, ControlIndicatorEvent, InputOps, ObservationOps, SystemOps};

#[derive(Default)]
struct Recorder(Mutex<Vec<(String, Point)>>);

impl ActionOps for Recorder {}
impl InputOps for Recorder {}
impl ObservationOps for Recorder {}
impl SystemOps for Recorder {
    fn present_control_indicator(
        &self,
        event: &ControlIndicatorEvent<'_>,
    ) -> Result<(), AdapterError> {
        if let ControlIndicatorEvent::Click { owner, point } = event {
            self.0
                .lock()
                .unwrap()
                .push(((*owner).to_owned(), (*point).clone()));
        }
        Ok(())
    }
}

#[test]
fn click_effect_requires_a_scoped_indicator_and_confirmed_delivery() {
    let adapter = Recorder::default();
    let point = Point { x: -400.0, y: 70.0 };
    let context = CommandContext::default().with_control_indicator_owner(Some("session".into()));
    for disposition in [
        DeliverySemantics::not_delivered(),
        DeliverySemantics::uncertain(),
    ] {
        present_control_click(&adapter, &context, &point, disposition);
    }
    present_control_click(
        &adapter,
        &CommandContext::default(),
        &point,
        DeliverySemantics::delivered_verified(),
    );
    assert!(adapter.0.lock().unwrap().is_empty());
    present_control_click(
        &adapter,
        &context,
        &point,
        DeliverySemantics::delivered_unverified(),
    );
    assert_eq!(
        adapter.0.lock().unwrap().as_slice(),
        [("session".into(), point)]
    );
}
