use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MarketState {
    pub global_demand_modifier: f64,
    pub interest_rate: f64,
}
