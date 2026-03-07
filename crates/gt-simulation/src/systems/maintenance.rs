use crate::world::GameWorld;

pub fn run(world: &mut GameWorld) {
    let tick = world.current_tick();

    // ── Precompute crew-based repair speed multipliers per corporation ──────
    // Multiplier = 1.0 + 0.1 * maintenance_crew_count, capped at 2.0.
    let crew_multipliers: std::collections::HashMap<u64, f64> = world
        .workforces
        .iter()
        .map(|(&corp_id, wf)| {
            let multiplier = (1.0 + 0.1 * wf.maintenance_crew_count as f64).min(2.0);
            (corp_id, multiplier)
        })
        .collect();

    // ── Process active node repairs ──────────────────────────────────────────
    let mut repairing_nodes: Vec<(u64, f64, u32, u64)> = world
        .infra_nodes
        .iter()
        .filter(|(_, node)| node.repairing && node.repair_ticks_left > 0)
        .map(|(&id, node)| (id, node.repair_health_per_tick, node.repair_ticks_left, node.owner))
        .collect();
    repairing_nodes.sort_unstable_by_key(|t| t.0);

    for (node_id, health_per_tick, ticks_left, owner_id) in repairing_nodes {
        // Apply crew speed bonus to health restoration rate
        let crew_mult = crew_multipliers.get(&owner_id).copied().unwrap_or(1.0);
        let effective_health_per_tick = health_per_tick * crew_mult;

        // Restore health proportionally
        if let Some(health) = world.healths.get_mut(&node_id) {
            health.condition = (health.condition + effective_health_per_tick).min(1.0);
        }

        let new_ticks = ticks_left - 1;
        if let Some(node) = world.infra_nodes.get_mut(&node_id) {
            node.repair_ticks_left = new_ticks;
            if new_ticks == 0 {
                node.repairing = false;
                node.repair_health_per_tick = 0.0;
                // Ensure full health on completion
                if let Some(health) = world.healths.get_mut(&node_id) {
                    health.condition = 1.0;
                }
                // Restore capacity
                let max_tp = node.max_throughput;
                if let Some(cap) = world.capacities.get_mut(&node_id) {
                    cap.max_throughput = max_tp;
                }
                world.event_queue.push(
                    tick,
                    gt_common::events::GameEvent::RepairCompleted { entity: node_id },
                );
            }
        }
    }

    // ── Process active edge repairs ──────────────────────────────────────────
    let mut repairing_edges: Vec<(u64, f64, u32, u64)> = world
        .infra_edges
        .iter()
        .filter(|(_, edge)| edge.repairing && edge.repair_ticks_left > 0)
        .map(|(&id, edge)| (id, edge.repair_health_per_tick, edge.repair_ticks_left, edge.owner))
        .collect();
    repairing_edges.sort_unstable_by_key(|t| t.0);

    for (edge_id, health_per_tick, ticks_left, owner_id) in repairing_edges {
        // Apply crew speed bonus to health restoration rate
        let crew_mult = crew_multipliers.get(&owner_id).copied().unwrap_or(1.0);
        let effective_health_per_tick = health_per_tick * crew_mult;

        let new_ticks = ticks_left - 1;
        if let Some(edge) = world.infra_edges.get_mut(&edge_id) {
            edge.health = (edge.health + effective_health_per_tick).min(1.0);
            edge.repair_ticks_left = new_ticks;
            if new_ticks == 0 {
                edge.repairing = false;
                edge.repair_health_per_tick = 0.0;
                edge.health = 1.0; // Ensure full health on completion
                world.event_queue.push(
                    tick,
                    gt_common::events::GameEvent::RepairCompleted { entity: edge_id },
                );
            }
        }
    }

    // ── Standard node maintenance (degradation + passive repair) ─────────────
    let mut nodes_info: Vec<(u64, u64, i64)> = world
        .infra_nodes
        .iter()
        .map(|(&id, node)| (id, node.owner, node.maintenance_cost))
        .collect();
    nodes_info.sort_unstable_by_key(|t| t.0);

    for (node_id, owner_id, maintenance_cost) in nodes_info {
        let has_budget = world
            .financials
            .get(&owner_id)
            .map(|f| f.cash > maintenance_cost)
            .unwrap_or(false);

        if let Some(health) = world.healths.get_mut(&node_id) {
            if has_budget {
                // With budget: slow degradation, repair if damaged (repair costs extra)
                health.degrade(0.0005);
                if health.condition < 0.8 {
                    // Repair costs 50% of maintenance cost per tick
                    let repair_cost = maintenance_cost / 2;
                    if let Some(fin) = world.financials.get_mut(&owner_id) {
                        if fin.cash > repair_cost {
                            fin.cash -= repair_cost;
                            health.condition = (health.condition + 0.002).min(1.0);
                        }
                    }
                }
            } else {
                // No budget: faster degradation, no repair
                health.degrade(0.003);
            }

            // Track maintenance cost on the health component
            // (Throughput reduction from health is handled by utilization::reset_capacities_to_base)
            health.maintenance_cost_per_tick = maintenance_cost;
        }
    }

    // ── Standard edge maintenance (degradation + passive repair) ────────────
    // Edges use their own `health` field (not the `healths` HashMap).
    // With maintenance budget: slow degradation (0.0003/tick).
    // Below 0.8 health: passive repair at 0.001/tick, costs 50% of maintenance.
    // Without budget: faster degradation (0.002/tick), no passive repair.
    // Skip edges currently under active repair (handled above).
    let mut edges_info: Vec<(u64, u64, i64, f64, bool)> = world
        .infra_edges
        .iter()
        .map(|(&id, edge)| (id, edge.owner, edge.maintenance_cost, edge.health, edge.repairing))
        .collect();
    edges_info.sort_unstable_by_key(|t| t.0);

    for (edge_id, owner_id, maintenance_cost, _health, is_repairing) in edges_info {
        // Skip edges under active repair — they're handled by the repair section above
        if is_repairing {
            continue;
        }

        let has_budget = world
            .financials
            .get(&owner_id)
            .map(|f| f.cash > maintenance_cost)
            .unwrap_or(false);

        if let Some(edge) = world.infra_edges.get_mut(&edge_id) {
            if has_budget {
                // With budget: slow degradation
                edge.health = (edge.health - 0.0003).max(0.0);
                if edge.health < 0.8 {
                    // Passive repair costs 50% of maintenance cost per tick
                    let repair_cost = maintenance_cost / 2;
                    if let Some(fin) = world.financials.get_mut(&owner_id) {
                        if fin.cash > repair_cost {
                            fin.cash -= repair_cost;
                            edge.health = (edge.health + 0.001).min(1.0);
                        }
                    }
                }
            } else {
                // No budget: faster degradation, no repair
                edge.health = (edge.health - 0.002).max(0.0);
            }
        }
    }

    // Degrade non-infrastructure health components normally
    let mut non_infra_health_ids: Vec<u64> = world
        .healths
        .keys()
        .filter(|id| !world.infra_nodes.contains_key(*id))
        .copied()
        .collect();
    non_infra_health_ids.sort_unstable();

    for id in non_infra_health_ids {
        if let Some(health) = world.healths.get_mut(&id) {
            health.degrade(0.001);
        }
    }
}
