use gt_common::types::{EntityId, TerrainType};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ZoningType {
    Residential,
    Commercial,
    Industrial,
    Mixed,
    Protected,
    Unzoned,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LandParcelComponent {
    pub cell_index: usize,
    pub terrain: TerrainType,
    pub elevation: f64,
    pub zoning: ZoningType,
    pub cost_modifier: f64,
    pub disaster_risk: f64,
    pub owner: Option<EntityId>,
}
