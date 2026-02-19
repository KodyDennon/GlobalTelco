use gt_common::types::EntityId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CityComponent {
    pub name: String,
    pub region_id: EntityId,
    pub cell_index: usize, // Center cell
    /// All cells this city occupies. Includes center cell.
    /// Population is distributed across these cells, densest at center.
    pub cells: Vec<usize>,
    pub population: u64,
    pub growth_rate: f64,
    pub development: f64,
    pub telecom_demand: f64,
    pub infrastructure_satisfaction: f64,
    // Employment
    pub jobs_available: u32,
    pub employment_rate: f64,
    // Demographics
    pub birth_rate: f64,
    pub death_rate: f64,
    pub migration_pressure: f64,
}

impl CityComponent {
    /// Compute migration score: higher = more attractive.
    pub fn attractiveness(&self) -> f64 {
        self.infrastructure_satisfaction * 0.3
            + self.development * 0.3
            + self.employment_rate * 0.4
    }
}
