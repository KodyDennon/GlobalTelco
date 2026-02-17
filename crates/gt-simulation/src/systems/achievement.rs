use crate::world::GameWorld;
use gt_common::types::{CreditRating, EntityId, NodeType};

pub fn run(world: &mut GameWorld) {
    let tick = world.current_tick();

    // Only run every 30 ticks for performance
    if !tick.is_multiple_of(30) {
        return;
    }

    let mut corp_ids: Vec<EntityId> = world.corporations.keys().copied().collect();
    corp_ids.sort_unstable();

    let total_regions = world.regions.len();

    for &corp_id in &corp_ids {
        // Ensure tracker exists
        if !world.achievements.contains_key(&corp_id) {
            world.achievements.insert(
                corp_id,
                crate::components::AchievementTracker::new(),
            );
        }

        let node_count = world
            .corp_infra_nodes
            .get(&corp_id)
            .map(|n| n.len())
            .unwrap_or(0);

        let fin = world.financials.get(&corp_id).cloned();
        let corp = world.corporations.get(&corp_id).cloned();

        let mut newly_unlocked: Vec<String> = Vec::new();

        // Check each achievement
        let tracker = match world.achievements.get(&corp_id) {
            Some(t) => t.clone(),
            None => continue,
        };

        // First Node
        if !tracker.is_unlocked("first_node") && node_count >= 1 {
            newly_unlocked.push("first_node".to_string());
        }

        // First Profit
        if !tracker.is_unlocked("first_profit") {
            if let Some(ref f) = fin {
                if f.revenue_per_tick > 0 {
                    newly_unlocked.push("first_profit".to_string());
                }
            }
        }

        // Ten Nodes
        if !tracker.is_unlocked("ten_nodes") && node_count >= 10 {
            newly_unlocked.push("ten_nodes".to_string());
        }

        // Hundred Nodes
        if !tracker.is_unlocked("hundred_nodes") && node_count >= 100 {
            newly_unlocked.push("hundred_nodes".to_string());
        }

        // Million Revenue
        if !tracker.is_unlocked("million_revenue") {
            if let Some(ref f) = fin {
                if f.revenue_per_tick * 365 >= 1_000_000 {
                    newly_unlocked.push("million_revenue".to_string());
                }
            }
        }

        // Billion Revenue
        if !tracker.is_unlocked("billion_revenue") {
            if let Some(ref f) = fin {
                if f.revenue_per_tick * 365 >= 1_000_000_000 {
                    newly_unlocked.push("billion_revenue".to_string());
                }
            }
        }

        // AAA Rating
        if !tracker.is_unlocked("aaa_rating") {
            if let Some(ref c) = corp {
                if c.credit_rating == CreditRating::AAA {
                    newly_unlocked.push("aaa_rating".to_string());
                }
            }
        }

        // Debt Free
        if !tracker.is_unlocked("debt_free") {
            if let Some(ref f) = fin {
                if f.debt == 0 && f.revenue_per_tick > f.cost_per_tick && f.cash > 0 {
                    newly_unlocked.push("debt_free".to_string());
                }
            }
        }

        // Global Backbone
        if !tracker.is_unlocked("global_backbone") {
            let has_backbone = world
                .corp_infra_nodes
                .get(&corp_id)
                .unwrap_or(&Vec::new())
                .iter()
                .any(|&nid| {
                    world
                        .infra_nodes
                        .get(&nid)
                        .map(|n| {
                            n.node_type == NodeType::DataCenter
                                || n.node_type == NodeType::SatelliteGround
                        })
                        .unwrap_or(false)
                });
            if has_backbone {
                newly_unlocked.push("global_backbone".to_string());
            }
        }

        // Ocean Cable
        if !tracker.is_unlocked("ocean_cable") {
            let has_submarine = world
                .corp_infra_nodes
                .get(&corp_id)
                .unwrap_or(&Vec::new())
                .iter()
                .any(|&nid| {
                    world
                        .infra_nodes
                        .get(&nid)
                        .map(|n| n.node_type == NodeType::SubmarineLanding)
                        .unwrap_or(false)
                });
            if has_submarine {
                newly_unlocked.push("ocean_cable".to_string());
            }
        }

        // First Contract
        if !tracker.is_unlocked("first_contract") {
            let has_active = world.contracts.values().any(|c| {
                (c.from == corp_id || c.to == corp_id)
                    && c.status == crate::components::ContractStatus::Active
            });
            if has_active {
                newly_unlocked.push("first_contract".to_string());
            }
        }

        // All Regions
        if !tracker.is_unlocked("all_regions") && total_regions > 0 {
            let corp_regions: std::collections::HashSet<EntityId> = world
                .corp_infra_nodes
                .get(&corp_id)
                .unwrap_or(&Vec::new())
                .iter()
                .filter_map(|&nid| {
                    world
                        .positions
                        .get(&nid)
                        .and_then(|p| p.region_id)
                })
                .collect();
            if corp_regions.len() >= total_regions {
                newly_unlocked.push("all_regions".to_string());
            }
        }

        // Update progress for node count
        if let Some(t) = world.achievements.get_mut(&corp_id) {
            t.set_progress("nodes", node_count as f64);
            if let Some(ref f) = fin {
                t.set_progress("annual_revenue", (f.revenue_per_tick * 365) as f64);
                t.set_progress("cash", f.cash as f64);
            }
        }

        // Apply unlocks
        for achievement in &newly_unlocked {
            if let Some(t) = world.achievements.get_mut(&corp_id) {
                t.unlock(achievement);
            }
            world.event_queue.push(
                tick,
                gt_common::events::GameEvent::AchievementUnlocked {
                    corporation: corp_id,
                    achievement: achievement.clone(),
                },
            );
        }
    }

    // Update victory conditions for player
    if let Some(player_id) = world.player_corp_id() {
        let mut victory = world.victory_state.take().unwrap_or_default();

        // Domination: % of regions with player infrastructure
        if total_regions > 0 {
            let player_regions: std::collections::HashSet<EntityId> = world
                .corp_infra_nodes
                .get(&player_id)
                .unwrap_or(&Vec::new())
                .iter()
                .filter_map(|&nid| world.positions.get(&nid).and_then(|p| p.region_id))
                .collect();
            victory.domination_score = player_regions.len() as f64 / total_regions as f64;
        }

        // Tech: % of research completed
        let total_tech = world.tech_research.len().max(1);
        let completed_tech = world
            .tech_research
            .values()
            .filter(|r| r.completed && r.researcher == Some(player_id))
            .count();
        victory.tech_score = completed_tech as f64 / total_tech as f64;

        // Wealth
        if let Some(fin) = world.financials.get(&player_id) {
            victory.wealth_score = (fin.cash as f64 / 10_000_000_000.0).min(1.0);
        }

        // Infrastructure
        let node_count = world
            .corp_infra_nodes
            .get(&player_id)
            .map(|n| n.len())
            .unwrap_or(0);
        victory.infrastructure_score = (node_count as f64 / 200.0).min(1.0);

        victory.update_total();

        // Check victory triggers
        if victory.domination_score >= 0.75 && victory.victory_type.is_none() {
            victory.victory_type = Some("Domination".to_string());
            world.event_queue.push(
                tick,
                gt_common::events::GameEvent::VictoryAchieved {
                    corporation: player_id,
                    victory_type: "Domination".to_string(),
                    score: victory.total_score,
                },
            );
        } else if victory.tech_score >= 1.0 && victory.victory_type.is_none() {
            victory.victory_type = Some("Technology".to_string());
            world.event_queue.push(
                tick,
                gt_common::events::GameEvent::VictoryAchieved {
                    corporation: player_id,
                    victory_type: "Technology".to_string(),
                    score: victory.total_score,
                },
            );
        } else if victory.wealth_score >= 1.0 && victory.victory_type.is_none() {
            victory.victory_type = Some("Wealth".to_string());
            world.event_queue.push(
                tick,
                gt_common::events::GameEvent::VictoryAchieved {
                    corporation: player_id,
                    victory_type: "Wealth".to_string(),
                    score: victory.total_score,
                },
            );
        }

        world.victory_state = Some(victory);
    }
}
