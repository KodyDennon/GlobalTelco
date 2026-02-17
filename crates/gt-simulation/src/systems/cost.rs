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
