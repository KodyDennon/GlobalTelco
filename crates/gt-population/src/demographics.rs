use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PopulationState {
    pub total_population: u64,
    pub urban_ratio: f64,
}
