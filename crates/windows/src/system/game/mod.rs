//! Game-session support: a sustained capture session, a scheduled input
//! timeline, and the per-sample observation the report is built from.

pub(in crate::system) mod frames;
pub(in crate::system) mod input;
pub(in crate::system) mod keys;
pub(in crate::system) mod observe;
pub(in crate::system) mod session;
pub(in crate::system) mod steps;
pub(in crate::system) mod target;
