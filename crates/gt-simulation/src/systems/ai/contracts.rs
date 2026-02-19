//! AI contract management.
//!
//! Handles proposing transit contracts to other corporations
//! and evaluating/accepting/rejecting incoming proposals.

use crate::components::*;
use crate::world::GameWorld;
use gt_common::types::*;

use super::helpers;

// ─── Contract Proposals ──────────────────────────────────────────────────────

/// Propose transit contracts to other corporations for bandwidth sharing.
pub fn propose(world: &mut GameWorld, corp_id: EntityId, ai: &AiState, tick: Tick) {
    let corp_nodes = world
        .corp_infra_nodes
        .get(&corp_id)
        .cloned()
        .unwrap_or_default();
    if corp_nodes.len() < 2 {
        return;
    }

    // Limit active contracts
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

    // Willingness varies by archetype
    let willingness = match ai.archetype {
        AIArchetype::AggressiveExpander => 0.7,
        AIArchetype::DefensiveConsolidator => 0.3,
        AIArchetype::TechInnovator => 0.5,
        AIArchetype::BudgetOperator => 0.6,
    };

    let check = helpers::deterministic_variety(tick, corp_id, 4) as f64 / 100.0;
    if check > willingness {
        return;
    }

    // Find a suitable partner corp
    let mut other_corps: Vec<EntityId> = world
        .corporations
        .keys()
        .copied()
        .filter(|&id| id != corp_id)
        .collect();
    other_corps.sort_unstable();

    for &other_id in &other_corps {
        // Skip if we already have a contract with them
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

        let contract = Contract::new_proposal(
            ContractType::Transit,
            corp_id,
            other_id,
            500.0,  // capacity
            300,    // price per tick
            tick,
            180,    // duration (~6 months)
            5_000,  // penalty
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
        break; // One proposal per cycle
    }
}

// ─── Contract Evaluation ─────────────────────────────────────────────────────

/// Evaluate and accept/reject incoming contract proposals.
pub fn evaluate_incoming(world: &mut GameWorld, corp_id: EntityId, ai: &AiState, tick: Tick) {
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

        let affordable = price < fin.revenue_per_tick / 4;
        let willingness = match ai.archetype {
            AIArchetype::AggressiveExpander => 0.8,
            AIArchetype::DefensiveConsolidator => 0.4,
            AIArchetype::TechInnovator => 0.6,
            AIArchetype::BudgetOperator => 0.3,
        };

        let check =
            ((tick.wrapping_mul(contract_id) >> 3) % 100) as f64 / 100.0;

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
            world.contracts.remove(&contract_id);
        }
    }
}
