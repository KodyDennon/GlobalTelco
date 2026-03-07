use indexmap::IndexMap;

use gt_common::types::{EntityId, Tick};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alliance {
    pub id: EntityId,
    pub name: String,
    pub member_corp_ids: Vec<EntityId>, // max 3
    pub trust_scores: IndexMap<EntityId, f64>, // per-member trust (0.0 - 1.0)
    pub revenue_share_pct: f64,
    pub formed_tick: Tick,
}

impl Alliance {
    pub fn new(
        id: EntityId,
        name: String,
        founding_members: Vec<EntityId>,
        revenue_share_pct: f64,
        tick: Tick,
    ) -> Self {
        let mut trust_scores = IndexMap::new();
        for &member in &founding_members {
            trust_scores.insert(member, 0.5); // Start at neutral trust
        }
        Self {
            id,
            name,
            member_corp_ids: founding_members,
            trust_scores,
            revenue_share_pct,
            formed_tick: tick,
        }
    }

    /// Returns true if the alliance has fewer than 2 members (should be dissolved).
    pub fn is_defunct(&self) -> bool {
        self.member_corp_ids.len() < 2
    }

    /// Returns true if any member's trust has fallen below the threshold.
    pub fn has_low_trust(&self, threshold: f64) -> bool {
        self.trust_scores.values().any(|&t| t < threshold)
    }

    /// Remove a member from the alliance.
    pub fn remove_member(&mut self, corp_id: EntityId) {
        self.member_corp_ids.retain(|&id| id != corp_id);
        self.trust_scores.shift_remove(&corp_id);
    }
}
