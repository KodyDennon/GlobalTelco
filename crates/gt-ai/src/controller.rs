use gt_common::types::{AIArchetype, AIStrategy, EntityId};

use crate::archetype::ArchetypeWeights;
use crate::strategy::{FinancialSnapshot, StrategySelector};

/// AI corporation controller
pub struct AiController {
    pub corporation_id: EntityId,
    pub archetype: AIArchetype,
    pub strategy: AIStrategy,
    pub weights: ArchetypeWeights,
}

impl AiController {
    pub fn new(corporation_id: EntityId, archetype: AIArchetype) -> Self {
        Self {
            corporation_id,
            archetype,
            strategy: AIStrategy::Expand,
            weights: ArchetypeWeights::for_archetype(archetype),
        }
    }

    /// Update strategy based on current financial state
    pub fn update_strategy(&mut self, snapshot: &FinancialSnapshot) {
        self.strategy = StrategySelector::select(self.archetype, snapshot);
    }

    /// Check if this AI should take on more debt
    pub fn should_take_loan(&self, snapshot: &FinancialSnapshot) -> bool {
        StrategySelector::should_take_loan(self.archetype, snapshot)
    }

    /// Check if this AI should invest in research
    pub fn should_research(&self, snapshot: &FinancialSnapshot) -> bool {
        StrategySelector::should_research(self.archetype, snapshot)
    }

    /// Minimum cash safety margin (won't build if cash drops below this)
    pub fn cash_safety_margin(&self, cost_per_tick: i64) -> i64 {
        (cost_per_tick as f64 * self.weights.desired_cash_reserve_ticks()) as i64
    }
}
