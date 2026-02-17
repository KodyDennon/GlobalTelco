use serde::{Deserialize, Serialize};

/// Global and regional market state tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketState {
    pub global_demand_modifier: f64,
    pub interest_rate: f64,
    pub economic_health: f64,
    pub inflation_rate: f64,
}

impl Default for MarketState {
    fn default() -> Self {
        Self {
            global_demand_modifier: 1.0,
            interest_rate: 0.05,
            economic_health: 0.5,
            inflation_rate: 0.02,
        }
    }
}

/// Regional supply/demand equilibrium result
#[derive(Debug, Clone)]
pub struct RegionalEquilibrium {
    pub region_id: u64,
    pub total_demand: f64,
    pub total_supply: f64,
    pub satisfaction: f64,
    pub supply_demand_ratio: f64,
}

impl RegionalEquilibrium {
    pub fn compute(region_id: u64, total_demand: f64, total_supply: f64) -> Self {
        let satisfaction = if total_demand > 0.0 {
            (total_supply / total_demand).min(1.0)
        } else {
            1.0
        };
        let supply_demand_ratio = if total_demand > 0.0 {
            total_supply / total_demand
        } else {
            1.0
        };
        Self {
            region_id,
            total_demand,
            total_supply,
            satisfaction,
            supply_demand_ratio,
        }
    }

    /// Suggests a tax rate adjustment based on supply/demand ratio
    pub fn suggested_tax_adjustment(&self) -> f64 {
        if self.supply_demand_ratio > 1.5 {
            -0.001 // Oversupply → lower taxes to attract business
        } else if self.supply_demand_ratio < 0.5 {
            0.001 // Undersupply → raise taxes
        } else {
            0.0
        }
    }
}

/// Competition level in a region and its effects
#[derive(Debug, Clone)]
pub struct CompetitionAnalysis {
    pub corp_count: usize,
    pub satisfaction_bonus: f64,
}

impl CompetitionAnalysis {
    pub fn from_corp_count(count: usize) -> Self {
        let bonus = match count {
            0 | 1 => 0.0,
            2 => 0.1,
            3 => 0.2,
            _ => 0.3,
        };
        Self {
            corp_count: count,
            satisfaction_bonus: bonus,
        }
    }
}

impl MarketState {
    /// Update economic health based on profitability ratio of corporations
    pub fn update_economic_health(&mut self, profitable: u32, total: u32) {
        self.economic_health = if total > 0 {
            profitable as f64 / total as f64
        } else {
            0.5
        };
    }

    /// Calculate the interest rate modifier based on credit rating
    pub fn interest_rate_for_rating(&self, rating: gt_common::types::CreditRating) -> f64 {
        let spread = match rating {
            gt_common::types::CreditRating::AAA => 0.0,
            gt_common::types::CreditRating::AA => 0.005,
            gt_common::types::CreditRating::A => 0.01,
            gt_common::types::CreditRating::BBB => 0.02,
            gt_common::types::CreditRating::BB => 0.04,
            gt_common::types::CreditRating::B => 0.07,
            gt_common::types::CreditRating::CCC => 0.12,
            gt_common::types::CreditRating::D => 0.25,
        };
        self.interest_rate + spread
    }
}
