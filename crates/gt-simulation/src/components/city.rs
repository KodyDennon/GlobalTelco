use gt_common::types::EntityId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CityComponent {
    pub name: String,
    pub region_id: EntityId,
    pub cell_index: usize,
    pub population: u64,
    pub growth_rate: f64,
    pub development: f64,
    pub telecom_demand: f64,
    pub infrastructure_satisfaction: f64,
}
