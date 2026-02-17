/// Calculator for telecom demand based on population and development
pub struct DemandCalculator;

impl DemandCalculator {
    /// Calculate raw telecom demand for a city
    pub fn city_demand(population: u64, development: f64) -> f64 {
        population as f64 * development * 0.001
    }

    /// Calculate demand satisfaction given available capacity
    pub fn satisfaction(demand: f64, capacity: f64) -> f64 {
        if demand > 0.0 {
            (capacity / demand).clamp(0.0, 1.0)
        } else {
            0.0
        }
    }

    /// Calculate infrastructure satisfaction for a specific cell
    pub fn city_infrastructure_satisfaction(
        city_demand: f64,
        city_capacity: f64,
        competition_bonus: f64,
    ) -> f64 {
        let base = if city_demand > 0.0 {
            (city_capacity / city_demand).min(1.0)
        } else {
            0.0
        };
        (base + competition_bonus).min(1.0)
    }
}
