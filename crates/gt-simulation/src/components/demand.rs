use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Demand {
    pub base_demand: f64,
    pub current_demand: f64,
    pub satisfaction: f64,
}
