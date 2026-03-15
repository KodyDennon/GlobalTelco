//! Typed array builders for hot-path rendering (infra nodes, edges, satellites).

use gt_common::types::EntityId;
use gt_simulation::components::infra_edge::DeploymentMethod;
use gt_simulation::world::GameWorld;

pub fn build_infra_arrays(world: &GameWorld) -> crate::InfraArrays {
    let ids: Vec<EntityId> = world.infra_nodes.keys().copied().collect();
    build_infra_arrays_from_ids(world, &ids)
}

pub fn build_infra_arrays_viewport(
    world: &GameWorld,
    west: f64,
    south: f64,
    east: f64,
    north: f64,
    min_level: u8,
) -> crate::InfraArrays {
    let aabb = rstar::AABB::from_corners([west, south], [east, north]);
    let ids: Vec<EntityId> = world
        .spatial_index
        .locate_in_envelope(&aabb)
        .filter(|sn| {
            if let Some(node) = world.infra_nodes.get(&sn.id) {
                node.network_level as u8 >= min_level
            } else {
                false
            }
        })
        .map(|sn| sn.id)
        .collect();
    build_infra_arrays_from_ids(world, &ids)
}

fn build_infra_arrays_from_ids(world: &GameWorld, ids_list: &[EntityId]) -> crate::InfraArrays {
    let count = ids_list.len();
    let mut ids = Vec::with_capacity(count);
    let mut owners = Vec::with_capacity(count);
    let mut positions = Vec::with_capacity(count * 2);
    let mut stats = Vec::with_capacity(count * 3);
    let mut node_types = Vec::with_capacity(count);
    let mut network_levels = Vec::with_capacity(count);
    let mut construction_flags = Vec::with_capacity(count);
    let mut cell_indices = Vec::with_capacity(count);

    for &eid in ids_list {
        let node = match world.infra_nodes.get(&eid) {
            Some(n) => n,
            None => continue,
        };
        ids.push(eid as u32);
        let owner = world.ownerships.get(&eid).map(|o| o.owner).unwrap_or(0);
        owners.push(owner as u32);
        let pos = world.positions.get(&eid);
        positions.push(pos.map(|p| p.x).unwrap_or(0.0));
        positions.push(pos.map(|p| p.y).unwrap_or(0.0));
        let health = world.healths.get(&eid).map(|h| h.condition).unwrap_or(1.0);
        let utilization = world
            .capacities
            .get(&eid)
            .map(|c| c.utilization())
            .unwrap_or(0.0);
        stats.push(health);
        stats.push(utilization);
        stats.push(node.max_throughput);
        node_types.push(node.node_type as u8);
        network_levels.push(node.network_level as u32);
        construction_flags.push(if world.constructions.contains_key(&eid) {
            1u8
        } else {
            0u8
        });
        cell_indices.push(node.cell_index as u32);
    }

    crate::InfraArrays {
        ids,
        owners,
        positions,
        stats,
        node_types,
        network_levels,
        construction_flags,
        cell_indices,
    }
}

pub fn build_edge_arrays(world: &GameWorld) -> crate::EdgeArrays {
    let ids: Vec<EntityId> = world.infra_edges.keys().copied().collect();
    build_edge_arrays_from_ids(world, &ids)
}

pub fn build_edge_arrays_viewport(
    world: &GameWorld,
    west: f64,
    south: f64,
    east: f64,
    north: f64,
    min_level: u8,
) -> crate::EdgeArrays {
    // Find edges where either endpoint is in the viewport AND tier is sufficient
    let aabb = rstar::AABB::from_corners([west, south], [east, north]);
    let visible_node_ids: std::collections::HashSet<EntityId> = world
        .spatial_index
        .locate_in_envelope(&aabb)
        .map(|sn| sn.id)
        .collect();

    let ids: Vec<EntityId> = world
        .infra_edges
        .iter()
        .filter(|(_, e)| {
            // Tier check: at least one endpoint must meet the tier requirement
            let node_a = world.infra_nodes.get(&e.source);
            let node_b = world.infra_nodes.get(&e.target);
            if let (Some(na), Some(nb)) = (node_a, node_b) {
                if (na.network_level as u8) < min_level && (nb.network_level as u8) < min_level {
                    return false;
                }
            }

            visible_node_ids.contains(&e.source) || visible_node_ids.contains(&e.target)
        })
        .map(|(&id, _)| id)
        .collect();

    build_edge_arrays_from_ids(world, &ids)
}

fn build_edge_arrays_from_ids(world: &GameWorld, ids_list: &[EntityId]) -> crate::EdgeArrays {
    let count = ids_list.len();
    let mut ids = Vec::with_capacity(count);
    let mut owners = Vec::with_capacity(count);
    let mut endpoints = Vec::with_capacity(count * 4);
    let mut stats = Vec::with_capacity(count * 2);
    let mut edge_types = Vec::with_capacity(count);
    let mut deployment_types = Vec::with_capacity(count);
    let mut waypoints_data = Vec::with_capacity(count * 4);
    let mut waypoint_offsets = Vec::with_capacity(count);
    let mut waypoint_lengths = Vec::with_capacity(count);

    for &eid in ids_list {
        let edge = match world.infra_edges.get(&eid) {
            Some(e) => e,
            None => continue,
        };
        ids.push(eid as u32);
        let owner = world.ownerships.get(&eid).map(|o| o.owner).unwrap_or(0);
        owners.push(owner as u32);
        let src = world.positions.get(&edge.source);
        let dst = world.positions.get(&edge.target);
        endpoints.push(src.map(|p| p.x).unwrap_or(0.0));
        endpoints.push(src.map(|p| p.y).unwrap_or(0.0));
        endpoints.push(dst.map(|p| p.x).unwrap_or(0.0));
        endpoints.push(dst.map(|p| p.y).unwrap_or(0.0));
        stats.push(edge.bandwidth);
        let utilization = world
            .capacities
            .get(&eid)
            .map(|c| c.utilization())
            .unwrap_or(0.0);
        stats.push(utilization);
        edge_types.push(edge.edge_type as u8);

        deployment_types.push(match edge.deployment {
            DeploymentMethod::Underground => 0,
            DeploymentMethod::Aerial => 1,
        });

        waypoint_offsets.push(waypoints_data.len() as u32);
        let len = edge.waypoints.len().min(255);
        waypoint_lengths.push(len as u8);
        for &(lon, lat) in edge.waypoints.iter().take(len) {
            waypoints_data.push(lon);
            waypoints_data.push(lat);
        }
    }

    crate::EdgeArrays {
        ids,
        owners,
        endpoints,
        stats,
        edge_types,
        deployment_types,
        waypoints_data,
        waypoint_offsets,
        waypoint_lengths,
    }
}

pub fn build_satellite_arrays(world: &GameWorld) -> crate::SatelliteArrays {
    let count = world.satellites.len();
    let mut ids = Vec::with_capacity(count);
    let mut owners = Vec::with_capacity(count);
    let mut positions = Vec::with_capacity(count * 2);
    let mut altitudes = Vec::with_capacity(count);
    let mut orbit_types = Vec::with_capacity(count);
    let mut statuses = Vec::with_capacity(count);
    let mut fuel_levels = Vec::with_capacity(count);

    for (&eid, sat) in &world.satellites {
        ids.push(eid as u32);
        let owner = world.ownerships.get(&eid).map(|o| o.owner).unwrap_or(0);
        owners.push(owner as u32);
        let pos = world.positions.get(&eid);
        positions.push(pos.map(|p| p.x).unwrap_or(0.0));
        positions.push(pos.map(|p| p.y).unwrap_or(0.0));
        altitudes.push(sat.altitude_km);
        orbit_types.push(sat.orbit_type as u32);
        statuses.push(sat.status as u32);
        let fuel_frac = if sat.fuel_capacity > 0.0 {
            sat.fuel_remaining / sat.fuel_capacity
        } else {
            0.0
        };
        fuel_levels.push(fuel_frac);
    }

    crate::SatelliteArrays {
        ids,
        owners,
        positions,
        altitudes,
        orbit_types,
        statuses,
        fuel_levels,
    }
}
