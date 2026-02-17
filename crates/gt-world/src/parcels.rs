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
pub struct LandParcel {
    pub cell_index: usize,
    pub terrain: TerrainType,
    pub elevation: f64,
    pub zoning: ZoningType,
    pub cost_modifier: f64,
    pub disaster_risk: f64,
    pub owner: Option<EntityId>,
}

impl LandParcel {
    pub fn new(cell_index: usize, terrain: TerrainType, elevation: f64) -> Self {
        let cost_modifier = terrain.construction_cost_multiplier();
        let disaster_risk = match terrain {
            TerrainType::Coastal => 0.3,
            TerrainType::Mountainous => 0.25,
            TerrainType::Desert => 0.1,
            TerrainType::Tundra | TerrainType::Frozen => 0.15,
            TerrainType::OceanShallow => 0.4,
            TerrainType::OceanDeep => 0.5,
            _ => 0.05,
        };
        let zoning = initial_zoning(terrain);

        Self {
            cell_index,
            terrain,
            elevation,
            zoning,
            cost_modifier,
            disaster_risk,
            owner: None,
        }
    }
}

fn initial_zoning(terrain: TerrainType) -> ZoningType {
    match terrain {
        TerrainType::Urban => ZoningType::Mixed,
        TerrainType::Suburban => ZoningType::Residential,
        TerrainType::Rural => ZoningType::Unzoned,
        TerrainType::Mountainous => ZoningType::Protected,
        TerrainType::Coastal => ZoningType::Mixed,
        _ => ZoningType::Unzoned,
    }
}

pub fn create_parcels(terrains: &[TerrainType], elevations: &[f64]) -> Vec<LandParcel> {
    terrains
        .iter()
        .enumerate()
        .filter(|(_, t)| t.is_land())
        .map(|(i, &terrain)| LandParcel::new(i, terrain, elevations[i]))
        .collect()
}
