//! FTTH (Fiber To The Home) coverage system.
//!
//! Validates the access network hierarchy for each NAP (NetworkAccessPoint).
//! A NAP is marked as "active_ftth" when it has a valid chain back to a CO:
//!   CO <--FeederFiber--> FDH <--DistributionFiber--> NAP
//!
//! Active NAPs auto-cover nearby buildings (cells with city population) within
//! their terrain-dependent service radius. Coverage contributions are added to
//! `cell_coverage` so other systems (revenue, demand) can use them.
//!
//! Service radius by terrain:
//!   - Urban / Dense urban: 2 km (short runs, dense subscriber base)
//!   - Suburban: 5 km (typical FTTH neighborhood serving area)
//!   - Rural: 10 km (long drop cables, sparse subscribers)
//!   - Other: 3 km default

use std::collections::{HashMap, HashSet};

use gt_common::types::{EdgeType, EntityId, NodeType, TerrainType};

use crate::world::GameWorld;

/// FTTH service radius in km based on terrain type at the NAP location.
/// Denser terrain means shorter radius but higher subscriber density.
pub fn ftth_service_radius_km(terrain: Option<TerrainType>) -> f64 {
    match terrain {
        Some(TerrainType::Urban) => 2.0,
        Some(TerrainType::Suburban) => 5.0,
        Some(TerrainType::Rural) => 10.0,
        Some(TerrainType::Desert) => 8.0,
        Some(TerrainType::Coastal) => 4.0,
        Some(TerrainType::Mountainous) => 3.0,
        Some(TerrainType::Tundra) => 6.0,
        Some(TerrainType::Frozen) => 4.0,
        // Ocean/no terrain — NAPs shouldn't be placed here, but handle gracefully
        Some(TerrainType::OceanShallow | TerrainType::OceanDeep | TerrainType::OceanTrench) => 1.0,
        None => 3.0,
    }
}

/// Run the FTTH system: validate NAP chains, mark active NAPs, and contribute
/// FTTH coverage to `cell_coverage`.
///
/// Called each tick after coverage and before revenue in the system order.
pub fn run(world: &mut GameWorld) {
    // Step 1: Build adjacency structures for chain validation.
    // We need to trace: NAP <-DistributionFiber-> FDH <-FeederFiber-> CO

    // Collect all operational (non-under-construction) nodes by type
    let mut co_nodes: HashSet<EntityId> = HashSet::new();
    let mut fdh_nodes: HashSet<EntityId> = HashSet::new();
    let mut nap_node_ids: Vec<EntityId> = Vec::new();

    for (&id, node) in &world.infra_nodes {
        // Skip nodes still under construction
        if world.constructions.contains_key(&id) {
            continue;
        }
        match node.node_type {
            // Any Central Office type qualifies as a CO for the FTTH chain
            NodeType::CentralOffice => {
                co_nodes.insert(id);
            }
            NodeType::FiberDistributionHub => {
                fdh_nodes.insert(id);
            }
            NodeType::NetworkAccessPoint => {
                nap_node_ids.push(id);
            }
            _ => {}
        }
    }

    // Sort NAP IDs for deterministic processing
    nap_node_ids.sort_unstable();

    // Build edge adjacency maps filtered by edge type.
    // distribution_fiber_adj: node_id -> set of connected node_ids via DistributionFiber
    // feeder_fiber_adj: node_id -> set of connected node_ids via FeederFiber
    let mut distribution_fiber_adj: HashMap<EntityId, Vec<EntityId>> = HashMap::new();
    let mut feeder_fiber_adj: HashMap<EntityId, Vec<EntityId>> = HashMap::new();

    for edge in world.infra_edges.values() {
        match edge.edge_type {
            EdgeType::DistributionFiber => {
                distribution_fiber_adj
                    .entry(edge.source)
                    .or_default()
                    .push(edge.target);
                distribution_fiber_adj
                    .entry(edge.target)
                    .or_default()
                    .push(edge.source);
            }
            EdgeType::FeederFiber => {
                feeder_fiber_adj
                    .entry(edge.source)
                    .or_default()
                    .push(edge.target);
                feeder_fiber_adj
                    .entry(edge.target)
                    .or_default()
                    .push(edge.source);
            }
            _ => {}
        }
    }

    // Step 2: For each FDH, check if it connects to any CO via FeederFiber.
    // An FDH is "backhaul-valid" if any of its FeederFiber neighbors is a CO.
    let mut valid_fdhs: HashSet<EntityId> = HashSet::new();
    for &fdh_id in &fdh_nodes {
        if let Some(neighbors) = feeder_fiber_adj.get(&fdh_id) {
            for &neighbor_id in neighbors {
                if co_nodes.contains(&neighbor_id) {
                    valid_fdhs.insert(fdh_id);
                    break;
                }
            }
        }
    }

    // Step 3: For each NAP, check if it connects to a valid FDH via DistributionFiber.
    // A NAP is "active_ftth" if any of its DistributionFiber neighbors is a valid FDH.
    let mut active_naps: HashSet<EntityId> = HashSet::new();
    for &nap_id in &nap_node_ids {
        if let Some(neighbors) = distribution_fiber_adj.get(&nap_id) {
            for &neighbor_id in neighbors {
                if valid_fdhs.contains(&neighbor_id) {
                    active_naps.insert(nap_id);
                    break;
                }
            }
        }
    }

    // Step 4: Update the active_ftth flag on all NAP nodes.
    // Also clear the flag on any non-NAP nodes that might have it set from a previous tick.
    for (&id, node) in world.infra_nodes.iter_mut() {
        if node.node_type == NodeType::NetworkAccessPoint {
            node.active_ftth = active_naps.contains(&id);
        } else {
            // Ensure non-NAP nodes never have the flag set
            node.active_ftth = false;
        }
    }

    // Step 5: Active NAPs contribute FTTH coverage to nearby cells within their
    // terrain-dependent service radius. This integrates with the existing
    // CellCoverage system so revenue and demand systems pick it up.

    // Build terrain lookup: cell_index → TerrainType
    let cell_terrain: HashMap<usize, TerrainType> = world
        .land_parcels
        .values()
        .map(|p| (p.cell_index, p.terrain))
        .collect();

    // Collect active NAP data: (nap_id, cell_index, owner, health, terrain)
    // We re-read from world after Step 4 since we just updated active_ftth.
    let mut active_nap_data: Vec<(EntityId, usize, EntityId, f64, Option<TerrainType>)> = Vec::new();
    for &nap_id in &nap_node_ids {
        let node = match world.infra_nodes.get(&nap_id) {
            Some(n) if n.active_ftth => n,
            _ => continue,
        };
        let health = world
            .healths
            .get(&nap_id)
            .map(|h| h.condition)
            .unwrap_or(1.0);
        let terrain = cell_terrain.get(&node.cell_index).copied();
        active_nap_data.push((nap_id, node.cell_index, node.owner, health, terrain));
    }

    // Sort for deterministic coverage accumulation
    active_nap_data.sort_unstable_by_key(|t| t.0);

    let cell_spacing = world.cell_spacing_km;

    for &(_nap_id, nap_cell, owner, health, terrain) in &active_nap_data {
        if health <= 0.0 {
            continue;
        }

        // Terrain-dependent service radius, scaled to be meaningful at grid resolution
        let base_radius_km = ftth_service_radius_km(terrain);
        let radius_km = base_radius_km.max(cell_spacing * 0.8);

        // FTTH bandwidth contribution: wired fiber delivers high bandwidth
        // NAP throughput scaled by health
        let nap_throughput = world
            .infra_nodes
            .get(&_nap_id)
            .map(|n| n.max_throughput)
            .unwrap_or(100.0);
        let ftth_bandwidth = nap_throughput * health;

        let nap_pos = match world.grid_cell_positions.get(nap_cell) {
            Some(p) => *p,
            None => continue,
        };
        let (nap_lat, nap_lon) = nap_pos;
        let lat_range = radius_km / 111.0;
        let cos_lat = (nap_lat.to_radians()).cos().max(0.1);
        let lon_range = radius_km / (111.0 * cos_lat);

        for (cell_idx, &(cell_lat, cell_lon)) in world.grid_cell_positions.iter().enumerate() {
            // Bounding box check
            if (cell_lat - nap_lat).abs() > lat_range || (cell_lon - nap_lon).abs() > lon_range {
                continue;
            }

            // Only cover cells belonging to a city (buildings exist there)
            if !world.cell_to_city.contains_key(&cell_idx) {
                continue;
            }

            // Haversine distance check
            let dist_km = haversine_km(nap_lat, nap_lon, cell_lat, cell_lon);
            if dist_km > radius_km {
                continue;
            }

            // Signal attenuation: linear falloff for wired FTTH (less severe than wireless)
            let distance_ratio = dist_km / radius_km;
            let attenuation = (1.0 - distance_ratio).max(0.0);
            let signal = ftth_bandwidth * attenuation;
            let bandwidth = ftth_bandwidth * attenuation;

            if signal < 0.01 {
                continue;
            }

            // Add FTTH coverage to the cell
            let entry = world
                .cell_coverage
                .entry(cell_idx)
                .or_default();
            entry.signal_strength += signal;
            entry.bandwidth += bandwidth;
            entry.node_count += 1;
            entry.add_corp_bandwidth(owner, bandwidth);
            if signal > entry.best_signal {
                entry.best_signal = signal;
                entry.dominant_owner = Some(owner);
            }
        }
    }
}

/// Haversine distance between two lat/lon points in km.
fn haversine_km(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let dlat = (lat1 - lat2).to_radians();
    let dlon = (lon1 - lon2).to_radians();
    let a = (dlat / 2.0).sin().powi(2)
        + lat1.to_radians().cos() * lat2.to_radians().cos() * (dlon / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().asin();
    6371.0 * c
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::InfraEdge;
    use crate::components::InfraNode;

    /// Helper: create a minimal GameWorld for testing.
    fn make_test_world() -> GameWorld {
        let config = gt_common::types::WorldConfig {
            seed: 42,
            map_size: gt_common::types::MapSize::Small,
            starting_era: gt_common::types::Era::Modern,
            difficulty: gt_common::types::DifficultyPreset::Normal,
            ai_corporations: 0,
            use_real_earth: false,
            corp_name: Some("TestCo".to_string()),
            continent_count: 2,
            ocean_percentage: 0.5,
            terrain_roughness: 0.5,
            climate_variation: 0.5,
            city_density: 0.5,
            disaster_frequency: 1.0,
            sandbox: false,
            max_ai_corporations: 8,
        };
        // Build a bare-bones world — we only need infra_nodes, infra_edges, constructions
        let mut world = GameWorld::new(config);
        // Clear generated data so we control the test precisely
        world.infra_nodes.clear();
        world.infra_edges.clear();
        world.constructions.clear();
        world
    }

    #[test]
    fn test_valid_ftth_chain_activates_nap() {
        let mut world = make_test_world();
        let corp = 1000;

        // Create CO -> FeederFiber -> FDH -> DistributionFiber -> NAP
        let co_id = 10;
        let fdh_id = 20;
        let nap_id = 30;

        world
            .infra_nodes
            .insert(co_id, InfraNode::new(NodeType::CentralOffice, 0, corp));
        world.infra_nodes.insert(
            fdh_id,
            InfraNode::new(NodeType::FiberDistributionHub, 1, corp),
        );
        world.infra_nodes.insert(
            nap_id,
            InfraNode::new(NodeType::NetworkAccessPoint, 2, corp),
        );

        // FeederFiber: CO <-> FDH
        let feeder_edge_id = 100;
        world.infra_edges.insert(
            feeder_edge_id,
            InfraEdge::new(EdgeType::FeederFiber, co_id, fdh_id, 1.0, corp),
        );

        // DistributionFiber: FDH <-> NAP
        let dist_edge_id = 101;
        world.infra_edges.insert(
            dist_edge_id,
            InfraEdge::new(EdgeType::DistributionFiber, fdh_id, nap_id, 0.5, corp),
        );

        run(&mut world);

        assert!(
            world.infra_nodes.get(&nap_id).unwrap().active_ftth,
            "NAP with valid CO->FDH->NAP chain should be active_ftth"
        );
    }

    #[test]
    fn test_missing_co_does_not_activate_nap() {
        let mut world = make_test_world();
        let corp = 1000;

        // FDH -> DistributionFiber -> NAP, but no CO connected
        let fdh_id = 20;
        let nap_id = 30;

        world.infra_nodes.insert(
            fdh_id,
            InfraNode::new(NodeType::FiberDistributionHub, 1, corp),
        );
        world.infra_nodes.insert(
            nap_id,
            InfraNode::new(NodeType::NetworkAccessPoint, 2, corp),
        );

        let dist_edge_id = 101;
        world.infra_edges.insert(
            dist_edge_id,
            InfraEdge::new(EdgeType::DistributionFiber, fdh_id, nap_id, 0.5, corp),
        );

        run(&mut world);

        assert!(
            !world.infra_nodes.get(&nap_id).unwrap().active_ftth,
            "NAP without CO in chain should NOT be active_ftth"
        );
    }

    #[test]
    fn test_wrong_edge_type_does_not_activate_nap() {
        let mut world = make_test_world();
        let corp = 1000;

        let co_id = 10;
        let fdh_id = 20;
        let nap_id = 30;

        world
            .infra_nodes
            .insert(co_id, InfraNode::new(NodeType::CentralOffice, 0, corp));
        world.infra_nodes.insert(
            fdh_id,
            InfraNode::new(NodeType::FiberDistributionHub, 1, corp),
        );
        world.infra_nodes.insert(
            nap_id,
            InfraNode::new(NodeType::NetworkAccessPoint, 2, corp),
        );

        // Use wrong edge type: FiberLocal instead of FeederFiber
        world.infra_edges.insert(
            100,
            InfraEdge::new(EdgeType::FiberLocal, co_id, fdh_id, 1.0, corp),
        );

        // Correct distribution fiber
        world.infra_edges.insert(
            101,
            InfraEdge::new(EdgeType::DistributionFiber, fdh_id, nap_id, 0.5, corp),
        );

        run(&mut world);

        assert!(
            !world.infra_nodes.get(&nap_id).unwrap().active_ftth,
            "NAP with wrong edge type (FiberLocal instead of FeederFiber) should NOT be active_ftth"
        );
    }

    #[test]
    fn test_under_construction_co_does_not_validate() {
        let mut world = make_test_world();
        let corp = 1000;

        let co_id = 10;
        let fdh_id = 20;
        let nap_id = 30;

        world
            .infra_nodes
            .insert(co_id, InfraNode::new(NodeType::CentralOffice, 0, corp));
        world.infra_nodes.insert(
            fdh_id,
            InfraNode::new(NodeType::FiberDistributionHub, 1, corp),
        );
        world.infra_nodes.insert(
            nap_id,
            InfraNode::new(NodeType::NetworkAccessPoint, 2, corp),
        );

        // CO is under construction
        world.constructions.insert(
            co_id,
            crate::components::Construction::new(0, 10),
        );

        world.infra_edges.insert(
            100,
            InfraEdge::new(EdgeType::FeederFiber, co_id, fdh_id, 1.0, corp),
        );
        world.infra_edges.insert(
            101,
            InfraEdge::new(EdgeType::DistributionFiber, fdh_id, nap_id, 0.5, corp),
        );

        run(&mut world);

        assert!(
            !world.infra_nodes.get(&nap_id).unwrap().active_ftth,
            "NAP with under-construction CO should NOT be active_ftth"
        );
    }

    #[test]
    fn test_nap_flag_clears_when_chain_breaks() {
        let mut world = make_test_world();
        let corp = 1000;

        let co_id = 10;
        let fdh_id = 20;
        let nap_id = 30;

        world
            .infra_nodes
            .insert(co_id, InfraNode::new(NodeType::CentralOffice, 0, corp));
        world.infra_nodes.insert(
            fdh_id,
            InfraNode::new(NodeType::FiberDistributionHub, 1, corp),
        );
        world.infra_nodes.insert(
            nap_id,
            InfraNode::new(NodeType::NetworkAccessPoint, 2, corp),
        );

        world.infra_edges.insert(
            100,
            InfraEdge::new(EdgeType::FeederFiber, co_id, fdh_id, 1.0, corp),
        );
        world.infra_edges.insert(
            101,
            InfraEdge::new(EdgeType::DistributionFiber, fdh_id, nap_id, 0.5, corp),
        );

        // First run: should activate
        run(&mut world);
        assert!(world.infra_nodes.get(&nap_id).unwrap().active_ftth);

        // Remove the feeder fiber edge, breaking the chain
        world.infra_edges.shift_remove(&100);


        // Second run: should deactivate
        run(&mut world);
        assert!(
            !world.infra_nodes.get(&nap_id).unwrap().active_ftth,
            "NAP should lose active_ftth when chain breaks"
        );
    }

    #[test]
    fn test_ftth_service_radius_by_terrain() {
        // Urban: compact radius
        assert_eq!(ftth_service_radius_km(Some(TerrainType::Urban)), 2.0);
        // Suburban: medium radius
        assert_eq!(ftth_service_radius_km(Some(TerrainType::Suburban)), 5.0);
        // Rural: wide radius
        assert_eq!(ftth_service_radius_km(Some(TerrainType::Rural)), 10.0);
        // No terrain: default
        assert_eq!(ftth_service_radius_km(None), 3.0);
    }

    #[test]
    fn test_active_nap_contributes_coverage() {
        let mut world = make_test_world();
        let corp = 1000;

        // Set up grid with a cell at the NAP location and a nearby city cell
        world.grid_cell_positions = vec![
            (40.0, -74.0),  // cell 0: CO location
            (40.001, -74.0), // cell 1: FDH location
            (40.002, -74.0), // cell 2: NAP location
            (40.003, -74.0), // cell 3: nearby city cell
        ];
        world.grid_cell_count = 4;
        world.cell_spacing_km = 0.1; // Very close grid for test

        // Mark cell 3 as belonging to a city so FTTH coverage extends there
        let city_id = 500;
        world.cell_to_city.insert(2, city_id);
        world.cell_to_city.insert(3, city_id);

        let co_id = 10;
        let fdh_id = 20;
        let nap_id = 30;

        world
            .infra_nodes
            .insert(co_id, InfraNode::new(NodeType::CentralOffice, 0, corp));
        world.infra_nodes.insert(
            fdh_id,
            InfraNode::new(NodeType::FiberDistributionHub, 1, corp),
        );
        world.infra_nodes.insert(
            nap_id,
            InfraNode::new(NodeType::NetworkAccessPoint, 2, corp),
        );

        world.infra_edges.insert(
            100,
            InfraEdge::new(EdgeType::FeederFiber, co_id, fdh_id, 1.0, corp),
        );
        world.infra_edges.insert(
            101,
            InfraEdge::new(EdgeType::DistributionFiber, fdh_id, nap_id, 0.5, corp),
        );

        run(&mut world);

        // NAP should be active
        assert!(world.infra_nodes.get(&nap_id).unwrap().active_ftth);

        // Cell 2 (NAP cell, in city) should have FTTH coverage
        assert!(
            world.cell_coverage.contains_key(&2),
            "NAP cell (in city) should have FTTH coverage"
        );
        let cov2 = world.cell_coverage.get(&2).unwrap();
        assert!(cov2.bandwidth > 0.0, "NAP cell coverage should have bandwidth");
        assert_eq!(cov2.dominant_owner, Some(corp));

        // Cell 3 (nearby city cell) should also have coverage
        assert!(
            world.cell_coverage.contains_key(&3),
            "Nearby city cell should have FTTH coverage from active NAP"
        );
        let cov3 = world.cell_coverage.get(&3).unwrap();
        assert!(cov3.bandwidth > 0.0, "Nearby city cell should have bandwidth");
    }

    #[test]
    fn test_inactive_nap_does_not_contribute_coverage() {
        let mut world = make_test_world();
        let corp = 1000;

        world.grid_cell_positions = vec![
            (40.0, -74.0),
            (40.001, -74.0),
        ];
        world.grid_cell_count = 2;
        world.cell_spacing_km = 0.1;
        world.cell_to_city.insert(0, 500);

        // NAP without chain — not active
        let nap_id = 30;
        world.infra_nodes.insert(
            nap_id,
            InfraNode::new(NodeType::NetworkAccessPoint, 0, corp),
        );

        run(&mut world);

        assert!(!world.infra_nodes.get(&nap_id).unwrap().active_ftth);
        // No coverage should be contributed
        assert!(
            world.cell_coverage.is_empty(),
            "Inactive NAP should not contribute any coverage"
        );
    }
}
