use crate::components::covert_ops::MissionType;
use crate::world::GameWorld;
use gt_common::types::EntityId;

pub fn run(world: &mut GameWorld) {
    let tick = world.current_tick();

    // Intel Decay: Reduce all intel levels periodically (matches Doc 3, Section 5)
    // Full (3) -> Basic (1) after 50 ticks, Basic (1) -> None (0) after 50 ticks
    if tick.is_multiple_of(50) {
        let mut keys_to_decay = Vec::new();
        for (key, &level) in &world.intel_levels {
            if level > 0 {
                keys_to_decay.push(*key);
            }
        }
        for key in keys_to_decay {
            if let Some(level) = world.intel_levels.get_mut(&key) {
                // If Full (3) or High (2), drop to Basic (1)
                // If Basic (1), drop to None (0)
                if *level > 1 {
                    *level = 1;
                } else {
                    *level = 0;
                }
            }
        }
    }

    // Only run mission processing every 5 ticks
    if !tick.is_multiple_of(5) {
        return;
    }

    // Collect corps with active missions
    let mut corps_with_missions: Vec<EntityId> = world
        .covert_ops
        .keys()
        .copied()
        .collect();
    corps_with_missions.sort_unstable();

    for corp_id in corps_with_missions {
        let ops = match world.covert_ops.get(&corp_id) {
            Some(o) => o.clone(),
            None => continue,
        };

        let mut completed_indices: Vec<usize> = Vec::new();

        for (i, mission) in ops.active_missions.iter().enumerate() {
            if mission.completed {
                continue;
            }
            if tick < mission.start_tick + mission.duration {
                continue;
            }
            completed_indices.push(i);
        }

        for &idx in completed_indices.iter().rev() {
            let mission = ops.active_missions[idx].clone();
            let rng_val = ((tick.wrapping_mul(corp_id).wrapping_mul(idx as u64 + 1)) >> 16) as f64
                / (1u64 << 48) as f64;
            let success = rng_val < mission.success_chance;

            match mission.mission_type {
                MissionType::Espionage => {
                    if success {
                        // Increment intel level (capped at 3)
                        let key = (corp_id, mission.target);
                        let current = world.intel_levels.get(&key).copied().unwrap_or(0);
                        if current < 3 {
                            world.intel_levels.insert(key, current + 1);
                        }

                        world.event_queue.push(
                            tick,
                            gt_common::events::GameEvent::EspionageCompleted {
                                spy: corp_id,
                                target: mission.target,
                            },
                        );
                    } else {
                        // Detected — reputation penalty
                        let penalty = mission.cost / 2;
                        if let Some(corp) = world.corporations.get_mut(&corp_id) {
                            corp.reputation = (corp.reputation - 5.0).max(0.0);
                        }
                        world.event_queue.push(
                            tick,
                            gt_common::events::GameEvent::EspionageDetected {
                                spy: corp_id,
                                target: mission.target,
                                penalty,
                            },
                        );
                    }
                }
                MissionType::Sabotage => {
                    if success {
                        // Damage target's infrastructure
                        let target_nodes: Vec<EntityId> = world
                            .corp_infra_nodes
                            .get(&mission.target)
                            .cloned()
                            .unwrap_or_default();

                        if let Some(&node_id) = target_nodes.first() {
                            if let Some(health) = world.healths.get_mut(&node_id) {
                                let damage = 0.3;
                                health.condition = (health.condition - damage).max(0.1);
                                world.event_queue.push(
                                    tick,
                                    gt_common::events::GameEvent::SabotageCompleted {
                                        saboteur: corp_id,
                                        target: mission.target,
                                        damage,
                                    },
                                );
                            }
                        }

                        // Attacker reputation cost
                        if let Some(corp) = world.corporations.get_mut(&corp_id) {
                            corp.reputation = (corp.reputation - 3.0).max(0.0);
                        }
                    } else {
                        // Detected — bigger reputation hit
                        let penalty = mission.cost;
                        if let Some(corp) = world.corporations.get_mut(&corp_id) {
                            corp.reputation = (corp.reputation - 10.0).max(0.0);
                        }
                        world.event_queue.push(
                            tick,
                            gt_common::events::GameEvent::SabotageDetected {
                                saboteur: corp_id,
                                target: mission.target,
                                penalty,
                            },
                        );
                    }
                }
            }

            // Mark mission as completed
            if let Some(ops) = world.covert_ops.get_mut(&corp_id) {
                if let Some(m) = ops.active_missions.get_mut(idx) {
                    m.completed = true;
                }
            }
        }

        // Clean up completed missions
        if let Some(ops) = world.covert_ops.get_mut(&corp_id) {
            ops.active_missions.retain(|m| !m.completed);
        }
    }
}
