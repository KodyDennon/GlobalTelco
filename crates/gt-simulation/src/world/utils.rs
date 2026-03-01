use gt_common::types::AIArchetype;

/// Haversine distance in km between two (lon, lat) points given as separate arguments.
pub(super) fn haversine_km_deg(lon1: f64, lat1: f64, lon2: f64, lat2: f64) -> f64 {
    let dlat = (lat1 - lat2).to_radians();
    let dlon = (lon1 - lon2).to_radians();
    let lat1_r = lat1.to_radians();
    let lat2_r = lat2.to_radians();
    let a = (dlat / 2.0).sin().powi(2) + lat1_r.cos() * lat2_r.cos() * (dlon / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().asin();
    6371.0 * c
}

pub(super) fn archetype_skill_bonus(archetype: AIArchetype) -> f64 {
    match archetype {
        AIArchetype::TechInnovator => 0.3,
        AIArchetype::DefensiveConsolidator => 0.2,
        AIArchetype::AggressiveExpander => 0.1,
        AIArchetype::BudgetOperator => 0.0,
        AIArchetype::SatellitePioneer => 0.25,
    }
}
