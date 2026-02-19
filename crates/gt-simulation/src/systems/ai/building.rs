//! Template-based network building for AI corporations.
//!
//! AI builds networks in a structured backbone-first pattern:
//!   Phase A: Backbone Hub — establish core/backbone in highest-value region
//!   Phase B: Aggregation Ring — connect aggregation nodes around cities
//!   Phase C: Access Layer — deploy cell towers in unserved city cells
//!   Phase D: Redundancy — build alternate backbone links for resilience
//!
//! Each archetype has variations in priorities and node type preferences.

use crate::components::*;
use crate::world::GameWorld;
use gt_common::types::*;

use super::helpers;

// ─── Main Template Entry Point ───────────────────────────────────────────────

/// Execute template-based building for an AI corporation.
/// Decides which build phase to execute based on current network state.
pub fn execute(
    world: &mut GameWorld,
    corp_id: EntityId,
    ai: &AiState,
    fin: &Financial,
    tick: Tick,
) {
    let corp_nodes = world
        .corp_infra_nodes
        .get(&corp_id)
        .cloned()
        .unwrap_or_default();

    let has_backbone = helpers::has_node_at_or_above_tier(world, corp_id, NetworkTier::Core);
    let aggregation_count = helpers::count_nodes_at_tier(world, corp_id, NetworkTier::Aggregation);
    let access_count = helpers::count_nodes_at_tier(world, corp_id, NetworkTier::Access);

    let target_cities = count_target_cities(world);
    let unserved_demand = estimate_unserved_demand(world, corp_id);

    if !has_backbone {
        build_backbone_hub(world, corp_id, ai, fin, tick);
    } else if aggregation_count < target_cities.min(5) {
        build_aggregation(world, corp_id, ai, fin, &corp_nodes, tick);
    } else if unserved_demand > 100.0 || access_count < aggregation_count * 3 {
        build_access_layer(world, corp_id, ai, fin, &corp_nodes, tick);
    } else if should_build_redundancy(world, corp_id, ai, fin) {
        build_redundancy(world, corp_id, &corp_nodes, tick);
    } else {
        // Network is mature — upgrade existing infrastructure
        upgrade_existing(world, corp_id, fin);
    }

    // Tech innovator always prioritizes research alongside building
    if ai.archetype == AIArchetype::TechInnovator {
        start_research_if_needed(world, corp_id, fin);
    }
}

// ─── Phase A: Backbone Hub ───────────────────────────────────────────────────

/// Build the initial backbone/core node. This is the AI's first strategic decision.
fn build_backbone_hub(
    world: &mut GameWorld,
    corp_id: EntityId,
    ai: &AiState,
    fin: &Financial,
    tick: Tick,
) {
    // Pick node type based on archetype
    let node_type = match ai.archetype {
        AIArchetype::TechInnovator => NodeType::SatelliteGround, // Skip terrestrial
        AIArchetype::BudgetOperator => NodeType::CentralOffice,  // Cheaper alternative
        _ => NodeType::DataCenter, // Standard core node
    };

    // Find the best city to place our hub
    let target_cell = pick_hub_city_cell(world, corp_id, ai);
    let Some(cell_index) = target_cell else {
        return;
    };

    let cost_threshold = match ai.archetype {
        AIArchetype::BudgetOperator => 3,
        _ => 2,
    };
    if fin.cash < node_type.construction_cost() as i64 * cost_threshold {
        return;
    }

    if let Some(hub_id) = helpers::build_node(world, corp_id, node_type, cell_index, tick) {
        // If we already have any nodes, connect the hub to the nearest one
        let existing_nodes: Vec<EntityId> = world
            .corp_infra_nodes
            .get(&corp_id)
            .unwrap_or(&Vec::new())
            .iter()
            .copied()
            .filter(|&id| id != hub_id)
            .collect();

        if !existing_nodes.is_empty() {
            if let Some(nearest) = helpers::find_nearest_node(world, &existing_nodes, cell_index) {
                helpers::build_edge(world, corp_id, hub_id, nearest, tick);
            }
        }
    }
}

/// Pick the best city cell for a backbone hub placement.
fn pick_hub_city_cell(
    world: &GameWorld,
    corp_id: EntityId,
    ai: &AiState,
) -> Option<usize> {
    let corp_cells = helpers::corp_cell_set(world, corp_id);

    // Score cities by population, avoiding cells we already occupy
    let mut city_scores: Vec<(usize, f64)> = Vec::new();
    for (_, city) in &world.cities {
        for &ci in &city.cells {
            if corp_cells.contains(&ci) {
                continue;
            }
            let cell_pop = city.population as f64 / city.cells.len().max(1) as f64;
            city_scores.push((ci, cell_pop));
        }
    }

    match ai.archetype {
        AIArchetype::AggressiveExpander => {
            // Pick highest-population city regardless of distance
            city_scores.sort_unstable_by(|a, b| {
                b.1.partial_cmp(&a.1)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then(a.0.cmp(&b.0))
            });
        }
        AIArchetype::DefensiveConsolidator => {
            // Pick nearest city to existing nodes (or highest pop if no nodes)
            if let Some(existing_nodes) = world.corp_infra_nodes.get(&corp_id) {
                if !existing_nodes.is_empty() {
                    // Score by inverse distance to nearest existing node
                    for entry in &mut city_scores {
                        if let Some(nearest) =
                            helpers::find_nearest_node(world, existing_nodes, entry.0)
                        {
                            let dist = world
                                .infra_nodes
                                .get(&nearest)
                                .and_then(|n| {
                                    let a = world.grid_cell_positions.get(n.cell_index)?;
                                    let b = world.grid_cell_positions.get(entry.0)?;
                                    Some(helpers::sq_distance(a, b))
                                })
                                .unwrap_or(f64::MAX);
                            // Weight: population / distance (closer is better)
                            entry.1 = entry.1 / (dist + 1.0);
                        }
                    }
                }
            }
            city_scores.sort_unstable_by(|a, b| {
                b.1.partial_cmp(&a.1)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then(a.0.cmp(&b.0))
            });
        }
        _ => {
            // Default: highest population
            city_scores.sort_unstable_by(|a, b| {
                b.1.partial_cmp(&a.1)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then(a.0.cmp(&b.0))
            });
        }
    }

    city_scores.first().map(|&(ci, _)| ci)
}

// ─── Phase B: Aggregation Ring ───────────────────────────────────────────────

/// Build aggregation nodes (CentralOffice/ExchangePoint) near cities and connect to core.
fn build_aggregation(
    world: &mut GameWorld,
    corp_id: EntityId,
    ai: &AiState,
    fin: &Financial,
    corp_nodes: &[EntityId],
    tick: Tick,
) {
    let corp_cells = helpers::corp_cell_set(world, corp_id);

    // Find cities that don't have an aggregation node nearby
    let mut candidates: Vec<(usize, f64)> = Vec::new();
    for (_, city) in &world.cities {
        // Skip cities where we already have aggregation or higher
        let has_agg = city.cells.iter().any(|ci| {
            corp_cells.contains(ci)
                && world
                    .corp_infra_nodes
                    .get(&corp_id)
                    .unwrap_or(&Vec::new())
                    .iter()
                    .any(|&nid| {
                        world
                            .infra_nodes
                            .get(&nid)
                            .map(|n| {
                                n.cell_index == *ci
                                    && (n.node_type.tier() as u8) >= (NetworkTier::Aggregation as u8)
                            })
                            .unwrap_or(false)
                    })
        });
        if has_agg {
            continue;
        }

        for &ci in &city.cells {
            if corp_cells.contains(&ci) {
                continue;
            }
            let cell_pop = city.population as f64 / city.cells.len().max(1) as f64;
            candidates.push((ci, cell_pop));
        }
    }

    candidates.sort_unstable_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.0.cmp(&b.0))
    });

    let node_type = match ai.archetype {
        AIArchetype::AggressiveExpander => NodeType::ExchangePoint,
        _ => NodeType::CentralOffice,
    };

    if let Some(&(cell_index, _)) = candidates.first() {
        if fin.cash < node_type.construction_cost() as i64 * 2 {
            return;
        }

        if let Some(new_id) = helpers::build_node(world, corp_id, node_type, cell_index, tick) {
            // Connect to nearest core/backbone node
            if let Some(core_node) = helpers::find_nearest_node_at_or_above_tier(
                world, corp_nodes, cell_index, NetworkTier::Core,
            ) {
                helpers::build_edge(world, corp_id, new_id, core_node, tick);
            } else if let Some(nearest) = helpers::find_nearest_node(world, corp_nodes, cell_index)
            {
                helpers::build_edge(world, corp_id, new_id, nearest, tick);
            }
        }
    }
}

// ─── Phase C: Access Layer ───────────────────────────────────────────────────

/// Deploy access-tier nodes (CellTowers) in city cells with unserved demand.
fn build_access_layer(
    world: &mut GameWorld,
    corp_id: EntityId,
    ai: &AiState,
    fin: &Financial,
    corp_nodes: &[EntityId],
    tick: Tick,
) {
    let corp_cells = helpers::corp_cell_set(world, corp_id);

    // Score candidate cells: population * (1 - coverage) / distance_to_aggregation
    let mut candidates: Vec<(usize, f64)> = Vec::new();
    for (_, city) in &world.cities {
        for &ci in &city.cells {
            if corp_cells.contains(&ci) {
                continue;
            }
            // Skip cells we already dominate
            let our_coverage = world
                .cell_coverage
                .get(&ci)
                .map(|c| c.dominant_owner == Some(corp_id))
                .unwrap_or(false);
            if our_coverage {
                continue;
            }

            let cell_pop = city.population as f64 / city.cells.len().max(1) as f64;
            let existing_coverage = world
                .cell_coverage
                .get(&ci)
                .map(|c| (c.bandwidth / (cell_pop * 0.05).max(1.0)).clamp(0.0, 1.0))
                .unwrap_or(0.0);

            // Factor in distance to nearest aggregation node
            let dist_factor = if let Some(agg_node) = helpers::find_nearest_node_at_or_above_tier(
                world,
                corp_nodes,
                ci,
                NetworkTier::Aggregation,
            ) {
                let agg_cell = world.infra_nodes.get(&agg_node).map(|n| n.cell_index).unwrap_or(0);
                let dist = world
                    .grid_cell_positions
                    .get(ci)
                    .and_then(|a| world.grid_cell_positions.get(agg_cell).map(|b| helpers::sq_distance(a, b)))
                    .unwrap_or(f64::MAX);
                1.0 / (dist + 1.0)
            } else {
                0.01 // Very low score if no aggregation to connect to
            };

            let score = cell_pop * (1.0 - existing_coverage) * dist_factor;
            if score > 0.1 {
                candidates.push((ci, score));
            }
        }
    }

    candidates.sort_unstable_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.0.cmp(&b.0))
    });

    let node_type = match ai.archetype {
        AIArchetype::BudgetOperator => {
            let v = helpers::deterministic_variety(tick, corp_id, 3);
            if v < 30 {
                NodeType::WirelessRelay
            } else {
                NodeType::CellTower
            }
        }
        _ => NodeType::CellTower,
    };

    if let Some(&(cell_index, _)) = candidates.first() {
        if fin.cash < node_type.construction_cost() as i64 * 2 {
            return;
        }

        if let Some(new_id) = helpers::build_node(world, corp_id, node_type, cell_index, tick) {
            // Connect to nearest aggregation-or-higher node
            if let Some(agg_node) = helpers::find_nearest_node_at_or_above_tier(
                world, corp_nodes, cell_index, NetworkTier::Aggregation,
            ) {
                helpers::build_edge(world, corp_id, new_id, agg_node, tick);
            } else if let Some(nearest) = helpers::find_nearest_node(world, corp_nodes, cell_index)
            {
                helpers::build_edge(world, corp_id, new_id, nearest, tick);
            }
        }
    }
}

// ─── Phase D: Redundancy ─────────────────────────────────────────────────────

/// Build alternate backbone links between hub nodes for resilience.
fn build_redundancy(
    world: &mut GameWorld,
    corp_id: EntityId,
    corp_nodes: &[EntityId],
    tick: Tick,
) {
    // Find pairs of core+ nodes that aren't directly connected
    let core_nodes: Vec<EntityId> = corp_nodes
        .iter()
        .copied()
        .filter(|&nid| {
            !world.constructions.contains_key(&nid)
                && world
                    .infra_nodes
                    .get(&nid)
                    .map(|n| (n.node_type.tier() as u8) >= (NetworkTier::Core as u8))
                    .unwrap_or(false)
        })
        .collect();

    if core_nodes.len() < 2 {
        return;
    }

    // Find first unconnected pair
    for i in 0..core_nodes.len() {
        for j in (i + 1)..core_nodes.len() {
            let a = core_nodes[i];
            let b = core_nodes[j];

            // Check if there's already a direct edge between them
            let already_connected = world.infra_edges.values().any(|e| {
                (e.source == a && e.target == b) || (e.source == b && e.target == a)
            });
            if already_connected {
                continue;
            }

            // Build redundant link
            if helpers::build_edge(world, corp_id, a, b, tick) {
                return; // One redundancy link per tick cycle
            }
        }
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn should_build_redundancy(
    world: &GameWorld,
    corp_id: EntityId,
    ai: &AiState,
    fin: &Financial,
) -> bool {
    let maintenance = fin.cost_per_tick.max(1);
    let cash_ratio = fin.cash / maintenance;

    let willing = match ai.archetype {
        AIArchetype::DefensiveConsolidator => cash_ratio > 30, // Redundancy-focused
        AIArchetype::AggressiveExpander => false,               // Skip redundancy
        _ => cash_ratio > 60,                                   // Others need large surplus
    };
    willing && helpers::count_nodes_at_tier(world, corp_id, NetworkTier::Core) >= 2
}

fn count_target_cities(world: &GameWorld) -> usize {
    world.cities.len().min(10)
}

fn estimate_unserved_demand(world: &GameWorld, corp_id: EntityId) -> f64 {
    let corp_cells = helpers::corp_cell_set(world, corp_id);
    let mut unserved = 0.0;

    for (_, city) in &world.cities {
        for &ci in &city.cells {
            let our_coverage = world
                .cell_coverage
                .get(&ci)
                .map(|c| c.dominant_owner == Some(corp_id))
                .unwrap_or(false);
            if our_coverage || corp_cells.contains(&ci) {
                continue;
            }

            let cell_pop = city.population as f64 / city.cells.len().max(1) as f64;
            let coverage = world
                .cell_coverage
                .get(&ci)
                .map(|c| (c.bandwidth / (cell_pop * 0.05).max(1.0)).clamp(0.0, 1.0))
                .unwrap_or(0.0);
            unserved += cell_pop * (1.0 - coverage);
        }
    }
    unserved
}

/// Upgrade the most utilized existing node (consolidation behavior).
fn upgrade_existing(world: &mut GameWorld, corp_id: EntityId, fin: &Financial) {
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

fn start_research_if_needed(world: &mut GameWorld, corp_id: EntityId, fin: &Financial) {
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
