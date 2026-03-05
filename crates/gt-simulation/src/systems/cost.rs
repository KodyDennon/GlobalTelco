use crate::components::infra_edge::DeploymentMethod;
use crate::world::GameWorld;

pub fn run(world: &mut GameWorld) {
    let tick = world.current_tick();
    
    // 1. Calculate and distribute all infrastructure-related costs (maintenance + insurance)
    // We use a temporary map to accumulate costs per corporation to avoid multiple borrows.
    let mut corp_costs: std::collections::HashMap<u64, i64> = std::collections::HashMap::new();

    // Infrastructure Node Maintenance & Insurance
    for (&node_id, node) in &world.infra_nodes {
        let maintenance = node.maintenance_cost;
        let insurance = if node.insured {
            calculate_node_insurance_premium(world, node_id, node, tick)
        } else {
            0
        };
        let total_node_cost = maintenance + insurance;

        if total_node_cost > 0 {
            if let Some(ownership) = world.ownerships.get(&node_id) {
                // Primary owner's share (100% - sum of co-owner shares)
                let co_owner_total_share: f64 = ownership.co_owners.iter().map(|(_, s)| *s).sum();
                let primary_share = (1.0 - co_owner_total_share).max(0.0);
                
                let primary_cost = (total_node_cost as f64 * primary_share) as i64;
                *corp_costs.entry(ownership.owner).or_insert(0) += primary_cost;

                for &(co_owner_id, share_pct) in &ownership.co_owners {
                    let co_owner_cost = (total_node_cost as f64 * share_pct) as i64;
                    *corp_costs.entry(co_owner_id).or_insert(0) += co_owner_cost;
                }
            }
        }
    }

    // Infrastructure Edge Maintenance & Insurance
    // (Insurance history is per-corp, so we need a pre-scan or use existing logic)
    let recent_damage_map = build_recent_damage_map(world, tick);

    for (&edge_id, edge) in &world.infra_edges {
        // Skip dynamic satellite edges (they don't have maintenance)
        if matches!(edge.edge_type, gt_common::types::EdgeType::IntraplaneISL | gt_common::types::EdgeType::CrossplaneISL | gt_common::types::EdgeType::SatelliteDownlink) {
            continue;
        }

        let maintenance = edge.maintenance_cost;
        let insurance = calculate_edge_insurance_premium(world, edge, &recent_damage_map, tick);
        let total_edge_cost = maintenance + insurance;

        if total_edge_cost > 0 {
            if let Some(ownership) = world.ownerships.get(&edge_id) {
                let co_owner_total_share: f64 = ownership.co_owners.iter().map(|(_, s)| *s).sum();
                let primary_share = (1.0 - co_owner_total_share).max(0.0);
                
                let primary_cost = (total_edge_cost as f64 * primary_share) as i64;
                *corp_costs.entry(ownership.owner).or_insert(0) += primary_cost;

                for &(co_owner_id, share_pct) in &ownership.co_owners {
                    let co_owner_cost = (total_edge_cost as f64 * share_pct) as i64;
                    *corp_costs.entry(co_owner_id).or_insert(0) += co_owner_cost;
                }
            }
        }
    }

    // 2. Add other fixed costs per corporation (salaries, debt, contracts)
    let mut corp_ids: Vec<u64> = world.corporations.keys().copied().collect();
    corp_ids.sort_unstable();

    for &corp_id in &corp_ids {
        let mut total_cost = corp_costs.get(&corp_id).copied().unwrap_or(0);

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

        // Sandbox mode: player doesn't pay costs
        let is_player = world.player_corp_id() == Some(corp_id);
        let effective_cost = if world.config().sandbox && is_player {
            0
        } else {
            total_cost
        };

        if let Some(fin) = world.financials.get_mut(&corp_id) {
            fin.cost_per_tick = total_cost; // Still show true cost for informational purposes
            fin.cash -= effective_cost;
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

fn build_recent_damage_map(world: &GameWorld, current_tick: u64) -> std::collections::HashMap<u64, bool> {
    let mut map = std::collections::HashMap::new();
    for (&corp_id, _) in &world.corporations {
        let damaged = world
            .infra_edges
            .values()
            .filter(|e| e.owner == corp_id)
            .any(|e| {
                e.last_damage_tick
                    .map(|t| current_tick.saturating_sub(t) <= 100)
                    .unwrap_or(false)
            });
        map.insert(corp_id, damaged);
    }
    map
}

fn calculate_node_insurance_premium(world: &GameWorld, _node_id: u64, node: &crate::components::InfraNode, current_tick: u64) -> i64 {
    // Check recent damage for the owner
    let any_recent_damage = world
        .infra_edges
        .values()
        .filter(|e| e.owner == node.owner)
        .any(|e| {
            e.last_damage_tick
                .map(|t| current_tick.saturating_sub(t) <= 100)
                .unwrap_or(false)
        });
    let damage_history_modifier = if any_recent_damage { 1.3 } else { 1.0 };

    let base = node.construction_cost as f64 * 0.002;

    // Regional disaster risk
    let region_risk = world
        .cell_to_region
        .get(&node.cell_index)
        .and_then(|&region_id| world.regions.get(&region_id))
        .map(|r| r.disaster_risk)
        .unwrap_or(1.0);

    (base * region_risk * damage_history_modifier) as i64
}

fn calculate_edge_insurance_premium(
    world: &GameWorld, 
    edge: &crate::components::infra_edge::InfraEdge,
    recent_damage_map: &std::collections::HashMap<u64, bool>,
    _current_tick: u64
) -> i64 {
    let src_insured = world.infra_nodes.get(&edge.source).map(|n| n.insured).unwrap_or(false);
    let dst_insured = world.infra_nodes.get(&edge.target).map(|n| n.insured).unwrap_or(false);

    if !src_insured && !dst_insured {
        return 0;
    }

    let damage_history_modifier = if recent_damage_map.get(&edge.owner).copied().unwrap_or(false) { 1.3 } else { 1.0 };
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

    // Regional disaster risk
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

    (base * deployment_risk * region_risk * damage_history_modifier) as i64
}
