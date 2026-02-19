use gt_common::types::EntityId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionComponent {
    pub name: String,
    pub cells: Vec<usize>,
    pub center_lat: f64,
    pub center_lon: f64,
    pub gdp: f64,
    pub population: u64,
    pub development: f64,
    pub regulatory_strictness: f64,
    pub tax_rate: f64,
    pub disaster_risk: f64,
    pub city_ids: Vec<EntityId>,
    /// Boundary polygon as ordered (lat, lon) pairs for rendering.
    #[serde(default)]
    pub boundary_polygon: Vec<(f64, f64)>,
}
