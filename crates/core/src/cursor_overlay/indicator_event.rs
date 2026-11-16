use crate::Point;

#[derive(Debug, Clone, Copy)]
pub enum ControlIndicatorEvent<'a> {
    Keys { owner: &'a str, display: &'a str },
    Click { owner: &'a str, point: &'a Point },
}
