use gt_common::types::EntityId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ownership {
    pub owner: EntityId,
    pub co_owners: Vec<(EntityId, f64)>,
}

impl Ownership {
    pub fn sole(owner: EntityId) -> Self {
        Self { owner, co_owners: Vec::new() }
    }
}
