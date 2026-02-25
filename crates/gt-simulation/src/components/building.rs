use gt_common::types::EntityId;
use serde::{Deserialize, Serialize};

/// Status of a building within a city's footprint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BuildingStatus {
    /// Active building generating telecom demand.
    Active,
    /// Abandoned building: still exists but generates zero demand.
    /// Caused by population decline. Can be reactivated if population rebounds.
    Abandoned,
}

/// A single building footprint in the simulation. Buildings are demand points
/// within a city's cell coverage area. They generate subscriber revenue when
/// served by NAPs (via DropCable or auto-coverage).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildingFootprint {
    /// The city entity this building belongs to.
    pub city_id: EntityId,
    /// Grid cell index where this building is located.
    pub cell_index: usize,
    /// Demand value this building contributes (population density proxy).
    /// Higher for downtown/commercial, lower for suburban/fringe.
    pub demand_value: f64,
    /// Current status (Active or Abandoned).
    pub status: BuildingStatus,
    /// Whether this building is in the suburban fringe (outermost ring).
    /// Fringe buildings are the first to be abandoned on decline and
    /// new buildings spawn in the fringe on growth.
    pub is_fringe: bool,
}

impl BuildingFootprint {
    pub fn new(city_id: EntityId, cell_index: usize, demand_value: f64, is_fringe: bool) -> Self {
        Self {
            city_id,
            cell_index,
            demand_value,
            status: BuildingStatus::Active,
            is_fringe,
        }
    }

    /// Returns the effective demand (0.0 if abandoned).
    pub fn effective_demand(&self) -> f64 {
        match self.status {
            BuildingStatus::Active => self.demand_value,
            BuildingStatus::Abandoned => 0.0,
        }
    }
}

/// Per-city building census: tracks the current and target building count
/// for dynamic spawn/destruction logic.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CityBuildingCensus {
    /// Total active buildings.
    pub active_count: u32,
    /// Total abandoned buildings.
    pub abandoned_count: u32,
    /// Target building count based on current population.
    pub target_count: u32,
    /// Previous tick's population (for detecting growth/decline).
    pub prev_population: u64,
}

impl CityBuildingCensus {
    pub fn new(population: u64) -> Self {
        let target = compute_building_target(population);
        Self {
            active_count: target,
            abandoned_count: 0,
            target_count: target,
            prev_population: population,
        }
    }
}

/// Compute the target number of buildings for a city based on population.
/// Uses a tiered formula matching the frontend's building layer tiers.
pub fn compute_building_target(population: u64) -> u32 {
    if population >= 5_000_000 {
        // Megalopolis: 1000-2000 buildings
        1000 + ((population - 5_000_000) as f64 / 10_000_000.0 * 1000.0).min(1000.0) as u32
    } else if population >= 1_000_000 {
        // Metropolis: 500-1000 buildings
        500 + ((population - 1_000_000) as f64 / 4_000_000.0 * 500.0) as u32
    } else if population >= 250_000 {
        // City: 200-500 buildings
        200 + ((population - 250_000) as f64 / 750_000.0 * 300.0) as u32
    } else if population >= 50_000 {
        // Town: 50-200 buildings
        50 + ((population - 50_000) as f64 / 200_000.0 * 150.0) as u32
    } else if population >= 5_000 {
        // Hamlet: 20-50 buildings
        20 + ((population - 5_000) as f64 / 45_000.0 * 30.0) as u32
    } else {
        // Village: 5-20 buildings
        (population as f64 / 5_000.0 * 15.0).max(5.0) as u32
    }
}
