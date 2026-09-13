//! OrbyNode remote nodes - pairing identity, heartbeat, and aggregation.

mod registry;

pub use registry::{Heartbeat, Node, NodeRecord, NodeRegistry, NodeStatus, Pairing};
