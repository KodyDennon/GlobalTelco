use serde::{Deserialize, Serialize};

/// Global population state tracking
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PopulationState {
    pub total_population: u64,
    pub urban_ratio: f64,
}

/// Result of population growth calculation for a single city
#[derive(Debug, Clone)]
pub struct GrowthResult {
    pub city_id: u64,
    pub old_population: u64,
    pub new_population: u64,
    pub growth: i64,
}

/// Calculate population growth for a city
pub fn calculate_city_growth(
    city_id: u64,
    population: u64,
    growth_rate: f64,
    infrastructure_satisfaction: f64,
) -> GrowthResult {
    // Growth is boosted by infra satisfaction, penalized by overcrowding
    let adjusted_rate = growth_rate * (0.5 + infrastructure_satisfaction * 0.5);
    let growth = (population as f64 * adjusted_rate) as i64;
    let new_population = (population as i64 + growth).max(100) as u64;

    GrowthResult {
        city_id,
        old_population: population,
        new_population,
        growth,
    }
}

/// Aggregate city populations into a region total
pub fn aggregate_region_population(city_populations: &[u64]) -> u64 {
    city_populations.iter().sum()
}
