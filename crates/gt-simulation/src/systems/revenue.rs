//! Traffic-based revenue system.
//!
//! Revenue is now tied to actual traffic flowing through infrastructure:
//! - Nodes earn revenue based on traffic routed through them
//! - Edges earn transit fees based on traffic on the link
//! - Coverage alone provides reduced subscription revenue
//! - Empty/disconnected nodes earn nothing
//! - FTTH building revenue: active NAPs generate per-building subscriber revenue
//!   - Buildings with direct DropCable connections get 100% revenue
//!   - Auto-covered buildings (within NAP radius, no DropCable) get 85% revenue (15% overhead)

use std::collections::HashSet;

use crate::world::GameWorld;
use gt_common::types::{EdgeType, EntityId, NodeType};

// ─── Public entry point ───────────────────────────────────────────────────────

pub fn run(world: &mut GameWorld) {
    let tick = world.current_tick();

    let mut corp_ids: Vec<u64> = world.corporations.keys().copied().collect();
    corp_ids.sort_unstable();

    for &corp_id in &corp_ids {
        let mut total_revenue: i64 = 0;

        total_revenue += calculate_node_traffic_revenue(world, corp_id);
        total_revenue += calculate_edge_traffic_revenue(world, corp_id);
        total_revenue += calculate_contract_revenue(world, corp_id);
        total_revenue += calculate_coverage_revenue(world, corp_id);
        total_revenue += calculate_building_revenue(world, corp_id);

        if let Some(fin) = world.financials.get_mut(&corp_id) {
            fin.revenue_per_tick = total_revenue;
            fin.cash += total_revenue;
        }

        if total_revenue > 0 {
            world.event_queue.push(
                tick,
                gt_common::events::GameEvent::RevenueEarned {
                    corporation: corp_id,
                    amount: total_revenue,
                },
            );
        }
    }
}

// ─── Node Revenue (traffic-based) ─────────────────────────────────────────────

fn calculate_node_traffic_revenue(world: &GameWorld, corp_id: EntityId) -> i64 {
    let corp_nodes = world
        .corp_infra_nodes
        .get(&corp_id)
        .cloned()
        .unwrap_or_default();

    let mut revenue: i64 = 0;

    for &node_id in &corp_nodes {
        if world.constructions.contains_key(&node_id) {
            continue;
        }

        let traffic = world
            .traffic_matrix
            .node_traffic
            .get(&node_id)
            .copied()
            .unwrap_or(0.0);

        if traffic <= 0.0 {
            continue;
        }

        let node = match world.infra_nodes.get(&node_id) {
            Some(n) => n,
            None => continue,
        };

        let rate = node.node_type.traffic_revenue_rate();
        let health = world.healths.get(&node_id).map(|h| h.condition).unwrap_or(1.0);
        let quality = quality_multiplier(world, node_id, health);

        revenue += (traffic * rate * quality) as i64;
    }

    revenue
}

// ─── Edge Revenue (transit fees) ──────────────────────────────────────────────

fn calculate_edge_traffic_revenue(world: &GameWorld, corp_id: EntityId) -> i64 {
    let mut edge_revs: Vec<(u64, i64)> = world
        .infra_edges
        .iter()
        .filter(|(_, e)| e.owner == corp_id)
        .map(|(&eid, edge)| {
            let traffic = world
                .traffic_matrix
                .edge_traffic
                .get(&eid)
                .copied()
                .unwrap_or(0.0);

            if traffic <= 0.0 {
                return (eid, 0);
            }

            let rate = edge.edge_type.traffic_revenue_rate();
            let health_factor = edge.health;
            (eid, (traffic * rate * health_factor) as i64)
        })
        .collect();
    edge_revs.sort_unstable_by_key(|t| t.0);
    edge_revs.iter().map(|t| t.1).sum()
}

// ─── Contract Revenue ─────────────────────────────────────────────────────────

fn calculate_contract_revenue(world: &GameWorld, corp_id: EntityId) -> i64 {
    let mut contract_revs: Vec<(u64, i64)> = world
        .contracts
        .iter()
        .filter(|(_, c)| {
            c.from == corp_id && c.status == crate::components::ContractStatus::Active
        })
        .map(|(&cid, c)| (cid, c.price_per_tick))
        .collect();
    contract_revs.sort_unstable_by_key(|t| t.0);
    contract_revs.iter().map(|t| t.1).sum()
}

// ─── Coverage Revenue (reduced weight) ────────────────────────────────────────

fn calculate_coverage_revenue(world: &GameWorld, corp_id: EntityId) -> i64 {
    let mut covered_population: u64 = 0;

    let mut coverage_cells: Vec<usize> = world.cell_coverage.keys().copied().collect();
    coverage_cells.sort_unstable();

    for cell_idx in coverage_cells {
        let coverage = match world.cell_coverage.get(&cell_idx) {
            Some(c) => c,
            None => continue,
        };
        if coverage.dominant_owner != Some(corp_id) || coverage.bandwidth <= 0.0 {
            continue;
        }

        if let Some(&city_id) = world.cell_to_city.get(&cell_idx) {
            if let Some(city) = world.cities.get(&city_id) {
                let cell_pop = city.population / city.cells.len().max(1) as u64;
                let satisfaction = city.infrastructure_satisfaction;
                covered_population += (cell_pop as f64 * satisfaction) as u64;
            }
        }
    }

    // Coverage subscription revenue: represents monthly fees from covered population.
    // At $0.15/pop/tick, covering 100K people at 50% satisfaction = $7,500/tick.
    (covered_population as f64 * 0.15) as i64
}

// ─── FTTH Building Revenue (per-building subscriber fees) ────────────────────

/// Base rate per tick per demand unit for FTTH building subscribers.
const BUILDING_BASE_RATE: f64 = 50.0;

/// Overhead deduction for auto-covered buildings (no dedicated DropCable).
/// Buildings covered by NAP radius alone get 85% of full revenue.
const AUTO_COVERAGE_FACTOR: f64 = 0.85;

/// Calculate revenue from buildings served by active FTTH NAPs.
///
/// Each active NAP (marked by the FTTH system) covers buildings within its
/// coverage radius. Revenue per building = base_rate * demand_value * service_quality.
///
/// Gap #15: Buildings with a direct DropCable edge from the NAP get 100% revenue.
/// Buildings auto-covered by radius alone get 85% (15% overhead deduction).
fn calculate_building_revenue(world: &GameWorld, corp_id: EntityId) -> i64 {
    // Collect this corporation's active FTTH NAPs
    let corp_nodes = world
        .corp_infra_nodes
        .get(&corp_id)
        .cloned()
        .unwrap_or_default();

    let mut active_naps: Vec<(EntityId, usize, f64, f64)> = Vec::new(); // (id, cell_index, health, utilization)

    for &node_id in &corp_nodes {
        let node = match world.infra_nodes.get(&node_id) {
            Some(n) => n,
            None => continue,
        };

        if node.node_type != NodeType::NetworkAccessPoint || !node.active_ftth {
            continue;
        }

        // Skip NAPs still under construction
        if world.constructions.contains_key(&node_id) {
            continue;
        }

        let health = world
            .healths
            .get(&node_id)
            .map(|h| h.condition)
            .unwrap_or(1.0);
        let utilization = node.utilization();

        active_naps.push((node_id, node.cell_index, health, utilization));
    }

    if active_naps.is_empty() {
        return 0;
    }

    // Sort for deterministic processing
    active_naps.sort_unstable_by_key(|t| t.0);

    // Build a set of NAP IDs that have at least one DropCable edge.
    // NAPs with DropCables serve those buildings at 100%; remaining auto-covered at 85%.
    let mut naps_with_drops: HashSet<EntityId> = HashSet::new();
    // Count DropCable edges per NAP for proportional full-rate calculation
    let mut drop_cable_count: std::collections::HashMap<EntityId, u32> =
        std::collections::HashMap::new();

    for edge in world.infra_edges.values() {
        if edge.edge_type != EdgeType::DropCable {
            continue;
        }
        // A DropCable connects from a NAP to a building (or vice versa).
        // Check if source or target is one of our active NAPs.
        for &(nap_id, _, _, _) in &active_naps {
            if edge.source == nap_id || edge.target == nap_id {
                naps_with_drops.insert(nap_id);
                *drop_cable_count.entry(nap_id).or_insert(0) += 1;
            }
        }
    }

    let cell_spacing = world.cell_spacing_km;
    let mut revenue: f64 = 0.0;

    for &(nap_id, nap_cell, health, utilization) in &active_naps {
        // Service quality = health * utilization_headroom (1.0 - utilization)
        let utilization_headroom = (1.0 - utilization).max(0.0);
        let service_quality = health * utilization_headroom;

        if service_quality <= 0.0 {
            continue;
        }

        // NAP coverage radius
        let base_radius_km = NodeType::NetworkAccessPoint.coverage_radius_km();
        // Scale radius for grid resolution (same logic as coverage system)
        let radius_km = base_radius_km.max(cell_spacing * 0.8);

        // Get NAP position
        let nap_pos = match world.grid_cell_positions.get(nap_cell) {
            Some(p) => *p,
            None => continue,
        };
        let (nap_lat, nap_lon) = nap_pos;
        let lat_range = radius_km / 111.0;
        let cos_lat = (nap_lat.to_radians()).cos().max(0.1);
        let lon_range = radius_km / (111.0 * cos_lat);

        // Number of DropCable connections from this NAP
        let drops = drop_cable_count.get(&nap_id).copied().unwrap_or(0);
        let has_drops = naps_with_drops.contains(&nap_id);

        // Scan cells within NAP radius for buildings (city population)
        let mut covered_demand: f64 = 0.0;
        let mut covered_cell_count: u32 = 0;

        for (cell_idx, &(cell_lat, cell_lon)) in world.grid_cell_positions.iter().enumerate() {
            // Bounding box check
            if (cell_lat - nap_lat).abs() > lat_range || (cell_lon - nap_lon).abs() > lon_range {
                continue;
            }

            // Check if cell belongs to a city (has "buildings" / demand)
            let city = match world
                .cell_to_city
                .get(&cell_idx)
                .and_then(|&cid| world.cities.get(&cid))
            {
                Some(c) => c,
                None => continue,
            };

            // Calculate demand value for this cell
            let cell_pop = city.population / city.cells.len().max(1) as u64;
            let demand_value = city.telecom_demand * (cell_pop as f64 / 1000.0); // Normalize: demand per 1000 people

            if demand_value <= 0.0 {
                continue;
            }

            covered_demand += demand_value;
            covered_cell_count += 1;
        }

        if covered_demand <= 0.0 {
            continue;
        }

        // Calculate revenue with overhead deduction (Gap #15).
        // If the NAP has DropCable connections, a portion of covered buildings
        // are served at full rate and the rest at the reduced auto-coverage rate.
        // We approximate: drop_cable buildings = min(drops, covered_cell_count).
        let full_rate_cells = if has_drops {
            drops.min(covered_cell_count)
        } else {
            0
        };
        let auto_rate_cells = covered_cell_count.saturating_sub(full_rate_cells);

        // Distribute demand proportionally across cells
        let demand_per_cell = if covered_cell_count > 0 {
            covered_demand / covered_cell_count as f64
        } else {
            0.0
        };

        let full_rate_revenue =
            full_rate_cells as f64 * demand_per_cell * BUILDING_BASE_RATE * service_quality;
        let auto_rate_revenue = auto_rate_cells as f64
            * demand_per_cell
            * BUILDING_BASE_RATE
            * service_quality
            * AUTO_COVERAGE_FACTOR;

        revenue += full_rate_revenue + auto_rate_revenue;
    }

    revenue as i64
}

// ─── Quality Multiplier ───────────────────────────────────────────────────────

fn quality_multiplier(world: &GameWorld, node_id: EntityId, health: f64) -> f64 {
    let health_factor = if health < 0.8 { health / 0.8 } else { 1.0 };

    let node_cell = world
        .infra_nodes
        .get(&node_id)
        .map(|n| n.cell_index)
        .unwrap_or(0);

    let satisfaction_bonus = world
        .cell_to_city
        .get(&node_cell)
        .and_then(|&cid| world.cities.get(&cid))
        .map(|city| {
            if city.infrastructure_satisfaction > 0.8 {
                0.1
            } else {
                0.0
            }
        })
        .unwrap_or(0.0);

    health_factor * (1.0 + satisfaction_bonus)
}
