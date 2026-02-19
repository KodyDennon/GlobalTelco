//! Traffic-based revenue system.
//!
//! Revenue is now tied to actual traffic flowing through infrastructure:
//! - Nodes earn revenue based on traffic routed through them
//! - Edges earn transit fees based on traffic on the link
//! - Coverage alone provides reduced subscription revenue
//! - Empty/disconnected nodes earn nothing

use crate::world::GameWorld;
use gt_common::types::EntityId;

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

    // Reduced from $0.10 to $0.02 — coverage alone is worth less now
    (covered_population as f64 * 0.02) as i64
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
