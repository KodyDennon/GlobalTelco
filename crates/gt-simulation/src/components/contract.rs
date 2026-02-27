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
    /// SLA uptime target as a percentage (e.g. 99.5 means 99.5% uptime).
    /// Derived from contract capacity tier when created.
    #[serde(default = "default_sla_target")]
    pub sla_target: f64,
    /// Accumulated SLA penalty amount when performance is below target.
    #[serde(default)]
    pub sla_penalty_accrued: Money,
    /// Current SLA performance as a percentage (e.g. 99.2 means 99.2% uptime).
    /// Updated each tick by the contract system based on actual network performance.
    #[serde(default = "default_sla_performance")]
    pub sla_current_performance: f64,
}

fn default_sla_target() -> f64 {
    98.0
}

fn default_sla_performance() -> f64 {
    100.0
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
        // Derive SLA target from capacity tier:
        // High capacity (>5000) = 99.5%, Medium (>1000) = 99.0%, Low = 98.0%
        let sla_target = if capacity > 5000.0 {
            99.5
        } else if capacity > 1000.0 {
            99.0
        } else {
            98.0
        };
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
            sla_target,
            sla_penalty_accrued: 0,
            sla_current_performance: 100.0,
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
