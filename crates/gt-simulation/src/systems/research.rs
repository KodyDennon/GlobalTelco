use crate::world::GameWorld;

pub fn run(world: &mut GameWorld) {
    let tick = world.current_tick();

    // Progress active research based on corp R&D budget
    let mut research_ids: Vec<u64> = world.tech_research.keys().copied().collect();
    research_ids.sort_unstable();
    let mut completed = Vec::new();

    for &research_id in &research_ids {
        let (researcher, category) = match world.tech_research.get(&research_id) {
            Some(r) if !r.completed => match r.researcher {
                Some(corp_id) => (corp_id, r.category),
                None => continue,
            },
            _ => continue,
        };

        // R&D investment: use 1% of cash or budget policy if set
        let investment = if let Some(fin) = world.financials.get(&researcher) {
            let budget = world
                .policies
                .get(&researcher)
                .and_then(|p| p.get("budget_research"))
                .and_then(|v| v.parse::<i64>().ok())
                .unwrap_or((fin.cash as f64 * 0.01) as i64);
            budget.max(0)
        } else {
            0
        };

        if investment <= 0 {
            continue;
        }

        // Deduct R&D cost
        if let Some(fin) = world.financials.get_mut(&researcher) {
            fin.cash -= investment;
        }

        // Advance research
        let just_completed = if let Some(research) = world.tech_research.get_mut(&research_id) {
            research.advance(investment)
        } else {
            false
        };

        if just_completed {
            completed.push((research_id, researcher, category));
        }
    }

    // Apply completed research bonuses
    for (research_id, researcher, category) in completed {
        let throughput_bonus = category.throughput_bonus();
        let cost_reduction = category.cost_reduction();

        // Apply throughput bonus to all owned nodes
        if throughput_bonus > 0.0 {
            let corp_nodes = world
                .corp_infra_nodes
                .get(&researcher)
                .cloned()
                .unwrap_or_default();

            for &node_id in &corp_nodes {
                if let Some(node) = world.infra_nodes.get_mut(&node_id) {
                    node.max_throughput *= 1.0 + throughput_bonus;
                }
                if let Some(cap) = world.capacities.get_mut(&node_id) {
                    cap.max_throughput *= 1.0 + throughput_bonus;
                }
            }
        }

        // Apply cost reduction
        if cost_reduction > 0.0 {
            if let Some(fin) = world.financials.get_mut(&researcher) {
                fin.cost_per_tick = (fin.cost_per_tick as f64 * (1.0 - cost_reduction)) as i64;
            }
        }

        let tech_name = world
            .tech_research
            .get(&research_id)
            .map(|r| r.name.clone())
            .unwrap_or_default();

        world.event_queue.push(
            tick,
            gt_common::events::GameEvent::ResearchCompleted {
                corporation: researcher,
                tech: tech_name,
            },
        );
    }
}
