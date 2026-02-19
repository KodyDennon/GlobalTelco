use crate::components::*;
use crate::world::GameWorld;
use gt_common::types::*;

pub fn run(world: &mut GameWorld) {
    let tick = world.current_tick();

    // Run AI every 5 ticks (and always on tick 1 so AI acts immediately)
    if tick > 1 && tick % 5 != 0 {
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
        // Proxy mode: maintain existing policies only, skip strategic decisions
        if ai_state.proxy_mode {
            continue;
        }

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

        // 6. Auction bidding
        ai_bid_on_auctions(world, corp_id, &ai_state, &financial, tick);

        // 7. Evaluate incoming acquisition proposals
        ai_evaluate_acquisition_proposals(world, corp_id, &ai_state, tick);

        // 8. Espionage (Aggressive Expander only)
        if ai_state.archetype == AIArchetype::AggressiveExpander {
            ai_espionage(world, corp_id, &ai_state, &financial, tick);
        }

        // 9. Lobbying
        ai_lobbying(world, corp_id, &ai_state, &financial, tick);
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

    // Coverage-aware expansion: find city cells with high population but no/low coverage
    // Score = population_in_cell × (1 - coverage_satisfaction) for cells we don't cover
    let mut candidate_cells: Vec<(usize, f64, EntityId)> = Vec::new();
    for (&city_id, city) in &world.cities {
        for &ci in &city.cells {
            if corp_cells.contains(&ci) {
                continue; // Already have a node here
            }
            // Check if we're the dominant owner — skip cells we already dominate
            let our_coverage = world.cell_coverage.get(&ci)
                .map(|c| c.dominant_owner == Some(corp_id))
                .unwrap_or(false);
            if our_coverage {
                continue;
            }

            let cell_pop = city.population as f64 / city.cells.len().max(1) as f64;
            let existing_coverage = world.cell_coverage.get(&ci)
                .map(|c| (c.bandwidth / (cell_pop * 0.05).max(1.0)).clamp(0.0, 1.0))
                .unwrap_or(0.0);

            // Higher score = better expansion target
            let score = cell_pop * (1.0 - existing_coverage);
            if score > 50.0 {
                candidate_cells.push((ci, score, city_id));
            }
        }
    }
    candidate_cells.sort_unstable_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.0.cmp(&b.0))
    });

    // Pick node type based on corp size and situation
    let node_count = corp_nodes.len();
    let node_type = pick_expansion_node_type(ai, node_count, tick, corp_id);

    if let Some(&(cell_index, _score, _city_id)) = candidate_cells.first() {
        ai_acquire_parcel(world, corp_id, cell_index);

        let terrain = world
            .cell_to_parcel
            .get(&cell_index)
            .and_then(|&pid| world.land_parcels.get(&pid))
            .map(|p| p.terrain)
            .unwrap_or(TerrainType::Rural);

        let node = InfraNode::new_on_terrain(node_type, cell_index, corp_id, terrain);
        let cost = node.construction_cost;
        let build_time = match node_type {
            NodeType::CellTower | NodeType::WirelessRelay => 10,
            NodeType::CentralOffice => 20,
            NodeType::ExchangePoint => 30,
            NodeType::DataCenter => 50,
            NodeType::SatelliteGround => 40,
            NodeType::SubmarineLanding => 60,
        };

        if fin.cash > cost * 2 {
            let node_id = world.allocate_entity();
            let maintenance = node.maintenance_cost;

            world.infra_nodes.insert(node_id, node);
            world
                .constructions
                .insert(node_id, Construction::new(tick, build_time));
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

            // Connect to nearest existing node
            if !corp_nodes.is_empty() {
                let nearest = find_nearest_node(world, &corp_nodes, cell_index);
                if let Some(nearest_id) = nearest {
                    build_ai_edge(world, corp_id, nearest_id, node_id, tick);
                }
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

/// Pick node type for AI expansion based on archetype, network size, and situational factors.
fn pick_expansion_node_type(ai: &AiState, node_count: usize, tick: Tick, corp_id: EntityId) -> NodeType {
    // Use tick + corp_id for deterministic variety
    let variety_seed = ((tick.wrapping_mul(corp_id) >> 3) % 100) as usize;

    match ai.archetype {
        AIArchetype::AggressiveExpander => {
            // Aggressors build lots of cell towers for fast coverage, with occasional offices
            if node_count > 8 && variety_seed < 20 {
                NodeType::ExchangePoint
            } else if variety_seed < 30 {
                NodeType::CentralOffice
            } else {
                NodeType::CellTower
            }
        }
        AIArchetype::DefensiveConsolidator => {
            // Defenders build solid infrastructure: offices and data centers
            if node_count > 5 && variety_seed < 25 {
                NodeType::DataCenter
            } else if variety_seed < 40 {
                NodeType::CellTower
            } else {
                NodeType::CentralOffice
            }
        }
        AIArchetype::TechInnovator => {
            // Tech innovators build advanced infrastructure
            if node_count > 6 && variety_seed < 20 {
                NodeType::DataCenter
            } else if variety_seed < 35 {
                NodeType::SatelliteGround
            } else if variety_seed < 60 {
                NodeType::CellTower
            } else {
                NodeType::CentralOffice
            }
        }
        AIArchetype::BudgetOperator => {
            // Budget operators stick to cheap cell towers and wireless relays
            if variety_seed < 25 {
                NodeType::WirelessRelay
            } else if variety_seed < 70 {
                NodeType::CellTower
            } else {
                NodeType::CentralOffice
            }
        }
    }
}

/// Find the nearest existing node to a given cell index.
fn find_nearest_node(world: &GameWorld, node_ids: &[EntityId], target_cell: usize) -> Option<EntityId> {
    let target_pos = world.grid_cell_positions.get(target_cell)?;

    let mut best: Option<(EntityId, f64)> = None;
    for &nid in node_ids {
        let node = match world.infra_nodes.get(&nid) {
            Some(n) => n,
            None => continue,
        };
        let node_pos = match world.grid_cell_positions.get(node.cell_index) {
            Some(p) => p,
            None => continue,
        };
        let dlat = target_pos.0 - node_pos.0;
        let dlon = target_pos.1 - node_pos.1;
        let dist = dlat * dlat + dlon * dlon;
        if best.is_none() || dist < best.unwrap().1 {
            best = Some((nid, dist));
        }
    }
    best.map(|(id, _)| id)
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
    let corp_nodes = world
        .corp_infra_nodes
        .get(&corp_id)
        .cloned()
        .unwrap_or_default();

    // Find city cells where a competitor is dominant but we could steal market share
    // Target cells where another corp dominates but satisfaction is low (opportunity!)
    let mut compete_targets: Vec<(usize, f64)> = Vec::new();
    for (_, city) in &world.cities {
        if city.infrastructure_satisfaction > 0.7 {
            continue; // Well-served city, skip
        }
        for &ci in &city.cells {
            let coverage = world.cell_coverage.get(&ci);
            let is_competitor_dominant = coverage
                .and_then(|c| c.dominant_owner)
                .map(|owner| owner != corp_id)
                .unwrap_or(false);
            let low_quality = coverage
                .map(|c| c.signal_strength < 50.0)
                .unwrap_or(true);

            if is_competitor_dominant || low_quality {
                let cell_pop = city.population as f64 / city.cells.len().max(1) as f64;
                let score = cell_pop * (1.0 - city.infrastructure_satisfaction);
                if score > 20.0 {
                    compete_targets.push((ci, score));
                }
            }
        }
    }

    compete_targets.sort_unstable_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.0.cmp(&b.0))
    });

    if let Some(&(cell_index, _)) = compete_targets.first() {
        // Build a cell tower to compete for coverage
        let node = InfraNode::new(NodeType::CellTower, cell_index, corp_id);
        let cost = node.construction_cost;

        if fin.cash > cost * 3 {
            ai_acquire_parcel(world, corp_id, cell_index);

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

            // Connect to nearest existing node
            if !corp_nodes.is_empty() {
                let nearest = find_nearest_node(world, &corp_nodes, cell_index);
                if let Some(nearest_id) = nearest {
                    build_ai_edge(world, corp_id, nearest_id, node_id, tick);
                }
            }
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

/// AI bids on active auctions if assets are worth acquiring.
fn ai_bid_on_auctions(
    world: &mut GameWorld,
    corp_id: EntityId,
    ai: &AiState,
    fin: &Financial,
    tick: Tick,
) {
    let mut auction_ids: Vec<EntityId> = world
        .auctions
        .iter()
        .filter(|(_, a)| {
            a.status == crate::components::AuctionStatus::Open && a.seller != corp_id
        })
        .map(|(&id, _)| id)
        .collect();
    auction_ids.sort_unstable();

    for auction_id in auction_ids {
        let auction = match world.auctions.get(&auction_id) {
            Some(a) => a.clone(),
            None => continue,
        };

        // Calculate asset value
        let asset_value: i64 = auction
            .assets
            .iter()
            .filter_map(|&id| world.infra_nodes.get(&id).map(|n| n.construction_cost))
            .sum();

        // Bid based on archetype willingness
        let willingness = match ai.archetype {
            AIArchetype::AggressiveExpander => 0.8,
            AIArchetype::DefensiveConsolidator => 0.4,
            AIArchetype::TechInnovator => 0.5,
            AIArchetype::BudgetOperator => 0.6,
        };

        let bid_multiplier = match ai.archetype {
            AIArchetype::AggressiveExpander => 0.6,
            AIArchetype::DefensiveConsolidator => 0.3,
            AIArchetype::TechInnovator => 0.4,
            AIArchetype::BudgetOperator => 0.25,
        };

        let check = ((tick.wrapping_mul(corp_id).wrapping_mul(auction_id) >> 8) % 100) as f64
            / 100.0;
        if check > willingness {
            continue;
        }

        let bid = (asset_value as f64 * bid_multiplier) as i64;
        if bid > 0 && fin.cash > bid * 2 {
            if let Some(a) = world.auctions.get_mut(&auction_id) {
                a.place_bid(corp_id, bid);
                world.event_queue.push(
                    tick,
                    gt_common::events::GameEvent::AuctionBidPlaced {
                        auction: auction_id,
                        bidder: corp_id,
                        amount: bid,
                    },
                );
            }
        }
    }
}

/// AI evaluates incoming acquisition proposals.
fn ai_evaluate_acquisition_proposals(
    world: &mut GameWorld,
    corp_id: EntityId,
    ai: &AiState,
    tick: Tick,
) {
    let mut proposals: Vec<(EntityId, EntityId, i64)> = world
        .acquisition_proposals
        .iter()
        .filter(|(_, p)| {
            p.target == corp_id
                && p.status == crate::components::AcquisitionStatus::Pending
        })
        .map(|(&id, p)| (id, p.acquirer, p.offer))
        .collect();
    proposals.sort_unstable_by_key(|t| t.0);

    for (proposal_id, _acquirer, offer) in proposals {
        // Calculate book value
        let node_count = world
            .corp_infra_nodes
            .get(&corp_id)
            .map(|n| n.len())
            .unwrap_or(0);
        let asset_value: i64 = world
            .corp_infra_nodes
            .get(&corp_id)
            .unwrap_or(&Vec::new())
            .iter()
            .filter_map(|&id| world.infra_nodes.get(&id).map(|n| n.construction_cost))
            .sum();
        let cash = world
            .financials
            .get(&corp_id)
            .map(|f| f.cash)
            .unwrap_or(0);
        let book_value = asset_value + cash;

        // Premium multiplier by archetype
        let required_premium = match ai.archetype {
            AIArchetype::AggressiveExpander => 2.0,   // Hard to buy
            AIArchetype::DefensiveConsolidator => 1.3, // More willing if premium is good
            AIArchetype::TechInnovator => 1.5,
            AIArchetype::BudgetOperator => 1.2,
        };

        let accept = offer >= (book_value as f64 * required_premium) as i64
            || (node_count <= 1 && offer > 0); // Accept if nearly bankrupt

        if let Some(p) = world.acquisition_proposals.get_mut(&proposal_id) {
            if accept {
                p.status = crate::components::AcquisitionStatus::Accepted;
                let acquirer = p.acquirer;
                let target = p.target;

                if let Some(fin) = world.financials.get_mut(&acquirer) {
                    fin.cash -= offer;
                }

                world.transfer_corporation_assets(target, acquirer);

                world.event_queue.push(
                    tick,
                    gt_common::events::GameEvent::AcquisitionAccepted { acquirer, target },
                );
            } else {
                p.status = crate::components::AcquisitionStatus::Rejected;
                let acquirer = p.acquirer;
                let target = p.target;
                world.event_queue.push(
                    tick,
                    gt_common::events::GameEvent::AcquisitionRejected { acquirer, target },
                );
            }
        }
    }
}

/// AI espionage — Aggressive Expanders spy on competitors.
fn ai_espionage(
    world: &mut GameWorld,
    corp_id: EntityId,
    _ai: &AiState,
    fin: &Financial,
    tick: Tick,
) {
    // Only if flush with cash and no active missions
    let has_active = world
        .covert_ops
        .get(&corp_id)
        .map(|c| !c.active_missions.is_empty())
        .unwrap_or(false);

    if has_active || fin.cash < 5_000_000 {
        return;
    }

    // Check deterministically every 50 ticks
    let check = ((tick.wrapping_mul(corp_id) >> 5) % 50) as u64;
    if check != 0 {
        return;
    }

    // Pick a competitor
    let mut competitors: Vec<EntityId> = world
        .corporations
        .keys()
        .copied()
        .filter(|&id| id != corp_id)
        .collect();
    competitors.sort_unstable();

    if let Some(&target) = competitors.first() {
        let target_security = world
            .covert_ops
            .get(&target)
            .map(|c| c.security_level)
            .unwrap_or(0);
        let cost = 100_000 + target_security as i64 * 100_000;

        if fin.cash > cost * 3 {
            if let Some(f) = world.financials.get_mut(&corp_id) {
                f.cash -= cost;
            }

            let region = world
                .regions
                .keys()
                .copied()
                .next()
                .unwrap_or(0);

            let mission = crate::components::Mission {
                mission_type: crate::components::MissionType::Espionage,
                target,
                region,
                start_tick: tick,
                duration: 20,
                cost,
                success_chance: 0.7,
                completed: false,
            };

            world
                .covert_ops
                .entry(corp_id)
                .or_insert_with(crate::components::CovertOps::new)
                .active_missions
                .push(mission);
        }
    }
}

/// AI lobbying behavior based on archetype.
fn ai_lobbying(
    world: &mut GameWorld,
    corp_id: EntityId,
    ai: &AiState,
    fin: &Financial,
    tick: Tick,
) {
    // Don't lobby too often
    let has_active = world
        .lobbying_campaigns
        .values()
        .any(|c| c.corporation == corp_id && c.active);
    if has_active {
        return;
    }

    let lobby_willingness = match ai.archetype {
        AIArchetype::AggressiveExpander => 0.5,
        AIArchetype::DefensiveConsolidator => 0.2,
        AIArchetype::TechInnovator => 0.3,
        AIArchetype::BudgetOperator => 0.7, // Loves tax breaks
    };

    let check = ((tick.wrapping_mul(corp_id) >> 6) % 100) as f64 / 100.0;
    if check > lobby_willingness || fin.cash < 2_000_000 {
        return;
    }

    // Pick a region where we have infra
    let corp_regions: Vec<EntityId> = world
        .corp_infra_nodes
        .get(&corp_id)
        .unwrap_or(&Vec::new())
        .iter()
        .filter_map(|&nid| world.positions.get(&nid).and_then(|p| p.region_id))
        .collect();

    if let Some(&region_id) = corp_regions.first() {
        let policy = match ai.archetype {
            AIArchetype::BudgetOperator => gt_common::types::LobbyPolicy::ReduceTax,
            AIArchetype::AggressiveExpander => {
                gt_common::types::LobbyPolicy::IncreasedCompetitorBurden
            }
            AIArchetype::TechInnovator => gt_common::types::LobbyPolicy::SubsidyRequest,
            AIArchetype::DefensiveConsolidator => gt_common::types::LobbyPolicy::RelaxZoning,
        };

        let budget = fin.cash / 10;
        let campaign = crate::components::LobbyingCampaign::new(
            corp_id, region_id, policy, budget, tick,
        );
        let campaign_id = world.allocate_entity();
        world.lobbying_campaigns.insert(campaign_id, campaign);
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

    // Enforce max distance (same rules as player edges, scaled to grid resolution)
    let max_distance_km = match fiber_type {
        EdgeType::Copper => world.cell_spacing_km * 1.5,
        EdgeType::FiberLocal => world.cell_spacing_km * 3.0,
        EdgeType::FiberRegional => world.cell_spacing_km * 8.0,
        EdgeType::FiberNational => world.cell_spacing_km * 20.0,
        EdgeType::Microwave => world.cell_spacing_km * 4.0,
        EdgeType::Satellite => f64::INFINITY,
        EdgeType::Submarine => world.cell_spacing_km * 30.0,
    };
    if length_km > max_distance_km {
        return;
    }

    // Terrain check: fiber/copper can't cross deep ocean
    let from_terrain = world.infra_nodes.get(&from)
        .and_then(|n| world.land_parcels.values().find(|p| p.cell_index == n.cell_index))
        .map(|p| p.terrain);
    let to_terrain = world.infra_nodes.get(&to)
        .and_then(|n| world.land_parcels.values().find(|p| p.cell_index == n.cell_index))
        .map(|p| p.terrain);
    match fiber_type {
        EdgeType::Copper | EdgeType::FiberLocal | EdgeType::FiberRegional | EdgeType::FiberNational => {
            if matches!(from_terrain, Some(TerrainType::OceanDeep)) || matches!(to_terrain, Some(TerrainType::OceanDeep)) {
                return;
            }
        }
        _ => {}
    }

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
