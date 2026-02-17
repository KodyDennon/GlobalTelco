use gt_common::types::{EntityId, Money, Tick};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ContractType {
    Peering,
    Transit,
    SLA,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ContractStatus {
    Proposed,
    Active,
    Expired,
    Breached,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contract {
    pub contract_type: ContractType,
    pub from: EntityId,
    pub to: EntityId,
    pub capacity: f64,
    pub price_per_tick: Money,
    pub start_tick: Tick,
    pub end_tick: Tick,
    pub status: ContractStatus,
    pub penalty: Money,
}

impl Contract {
    #[allow(clippy::too_many_arguments)]
    pub fn new_proposal(
        contract_type: ContractType,
        from: EntityId,
        to: EntityId,
        capacity: f64,
        price_per_tick: Money,
        current_tick: Tick,
        duration: Tick,
        penalty: Money,
    ) -> Self {
        Self {
            contract_type,
            from,
            to,
            capacity,
            price_per_tick,
            start_tick: current_tick,
            end_tick: current_tick + duration,
            status: ContractStatus::Proposed,
            penalty,
        }
    }

    pub fn is_expired(&self, current_tick: Tick) -> bool {
        current_tick >= self.end_tick && self.status == ContractStatus::Active
    }

    pub fn activate(&mut self, tick: Tick) {
        self.status = ContractStatus::Active;
        self.start_tick = tick;
        let duration = self.end_tick - self.start_tick;
        self.end_tick = tick + duration;
    }
}
