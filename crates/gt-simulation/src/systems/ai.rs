use crate::components::*;
use crate::world::GameWorld;
use gt_common::types::*;

pub fn run(world: &mut GameWorld) {
    let tick = world.current_tick();

    // Only run AI every 10 ticks to reduce computation
    if !tick.is_multiple_of(10) {
        return;
    }

    // Collect AI corp data
    let mut ai_corps: Vec<(EntityId, AiState)> = world
        .ai_states
        .iter()
        .map(|(&id, state)| (id, state.clone()))
        .collect();
    ai_corps.sort_unstable_by_key(|t| t.0);

    for (corp_id, ai_state) in ai_corps {
        let financial = match world.financials.get(&corp_id) {
            Some(f) => f.clone(),
            None => continue,
        };

        // 1. Strategy selection based on financial health
        let new_strategy = select_strategy(&ai_state, &financial);
        if let Some(state) = world.ai_states.get_mut(&corp_id) {
            state.strategy = new_strategy;
        }

        // 2. Execute strategy actions
        match new_strategy {
            AIStrategy::Expand => ai_expand(world, corp_id, &ai_state, &financial, tick),
            AIStrategy::Consolidate => ai_consolidate(world, corp_id, &ai_state, tick),
            AIStrategy::Compete => ai_compete(world, corp_id, &ai_state, &financial, tick),
            AIStrategy::Survive => ai_survive(world, corp_id, &financial, tick),
        }
    }
}

fn select_strategy(ai: &AiState, fin: &Financial) -> AIStrategy {
    let cash_ratio = fin.cash as f64 / (fin.cost_per_tick as f64 * 90.0).max(1.0);
    let profit = fin.revenue_per_tick - fin.cost_per_tick;
    let debt_heavy = fin.debt > fin.cash * 2;

    if cash_ratio < 1.0 || (profit < 0 && fin.cash < fin.cost_per_tick * 30) {
        return AIStrategy::Survive;
    }

    match ai.archetype {
        AIArchetype::AggressiveExpander => {
            if cash_ratio > 3.0 {
                AIStrategy::Expand
            } else if profit > 0 {
                AIStrategy::Compete
            } else {
                AIStrategy::Consolidate
            }
        }
        AIArchetype::DefensiveConsolidator => {
            if debt_heavy {
                AIStrategy::Survive
            } else {
                AIStrategy::Consolidate
            }
        }
        AIArchetype::TechInnovator => {
            if cash_ratio > 4.0 {
                AIStrategy::Expand
            } else {
                AIStrategy::Consolidate
            }
        }
        AIArchetype::BudgetOperator => {
            if profit > 0 && cash_ratio > 6.0 && !debt_heavy {
                AIStrategy::Expand
            } else {
                AIStrategy::Consolidate
            }
        }
    }
}

fn ai_expand(world: &mut GameWorld, corp_id: EntityId, ai: &AiState, fin: &Financial, tick: Tick) {
    let corp_nodes = world
        .corp_infra_nodes
        .get(&corp_id)
        .cloned()
        .unwrap_or_default();

    // Find regions where this corp has no presence
    let corp_regions: std::collections::HashSet<usize> = corp_nodes
        .iter()
        .filter_map(|&nid| world.infra_nodes.get(&nid).map(|n| n.cell_index))
        .collect();

    // Find a city with high demand but no corp infrastructure
    let target_city = world
        .cities
        .iter()
        .filter(|(_, city)| !corp_regions.contains(&city.cell_index) && city.telecom_demand > 100.0)
        .max_by(|(_, a), (_, b)| {
            a.telecom_demand
                .partial_cmp(&b.telecom_demand)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(&id, city)| (id, city.cell_index));

    if let Some((_city_id, cell_index)) = target_city {
        // Check if we can afford a central office
        let node = InfraNode::new(NodeType::CentralOffice, cell_index, corp_id);
        let cost = node.construction_cost;

        if fin.cash > cost * 2 {
            // Keep a safety margin
            let node_id = world.allocate_entity();
            let maintenance = node.maintenance_cost;
            let _throughput = node.max_throughput;

            world.infra_nodes.insert(node_id, node);
            world
                .constructions
                .insert(node_id, Construction::new(tick, 20));
            world.ownerships.insert(node_id, Ownership::sole(corp_id));
            world.healths.insert(node_id, Health::new());
            world.capacities.insert(node_id, Capacity::new(0.0));

            if let Some(&(lat, lon)) = world.grid_cell_positions.get(cell_index) {
                let region_id = world.cell_to_region.get(&cell_index).copied();
                world.positions.insert(
                    node_id,
                    Position {
                        x: lon,
                        y: lat,
                        region_id,
                    },
                );
            }

            world
                .corp_infra_nodes
                .entry(corp_id)
                .or_default()
                .push(node_id);
            world.network.add_node(node_id);

            if let Some(f) = world.financials.get_mut(&corp_id) {
                f.cash -= cost;
                f.cost_per_tick += maintenance;
            }

            world.event_queue.push(
                tick,
                gt_common::events::GameEvent::ConstructionStarted {
                    entity: node_id,
                    tick,
                },
            );

            // Also build an edge connecting to nearest existing node
            if let Some(&nearest) = corp_nodes.first() {
                build_ai_edge(world, corp_id, nearest, node_id, tick);
            }
        }
    }

    // Tech innovator: prioritize research
    if ai.archetype == AIArchetype::TechInnovator {
        let has_research = world
            .tech_research
            .values()
            .any(|r| r.researcher == Some(corp_id) && !r.completed);
        if !has_research && fin.cash > 5_000_000 {
            let research_id = world.allocate_entity();
            let mut research =
                TechResearch::new(ResearchCategory::OpticalNetworks, "AI Optical Research");
            research.researcher = Some(corp_id);
            world.tech_research.insert(research_id, research);
        }
    }
}

fn ai_consolidate(world: &mut GameWorld, corp_id: EntityId, _ai: &AiState, _tick: Tick) {
    // Upgrade existing nodes if they're highly utilized
    let corp_nodes = world
        .corp_infra_nodes
        .get(&corp_id)
        .cloned()
        .unwrap_or_default();

    for &node_id in &corp_nodes {
        if world.constructions.contains_key(&node_id) {
            continue;
        }

        let utilization = world
            .capacities
            .get(&node_id)
            .map(|c| c.utilization())
            .unwrap_or(0.0);

        // Upgrade if utilization > 80%
        if utilization > 0.8 {
            let (cost, owner) = match world.infra_nodes.get(&node_id) {
                Some(n) => (n.construction_cost / 2, n.owner),
                None => continue,
            };

            if let Some(fin) = world.financials.get(&owner) {
                if fin.cash > cost * 3 {
                    if let Some(f) = world.financials.get_mut(&owner) {
                        f.cash -= cost;
                    }
                    if let Some(node) = world.infra_nodes.get_mut(&node_id) {
                        node.max_throughput *= 1.5;
                    }
                    if let Some(cap) = world.capacities.get_mut(&node_id) {
                        cap.max_throughput *= 1.5;
                    }
                    if let Some(health) = world.healths.get_mut(&node_id) {
                        health.condition = 1.0;
                    }
                    break; // Only one upgrade per AI cycle
                }
            }
        }
    }
}

fn ai_compete(
    world: &mut GameWorld,
    corp_id: EntityId,
    _ai: &AiState,
    fin: &Financial,
    tick: Tick,
) {
    // Build cell towers in high-demand areas to compete
    let corp_cells: std::collections::HashSet<usize> = world
        .corp_infra_nodes
        .get(&corp_id)
        .unwrap_or(&Vec::new())
        .iter()
        .filter_map(|&nid| world.infra_nodes.get(&nid).map(|n| n.cell_index))
        .collect();

    // Find cities where we have presence but low coverage
    let target = world
        .cities
        .iter()
        .filter(|(_, city)| {
            corp_cells.contains(&city.cell_index) && city.infrastructure_satisfaction < 0.5
        })
        .max_by(|(_, a), (_, b)| {
            a.telecom_demand
                .partial_cmp(&b.telecom_demand)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(_, city)| city.cell_index);

    if let Some(cell_index) = target {
        let node = InfraNode::new(NodeType::CellTower, cell_index, corp_id);
        let cost = node.construction_cost;

        if fin.cash > cost * 3 {
            let node_id = world.allocate_entity();
            let maintenance = node.maintenance_cost;

            world.infra_nodes.insert(node_id, node);
            world
                .constructions
                .insert(node_id, Construction::new(tick, 10));
            world.ownerships.insert(node_id, Ownership::sole(corp_id));
            world.healths.insert(node_id, Health::new());
            world.capacities.insert(node_id, Capacity::new(0.0));

            if let Some(&(lat, lon)) = world.grid_cell_positions.get(cell_index) {
                let region_id = world.cell_to_region.get(&cell_index).copied();
                world.positions.insert(
                    node_id,
                    Position {
                        x: lon,
                        y: lat,
                        region_id,
                    },
                );
            }

            world
                .corp_infra_nodes
                .entry(corp_id)
                .or_default()
                .push(node_id);
            world.network.add_node(node_id);

            if let Some(f) = world.financials.get_mut(&corp_id) {
                f.cash -= cost;
                f.cost_per_tick += maintenance;
            }

            world.event_queue.push(
                tick,
                gt_common::events::GameEvent::ConstructionStarted {
                    entity: node_id,
                    tick,
                },
            );
        }
    }
}

fn ai_survive(world: &mut GameWorld, corp_id: EntityId, fin: &Financial, _tick: Tick) {
    // If in survival mode and has debt, try to take a loan to stay afloat
    if fin.cash < fin.cost_per_tick * 10 && fin.debt < fin.cash.abs() * 2 {
        // Cut costs by decommissioning least-utilized nodes
        let corp_nodes = world
            .corp_infra_nodes
            .get(&corp_id)
            .cloned()
            .unwrap_or_default();

        if corp_nodes.len() > 1 {
            // Find least utilized node
            let least_utilized = corp_nodes
                .iter()
                .filter(|&&nid| !world.constructions.contains_key(&nid))
                .min_by(|&&a, &&b| {
                    let ua = world
                        .capacities
                        .get(&a)
                        .map(|c| c.utilization())
                        .unwrap_or(0.0);
                    let ub = world
                        .capacities
                        .get(&b)
                        .map(|c| c.utilization())
                        .unwrap_or(0.0);
                    ua.partial_cmp(&ub).unwrap_or(std::cmp::Ordering::Equal)
                })
                .copied();

            if let Some(node_id) = least_utilized {
                // Decommission to save costs
                if let Some(node) = world.infra_nodes.remove(&node_id) {
                    let salvage = node.construction_cost / 5;
                    if let Some(f) = world.financials.get_mut(&corp_id) {
                        f.cash += salvage;
                        f.cost_per_tick = (f.cost_per_tick - node.maintenance_cost).max(0);
                    }
                    world.network.remove_node(node_id);
                    world.positions.remove(&node_id);
                    world.healths.remove(&node_id);
                    world.capacities.remove(&node_id);
                    world.ownerships.remove(&node_id);
                    if let Some(nodes) = world.corp_infra_nodes.get_mut(&corp_id) {
                        nodes.retain(|&id| id != node_id);
                    }
                }
            }
        }
    }
}

fn build_ai_edge(
    world: &mut GameWorld,
    corp_id: EntityId,
    from: EntityId,
    to: EntityId,
    _tick: Tick,
) {
    let from_pos = world.positions.get(&from);
    let to_pos = world.positions.get(&to);

    let length_km = match (from_pos, to_pos) {
        (Some(a), Some(b)) => {
            let dlat = (a.y - b.y).to_radians();
            let dlon = (a.x - b.x).to_radians();
            let lat1 = a.y.to_radians();
            let lat2 = b.y.to_radians();
            let a_val =
                (dlat / 2.0).sin().powi(2) + lat1.cos() * lat2.cos() * (dlon / 2.0).sin().powi(2);
            let c = 2.0 * a_val.sqrt().asin();
            6371.0 * c
        }
        _ => 100.0,
    };

    let edge = InfraEdge::new(EdgeType::FiberOptic, from, to, length_km, corp_id);
    let cost = edge.construction_cost;
    let maintenance = edge.maintenance_cost;

    if let Some(fin) = world.financials.get(&corp_id) {
        if fin.cash < cost {
            return;
        }
    }

    let edge_id = world.allocate_entity();
    world.infra_edges.insert(edge_id, edge);
    world.network.add_edge(from, to);

    if let Some(f) = world.financials.get_mut(&corp_id) {
        f.cash -= cost;
        f.cost_per_tick += maintenance;
    }
}
