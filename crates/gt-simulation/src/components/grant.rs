use gt_common::types::{EntityId, Money, Tick};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GrantStatus {
    /// Available for bidding
    Available,
    /// Awarded to a corporation
    Awarded,
    /// Successfully completed
    Completed,
    /// Deadline passed without completion
    Expired,
}

/// A government grant incentivizes infrastructure build-out in underserved regions.
/// Corporations bid for grants, then must meet coverage requirements by a deadline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernmentGrant {
    pub region_id: EntityId,
    pub requirement_description: String,
    pub required_coverage_pct: f64,
    pub reward_cash: Money,
    pub tax_break_pct: f64,
    pub deadline_tick: Tick,
    pub awarded_corp: Option<EntityId>,
    pub progress: f64, // 0.0 to 1.0
    pub status: GrantStatus,
    pub created_tick: Tick,
}

impl GovernmentGrant {
    pub fn new(
        region_id: EntityId,
        required_coverage_pct: f64,
        reward_cash: Money,
        tax_break_pct: f64,
        deadline_tick: Tick,
        created_tick: Tick,
    ) -> Self {
        let description = format!(
            "Achieve {:.0}% coverage in region for ${:.0}M reward",
            required_coverage_pct * 100.0,
            reward_cash as f64 / 1_000_000.0,
        );
        Self {
            region_id,
            requirement_description: description,
            required_coverage_pct,
            reward_cash,
            tax_break_pct,
            deadline_tick,
            awarded_corp: None,
            progress: 0.0,
            status: GrantStatus::Available,
            created_tick,
        }
    }

    /// Whether the grant has expired at the given tick.
    pub fn is_expired(&self, tick: Tick) -> bool {
        tick >= self.deadline_tick && self.status != GrantStatus::Completed
    }

    /// Whether the grant is still available for bidding.
    pub fn is_available(&self) -> bool {
        self.status == GrantStatus::Available
    }

    /// Whether the grant requirements are met.
    pub fn is_complete(&self) -> bool {
        self.progress >= 1.0
    }
}
