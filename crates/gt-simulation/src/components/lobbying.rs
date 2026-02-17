use gt_common::types::{EntityId, LobbyPolicy, Money, Tick};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LobbyingCampaign {
    pub corporation: EntityId,
    pub region: EntityId,
    pub policy: LobbyPolicy,
    pub budget_spent: Money,
    pub budget_total: Money,
    pub influence: f64,
    pub start_tick: Tick,
    pub active: bool,
}

impl LobbyingCampaign {
    pub fn new(
        corporation: EntityId,
        region: EntityId,
        policy: LobbyPolicy,
        budget: Money,
        tick: Tick,
    ) -> Self {
        Self {
            corporation,
            region,
            policy,
            budget_spent: 0,
            budget_total: budget,
            influence: 0.0,
            start_tick: tick,
            active: true,
        }
    }

    pub fn influence_threshold(&self) -> f64 {
        match self.policy {
            LobbyPolicy::ReduceTax => 0.6,
            LobbyPolicy::RelaxZoning => 0.5,
            LobbyPolicy::FastTrackPermits => 0.4,
            LobbyPolicy::IncreasedCompetitorBurden => 0.7,
            LobbyPolicy::SubsidyRequest => 0.8,
        }
    }
}
