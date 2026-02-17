use crate::world::GameWorld;

pub fn run(world: &mut GameWorld) {
    let tick = world.current_tick();

    // Calculate revenue from infrastructure utilization
    // Revenue = utilization * base_rate * health_factor
    let mut corp_ids: Vec<u64> = world.corporations.keys().copied().collect();
    corp_ids.sort_unstable();

    for &corp_id in &corp_ids {
        let corp_nodes = world
            .corp_infra_nodes
            .get(&corp_id)
            .cloned()
            .unwrap_or_default();

        let mut total_revenue: i64 = 0;

        for &node_id in &corp_nodes {
            // Skip nodes under construction
            if world.constructions.contains_key(&node_id) {
                continue;
            }

            let utilization = world
                .capacities
                .get(&node_id)
                .map(|c| c.utilization())
                .unwrap_or(0.0);

            let health_factor = world
                .healths
                .get(&node_id)
                .map(|h| h.condition)
                .unwrap_or(1.0);

            let base_revenue = world
                .infra_nodes
                .get(&node_id)
                .map(|n| {
                    // Revenue scales with throughput and node type
                    let rate = match n.node_type {
                        gt_common::types::NodeType::DataCenter => 0.05,
                        gt_common::types::NodeType::ExchangePoint => 0.03,
                        gt_common::types::NodeType::SubmarineLanding => 0.08,
                        gt_common::types::NodeType::SatelliteGround => 0.04,
                        _ => 0.02,
                    };
                    (n.max_throughput * rate) as i64
                })
                .unwrap_or(0);

            let node_revenue = (base_revenue as f64 * utilization * health_factor) as i64;
            total_revenue += node_revenue;
        }

        // Revenue from active contracts (where this corp provides service)
        let contract_revenue: i64 = world
            .contracts
            .values()
            .filter(|c| c.from == corp_id && c.status == crate::components::ContractStatus::Active)
            .map(|c| c.price_per_tick)
            .sum();

        total_revenue += contract_revenue;

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
