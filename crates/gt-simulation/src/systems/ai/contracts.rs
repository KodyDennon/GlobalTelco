//! AI contract management — interconnection-aware.
//!
//! Handles proposing transit/peering contracts and evaluating incoming proposals.
//! Each archetype has different interconnection preferences:
//! - Aggressive Expander: prefers building own infra, peers reluctantly
//! - Defensive Consolidator: actively seeks peering to strengthen network
//! - Tech Innovator: sells transit at premium rates, invests in IXPs
//! - Budget Operator: buys cheap transit rather than building backbone

use crate::components::*;
use crate::world::GameWorld;
use gt_common::types::*;

use super::helpers;

// ─── Contract Proposals ──────────────────────────────────────────────────────

/// Propose interconnection contracts based on archetype strategy.
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
    let max_contracts = match ai.archetype {
        AIArchetype::AggressiveExpander => 2,
        AIArchetype::DefensiveConsolidator => 5,
        AIArchetype::TechInnovator => 4,
        AIArchetype::BudgetOperator => 4,
        AIArchetype::SatellitePioneer => 2,
    };
    if active_contracts >= max_contracts {
        return;
    }

    // Willingness to propose contracts (per archetype)
    let willingness = match ai.archetype {
        AIArchetype::AggressiveExpander => 0.3,     // prefers own infra
        AIArchetype::DefensiveConsolidator => 0.7,   // actively seeks peering
        AIArchetype::TechInnovator => 0.5,           // moderate
        AIArchetype::BudgetOperator => 0.8,          // loves cheap transit
        AIArchetype::SatellitePioneer => 0.3,        // prefers satellite infra, few terrestrial contracts
    };

    let check = helpers::deterministic_variety(tick, corp_id, 4) as f64 / 100.0;
    if check > willingness {
        return;
    }

    // Find suitable partner corps
    let mut other_corps: Vec<EntityId> = world
        .corporations
        .keys()
        .copied()
        .filter(|&id| id != corp_id)
        .collect();
    other_corps.sort_unstable();

    let own_node_count = corp_nodes.len();

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

        let other_node_count = world
            .corp_infra_nodes
            .get(&other_id)
            .map(|n| n.len())
            .unwrap_or(0);
        if other_node_count == 0 {
            continue;
        }

        // Determine contract type and terms based on archetype
        let (contract_type, capacity, price, duration) =
            select_contract_terms(ai, own_node_count, other_node_count);

        let penalty = (price * duration as i64 / 10).max(1_000);

        let contract = Contract::new_proposal(
            contract_type,
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
        break; // One proposal per cycle
    }
}

/// Select contract type and terms based on AI archetype and relative network sizes.
fn select_contract_terms(
    ai: &AiState,
    own_nodes: usize,
    other_nodes: usize,
) -> (ContractType, f64, Money, Tick) {
    let size_ratio = own_nodes as f64 / other_nodes.max(1) as f64;
    let similar_size = size_ratio > 0.5 && size_ratio < 2.0;

    match ai.archetype {
        AIArchetype::AggressiveExpander => {
            // Reluctant peerer — only peers with similar-size corps, never buys transit
            if similar_size {
                (ContractType::Peering, 500.0, 0, 180)
            } else {
                // Sells transit to smaller corps
                (ContractType::Transit, 300.0, 500, 120)
            }
        }
        AIArchetype::DefensiveConsolidator => {
            // Actively seeks peering with similarly-sized corps
            if similar_size {
                (ContractType::Peering, 1000.0, 0, 240)
            } else if own_nodes < other_nodes {
                // Buys transit from larger corps at moderate price
                (ContractType::Transit, 500.0, 300, 180)
            } else {
                // Sells transit to smaller corps
                (ContractType::Transit, 500.0, 200, 180)
            }
        }
        AIArchetype::TechInnovator => {
            // Sells premium transit, peers selectively
            if similar_size {
                (ContractType::Peering, 800.0, 0, 200)
            } else {
                // Premium transit pricing
                (ContractType::Transit, 500.0, 800, 150)
            }
        }
        AIArchetype::BudgetOperator => {
            // Buys cheap transit, avoids building backbone
            if own_nodes < other_nodes {
                // Buys transit from larger corps at low price
                (ContractType::Transit, 800.0, 200, 200)
            } else if similar_size {
                (ContractType::Peering, 600.0, 0, 200)
            } else {
                (ContractType::Transit, 400.0, 150, 180)
            }
        }
        AIArchetype::SatellitePioneer => {
            // Focuses on satellite — peers selectively for ground station access
            if similar_size {
                (ContractType::Peering, 600.0, 0, 200)
            } else {
                // Sells premium satellite transit
                (ContractType::Transit, 400.0, 700, 150)
            }
        }
    }
}

// ─── Contract Evaluation ─────────────────────────────────────────────────────

/// Evaluate and accept/reject incoming contract proposals with archetype-aware logic.
pub fn evaluate_incoming(world: &mut GameWorld, corp_id: EntityId, ai: &AiState, tick: Tick) {
    let mut proposals: Vec<(EntityId, EntityId, Money, ContractType, f64)> = world
        .contracts
        .iter()
        .filter(|(_, c)| c.to == corp_id && c.status == ContractStatus::Proposed)
        .map(|(&id, c)| (id, c.from, c.price_per_tick, c.contract_type, c.capacity))
        .collect();
    proposals.sort_unstable_by_key(|t| t.0);

    for (contract_id, proposer, price, contract_type, _capacity) in proposals {
        let fin = match world.financials.get(&corp_id) {
            Some(f) => f,
            None => continue,
        };

        // Peering contracts are free — always consider them
        let affordable = if contract_type == ContractType::Peering {
            true
        } else {
            // Transit: ensure price is < 25% of revenue (don't overspend on transit)
            price < fin.revenue_per_tick / 4
        };

        // Archetype-specific acceptance logic
        let accept = match ai.archetype {
            AIArchetype::AggressiveExpander => {
                // Rejects most incoming contracts — prefers building own infra
                // Only accepts peering from similar-size corps
                if contract_type == ContractType::Peering {
                    let own_nodes = world
                        .corp_infra_nodes
                        .get(&corp_id)
                        .map(|n| n.len())
                        .unwrap_or(0);
                    let other_nodes = world
                        .corp_infra_nodes
                        .get(&proposer)
                        .map(|n| n.len())
                        .unwrap_or(0);
                    let ratio = own_nodes as f64 / other_nodes.max(1) as f64;
                    ratio > 0.5 && ratio < 2.0
                } else {
                    // Only accepts transit if very cheap
                    affordable && price < fin.revenue_per_tick / 8
                }
            }
            AIArchetype::DefensiveConsolidator => {
                // Welcomes peering, accepts reasonable transit
                if contract_type == ContractType::Peering {
                    true
                } else {
                    affordable
                }
            }
            AIArchetype::TechInnovator => {
                // Accepts peering with tech-heavy corps, sells transit at premium
                if contract_type == ContractType::Peering {
                    true
                } else {
                    // Rejects lowball transit offers — wants premium prices
                    affordable && price > 0
                }
            }
            AIArchetype::BudgetOperator => {
                // Accepts cheap transit eagerly, peers with everyone
                if contract_type == ContractType::Peering {
                    true
                } else {
                    affordable
                }
            }
            AIArchetype::SatellitePioneer => {
                // Accepts peering for ground station access, selective on transit
                if contract_type == ContractType::Peering {
                    true
                } else {
                    // Only accepts transit if affordable — prefers satellite links
                    affordable && price > 0
                }
            }
        };

        let random_check =
            ((tick.wrapping_mul(contract_id) >> 3) % 100) as f64 / 100.0;

        if accept && random_check < 0.8 {
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
