//! Shared AI helper functions.
//!
//! Utilities used across multiple AI subsystems: node lookup, edge construction,
//! parcel acquisition, distance calculation, and tier-aware edge type selection.

use crate::components::*;
use crate::world::GameWorld;
use gt_common::types::*;

// ─── Node Lookup ─────────────────────────────────────────────────────────────

/// Find the nearest existing node (by cell position) from a set of candidate node IDs.
pub fn find_nearest_node(
    world: &GameWorld,
    node_ids: &[EntityId],
    target_cell: usize,
) -> Option<EntityId> {
    let target_pos = world.grid_cell_positions.get(target_cell)?;

    let mut best: Option<(EntityId, f64)> = None;
    for &nid in node_ids {
        let node = match world.infra_nodes.get(&nid) {
            Some(n) => n,
            None => continue,
        };
        let node_pos = match world.grid_cell_positions.get(node.cell_index) {
            Some(p) => p,
            None => continue,
        };
        let dist = sq_distance(target_pos, node_pos);
        if best.is_none() || dist < best.unwrap().1 {
            best = Some((nid, dist));
        }
    }
    best.map(|(id, _)| id)
}

/// Find the nearest node of a specific tier from a set of node IDs.
pub fn find_nearest_node_of_tier(
    world: &GameWorld,
    node_ids: &[EntityId],
    target_cell: usize,
    tier: NetworkTier,
) -> Option<EntityId> {
    let target_pos = world.grid_cell_positions.get(target_cell)?;

    let mut best: Option<(EntityId, f64)> = None;
    for &nid in node_ids {
        let node = match world.infra_nodes.get(&nid) {
            Some(n) => n,
            None => continue,
        };
        if node.node_type.tier() != tier {
            continue;
        }
        let node_pos = match world.grid_cell_positions.get(node.cell_index) {
            Some(p) => p,
            None => continue,
        };
        let dist = sq_distance(target_pos, node_pos);
        if best.is_none() || dist < best.unwrap().1 {
            best = Some((nid, dist));
        }
    }
    best.map(|(id, _)| id)
}

/// Find nodes at or above a given tier.
pub fn find_nearest_node_at_or_above_tier(
    world: &GameWorld,
    node_ids: &[EntityId],
    target_cell: usize,
    min_tier: NetworkTier,
) -> Option<EntityId> {
    let target_pos = world.grid_cell_positions.get(target_cell)?;

    let mut best: Option<(EntityId, f64)> = None;
    for &nid in node_ids {
        let node = match world.infra_nodes.get(&nid) {
            Some(n) => n,
            None => continue,
        };
        if (node.node_type.tier() as u8) < (min_tier as u8) {
            continue;
        }
        let node_pos = match world.grid_cell_positions.get(node.cell_index) {
            Some(p) => p,
            None => continue,
        };
        let dist = sq_distance(target_pos, node_pos);
        if best.is_none() || dist < best.unwrap().1 {
            best = Some((nid, dist));
        }
    }
    best.map(|(id, _)| id)
}

// ─── Distance ────────────────────────────────────────────────────────────────

/// Squared Euclidean distance between two (lat, lon) positions. Good for comparison.
pub fn sq_distance(a: &(f64, f64), b: &(f64, f64)) -> f64 {
    let dlat = a.0 - b.0;
    let dlon = a.1 - b.1;
    dlat * dlat + dlon * dlon
}

/// Haversine distance in kilometers between two (lat, lon) positions.
pub fn haversine_km(a: &(f64, f64), b: &(f64, f64)) -> f64 {
    let dlat = (a.0 - b.0).to_radians();
    let dlon = (a.1 - b.1).to_radians();
    let lat1 = a.0.to_radians();
    let lat2 = b.0.to_radians();
    let a_val = (dlat / 2.0).sin().powi(2) + lat1.cos() * lat2.cos() * (dlon / 2.0).sin().powi(2);
    let c = 2.0 * a_val.sqrt().asin();
    6371.0 * c
}

// ─── Tier-Aware Edge Type Selection ──────────────────────────────────────────

/// Pick the best edge type for connecting two nodes, respecting tier compatibility.
pub fn pick_edge_type_for_tiers(world: &GameWorld, from: EntityId, to: EntityId) -> Option<EdgeType> {
    let from_type = world.infra_nodes.get(&from)?.node_type;
    let to_type = world.infra_nodes.get(&to)?.node_type;

    // Try edge types in preference order: fiber first, then alternatives
    let candidates = [
        EdgeType::FiberLocal,
        EdgeType::FiberRegional,
        EdgeType::FiberNational,
        EdgeType::Copper,
        EdgeType::Microwave,
        EdgeType::Satellite,
        EdgeType::Submarine,
    ];

    candidates.into_iter().find(|&edge_type| edge_type.can_connect(from_type, to_type))
}

/// Legacy tier-unaware fiber selection (fallback).
pub fn pick_fiber_type_by_level(world: &GameWorld, from: EntityId, to: EntityId) -> EdgeType {
    let from_level = world
        .infra_nodes
        .get(&from)
        .map(|n| n.network_level)
        .unwrap_or(NetworkLevel::Local);
    let to_level = world
        .infra_nodes
        .get(&to)
        .map(|n| n.network_level)
        .unwrap_or(NetworkLevel::Local);

    let max_level = std::cmp::max(from_level, to_level);
    match max_level {
        NetworkLevel::Local => EdgeType::FiberLocal,
        NetworkLevel::Regional => EdgeType::FiberRegional,
        _ => EdgeType::FiberNational,
    }
}

// ─── Parcel Acquisition ──────────────────────────────────────────────────────

/// AI acquires a land parcel at a given cell (sets ownership if unowned and affordable).
pub fn acquire_parcel(world: &mut GameWorld, corp_id: EntityId, cell_index: usize) {
    if let Some(&parcel_id) = world.cell_to_parcel.get(&cell_index) {
        if let Some(parcel) = world.land_parcels.get_mut(&parcel_id) {
            if parcel.owner.is_none() {
                let acquisition_cost = (100_000.0 * parcel.cost_modifier) as Money;
                if let Some(fin) = world.financials.get_mut(&corp_id) {
                    if fin.cash > acquisition_cost {
                        fin.cash -= acquisition_cost;
                        parcel.owner = Some(corp_id);
                    }
                }
            }
        }
    }
}

// ─── Node Construction ───────────────────────────────────────────────────────

/// Build a node for an AI corporation at the given cell with a random positional offset.
/// Uses cell_index for terrain/region lookup but places the node at a jittered position
/// so it doesn't snap to the cell center. Returns the new node ID if successful.
pub fn build_node(
    world: &mut GameWorld,
    corp_id: EntityId,
    node_type: NodeType,
    cell_index: usize,
    tick: Tick,
) -> Option<EntityId> {
    let terrain = world
        .cell_to_parcel
        .get(&cell_index)
        .and_then(|&pid| world.land_parcels.get(&pid))
        .map(|p| p.terrain)
        .unwrap_or(TerrainType::Rural);

    let node = InfraNode::new_on_terrain(node_type, cell_index, corp_id, terrain);
    let cost = node.construction_cost;
    let maintenance = node.maintenance_cost;

    // Check affordability (need 2x cost as buffer)
    let can_afford = world
        .financials
        .get(&corp_id)
        .map(|f| f.cash > cost * 2)
        .unwrap_or(false);
    if !can_afford {
        return None;
    }

    let build_time = match node_type {
        // Original 8 (unchanged)
        NodeType::CellTower | NodeType::WirelessRelay => 10,
        NodeType::CentralOffice => 20,
        NodeType::ExchangePoint => 30,
        NodeType::DataCenter | NodeType::BackboneRouter => 50,
        NodeType::SatelliteGround => 40,
        NodeType::SubmarineLanding => 60,

        // Era 1: Telegraph — fast to build
        NodeType::TelegraphOffice => 5,
        NodeType::TelegraphRelay => 3,
        NodeType::CableHut => 8,

        // Era 2: Telephone
        NodeType::ManualExchange => 10,
        NodeType::AutomaticExchange => 15,
        NodeType::TelephonePole => 2,
        NodeType::LongDistanceRelay => 12,

        // Era 3: Early Digital
        NodeType::DigitalSwitch => 20,
        NodeType::MicrowaveTower => 15,
        NodeType::CoaxHub => 10,
        NodeType::EarlyDataCenter => 40,
        NodeType::SatelliteGroundStation => 35,

        // Era 4: Internet
        NodeType::FiberPOP => 25,
        NodeType::InternetExchangePoint => 35,
        NodeType::SubseaLandingStation => 70,
        NodeType::ColocationFacility => 45,
        NodeType::ISPGateway => 15,

        // Era 5: Modern
        NodeType::MacroCell => 12,
        NodeType::SmallCell => 5,
        NodeType::EdgeDataCenter => 30,
        NodeType::HyperscaleDataCenter => 120,
        NodeType::CloudOnRamp => 25,
        NodeType::ContentDeliveryNode => 20,
        NodeType::FiberSplicePoint => 2,
        NodeType::DWDM_Terminal => 35,
        NodeType::FiberDistributionHub => 5,
        NodeType::NetworkAccessPoint => 3,

        // Era 6: Near Future
        NodeType::LEO_SatelliteGateway => 80,
        NodeType::QuantumRepeater => 60,
        NodeType::MeshDroneRelay => 5,
        NodeType::UnderwaterDataCenter => 150,
        NodeType::NeuromorphicEdgeNode => 40,
        NodeType::TerahertzRelay => 10,
    };

    acquire_parcel(world, corp_id, cell_index);

    let node_id = world.allocate_entity();
    world.infra_nodes.insert(node_id, node);
    world
        .constructions
        .insert(node_id, Construction::new(tick, build_time));
    world.ownerships.insert(node_id, Ownership::sole(corp_id));
    world.healths.insert(node_id, Health::new());
    world.capacities.insert(node_id, Capacity::new(0.0));

    if let Some(&(cell_lat, cell_lon)) = world.grid_cell_positions.get(cell_index) {
        // Add random jitter so nodes don't sit exactly on cell centers.
        // Jitter is ~20% of cell spacing (in degrees, roughly cell_spacing_km / 111 * 0.2).
        let jitter_range = (world.cell_spacing_km / 111.0) * 0.2;
        let rng_val = world.deterministic_random();
        let jitter_lon = (rng_val * 2.0 - 1.0) * jitter_range;
        let rng_val2 = world.deterministic_random();
        let jitter_lat = (rng_val2 * 2.0 - 1.0) * jitter_range;

        let lon = cell_lon + jitter_lon;
        let lat = cell_lat + jitter_lat;

        let region_id = world.cell_to_region.get(&cell_index).copied();
        world.positions.insert(
            node_id,
            Position {
                x: lon,
                y: lat,
                region_id,
            },
        );
    }

    world
        .corp_infra_nodes
        .entry(corp_id)
        .or_default()
        .push(node_id);
    world.network.add_node(node_id);

    if let Some(f) = world.financials.get_mut(&corp_id) {
        f.cash -= cost;
        f.cost_per_tick += maintenance;
    }

    world.event_queue.push(
        tick,
        gt_common::events::GameEvent::ConstructionStarted {
            entity: node_id,
            tick,
        },
    );

    Some(node_id)
}

// ─── Edge Construction ───────────────────────────────────────────────────────

/// Build an edge between two nodes. Uses tier-aware edge type selection.
/// Returns true if the edge was successfully built.
pub fn build_edge(
    world: &mut GameWorld,
    corp_id: EntityId,
    from: EntityId,
    to: EntityId,
    _tick: Tick,
) -> bool {
    let from_pos = world.positions.get(&from);
    let to_pos = world.positions.get(&to);

    let length_km = match (from_pos, to_pos) {
        (Some(a), Some(b)) => haversine_km(&(a.y, a.x), &(b.y, b.x)),
        _ => 100.0,
    };

    // Pick tier-compatible edge type, fallback to level-based
    let edge_type = pick_edge_type_for_tiers(world, from, to)
        .unwrap_or_else(|| pick_fiber_type_by_level(world, from, to));

    // Enforce max distance using centralized multiplier
    let max_distance_km = world.cell_spacing_km * edge_type.distance_multiplier();
    if length_km > max_distance_km {
        return false;
    }

    // Terrain check: fiber/copper can't cross deep ocean
    let terrain_blocked = is_terrain_blocked(world, from, to, edge_type);
    if terrain_blocked {
        return false;
    }

    let edge = InfraEdge::new(edge_type, from, to, length_km, corp_id);
    let cost = edge.construction_cost;
    let maintenance = edge.maintenance_cost;

    let can_afford = world
        .financials
        .get(&corp_id)
        .map(|f| f.cash >= cost)
        .unwrap_or(false);
    if !can_afford {
        return false;
    }

    let edge_id = world.allocate_entity();
    world.infra_edges.insert(edge_id, edge);
    world.network.add_edge_with_id(from, to, edge_id);

    if let Some(f) = world.financials.get_mut(&corp_id) {
        f.cash -= cost;
        f.cost_per_tick += maintenance;
    }

    true
}

/// Check if a terrain blocks a given edge type between two nodes.
fn is_terrain_blocked(
    world: &GameWorld,
    from: EntityId,
    to: EntityId,
    edge_type: EdgeType,
) -> bool {
    if edge_type.is_underground_capable() && !edge_type.is_submarine() {
        // Land-based cables can't cross deep ocean
        let from_terrain = node_terrain(world, from);
        let to_terrain = node_terrain(world, to);
        matches!(from_terrain, Some(TerrainType::OceanDeep | TerrainType::OceanTrench))
            || matches!(to_terrain, Some(TerrainType::OceanDeep | TerrainType::OceanTrench))
    } else {
        false
    }
}

fn node_terrain(world: &GameWorld, node_id: EntityId) -> Option<TerrainType> {
    world
        .infra_nodes
        .get(&node_id)
        .and_then(|n| {
            world
                .land_parcels
                .values()
                .find(|p| p.cell_index == n.cell_index)
        })
        .map(|p| p.terrain)
}

// ─── Corp Node Queries ───────────────────────────────────────────────────────

/// Count how many operational (non-under-construction) nodes a corp has at a given tier.
pub fn count_nodes_at_tier(world: &GameWorld, corp_id: EntityId, tier: NetworkTier) -> usize {
    world
        .corp_infra_nodes
        .get(&corp_id)
        .unwrap_or(&Vec::new())
        .iter()
        .filter(|&&nid| {
            !world.constructions.contains_key(&nid)
                && world
                    .infra_nodes
                    .get(&nid)
                    .map(|n| n.node_type.tier() == tier)
                    .unwrap_or(false)
        })
        .count()
}

/// Check if corp has any node at or above a tier.
pub fn has_node_at_or_above_tier(world: &GameWorld, corp_id: EntityId, tier: NetworkTier) -> bool {
    world
        .corp_infra_nodes
        .get(&corp_id)
        .unwrap_or(&Vec::new())
        .iter()
        .any(|&nid| {
            !world.constructions.contains_key(&nid)
                && world
                    .infra_nodes
                    .get(&nid)
                    .map(|n| (n.node_type.tier() as u8) >= (tier as u8))
                    .unwrap_or(false)
        })
}

/// Get the set of cell indices where a corp has nodes.
pub fn corp_cell_set(world: &GameWorld, corp_id: EntityId) -> std::collections::HashSet<usize> {
    world
        .corp_infra_nodes
        .get(&corp_id)
        .unwrap_or(&Vec::new())
        .iter()
        .filter_map(|&nid| world.infra_nodes.get(&nid).map(|n| n.cell_index))
        .collect()
}

/// Deterministic pseudo-random value from tick and corp_id for variety.
pub fn deterministic_variety(tick: Tick, corp_id: EntityId, shift: u32) -> usize {
    ((tick.wrapping_mul(corp_id) >> shift) % 100) as usize
}
