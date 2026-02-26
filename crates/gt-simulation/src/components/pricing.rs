use gt_common::types::{EntityId, Money};
use serde::{Deserialize, Serialize};

/// Regional pricing tier for a corporation's services.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PriceTier {
    Budget,
    Standard,
    Premium,
}

impl Default for PriceTier {
    fn default() -> Self {
        PriceTier::Standard
    }
}

impl PriceTier {
    /// Revenue multiplier relative to base service price.
    pub fn revenue_multiplier(&self) -> f64 {
        match self {
            PriceTier::Budget => 0.6,
            PriceTier::Standard => 1.0,
            PriceTier::Premium => 1.8,
        }
    }

    /// Customer acquisition rate modifier.
    /// Budget attracts more customers, premium attracts fewer but pays more.
    pub fn acquisition_modifier(&self) -> f64 {
        match self {
            PriceTier::Budget => 1.4,
            PriceTier::Standard => 1.0,
            PriceTier::Premium => 0.6,
        }
    }

    /// Churn rate modifier (higher = more customers leave).
    pub fn churn_modifier(&self) -> f64 {
        match self {
            PriceTier::Budget => 0.8,
            PriceTier::Standard => 1.0,
            PriceTier::Premium => 1.3,
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "Budget" => PriceTier::Budget,
            "Premium" => PriceTier::Premium,
            _ => PriceTier::Standard,
        }
    }
}

/// Per-region pricing for a corporation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionPricing {
    pub corp_id: EntityId,
    pub region_id: EntityId,
    pub tier: PriceTier,
    pub price_per_unit: Money,
}
