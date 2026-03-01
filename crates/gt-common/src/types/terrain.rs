use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TerrainType {
    Urban,
    Suburban,
    Rural,
    Mountainous,
    Desert,
    Coastal,
    OceanShallow,
    OceanDeep,
    /// Extreme ocean depths (trenches, subduction zones). Very high construction cost
    /// for submarine cables and lowest reliability.
    OceanTrench,
    Tundra,
    Frozen,
}

impl TerrainType {
    pub fn construction_cost_multiplier(&self) -> f64 {
        match self {
            TerrainType::Urban => 2.0,
            TerrainType::Suburban => 1.2,
            TerrainType::Rural => 1.0,
            TerrainType::Mountainous => 3.0,
            TerrainType::Desert => 1.8,
            TerrainType::Coastal => 1.5,
            TerrainType::OceanShallow => 5.0,
            TerrainType::OceanDeep => 10.0,
            TerrainType::OceanTrench => 15.0,
            TerrainType::Tundra => 2.5,
            TerrainType::Frozen => 4.0,
        }
    }

    pub fn maintenance_cost_multiplier(&self) -> f64 {
        match self {
            TerrainType::Urban => 1.5,
            TerrainType::Suburban => 1.1,
            TerrainType::Rural => 1.0,
            TerrainType::Mountainous => 2.0,
            TerrainType::Desert => 1.5,
            TerrainType::Coastal => 1.3,
            TerrainType::OceanShallow => 3.0,
            TerrainType::OceanDeep => 5.0,
            TerrainType::OceanTrench => 8.0,
            TerrainType::Tundra => 1.8,
            TerrainType::Frozen => 2.5,
        }
    }

    pub fn reliability_modifier(&self) -> f64 {
        match self {
            TerrainType::Urban => 1.0,
            TerrainType::Suburban => 1.0,
            TerrainType::Rural => 0.95,
            TerrainType::Mountainous => 0.85,
            TerrainType::Desert => 0.90,
            TerrainType::Coastal => 0.88,
            TerrainType::OceanShallow => 0.80,
            TerrainType::OceanDeep => 0.75,
            TerrainType::OceanTrench => 0.65,
            TerrainType::Tundra => 0.85,
            TerrainType::Frozen => 0.80,
        }
    }

    pub fn is_land(&self) -> bool {
        !matches!(
            self,
            TerrainType::OceanShallow | TerrainType::OceanDeep | TerrainType::OceanTrench
        )
    }

    /// Returns the bathymetry cost multiplier for submarine cable construction.
    ///
    /// This is separate from the general `construction_cost_multiplier` because
    /// submarine cables have specialized depth-based cost scaling:
    /// - Continental shelf (shallow): 1.0x base cost
    /// - Deep ocean: 2.0x base cost (pressure, specialized equipment)
    /// - Ocean trenches: 3.0x base cost (extreme depth, seismic risk)
    /// - Non-ocean terrain: 1.0x (landing station endpoints)
    pub fn submarine_bathymetry_multiplier(&self) -> f64 {
        match self {
            TerrainType::Coastal => 1.0,
            TerrainType::OceanShallow => 1.0,
            TerrainType::OceanDeep => 2.0,
            TerrainType::OceanTrench => 3.0,
            _ => 1.0, // land endpoints (landing stations)
        }
    }

    /// Returns true if this terrain is an ocean type (shallow, deep, or trench).
    pub fn is_ocean(&self) -> bool {
        matches!(
            self,
            TerrainType::OceanShallow | TerrainType::OceanDeep | TerrainType::OceanTrench
        )
    }
}
