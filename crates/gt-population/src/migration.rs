/// Score for a city's attractiveness for migration
#[derive(Debug, Clone)]
pub struct CityScore {
    pub city_id: u64,
    pub score: f64,
    pub population: u64,
}

/// Engine for computing inter-city migration
pub struct MigrationEngine;

impl MigrationEngine {
    /// Score a city for migration attractiveness
    pub fn score_city(
        city_id: u64,
        infrastructure_satisfaction: f64,
        development: f64,
        population: u64,
    ) -> CityScore {
        let score = infrastructure_satisfaction * 0.5 + development * 0.5;
        CityScore {
            city_id,
            score,
            population,
        }
    }

    /// Calculate net migration for a city given its score relative to average
    /// Returns the number of migrants (positive = inflow, negative = outflow)
    pub fn calculate_migration(population: u64, city_score: f64, average_score: f64) -> i64 {
        if population < 1000 {
            return 0;
        }
        let diff = city_score - average_score;
        // 0.1% of population per tick, scaled by score difference
        (population as f64 * 0.001 * diff) as i64
    }

    /// Compute the average score across all cities
    pub fn average_score(scores: &[CityScore]) -> f64 {
        if scores.is_empty() {
            return 0.0;
        }
        scores.iter().map(|s| s.score).sum::<f64>() / scores.len() as f64
    }
}
