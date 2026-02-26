use crate::components::grant::{GovernmentGrant, GrantStatus};
use crate::world::GameWorld;
use gt_common::types::EntityId;

pub fn run(world: &mut GameWorld) {
    let tick = world.current_tick();

    // Only run every 10 ticks for performance
    if !tick.is_multiple_of(10) {
        return;
    }

    // --- Generate new grants for underserved regions every 100 ticks ---
    if tick.is_multiple_of(100) && tick > 0 {
        generate_grants(world);
    }

    // --- Track progress on awarded grants based on coverage ---
    let grant_ids: Vec<EntityId> = world.grants.keys().copied().collect();
    let mut completions: Vec<(EntityId, EntityId, i64)> = Vec::new(); // (grant_id, corp_id, reward)
    let mut expirations: Vec<(EntityId, EntityId)> = Vec::new(); // (grant_id, region_id)

    for &grant_id in &grant_ids {
        let grant = match world.grants.get(&grant_id) {
            Some(g) => g.clone(),
            None => continue,
        };

        match grant.status {
            GrantStatus::Awarded => {
                let corp_id = match grant.awarded_corp {
                    Some(id) => id,
                    None => continue,
                };

                // Calculate coverage progress for the region
                let coverage = calculate_region_coverage(world, grant.region_id, corp_id);
                let progress =
                    (coverage / grant.required_coverage_pct).clamp(0.0, 1.0);

                if let Some(g) = world.grants.get_mut(&grant_id) {
                    g.progress = progress;
                }

                // Auto-complete if progress reaches 100%
                if progress >= 1.0 {
                    completions.push((grant_id, corp_id, grant.reward_cash));
                }
                // Expire if deadline passed
                else if tick >= grant.deadline_tick {
                    expirations.push((grant_id, grant.region_id));
                }
            }
            GrantStatus::Available => {
                // Expire unclaimed grants past deadline
                if tick >= grant.deadline_tick {
                    expirations.push((grant_id, grant.region_id));
                }
            }
            _ => {}
        }
    }

    // Process completions
    for (grant_id, corp_id, reward) in completions {
        if let Some(fin) = world.financials.get_mut(&corp_id) {
            fin.cash += reward;
        }

        // Apply tax break
        if let Some(grant) = world.grants.get(&grant_id) {
            if grant.tax_break_pct > 0.0 {
                let region_id = grant.region_id;
                if let Some(region) = world.regions.get_mut(&region_id) {
                    region.tax_rate =
                        (region.tax_rate * (1.0 - grant.tax_break_pct)).max(0.01);
                }
            }
        }

        if let Some(g) = world.grants.get_mut(&grant_id) {
            g.status = GrantStatus::Completed;
        }

        world.event_queue.push(
            tick,
            gt_common::events::GameEvent::GrantCompleted {
                grant_id,
                corporation: corp_id,
                reward,
            },
        );
    }

    // Process expirations
    for (grant_id, region_id) in expirations {
        if let Some(g) = world.grants.get_mut(&grant_id) {
            g.status = GrantStatus::Expired;
        }

        world.event_queue.push(
            tick,
            gt_common::events::GameEvent::GrantExpired {
                grant_id,
                region: region_id,
            },
        );
    }

    // Cleanup old completed/expired grants (older than 500 ticks)
    let to_remove: Vec<EntityId> = world
        .grants
        .iter()
        .filter(|(_, g)| {
            (g.status == GrantStatus::Completed || g.status == GrantStatus::Expired)
                && tick > g.created_tick + 500
        })
        .map(|(&id, _)| id)
        .collect();
    for id in to_remove {
        world.grants.remove(&id);
    }
}

/// Generate grants for underserved regions.
/// An underserved region is one where average coverage satisfaction is below 30%.
fn generate_grants(world: &mut GameWorld) {
    let tick = world.current_tick();

    // Collect regions with low coverage
    let mut region_ids: Vec<EntityId> = world.regions.keys().copied().collect();
    region_ids.sort_unstable(); // deterministic order

    // Use a simple deterministic RNG based on tick and seed
    let rng_base = tick.wrapping_mul(0x517cc1b727220a95);

    let mut grants_created = 0;
    let max_grants_per_cycle = 3;

    for &region_id in &region_ids {
        if grants_created >= max_grants_per_cycle {
            break;
        }

        // Skip regions that already have an active/available grant
        let has_active_grant = world.grants.values().any(|g| {
            g.region_id == region_id
                && (g.status == GrantStatus::Available || g.status == GrantStatus::Awarded)
        });
        if has_active_grant {
            continue;
        }

        // Check if region is underserved (average demand satisfaction < 30%)
        let region = match world.regions.get(&region_id) {
            Some(r) => r,
            None => continue,
        };

        let city_ids = region.city_ids.clone();
        if city_ids.is_empty() {
            continue;
        }

        let avg_satisfaction: f64 = city_ids
            .iter()
            .filter_map(|cid| world.demands.get(cid))
            .map(|d| d.satisfaction)
            .sum::<f64>()
            / city_ids.len() as f64;

        if avg_satisfaction >= 0.3 {
            continue;
        }

        // Deterministic random check: ~50% chance per eligible region per cycle
        let hash = rng_base.wrapping_add(region_id.wrapping_mul(0x9e3779b97f4a7c15));
        if hash % 2 != 0 {
            continue;
        }

        // Scale reward based on region GDP and underservice level
        let region_gdp = region.gdp;
        let underservice_factor = 1.0 - avg_satisfaction;
        let reward = ((region_gdp * underservice_factor * 0.05) as i64).max(500_000);
        let required_coverage = 0.3 + (underservice_factor * 0.3); // 30-60% coverage target
        let tax_break = 0.05 + (underservice_factor * 0.10); // 5-15% tax break
        let deadline = tick + 300; // 300 ticks to complete

        let grant_id = world.allocate_entity();
        let grant = GovernmentGrant::new(
            region_id,
            required_coverage,
            reward,
            tax_break,
            deadline,
            tick,
        );
        world.grants.insert(grant_id, grant);

        world.event_queue.push(
            tick,
            gt_common::events::GameEvent::GrantAvailable {
                grant_id,
                region: region_id,
                reward,
            },
        );

        grants_created += 1;
    }
}

/// Calculate coverage ratio for a specific corporation in a region.
/// Returns 0.0-1.0 based on what fraction of the region's cities have
/// infrastructure nodes owned by the corp.
fn calculate_region_coverage(
    world: &GameWorld,
    region_id: EntityId,
    corp_id: EntityId,
) -> f64 {
    let region = match world.regions.get(&region_id) {
        Some(r) => r,
        None => return 0.0,
    };

    let city_ids = &region.city_ids;
    if city_ids.is_empty() {
        return 0.0;
    }

    // Get nodes owned by this corp
    let corp_nodes = match world.corp_infra_nodes.get(&corp_id) {
        Some(nodes) => nodes,
        None => return 0.0,
    };

    // For each city, check if there's at least one corp node in or near it
    let mut covered_cities = 0;
    for &city_id in city_ids {
        let city = match world.cities.get(&city_id) {
            Some(c) => c,
            None => continue,
        };

        // Check if any corp node is in one of the city's cells
        let has_coverage = corp_nodes.iter().any(|&node_id| {
            if let Some(pos) = world.positions.get(&node_id) {
                // Check if the node's nearest cell is one of this city's cells
                if let Some((cell_idx, _)) = world.find_nearest_cell(pos.x, pos.y) {
                    city.cells.contains(&cell_idx)
                } else {
                    false
                }
            } else {
                false
            }
        });

        if has_coverage {
            covered_cities += 1;
        }
    }

    covered_cities as f64 / city_ids.len() as f64
}
