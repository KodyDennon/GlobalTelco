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

        // 3. Finance management (all strategies)
        ai_manage_finances(world, corp_id, &ai_state, tick);

        // 4. Contract proposals (Expand and Compete strategies)
        if matches!(new_strategy, AIStrategy::Expand | AIStrategy::Compete) {
            ai_propose_contracts(world, corp_id, &ai_state, tick);
        }

        // 5. Accept/reject pending contract proposals
        ai_evaluate_incoming_contracts(world, corp_id, &ai_state, tick);
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

    // Find cells where this corp has presence
    let corp_cells: std::collections::HashSet<usize> = corp_nodes
        .iter()
        .filter_map(|&nid| world.infra_nodes.get(&nid).map(|n| n.cell_index))
        .collect();

    // Find a city with high demand but no corp infrastructure
    // Collect and sort to ensure deterministic tie-breaking
    let mut candidate_cities: Vec<(EntityId, usize, f64)> = world
        .cities
        .iter()
        .filter(|(_, city)| !corp_cells.contains(&city.cell_index) && city.telecom_demand > 100.0)
        .map(|(&id, city)| (id, city.cell_index, city.telecom_demand))
        .collect();
    candidate_cities.sort_unstable_by(|a, b| {
        b.2.partial_cmp(&a.2)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.0.cmp(&b.0))
    });
    let target_city = candidate_cities.first().map(|&(id, cell, _)| (id, cell));

    if let Some((_city_id, cell_index)) = target_city {
        // Acquire the land parcel first
        ai_acquire_parcel(world, corp_id, cell_index);

        // Build a central office with terrain-aware costs
        let terrain = world
            .cell_to_parcel
            .get(&cell_index)
            .and_then(|&pid| world.land_parcels.get(&pid))
            .map(|p| p.terrain)
            .unwrap_or(TerrainType::Rural);

        let node = InfraNode::new_on_terrain(NodeType::CentralOffice, cell_index, corp_id, terrain);
        let cost = node.construction_cost;

        if fin.cash > cost * 2 {
            let node_id = world.allocate_entity();
            let maintenance = node.maintenance_cost;

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
                    break;
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
    let corp_cells: std::collections::HashSet<usize> = world
        .corp_infra_nodes
        .get(&corp_id)
        .unwrap_or(&Vec::new())
        .iter()
        .filter_map(|&nid| world.infra_nodes.get(&nid).map(|n| n.cell_index))
        .collect();

    let mut compete_candidates: Vec<(EntityId, usize, f64)> = world
        .cities
        .iter()
        .filter(|(_, city)| {
            corp_cells.contains(&city.cell_index) && city.infrastructure_satisfaction < 0.5
        })
        .map(|(&id, city)| (id, city.cell_index, city.telecom_demand))
        .collect();
    compete_candidates.sort_unstable_by(|a, b| {
        b.2.partial_cmp(&a.2)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.0.cmp(&b.0))
    });
    let target = compete_candidates.first().map(|&(_, cell, _)| cell);

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
    if fin.cash < fin.cost_per_tick * 10 && fin.debt < fin.cash.abs() * 2 {
        let corp_nodes = world
            .corp_infra_nodes
            .get(&corp_id)
            .cloned()
            .unwrap_or_default();

        if corp_nodes.len() > 1 {
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

/// AI acquires a land parcel (sets ownership if unowned).
fn ai_acquire_parcel(world: &mut GameWorld, corp_id: EntityId, cell_index: usize) {
    if let Some(&parcel_id) = world.cell_to_parcel.get(&cell_index) {
        if let Some(parcel) = world.land_parcels.get_mut(&parcel_id) {
            if parcel.owner.is_none() {
                // Acquire the parcel — deduct cost
                let acquisition_cost =
                    (100_000.0 * parcel.cost_modifier) as Money;
                if let Some(fin) = world.financials.get_mut(&corp_id) {
                    if fin.cash > acquisition_cost {
                        fin.cash -= acquisition_cost;
                        parcel.owner = Some(corp_id);
                    }
                }
            }
        }
    }
}

/// AI finance management: take loans when cash is low, repay when flush.
fn ai_manage_finances(
    world: &mut GameWorld,
    corp_id: EntityId,
    ai: &AiState,
    _tick: Tick,
) {
    let fin = match world.financials.get(&corp_id) {
        Some(f) => f.clone(),
        None => return,
    };

    let credit_rating = world
        .corporations
        .get(&corp_id)
        .map(|c| c.credit_rating)
        .unwrap_or(CreditRating::D);

    // Debt tolerance varies by archetype
    let max_debt_ratio = match ai.archetype {
        AIArchetype::AggressiveExpander => 3.0,
        AIArchetype::DefensiveConsolidator => 0.5,
        AIArchetype::TechInnovator => 2.0,
        AIArchetype::BudgetOperator => 0.3,
    };

    let cash_runway = if fin.cost_per_tick > 0 {
        fin.cash as f64 / fin.cost_per_tick as f64
    } else {
        999.0
    };

    let current_debt_ratio = if fin.cash > 0 {
        fin.debt as f64 / fin.cash as f64
    } else {
        0.0
    };

    // Take a loan if cash is running low and we're under debt tolerance
    if cash_runway < 30.0
        && current_debt_ratio < max_debt_ratio
        && credit_rating != CreditRating::D
        && credit_rating != CreditRating::CCC
    {
        let loan_amount = fin.cost_per_tick * 90; // 90 ticks of operating costs
        if loan_amount > 0 {
            let interest_rate = match credit_rating {
                CreditRating::AAA => 0.03,
                CreditRating::AA => 0.04,
                CreditRating::A => 0.05,
                CreditRating::BBB => 0.07,
                CreditRating::BB => 0.10,
                CreditRating::B => 0.15,
                _ => return,
            };

            let debt = DebtInstrument::new(corp_id, loan_amount, interest_rate, 365);
            let payment = debt.payment_per_tick;
            let loan_id = world.allocate_entity();
            world.debt_instruments.insert(loan_id, debt);

            if let Some(f) = world.financials.get_mut(&corp_id) {
                f.cash += loan_amount;
                f.debt += loan_amount;
                f.cost_per_tick += payment;
            }

            world.event_queue.push(
                _tick,
                gt_common::events::GameEvent::LoanTaken {
                    corporation: corp_id,
                    amount: loan_amount,
                },
            );
        }
    }

    // Early repay loans if very flush (> 6x cash runway and conservative archetype)
    if cash_runway > 180.0
        && fin.debt > 0
        && matches!(
            ai.archetype,
            AIArchetype::DefensiveConsolidator | AIArchetype::BudgetOperator
        )
    {
        // Find a loan to repay early (sort to ensure determinism)
        let mut candidate_loans: Vec<(EntityId, Money)> = world
            .debt_instruments
            .iter()
            .filter(|(_, d)| d.holder == corp_id && d.principal > 0)
            .map(|(&id, d)| (id, d.principal))
            .collect();
        candidate_loans.sort_unstable_by_key(|t| t.0);
        let loan_to_repay = candidate_loans.first().copied();

        if let Some((loan_id, principal)) = loan_to_repay {
            let repay_amount = principal.min(fin.cash / 4); // Don't spend more than 25% of cash
            if repay_amount > 0 {
                if let Some(debt) = world.debt_instruments.get_mut(&loan_id) {
                    debt.principal -= repay_amount;
                    if let Some(f) = world.financials.get_mut(&corp_id) {
                        f.cash -= repay_amount;
                        f.debt = (f.debt - repay_amount).max(0);
                        if debt.is_paid_off() {
                            f.cost_per_tick = (f.cost_per_tick - debt.payment_per_tick).max(0);
                        }
                    }
                    if debt.is_paid_off() {
                        world.debt_instruments.remove(&loan_id);
                    }
                }
            }
        }
    }
}

/// AI proposes transit contracts to other corporations for bandwidth sharing.
fn ai_propose_contracts(
    world: &mut GameWorld,
    corp_id: EntityId,
    ai: &AiState,
    tick: Tick,
) {
    // Only propose if we have infrastructure to sell capacity from
    let corp_nodes = world
        .corp_infra_nodes
        .get(&corp_id)
        .cloned()
        .unwrap_or_default();
    if corp_nodes.len() < 2 {
        return;
    }

    // Don't propose too many contracts
    let active_contracts = world
        .contracts
        .values()
        .filter(|c| {
            (c.from == corp_id || c.to == corp_id)
                && matches!(c.status, ContractStatus::Active | ContractStatus::Proposed)
        })
        .count();
    if active_contracts >= 3 {
        return;
    }

    // Contract willingness based on archetype
    let willingness = match ai.archetype {
        AIArchetype::AggressiveExpander => 0.7,
        AIArchetype::DefensiveConsolidator => 0.3,
        AIArchetype::TechInnovator => 0.5,
        AIArchetype::BudgetOperator => 0.6,
    };

    // Use deterministic check based on tick and corp_id
    let check = ((tick.wrapping_mul(corp_id) >> 4) % 100) as f64 / 100.0;
    if check > willingness {
        return;
    }

    // Find another corp to propose to (one that has infrastructure in a region we don't)
    let mut other_corps: Vec<EntityId> = world
        .corporations
        .keys()
        .copied()
        .filter(|&id| id != corp_id)
        .collect();
    other_corps.sort_unstable();

    for &other_id in &other_corps {
        // Check we don't already have a contract with them
        let has_contract = world.contracts.values().any(|c| {
            ((c.from == corp_id && c.to == other_id)
                || (c.from == other_id && c.to == corp_id))
                && matches!(c.status, ContractStatus::Active | ContractStatus::Proposed)
        });
        if has_contract {
            continue;
        }

        // Check they have infrastructure
        let other_has_infra = world
            .corp_infra_nodes
            .get(&other_id)
            .map(|n| !n.is_empty())
            .unwrap_or(false);
        if !other_has_infra {
            continue;
        }

        // Propose a transit contract
        let capacity = 500.0;
        let price = 300; // per tick
        let duration = 180; // ~6 months
        let penalty = 5_000;

        let contract = Contract::new_proposal(
            ContractType::Transit,
            corp_id,
            other_id,
            capacity,
            price,
            tick,
            duration,
            penalty,
        );
        let contract_id = world.allocate_entity();
        world.contracts.insert(contract_id, contract);
        world.event_queue.push(
            tick,
            gt_common::events::GameEvent::ContractProposed {
                entity: contract_id,
                from: corp_id,
                to: other_id,
            },
        );
        break; // Only one proposal per cycle
    }
}

/// AI evaluates and accepts/rejects incoming contract proposals.
fn ai_evaluate_incoming_contracts(
    world: &mut GameWorld,
    corp_id: EntityId,
    ai: &AiState,
    tick: Tick,
) {
    let mut proposals: Vec<(EntityId, EntityId, Money)> = world
        .contracts
        .iter()
        .filter(|(_, c)| c.to == corp_id && c.status == ContractStatus::Proposed)
        .map(|(&id, c)| (id, c.from, c.price_per_tick))
        .collect();
    proposals.sort_unstable_by_key(|t| t.0);

    for (contract_id, _provider, price) in proposals {
        let fin = match world.financials.get(&corp_id) {
            Some(f) => f,
            None => continue,
        };

        // Accept if we can afford it and archetype is willing
        let affordable = price < fin.revenue_per_tick / 4;
        let willingness = match ai.archetype {
            AIArchetype::AggressiveExpander => 0.8,
            AIArchetype::DefensiveConsolidator => 0.4,
            AIArchetype::TechInnovator => 0.6,
            AIArchetype::BudgetOperator => 0.3,
        };

        let check = ((tick.wrapping_mul(contract_id) >> 3) % 100) as f64 / 100.0;

        if affordable && check < willingness {
            if let Some(c) = world.contracts.get_mut(&contract_id) {
                c.activate(tick);
                world.event_queue.push(
                    tick,
                    gt_common::events::GameEvent::ContractAccepted {
                        entity: contract_id,
                    },
                );
            }
        } else {
            // Reject
            world.contracts.remove(&contract_id);
        }
    }
}

/// Pick the appropriate fiber type based on the network levels of connected nodes.
fn pick_fiber_type(world: &GameWorld, from: EntityId, to: EntityId) -> EdgeType {
    let from_level = world
        .infra_nodes
        .get(&from)
        .map(|n| n.network_level)
        .unwrap_or(NetworkLevel::Local);
    let to_level = world
        .infra_nodes
        .get(&to)
        .map(|n| n.network_level)
        .unwrap_or(NetworkLevel::Local);

    let max_level = std::cmp::max(from_level, to_level);
    match max_level {
        NetworkLevel::Local => EdgeType::FiberLocal,
        NetworkLevel::Regional => EdgeType::FiberRegional,
        _ => EdgeType::FiberNational,
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

    let fiber_type = pick_fiber_type(world, from, to);
    let edge = InfraEdge::new(fiber_type, from, to, length_km, corp_id);
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
