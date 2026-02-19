use crate::world::GameWorld;

const DISASTER_TYPES: &[(&str, f64)] = &[
    ("Earthquake", 0.15),
    ("Hurricane", 0.15),
    ("Flooding", 0.20),
    ("Landslide", 0.10),
    ("CyberAttack", 0.15),
    ("PoliticalUnrest", 0.10),
    ("RegulatoryChange", 0.10),
    ("EquipmentFailure", 0.05),
];

pub fn run(world: &mut GameWorld) {
    let tick = world.current_tick();

    // Disasters occur randomly based on region disaster risk
    // Check every 50 ticks to avoid excessive event spam
    if !tick.is_multiple_of(50) {
        return;
    }

    let mut region_data: Vec<(u64, f64, Vec<usize>)> = world
        .regions
        .iter()
        .map(|(&id, r)| (id, r.disaster_risk, r.cells.clone()))
        .collect();
    region_data.sort_unstable_by_key(|t| t.0);

    for (region_id, disaster_risk, cells) in region_data {
        let roll = world.deterministic_random();

        // Base 2% chance per check, scaled by disaster risk
        if roll < 0.02 * disaster_risk {
            let severity = world.deterministic_random() * 0.5 + 0.1; // 0.1 to 0.6

            // Pick disaster type based on weighted random
            let type_roll = world.deterministic_random();
            let mut cumulative = 0.0;
            let mut disaster_name = "Earthquake";
            for &(name, weight) in DISASTER_TYPES {
                cumulative += weight;
                if type_roll < cumulative {
                    disaster_name = name;
                    break;
                }
            }

            // Damage infrastructure in this region
            let mut affected_nodes: Vec<u64> = world
                .infra_nodes
                .iter()
                .filter(|(_, node)| cells.contains(&node.cell_index))
                .map(|(&id, _)| id)
                .collect();
            affected_nodes.sort_unstable();

            let affected_count = affected_nodes.len() as u32;

            for &node_id in &affected_nodes {
                let damage = severity * 0.3;
                if let Some(health) = world.healths.get_mut(&node_id) {
                    health.degrade(damage);
                    // Capacity reduction is handled by utilization::reset_capacities_to_base
                    // which applies health-based throughput scaling each tick
                }

                // Insurance payout: covers 60% of repair cost for insured nodes
                let is_insured = world.infra_nodes.get(&node_id).map(|n| n.insured).unwrap_or(false);
                if is_insured {
                    let repair_cost = world.infra_nodes.get(&node_id)
                        .map(|n| (n.construction_cost as f64 * damage * 0.2) as i64)
                        .unwrap_or(0);
                    let payout = (repair_cost as f64 * 0.6) as i64;
                    let owner = world.infra_nodes.get(&node_id).map(|n| n.owner);
                    if let Some(owner_id) = owner {
                        if let Some(fin) = world.financials.get_mut(&owner_id) {
                            fin.cash += payout;
                        }
                        world.event_queue.push(
                            tick,
                            gt_common::events::GameEvent::InsurancePayout {
                                entity: node_id,
                                amount: payout,
                            },
                        );
                    }
                }
            }

            // Population displacement for affected cities
            let mut city_ids: Vec<u64> = world
                .cities
                .iter()
                .filter(|(_, c)| cells.contains(&c.cell_index))
                .map(|(&id, _)| id)
                .collect();
            city_ids.sort_unstable();

            for &city_id in &city_ids {
                if severity > 0.3 {
                    if let Some(city) = world.cities.get_mut(&city_id) {
                        let displaced = (city.population as f64 * severity * 0.05) as u64;
                        city.population = city.population.saturating_sub(displaced);
                        city.migration_pressure += severity * 0.2;
                    }
                }
            }

            world.event_queue.push(
                tick,
                gt_common::events::GameEvent::DisasterStruck {
                    region: region_id,
                    severity,
                    disaster_type: disaster_name.to_string(),
                    affected_nodes: affected_count,
                },
            );
        }
    }
}
