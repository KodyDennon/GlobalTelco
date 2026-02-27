use gt_common::types::EntityId;

use crate::world::GameWorld;

/// Satellite revenue system — calculates revenue from satellite subscribers.
/// Runs after terminal_distribution, before existing revenue system.
pub fn run(world: &mut GameWorld) {
    let tick = world.current_tick();

    // Collect subscription data
    let subs: Vec<((EntityId, EntityId), u32, i64)> = world
        .satellite_subscriptions
        .iter()
        .map(|((city_id, corp_id), sub)| ((*city_id, *corp_id), sub.subscribers, sub.monthly_rate))
        .collect();

    // Calculate revenue per corporation
    let mut corp_revenue: std::collections::HashMap<EntityId, i64> =
        std::collections::HashMap::new();

    for ((_city_id, corp_id), subscribers, monthly_rate) in &subs {
        if *subscribers == 0 {
            continue;
        }

        // Quality factor based on satellite capacity utilization
        let quality = calculate_quality_factor(world, *corp_id);

        // Revenue = subscribers * monthly_rate * quality_factor
        // Monthly rate is per-tick (simplified: 1 tick = 1 game-minute, ~43200 ticks/month)
        // Scale to per-tick: rate / 43200 * subscribers
        let per_tick_revenue = (*subscribers as f64 * *monthly_rate as f64 * quality / 720.0) as i64;

        *corp_revenue.entry(*corp_id).or_insert(0) += per_tick_revenue;
    }

    // Apply revenue
    for (corp_id, revenue) in corp_revenue {
        if revenue > 0 {
            if let Some(fin) = world.financials.get_mut(&corp_id) {
                fin.cash += revenue;
                fin.revenue_per_tick += revenue;
            }

            world.event_queue.push(
                tick,
                gt_common::events::GameEvent::RevenueEarned {
                    corporation: corp_id,
                    amount: revenue,
                },
            );
        }
    }
}

fn calculate_quality_factor(world: &GameWorld, corp_id: EntityId) -> f64 {
    // Quality = function of (available capacity / demand)
    // More satellites = better quality = higher factor

    let operational_sats = world
        .satellites
        .iter()
        .filter(|(id, s)| {
            s.status == gt_common::types::SatelliteStatus::Operational
                && world.ownerships.get(id).map(|o| o.owner) == Some(corp_id)
        })
        .count();

    let total_subscribers: u32 = world
        .satellite_subscriptions
        .iter()
        .filter(|((_, cid), _)| *cid == corp_id)
        .map(|(_, sub)| sub.subscribers)
        .sum();

    if total_subscribers == 0 {
        return 1.0;
    }

    // Each satellite can serve ~1000 subscribers well
    let capacity = operational_sats as f64 * 1000.0;
    let ratio = capacity / total_subscribers as f64;

    // Clamp quality between 0.3 and 1.0
    ratio.min(1.0).max(0.3)
}
