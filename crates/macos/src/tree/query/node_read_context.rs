pub(crate) struct NodeReadContext<'a> {
    pub(crate) tree: &'a crate::tree::TreeBuildContext,
    pub(crate) stats: &'a mut acdesktop_core::LocatorStats,
    pub(crate) usage: &'a mut crate::tree::observation_usage::ObservationUsage,
    pub(crate) requirements: acdesktop_core::EvidenceRequirements,
    pub(crate) deadline: std::time::Instant,
    pub(crate) child_plan: super::child_read_plan::ChildReadPlan,
}
