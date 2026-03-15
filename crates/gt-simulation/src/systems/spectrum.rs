//! Spectrum Interference System (Phase 8.3)
//!
//! After spectrum assignment, checks for nearby same-band nodes within an
//! interference radius. Interference radius depends on band frequency
//! (lower frequency = larger radius). For each interfering neighbor,
//! effective throughput is reduced by 15% (diminishing — multiplicative).
//!
//! Also applies carrier aggregation: combined node capacity = sum of
//! individual band capacities when multiple bands are assigned.

use crate::world::GameWorld;
use gt_common::types::{FrequencyBand, NodeType};
use std::collections::HashMap;

/// Run the spectrum interference and carrier aggregation system.
///
/// This system:
/// 1. For each wireless node with assigned bands, computes aggregated capacity
///    from all assigned bands (carrier aggregation).
/// 2. Detects same-band interference from nearby wireless nodes and applies
///    a throughput penalty (15% per interferer, diminishing multiplicatively).
/// 3. Writes the effective throughput into the Capacity component.
pub fn run(world: &mut GameWorld) {
    // Collect wireless node data for interference checks.
    // We need: (node_id, owner, position (lon, lat), assigned_bands, base_throughput)
    let wireless_nodes: Vec<(u64, u64, f64, f64, Vec<String>, f64)> = {
        let mut v: Vec<_> = world
            .infra_nodes
            .iter()
            .filter(|(id, _)| !world.constructions.contains_key(*id))
            .filter(|(_, node)| is_wireless_node(node.node_type))
            .filter_map(|(&id, node)| {
                let pos = world.positions.get(&id)?;
                Some((
                    id,
                    node.owner,
                    pos.x,  // longitude
                    pos.y,  // latitude
                    node.assigned_bands.clone(),
                    node.max_throughput,
                ))
            })
            .collect();
        v.sort_unstable_by_key(|t| t.0);
        v
    };

    if wireless_nodes.is_empty() {
        return;
    }

    // For each wireless node, compute effective throughput:
    // 1. Carrier aggregation: sum bandwidth contributions from all assigned bands
    // 2. Interference penalty: for each assigned band, count same-band neighbors
    //    within interference radius and apply 0.85^count penalty to that band's contribution
    let mut effective_throughput: HashMap<u64, f64> = HashMap::new();

    for &(node_id, _, lon, lat, ref bands, base_throughput) in &wireless_nodes {
        if bands.is_empty() {
            // No bands assigned: operate at 50% throughput (existing behavior)
            effective_throughput.insert(node_id, base_throughput * 0.5);
            continue;
        }

        // Carrier aggregation: compute combined capacity from all assigned bands.
        // Each band contributes a fraction of its max_bandwidth_mhz relative to the
        // node's base throughput, scaled by a normalized bandwidth factor.
        let mut total_effective = 0.0;

        for band_name in bands {
            let band = match FrequencyBand::from_name(band_name) {
                Some(b) => b,
                None => continue,
            };

            // Per-band capacity contribution:
            // base_throughput * (band_bandwidth / reference_bandwidth)
            // Reference bandwidth = 100 MHz (Band3500MHz), so a Band700MHz (45 MHz)
            // contributes ~45% of base, while Band39GHz (1000 MHz) contributes ~1000%.
            // This makes carrier aggregation with multiple bands genuinely beneficial.
            let bandwidth_factor = band.max_bandwidth_mhz() / 100.0;
            let band_capacity = base_throughput * bandwidth_factor;

            // Count same-band interferers within interference radius
            let interference_radius_km = interference_radius_for_band(&band);
            
            // Optimization: Use spatial index to find nearby nodes
            // Convert radius_km to approximate degrees for RTree query
            let deg_range = interference_radius_km / 111.0;
            let envelope = rstar::AABB::from_corners(
                [lon - deg_range * 2.0, lat - deg_range],
                [lon + deg_range * 2.0, lat + deg_range]
            );
            
            let mut interferer_count = 0;
            for spatial_node in world.spatial_index.locate_in_envelope(&envelope) {
                if spatial_node.id == node_id { continue; }
                
                // Check if this candidate node uses the same band
                if let Some(other_node) = world.infra_nodes.get(&spatial_node.id) {
                    if other_node.assigned_bands.contains(band_name) {
                        // Precise distance check
                        let dist = haversine_km(lat, lon, spatial_node.pos[1], spatial_node.pos[0]);
                        if dist <= interference_radius_km {
                            interferer_count += 1;
                        }
                    }
                }
            }

            // Apply diminishing interference penalty: 0.85^interferer_count
            let interference_factor = 0.85_f64.powi(interferer_count as i32);
            total_effective += band_capacity * interference_factor;
        }

        effective_throughput.insert(node_id, total_effective);
    }

    // Apply effective throughput to Capacity components
    for (node_id, throughput) in effective_throughput {
        if let Some(cap) = world.capacities.get_mut(&node_id) {
            // Only reduce — don't increase beyond the health-adjusted value already set
            // by utilization::reset_node_effective_throughput. Take the minimum of
            // the existing (health-adjusted) value and our spectrum-computed value.
            cap.max_throughput = cap.max_throughput.min(throughput);
        }
    }
}

/// Interference radius in km for a given frequency band.
/// Lower frequencies propagate further and have larger interference zones.
fn interference_radius_for_band(band: &FrequencyBand) -> f64 {
    match band {
        // Low band: wide interference radius
        FrequencyBand::Band700MHz => 50.0,
        FrequencyBand::Band800MHz => 40.0,
        FrequencyBand::Band900MHz => 35.0,
        // Mid band: moderate interference
        FrequencyBand::Band1800MHz => 15.0,
        FrequencyBand::Band2100MHz => 12.0,
        FrequencyBand::Band2600MHz => 8.0,
        // High band / mmWave: short interference radius
        FrequencyBand::Band3500MHz => 4.0,
        FrequencyBand::Band28GHz => 1.0,
        FrequencyBand::Band39GHz => 0.5,
        // Satellite bands: wide interference radius due to space-to-ground
        FrequencyBand::BandKu => 100.0,
        FrequencyBand::BandKa => 80.0,
        FrequencyBand::BandV => 60.0,
        FrequencyBand::BandQ => 40.0,
    }
}

fn is_wireless_node(node_type: NodeType) -> bool {
    matches!(
        node_type,
        NodeType::CellTower
            | NodeType::MacroCell
            | NodeType::SmallCell
            | NodeType::WirelessRelay
            | NodeType::MicrowaveTower
            | NodeType::SatelliteGroundStation
            | NodeType::MeshDroneRelay
            | NodeType::TerahertzRelay
    )
}

use gt_common::geo::haversine_km;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interference_radius_ordering() {
        // Lower frequencies should have larger interference radii
        let r700 = interference_radius_for_band(&FrequencyBand::Band700MHz);
        let r1800 = interference_radius_for_band(&FrequencyBand::Band1800MHz);
        let r3500 = interference_radius_for_band(&FrequencyBand::Band3500MHz);
        let r39g = interference_radius_for_band(&FrequencyBand::Band39GHz);
        assert!(r700 > r1800);
        assert!(r1800 > r3500);
        assert!(r3500 > r39g);
    }

    #[test]
    fn test_haversine_same_point() {
        let dist = haversine_km(51.5074, -0.1278, 51.5074, -0.1278);
        assert!(dist.abs() < 0.001);
    }

    #[test]
    fn test_haversine_known_distance() {
        // London to Paris: ~344 km
        let dist = haversine_km(51.5074, -0.1278, 48.8566, 2.3522);
        assert!((dist - 343.5).abs() < 5.0);
    }

    #[test]
    fn test_is_wireless_node() {
        assert!(is_wireless_node(NodeType::CellTower));
        assert!(is_wireless_node(NodeType::MacroCell));
        assert!(is_wireless_node(NodeType::SmallCell));
        assert!(!is_wireless_node(NodeType::DataCenter));
        assert!(!is_wireless_node(NodeType::FiberPOP));
        assert!(!is_wireless_node(NodeType::CentralOffice));
    }

    #[test]
    fn test_diminishing_interference_penalty() {
        // 0 interferers: 0.85^0 = 1.0
        assert!((0.85_f64.powi(0) - 1.0).abs() < f64::EPSILON);
        // 1 interferer: 0.85^1 = 0.85
        assert!((0.85_f64.powi(1) - 0.85).abs() < f64::EPSILON);
        // 2 interferers: 0.85^2 = 0.7225
        assert!((0.85_f64.powi(2) - 0.7225).abs() < 0.0001);
        // 5 interferers: 0.85^5 ~= 0.4437
        assert!((0.85_f64.powi(5) - 0.4437).abs() < 0.001);
    }
}
