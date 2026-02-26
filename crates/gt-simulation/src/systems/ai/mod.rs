//! AI Corporation Engine.
//!
//! Modular AI system that drives non-player corporations. Each subsystem is
//! a separate module with clear responsibilities:
//!
//! - `building`   — Template-based network construction (backbone-first pattern)
//! - `finance`    — Loan management, debt repayment
//! - `contracts`  — Contract proposals and evaluation
//! - `diplomacy`  — Auctions, acquisitions, espionage, lobbying
//! - `helpers`    — Shared utilities (node lookup, edge building, parcel acquisition)
//!
//! ## Build Template (backbone-first)
//!
//! AI corps build networks in structured phases:
//!   A. Backbone Hub  — establish core/backbone node in highest-value region
//!   B. Aggregation   — connect CentralOffice/ExchangePoint near cities
//!   C. Access Layer  — deploy CellTowers in unserved city cells
//!   D. Redundancy    — alternate backbone links for disaster resilience
//!
//! Each archetype varies behavior within these phases.

pub mod building;
pub mod contracts;
pub mod diplomacy;
pub mod finance;
pub mod helpers;

use crate::components::*;
use crate::world::GameWorld;
use gt_common::types::*;

// ─── Main Entry Point ────────────────────────────────────────────────────────

pub fn run(world: &mut GameWorld) {
    let tick = world.current_tick();

    // Run AI every 5 ticks (and always on tick 1 so AI acts immediately)
    if tick > 1 && tick % 5 != 0 {
        return;
    }

    let mut ai_corps: Vec<(EntityId, AiState)> = world
        .ai_states
        .iter()
        .map(|(&id, state)| (id, state.clone()))
        .collect();
    ai_corps.sort_unstable_by_key(|t| t.0);

    for (corp_id, ai_state) in ai_corps {
        if ai_state.proxy_mode {
            // Proxy mode: only do basic maintenance (repair damaged nodes)
            run_proxy_maintenance(world, corp_id, tick);
            continue;
        }

        let financial = match world.financials.get(&corp_id) {
            Some(f) => f.clone(),
            None => continue,
        };

        run_corporation(world, corp_id, &ai_state, &financial, tick);
    }

    // Check for AI-to-AI mergers (DefensiveConsolidator acquires small neighbors)
    if tick > 0 && tick % 200 == 0 {
        check_ai_mergers(world);
    }
}

// ─── Proxy Maintenance (disconnected player corps) ──────────────────────────

fn run_proxy_maintenance(world: &mut GameWorld, corp_id: EntityId, tick: Tick) {
    // Proxy only does basic upkeep: repair damaged nodes if affordable
    let node_ids: Vec<EntityId> = world
        .corp_infra_nodes
        .get(&corp_id)
        .cloned()
        .unwrap_or_default();

    let cash = world.financials.get(&corp_id).map(|f| f.cash).unwrap_or(0);
    if cash <= 0 {
        return;
    }

    for nid in node_ids {
        let needs_repair = world
            .healths
            .get(&nid)
            .map(|h| h.condition < 0.5)
            .unwrap_or(false);
        if !needs_repair {
            continue;
        }

        // Estimate repair cost (20% of construction cost)
        let repair_cost = world
            .infra_nodes
            .get(&nid)
            .map(|n| (n.construction_cost as f64 * 0.2) as i64)
            .unwrap_or(0);

        let current_cash = world.financials.get(&corp_id).map(|f| f.cash).unwrap_or(0);
        if current_cash < repair_cost {
            continue;
        }

        // Apply repair
        if let Some(fin) = world.financials.get_mut(&corp_id) {
            fin.cash -= repair_cost;
        }
        if let Some(h) = world.healths.get_mut(&nid) {
            h.condition = (h.condition + 0.5).min(1.0);
        }
        if let Some(cap) = world.capacities.get_mut(&nid) {
            cap.current_load = (cap.current_load - cap.max_throughput * 0.4).max(0.0);
        }

        world.event_queue.push(
            tick,
            gt_common::events::GameEvent::RepairStarted {
                entity: nid,
                cost: repair_cost,
            },
        );
    }
}

// ─── Per-Corporation Orchestrator ────────────────────────────────────────────

fn run_corporation(
    world: &mut GameWorld,
    corp_id: EntityId,
    ai: &AiState,
    fin: &Financial,
    tick: Tick,
) {
    // 1. Strategy selection
    let strategy = select_strategy(ai, fin);
    if let Some(state) = world.ai_states.get_mut(&corp_id) {
        state.strategy = strategy;
    }

    // 2. Execute strategy actions
    execute_strategy(world, corp_id, ai, fin, strategy, tick);

    // 3. Finance management (all strategies)
    finance::manage(world, corp_id, ai, tick);

    // 4. Contract proposals (Expand and Compete only)
    if matches!(strategy, AIStrategy::Expand | AIStrategy::Compete) {
        contracts::propose(world, corp_id, ai, tick);
    }

    // 5. Evaluate incoming contracts
    contracts::evaluate_incoming(world, corp_id, ai, tick);

    // 6. Auction bidding
    diplomacy::bid_on_auctions(world, corp_id, ai, fin, tick);

    // 7. Evaluate acquisition proposals
    diplomacy::evaluate_acquisitions(world, corp_id, ai, tick);

    // 8. Espionage (Aggressive Expander only)
    if ai.archetype == AIArchetype::AggressiveExpander {
        diplomacy::conduct_espionage(world, corp_id, fin, tick);
    }

    // 9. Lobbying
    diplomacy::lobby(world, corp_id, ai, fin, tick);

    // 10. Evaluate co-ownership proposals
    diplomacy::evaluate_co_ownership_proposals(world, corp_id, ai, tick);
}

// ─── Strategy Selection ──────────────────────────────────────────────────────

fn select_strategy(ai: &AiState, fin: &Financial) -> AIStrategy {
    let cash_ratio = fin.cash as f64 / (fin.cost_per_tick as f64 * 90.0).max(1.0);
    let profit = fin.revenue_per_tick - fin.cost_per_tick;
    let debt_heavy = fin.debt > fin.cash * 2;

    // Survival threshold: very low cash or losing money with no runway
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

// ─── Strategy Execution ──────────────────────────────────────────────────────

fn execute_strategy(
    world: &mut GameWorld,
    corp_id: EntityId,
    ai: &AiState,
    fin: &Financial,
    strategy: AIStrategy,
    tick: Tick,
) {
    match strategy {
        AIStrategy::Expand => building::execute(world, corp_id, ai, fin, tick),
        AIStrategy::Consolidate => building::execute(world, corp_id, ai, fin, tick),
        AIStrategy::Compete => execute_compete(world, corp_id, ai, fin, tick),
        AIStrategy::Survive => execute_survive(world, corp_id, fin),
    }
}

// ─── Compete Strategy ────────────────────────────────────────────────────────

/// In Compete mode, AI targets competitor-dominated cells with low satisfaction.
fn execute_compete(
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

    // Find city cells where a competitor dominates but satisfaction is low
    let mut targets: Vec<(usize, f64)> = Vec::new();
    for (_, city) in &world.cities {
        if city.infrastructure_satisfaction > 0.7 {
            continue;
        }
        for &ci in &city.cells {
            let coverage = world.cell_coverage.get(&ci);
            let is_competitor = coverage
                .and_then(|c| c.dominant_owner)
                .map(|owner| owner != corp_id)
                .unwrap_or(false);
            let low_quality = coverage.map(|c| c.signal_strength < 50.0).unwrap_or(true);

            if is_competitor || low_quality {
                let cell_pop = city.population as f64 / city.cells.len().max(1) as f64;
                let score = cell_pop * (1.0 - city.infrastructure_satisfaction);
                if score > 20.0 {
                    targets.push((ci, score));
                }
            }
        }
    }

    targets.sort_unstable_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.0.cmp(&b.0))
    });

    if let Some(&(cell_index, _)) = targets.first() {
        let node_type = NodeType::CellTower;
        if fin.cash < node_type.construction_cost() as i64 * 3 {
            return;
        }

        if let Some(new_id) = helpers::build_node(world, corp_id, node_type, cell_index, tick) {
            if !corp_nodes.is_empty() {
                if let Some(nearest) =
                    helpers::find_nearest_node(world, &corp_nodes, cell_index)
                {
                    helpers::build_edge(world, corp_id, nearest, new_id, tick);
                }
            }
        }
    }
}

// ─── AI-to-AI Mergers ───────────────────────────────────────────────────────

/// DefensiveConsolidator AI corps check for small nearby AI corps to acquire.
/// Requirements: consolidator has > 1_000_000 cash, target has < 10 nodes.
/// At most one merger per 200-tick cycle.
fn check_ai_mergers(world: &mut GameWorld) {
    let tick = world.current_tick();

    // Collect DefensiveConsolidator AIs with sufficient cash
    let mut consolidators: Vec<(EntityId, i64)> = world
        .ai_states
        .iter()
        .filter(|(_, ai)| ai.archetype == AIArchetype::DefensiveConsolidator && !ai.proxy_mode)
        .filter_map(|(&id, _)| {
            let cash = world.financials.get(&id)?.cash;
            if cash > 1_000_000 {
                Some((id, cash))
            } else {
                None
            }
        })
        .collect();
    consolidators.sort_unstable_by_key(|t| t.0);

    // Collect small AI corps (< 10 nodes, not player)
    let mut small_corps: Vec<(EntityId, usize)> = world
        .ai_states
        .keys()
        .copied()
        .filter_map(|id| {
            let node_count = world
                .corp_infra_nodes
                .get(&id)
                .map(|n| n.len())
                .unwrap_or(0);
            if node_count < 10 {
                Some((id, node_count))
            } else {
                None
            }
        })
        .collect();
    small_corps.sort_unstable_by_key(|t| t.0);

    // Try one merger per cycle
    for (consolidator_id, consolidator_cash) in &consolidators {
        for (target_id, _target_nodes) in &small_corps {
            // Can't merge with yourself
            if consolidator_id == target_id {
                continue;
            }

            // Check that target is still an AI corp (not already merged this cycle)
            if !world.ai_states.contains_key(target_id) {
                continue;
            }

            // Check that the target has nodes in a nearby region (shares at least one region)
            let consolidator_regions: std::collections::HashSet<Option<EntityId>> = world
                .corp_infra_nodes
                .get(consolidator_id)
                .unwrap_or(&Vec::new())
                .iter()
                .filter_map(|&nid| world.positions.get(&nid).map(|p| p.region_id))
                .collect();

            let target_regions: std::collections::HashSet<Option<EntityId>> = world
                .corp_infra_nodes
                .get(target_id)
                .unwrap_or(&Vec::new())
                .iter()
                .filter_map(|&nid| world.positions.get(&nid).map(|p| p.region_id))
                .collect();

            let shares_region = consolidator_regions.intersection(&target_regions).next().is_some();

            // Also allow merger if target has zero nodes (startup with no infra)
            let target_has_no_nodes = world
                .corp_infra_nodes
                .get(target_id)
                .map(|n| n.is_empty())
                .unwrap_or(true);

            if !shares_region && !target_has_no_nodes {
                continue;
            }

            // Calculate acquisition cost: target's asset value + cash
            let target_asset_value: i64 = world
                .corp_infra_nodes
                .get(target_id)
                .unwrap_or(&Vec::new())
                .iter()
                .filter_map(|&nid| world.infra_nodes.get(&nid).map(|n| n.construction_cost))
                .sum();
            let target_cash = world
                .financials
                .get(target_id)
                .map(|f| f.cash.max(0))
                .unwrap_or(0);
            let acquisition_cost = ((target_asset_value + target_cash) as f64 * 1.2) as i64;

            if *consolidator_cash < acquisition_cost {
                continue;
            }

            // Execute the merger: deduct cost, transfer assets, emit event
            if let Some(fin) = world.financials.get_mut(consolidator_id) {
                fin.cash -= acquisition_cost;
            }

            world.transfer_corporation_assets(*target_id, *consolidator_id);

            world.event_queue.push(
                tick,
                gt_common::events::GameEvent::CorporationMerged {
                    absorbed: *target_id,
                    absorber: *consolidator_id,
                },
            );

            // Only one merger per cycle
            return;
        }
    }
}

// ─── Survive Strategy ────────────────────────────────────────────────────────

/// In Survive mode, AI sells least-utilized nodes to stay afloat.
fn execute_survive(world: &mut GameWorld, corp_id: EntityId, fin: &Financial) {
    if fin.cash >= fin.cost_per_tick * 10 || fin.debt >= fin.cash.abs() * 2 {
        return;
    }

    let corp_nodes = world
        .corp_infra_nodes
        .get(&corp_id)
        .cloned()
        .unwrap_or_default();

    if corp_nodes.len() <= 1 {
        return;
    }

    // Find least utilized operational node
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
