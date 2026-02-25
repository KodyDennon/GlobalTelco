use crate::components::infra_edge::DeploymentMethod;
use crate::world::GameWorld;

pub fn run(world: &mut GameWorld) {
    let tick = world.current_tick();
    let mut corp_ids: Vec<u64> = world.corporations.keys().copied().collect();
    corp_ids.sort_unstable();

    for &corp_id in &corp_ids {
        let mut total_cost: i64 = 0;

        // Infrastructure maintenance costs
        let corp_nodes = world
            .corp_infra_nodes
            .get(&corp_id)
            .cloned()
            .unwrap_or_default();

        for &node_id in &corp_nodes {
            if let Some(node) = world.infra_nodes.get(&node_id) {
                total_cost += node.maintenance_cost;
            }
        }

        // Edge maintenance costs
        let edge_costs: i64 = world
            .infra_edges
            .values()
            .filter(|e| e.owner == corp_id)
            .map(|e| e.maintenance_cost)
            .sum();
        total_cost += edge_costs;

        // ── Insurance premiums (Gap #18) ─────────────────────────────────────
        // Only charged for insured infrastructure nodes.
        // Premium = asset_value × 0.002 × deployment_risk × regional_disaster_risk × damage_history
        let insurance_premium = calculate_insurance_premiums(world, corp_id, tick);
        total_cost += insurance_premium;

        // Workforce salary costs
        if let Some(wf) = world.workforces.get(&corp_id) {
            total_cost += wf.salary_per_tick;
        }

        // Debt payments
        let debt_payments: i64 = world
            .debt_instruments
            .values()
            .filter(|d| d.holder == corp_id)
            .map(|d| d.payment_per_tick)
            .sum();
        total_cost += debt_payments;

        // Contract payments (where this corp is buying service)
        let contract_costs: i64 = world
            .contracts
            .values()
            .filter(|c| c.to == corp_id && c.status == crate::components::ContractStatus::Active)
            .map(|c| c.price_per_tick)
            .sum();
        total_cost += contract_costs;

        if let Some(fin) = world.financials.get_mut(&corp_id) {
            fin.cost_per_tick = total_cost;
            fin.cash -= total_cost;
        }

        if total_cost > 0 {
            world.event_queue.push(
                tick,
                gt_common::events::GameEvent::CostIncurred {
                    corporation: corp_id,
                    amount: total_cost,
                },
            );
        }
    }
}

/// Calculate insurance premiums for all insured infrastructure owned by `corp_id`.
///
/// Formula per insured node:
///   base_premium = construction_cost × 0.002
///   × regional_disaster_risk (from region.disaster_risk)
///   × damage_history_modifier (1.3 if any corp edge was damaged in last 100 ticks)
///
/// For insured edges (edges connected to insured source/target nodes):
///   base_premium = construction_cost × 0.002
///   × deployment_risk (Aerial 1.5, Underground 1.0, Submarine 3.0)
///   × regional_disaster_risk
///   × damage_history_modifier
fn calculate_insurance_premiums(world: &GameWorld, corp_id: u64, current_tick: u64) -> i64 {
    // Check if any corp edge was damaged in the last 100 ticks (damage history modifier)
    let any_recent_damage = world
        .infra_edges
        .values()
        .filter(|e| e.owner == corp_id)
        .any(|e| {
            e.last_damage_tick
                .map(|t| current_tick.saturating_sub(t) <= 100)
                .unwrap_or(false)
        });
    let damage_history_modifier = if any_recent_damage { 1.3 } else { 1.0 };

    let mut total_premium: f64 = 0.0;

    // Insured nodes
    let corp_nodes = world
        .corp_infra_nodes
        .get(&corp_id)
        .cloned()
        .unwrap_or_default();

    for &node_id in &corp_nodes {
        if let Some(node) = world.infra_nodes.get(&node_id) {
            if !node.insured {
                continue;
            }

            let base = node.construction_cost as f64 * 0.002;

            // Regional disaster risk
            let region_risk = world
                .cell_to_region
                .get(&node.cell_index)
                .and_then(|&region_id| world.regions.get(&region_id))
                .map(|r| r.disaster_risk)
                .unwrap_or(1.0);

            total_premium += base * region_risk * damage_history_modifier;
        }
    }

    // Insured edges: an edge is insured if it is owned by this corp
    // and either its source or target node is insured.
    for edge in world.infra_edges.values() {
        if edge.owner != corp_id {
            continue;
        }

        let src_insured = world
            .infra_nodes
            .get(&edge.source)
            .map(|n| n.insured)
            .unwrap_or(false);
        let dst_insured = world
            .infra_nodes
            .get(&edge.target)
            .map(|n| n.insured)
            .unwrap_or(false);

        if !src_insured && !dst_insured {
            continue;
        }

        let base = edge.construction_cost as f64 * 0.002;

        // Deployment risk multiplier
        let is_submarine = matches!(
            edge.edge_type,
            gt_common::types::EdgeType::Submarine
                | gt_common::types::EdgeType::SubseaTelegraphCable
                | gt_common::types::EdgeType::SubseaFiberCable
        );
        let deployment_risk = if is_submarine {
            3.0
        } else {
            match edge.deployment {
                DeploymentMethod::Aerial => 1.5,
                DeploymentMethod::Underground => 1.0,
            }
        };

        // Regional disaster risk (use source node's region, fall back to target)
        let region_risk = world
            .infra_nodes
            .get(&edge.source)
            .and_then(|n| world.cell_to_region.get(&n.cell_index))
            .or_else(|| {
                world
                    .infra_nodes
                    .get(&edge.target)
                    .and_then(|n| world.cell_to_region.get(&n.cell_index))
            })
            .and_then(|&region_id| world.regions.get(&region_id))
            .map(|r| r.disaster_risk)
            .unwrap_or(1.0);

        total_premium += base * deployment_risk * region_risk * damage_history_modifier;
    }

    total_premium as i64
}
