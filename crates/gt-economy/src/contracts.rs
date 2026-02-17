use gt_common::types::{Money, Tick};
use serde::{Deserialize, Serialize};

/// Terms for a contract proposal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractTerms {
    pub capacity: f64,
    pub price_per_tick: Money,
    pub duration_ticks: Tick,
    pub penalty: Money,
}

impl ContractTerms {
    pub fn standard(capacity: f64, duration_ticks: Tick) -> Self {
        let price_per_tick = (capacity * 0.1) as Money;
        let penalty = price_per_tick * (duration_ticks as Money) / 4;
        Self {
            capacity,
            price_per_tick,
            duration_ticks,
            penalty,
        }
    }
}

/// Evaluates whether a contract is worthwhile for a corporation
pub struct ContractEvaluator;

impl ContractEvaluator {
    /// Score a contract proposal from the receiver's perspective
    /// Returns a score where > 0.5 means accept, < 0.5 means reject
    pub fn evaluate_proposal(
        terms: &ContractTerms,
        receiver_cash: Money,
        receiver_available_capacity: f64,
        receiver_revenue_per_tick: Money,
    ) -> f64 {
        let mut score = 0.5;

        // Can we fulfill the capacity requirement?
        if receiver_available_capacity < terms.capacity {
            return 0.0; // Can't fulfill, don't accept
        }

        // Revenue relative to current revenue
        if receiver_revenue_per_tick > 0 {
            let revenue_ratio = terms.price_per_tick as f64 / receiver_revenue_per_tick as f64;
            score += revenue_ratio.min(0.2); // Up to +0.2 for good revenue
        } else {
            score += 0.15; // Any revenue is good when we have none
        }

        // Penalty risk assessment
        let penalty_ratio = terms.penalty as f64 / receiver_cash.max(1) as f64;
        if penalty_ratio > 0.5 {
            score -= 0.3; // High penalty risk
        } else if penalty_ratio > 0.2 {
            score -= 0.1;
        }

        // Capacity utilization check: don't over-commit
        let capacity_commitment = terms.capacity / receiver_available_capacity.max(1.0);
        if capacity_commitment > 0.8 {
            score -= 0.2; // Over-committing
        }

        score.clamp(0.0, 1.0)
    }

    /// Determine a fair price for a contract based on market conditions
    pub fn fair_price(capacity: f64, duration_ticks: Tick, supply_demand_ratio: f64) -> Money {
        let base_price = (capacity * 0.1) as Money;
        let market_adjustment = if supply_demand_ratio < 0.8 {
            1.3 // Seller's market
        } else if supply_demand_ratio > 1.5 {
            0.7 // Buyer's market
        } else {
            1.0
        };
        (base_price as f64 * market_adjustment * (duration_ticks as f64 / 100.0).max(1.0)) as Money
    }
}
