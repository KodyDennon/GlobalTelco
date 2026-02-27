use crate::components::ContractStatus;
use crate::world::GameWorld;

pub fn run(world: &mut GameWorld) {
    let tick = world.current_tick();
    let mut expired = Vec::new();
    let mut breached = Vec::new();

    // Check contract lifecycle
    let mut contract_ids: Vec<u64> = world.contracts.keys().copied().collect();
    contract_ids.sort_unstable();
    for &contract_id in &contract_ids {
        let status = match world.contracts.get(&contract_id) {
            Some(c) => {
                if c.is_expired(tick) {
                    Some(ContractStatus::Expired)
                } else if c.status == ContractStatus::Active {
                    // Check for breach: does the provider have enough capacity?
                    let provider_capacity: f64 = world
                        .corp_infra_nodes
                        .get(&c.from)
                        .map(|nodes| {
                            nodes
                                .iter()
                                .filter_map(|&nid| world.capacities.get(&nid))
                                .map(|cap| (cap.max_throughput - cap.current_load).max(0.0))
                                .sum()
                        })
                        .unwrap_or(0.0);

                    if provider_capacity < c.capacity * 0.5 {
                        Some(ContractStatus::Breached)
                    } else {
                        None
                    }
                } else if c.status == ContractStatus::Proposed {
                    // Auto-expire proposals after 30 ticks
                    if tick > c.start_tick + 30 {
                        Some(ContractStatus::Expired)
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            None => None,
        };

        match status {
            Some(ContractStatus::Expired) => expired.push(contract_id),
            Some(ContractStatus::Breached) => breached.push(contract_id),
            _ => {}
        }
    }

    // Process expired contracts — settle SLA penalties on expiry
    for &id in &expired {
        // Settle accrued SLA penalty: transfer from provider to consumer
        if let Some(contract) = world.contracts.get(&id) {
            let penalty = contract.sla_penalty_accrued;
            let provider = contract.from;
            let consumer = contract.to;

            if penalty > 0 {
                if let Some(fin) = world.financials.get_mut(&provider) {
                    fin.cash -= penalty;
                }
                if let Some(fin) = world.financials.get_mut(&consumer) {
                    fin.cash += penalty;
                }
                world.event_queue.push(
                    tick,
                    gt_common::events::GameEvent::SLAPenaltyPaid {
                        provider,
                        consumer,
                        contract: id,
                        amount: penalty,
                    },
                );
            }
        }

        if let Some(contract) = world.contracts.get_mut(&id) {
            contract.status = ContractStatus::Expired;
        }
        world.event_queue.push(
            tick,
            gt_common::events::GameEvent::ContractExpired { entity: id },
        );
    }

    // Process breached contracts — apply penalties
    for &id in &breached {
        if let Some(contract) = world.contracts.get_mut(&id) {
            contract.status = ContractStatus::Breached;
            let penalty = contract.penalty;
            let provider = contract.from;

            if let Some(fin) = world.financials.get_mut(&provider) {
                fin.cash -= penalty;
            }
        }
    }

    // ── Update SLA performance for active contracts ────────────────────────
    // SLA performance is based on the provider's actual network health and
    // available capacity relative to the contracted capacity.
    let active_contract_ids: Vec<u64> = world
        .contracts
        .iter()
        .filter(|(_, c)| c.status == ContractStatus::Active)
        .map(|(&id, _)| id)
        .collect();

    for &cid in &active_contract_ids {
        let (provider_id, contracted_capacity, sla_target) = match world.contracts.get(&cid) {
            Some(c) => (c.from, c.capacity, c.sla_target),
            None => continue,
        };

        // Calculate provider's average node health
        let (health_sum, health_count) = world
            .corp_infra_nodes
            .get(&provider_id)
            .map(|nodes| {
                let mut sum = 0.0_f64;
                let mut count = 0_u32;
                for &nid in nodes {
                    if let Some(h) = world.healths.get(&nid) {
                        sum += h.condition;
                        count += 1;
                    }
                }
                (sum, count)
            })
            .unwrap_or((0.0, 0));

        let avg_health = if health_count > 0 {
            health_sum / health_count as f64
        } else {
            0.0
        };

        // Calculate available capacity vs contracted capacity
        let available_capacity: f64 = world
            .corp_infra_nodes
            .get(&provider_id)
            .map(|nodes| {
                nodes
                    .iter()
                    .filter_map(|&nid| world.capacities.get(&nid))
                    .map(|cap| (cap.max_throughput - cap.current_load).max(0.0))
                    .sum()
            })
            .unwrap_or(0.0);

        let capacity_ratio = if contracted_capacity > 0.0 {
            (available_capacity / contracted_capacity).min(1.0)
        } else {
            1.0
        };

        // SLA performance = health * capacity_fulfillment * 100
        // This represents the percentage uptime/quality being delivered
        let performance = avg_health * capacity_ratio * 100.0;

        if let Some(contract) = world.contracts.get_mut(&cid) {
            contract.sla_current_performance = performance;

            // Accrue penalty if below SLA target
            if performance < sla_target {
                // Penalty accrual: proportional to how far below target
                let shortfall = sla_target - performance;
                let penalty_per_tick = (contract.penalty as f64 * shortfall / 100.0) as i64;
                contract.sla_penalty_accrued += penalty_per_tick;
            }
        }
    }

    // Clean up old expired/breached contracts (after 100 ticks)
    let to_remove: Vec<u64> = world
        .contracts
        .iter()
        .filter(|(_, c)| {
            (c.status == ContractStatus::Expired || c.status == ContractStatus::Breached)
                && tick > c.end_tick + 100
        })
        .map(|(&id, _)| id)
        .collect();

    for id in to_remove {
        world.contracts.remove(&id);
    }
}
