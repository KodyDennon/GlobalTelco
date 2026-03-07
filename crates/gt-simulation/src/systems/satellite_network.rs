use std::collections::HashMap;

use gt_common::types::{EdgeType, EntityId, SatelliteStatus};

use crate::components::infra_edge::{DeploymentMethod, InfraEdge};
use crate::components::Ownership;
use crate::world::GameWorld;

/// Satellite network system — rebuilds dynamic ISL + downlink edges each tick.
/// Runs after orbital, before maintenance.
pub fn run(world: &mut GameWorld) {
    // Remove all previous dynamic satellite edges
    let old_edges: Vec<EntityId> = world.dynamic_satellite_edges.drain(..).collect();
    for edge_id in old_edges {
        // Look up source/target before removing
        if let Some(edge) = world.infra_edges.shift_remove(&edge_id) {
            world.network.remove_edge(edge.source, edge.target);
        }
        // Fix: Also remove ownership component to prevent leak
        world.ownerships.shift_remove(&edge_id);
    }

    // Collect operational satellites
    let operational_sats: Vec<(EntityId, f64, f64, f64, EntityId, u32, u32)> = world
        .satellites
        .iter()
        .filter(|(_, s)| s.status == SatelliteStatus::Operational)
        .map(|(id, s)| {
            let pos = world.positions.get(id).cloned().unwrap_or_default();
            (
                *id,
                pos.x,  // lon
                pos.y,  // lat
                s.altitude_km,
                s.constellation_id,
                s.plane_index,
                s.index_in_plane,
            )
        })
        .collect();

    // Build ISL links (intra-plane: connect to 2 nearest neighbors in same plane)
    build_intraplane_isl(world, &operational_sats);

    // Build cross-plane ISL if research completed
    let has_crossplane = has_crossplane_research(world);
    if has_crossplane {
        build_crossplane_isl(world, &operational_sats);
    }

    // Build downlinks to ground stations
    build_downlinks(world, &operational_sats);
}

fn build_intraplane_isl(
    world: &mut GameWorld,
    sats: &[(EntityId, f64, f64, f64, EntityId, u32, u32)],
) {
    // Group sats by (constellation_id, plane_index)
    let mut planes: HashMap<(EntityId, u32), Vec<(EntityId, u32)>> = HashMap::new();
    for (id, _, _, _, constellation_id, plane_idx, idx_in_plane) in sats {
        if *constellation_id == 0 {
            continue;
        }
        planes
            .entry((*constellation_id, *plane_idx))
            .or_default()
            .push((*id, *idx_in_plane));
    }

    for (_, mut plane_sats) in planes {
        if plane_sats.len() < 2 {
            continue;
        }
        plane_sats.sort_by_key(|(_, idx)| *idx);

        // Connect each sat to its neighbors in the ring
        for i in 0..plane_sats.len() {
            let next_i = (i + 1) % plane_sats.len();
            let (sat_a, _) = plane_sats[i];
            let (sat_b, _) = plane_sats[next_i];

            create_dynamic_edge(world, sat_a, sat_b, EdgeType::IntraplaneISL);
        }
    }
}

/// Spatial grid cell size in degrees for cross-plane ISL lookup.
/// 5000km max range ≈ ~45 degrees at equator. Use 20-degree cells for
/// a balance between grid resolution and neighbor search radius.
const SPATIAL_GRID_DEG: f64 = 20.0;

fn spatial_key(lon: f64, lat: f64) -> (i32, i32) {
    (
        (lon / SPATIAL_GRID_DEG).floor() as i32,
        (lat / SPATIAL_GRID_DEG).floor() as i32,
    )
}

fn build_crossplane_isl(
    world: &mut GameWorld,
    sats: &[(EntityId, f64, f64, f64, EntityId, u32, u32)],
) {
    // Group sats by constellation_id
    let mut constellations: HashMap<EntityId, Vec<(EntityId, f64, f64, u32)>> = HashMap::new();
    for (id, lon, lat, _, constellation_id, plane_idx, _) in sats {
        if *constellation_id == 0 {
            continue;
        }
        constellations
            .entry(*constellation_id)
            .or_default()
            .push((*id, *lon, *lat, *plane_idx));
    }

    for (_, constellation_sats) in constellations {
        // Build spatial hash grid for this constellation
        let mut grid: HashMap<(i32, i32), Vec<usize>> = HashMap::new();
        for (idx, (_, lon, lat, _)) in constellation_sats.iter().enumerate() {
            let key = spatial_key(*lon, *lat);
            grid.entry(key).or_default().push(idx);
        }

        // For each sat, find nearest sat in adjacent plane using spatial grid
        for i in 0..constellation_sats.len() {
            let (sat_a, lon_a, lat_a, plane_a) = constellation_sats[i];
            let key = spatial_key(lon_a, lat_a);

            let mut best_dist = f64::MAX;
            let mut best_id = None;

            // Check 3x3 neighborhood of grid cells (covers ~60 degrees = well beyond 5000km)
            for dx in -1..=1 {
                for dy in -1..=1 {
                    let neighbor_key = (key.0 + dx, key.1 + dy);
                    if let Some(indices) = grid.get(&neighbor_key) {
                        for &j in indices {
                            if i == j {
                                continue;
                            }
                            let (sat_b, lon_b, lat_b, plane_b) = constellation_sats[j];

                            // Only connect to adjacent planes
                            if plane_b != plane_a + 1 && plane_a != plane_b + 1 {
                                continue;
                            }

                            let dist = haversine_distance_km(lon_a, lat_a, lon_b, lat_b);
                            if dist < best_dist {
                                best_dist = dist;
                                best_id = Some(sat_b);
                            }
                        }
                    }
                }
            }

            // Max cross-plane ISL range: 5000km
            if let Some(target) = best_id {
                if best_dist < 5000.0 {
                    create_dynamic_edge(world, sat_a, target, EdgeType::CrossplaneISL);
                }
            }
        }
    }
}

fn build_downlinks(
    world: &mut GameWorld,
    sats: &[(EntityId, f64, f64, f64, EntityId, u32, u32)],
) {
    // Collect ground stations
    let ground_stations: Vec<(EntityId, f64, f64, EntityId)> = world
        .infra_nodes
        .iter()
        .filter(|(_, node)| node.node_type.is_satellite_ground_station())
        .map(|(id, _node)| {
            let pos = world.positions.get(id).cloned().unwrap_or_default();
            let owner = world.ownerships.get(id).map(|o| o.owner).unwrap_or(0);
            (*id, pos.x, pos.y, owner)
        })
        .collect();

    if ground_stations.is_empty() {
        return;
    }

    for (sat_id, sat_lon, sat_lat, sat_alt, _, _, _) in sats {
        let sat_owner = world.ownerships.get(sat_id).map(|o| o.owner).unwrap_or(0);

        // Find nearest owned or allied ground station within line-of-sight
        let max_downlink_range = footprint_range_km(*sat_alt);

        let mut best_dist = f64::MAX;
        let mut best_gs = None;

        for (gs_id, gs_lon, gs_lat, gs_owner) in &ground_stations {
            // Must be owned by same corp or allied
            if *gs_owner != sat_owner && !are_allied(world, sat_owner, *gs_owner) {
                continue;
            }

            let dist = haversine_distance_km(*sat_lon, *sat_lat, *gs_lon, *gs_lat);
            if dist < max_downlink_range && dist < best_dist {
                best_dist = dist;
                best_gs = Some(*gs_id);
            }
        }

        if let Some(gs_id) = best_gs {
            let edge_type = match world.satellites.get(sat_id) {
                Some(s) => match s.orbit_type {
                    gt_common::types::OrbitType::GEO => EdgeType::GEO_GroundLink,
                    gt_common::types::OrbitType::MEO => EdgeType::MEO_GroundLink,
                    _ => EdgeType::SatelliteDownlink,
                },
                None => EdgeType::SatelliteDownlink,
            };
            create_dynamic_edge(world, *sat_id, gs_id, edge_type);
        }
    }
}

fn create_dynamic_edge(world: &mut GameWorld, from: EntityId, to: EntityId, edge_type: EdgeType) {
    let edge_id = world.allocate_entity();
    let owner = world.ownerships.get(&from).map(|o| o.owner).unwrap_or(0);

    let edge = InfraEdge {
        edge_type,
        source: from,
        target: to,
        bandwidth: edge_type.bandwidth() as f64,
        current_load: 0.0,
        latency_ms: 0.0,
        length_km: 0.0,
        construction_cost: 0,
        maintenance_cost: 0,
        owner,
        health: 1.0,
        waypoints: Vec::new(),
        deployment: DeploymentMethod::Aerial,
        repairing: false,
        repair_ticks_left: 0,
        repair_health_per_tick: 0.0,
        last_damage_tick: None,
        revenue_generated: 0,
    };

    world.infra_edges.insert(edge_id, edge);
    world
        .ownerships
        .entry(edge_id)
        .or_insert(Ownership {
            owner,
            co_owners: Vec::new(),
        });

    // Add to network graph
    world.network.add_edge_with_id(from, to, edge_id);

    // Mark connected nodes dirty for routing
    world.network.invalidate_node(from);
    world.network.invalidate_node(to);

    world.dynamic_satellite_edges.push(edge_id);
}

fn footprint_range_km(altitude_km: f64) -> f64 {
    // Simplified: max ground distance for a downlink at 10 degree min elevation
    let r_e = 6371.0;
    let h = altitude_km;
    let elev_rad = 10.0_f64.to_radians();
    let _rho = (r_e / (r_e + h)).acos();
    let eta = (r_e / (r_e + h) * elev_rad.cos()).acos() - elev_rad;
    let lambda = std::f64::consts::FRAC_PI_2 - eta - elev_rad;
    lambda * r_e
}

fn haversine_distance_km(lon1: f64, lat1: f64, lon2: f64, lat2: f64) -> f64 {
    let r = 6371.0;
    let dlat = (lat2 - lat1).to_radians();
    let dlon = (lon2 - lon1).to_radians();
    let a = (dlat / 2.0).sin().powi(2)
        + lat1.to_radians().cos() * lat2.to_radians().cos() * (dlon / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().asin();
    r * c
}

fn are_allied(world: &GameWorld, corp_a: EntityId, corp_b: EntityId) -> bool {
    world.alliances.values().any(|alliance| {
        alliance.member_corp_ids.contains(&corp_a) && alliance.member_corp_ids.contains(&corp_b)
    })
}

fn has_crossplane_research(world: &GameWorld) -> bool {
    // Check if any corporation has completed the cross-plane ISL research
    world.tech_research.values().any(|tr| {
        tr.name == "CrossPlaneISL"
            && tr.progress >= tr.total_cost as f64
    })
}
