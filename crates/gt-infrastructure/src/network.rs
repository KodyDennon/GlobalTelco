use gt_common::types::EntityId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NetworkGraph {
    nodes: Vec<EntityId>,
    edges: Vec<(EntityId, EntityId)>,
}

impl NetworkGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }
}
