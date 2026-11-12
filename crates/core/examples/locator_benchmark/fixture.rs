use crate::fixture_node::FixtureNode;
use acdesktop_core::WindowInfo;

#[derive(Clone)]
pub(crate) struct Fixture {
    pub nodes: Vec<FixtureNode>,
    pub roots: Vec<u32>,
    pub window: WindowInfo,
}
