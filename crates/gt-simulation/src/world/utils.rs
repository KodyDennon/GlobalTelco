use gt_common::types::AIArchetype;

/// Haversine distance in km between two (lon, lat) points given as separate arguments.
/// Delegates to `gt_common::geo::haversine_km` with swapped parameter order.
pub(super) fn haversine_km_deg(lon1: f64, lat1: f64, lon2: f64, lat2: f64) -> f64 {
    gt_common::geo::haversine_km(lat1, lon1, lat2, lon2)
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
