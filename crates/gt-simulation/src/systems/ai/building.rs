//! Template-based network building for AI corporations.
//!
//! AI builds networks in a structured backbone-first pattern:
//!   Phase A: Backbone Hub — establish core/backbone in highest-value region
//!   Phase B: Aggregation Ring — connect aggregation nodes around cities
//!   Phase C: Access Layer — deploy cell towers in unserved city cells
//!   Phase D: Redundancy — build alternate backbone links for resilience
//!   Phase E: FTTH Auto-Management — tiered FTTH deployment based on NAP count
//!
//! Each archetype has variations in priorities and node type preferences.
//!
//! FTTH tiers:
//!   Small (1-50 NAPs): no auto-management (player/AI handles manually)
//!   Medium (50-200): auto-connect drop cables from existing NAPs
//!   Large (200+): auto-place FDHs and NAPs in underserved areas

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

    // Phase E: Tiered FTTH auto-management based on NAP count
    manage_ftth(world, corp_id, fin, &corp_nodes, tick);

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

// ─── Phase E: Tiered FTTH Auto-Management ───────────────────────────────────

/// FTTH auto-deployment based on corporation NAP count (tiered management).
///
/// - Small (1-50 NAPs): no auto-management — player/AI handles manually
/// - Medium (50-200 NAPs): auto-connect drop cables from existing NAPs to nearby buildings
/// - Large (200+): auto-place FDHs and NAPs in underserved areas within policy budget
///
/// This models the "tiered management" concept where larger corps have departments
/// that handle routine FTTH expansion automatically.
fn manage_ftth(
    world: &mut GameWorld,
    corp_id: EntityId,
    fin: &Financial,
    corp_nodes: &[EntityId],
    tick: Tick,
) {
    // Count operational NAPs owned by this corp
    let nap_count = corp_nodes
        .iter()
        .filter(|&&nid| {
            !world.constructions.contains_key(&nid)
                && world
                    .infra_nodes
                    .get(&nid)
                    .map(|n| n.node_type == NodeType::NetworkAccessPoint)
                    .unwrap_or(false)
        })
        .count();

    if nap_count < 50 {
        // Small corp: no auto-management
        return;
    }

    // Budget check: FTTH auto-deployment uses at most 5% of cash per AI tick
    let ftth_budget = fin.cash / 20;
    if ftth_budget < 50_000 {
        return; // Not enough spare cash for FTTH expansion
    }

    if nap_count < 200 {
        // Medium tier: auto-connect drop cables from existing active NAPs to
        // nearby buildings (cells with city population but no drop cable coverage).
        ftth_medium_auto_connect(world, corp_id, corp_nodes, tick);
    } else {
        // Large tier: auto-place FDHs and NAPs in underserved areas.
        ftth_large_auto_deploy(world, corp_id, fin, corp_nodes, tick);
    }
}

/// Medium tier FTTH: find active NAPs without drop cable connections and
/// build DropCable edges to connect them to nearby FDH or existing infrastructure.
/// This simulates a department that connects already-deployed NAPs to the
/// distribution network for last-mile service.
fn ftth_medium_auto_connect(
    world: &mut GameWorld,
    corp_id: EntityId,
    corp_nodes: &[EntityId],
    tick: Tick,
) {
    // Find active NAPs that have no outgoing DropCable edges
    let nap_with_drop: std::collections::HashSet<EntityId> = world
        .infra_edges
        .values()
        .filter(|e| e.edge_type == EdgeType::DropCable && e.owner == corp_id)
        .flat_map(|e| [e.source, e.target])
        .collect();

    let unconnected_naps: Vec<EntityId> = corp_nodes
        .iter()
        .copied()
        .filter(|&nid| {
            !world.constructions.contains_key(&nid)
                && world
                    .infra_nodes
                    .get(&nid)
                    .map(|n| {
                        n.node_type == NodeType::NetworkAccessPoint
                            && n.active_ftth
                            && !nap_with_drop.contains(&nid)
                    })
                    .unwrap_or(false)
        })
        .collect();

    if unconnected_naps.is_empty() {
        return;
    }

    // Connect up to 2 NAPs per AI tick cycle to avoid burst spending
    let max_connections = 2.min(unconnected_naps.len());
    for &nap_id in &unconnected_naps[..max_connections] {
        let nap_cell = match world.infra_nodes.get(&nap_id) {
            Some(n) => n.cell_index,
            None => continue,
        };

        // Find the nearest FDH to connect the NAP to via DistributionFiber
        // (if not already connected — the FTTH chain may already exist)
        let fdh_nodes: Vec<EntityId> = corp_nodes
            .iter()
            .copied()
            .filter(|&nid| {
                world
                    .infra_nodes
                    .get(&nid)
                    .map(|n| n.node_type == NodeType::FiberDistributionHub)
                    .unwrap_or(false)
            })
            .collect();

        if let Some(nearest_fdh) = helpers::find_nearest_node(world, &fdh_nodes, nap_cell) {
            // Check if there's already a DistributionFiber edge between them
            let already_connected = world.infra_edges.values().any(|e| {
                e.edge_type == EdgeType::DistributionFiber
                    && ((e.source == nap_id && e.target == nearest_fdh)
                        || (e.source == nearest_fdh && e.target == nap_id))
            });
            if !already_connected {
                helpers::build_edge(world, corp_id, nap_id, nearest_fdh, tick);
            }
        }
    }
}

/// Large tier FTTH: auto-place FDHs and NAPs in underserved city areas.
/// Targets city cells with high population but no existing FTTH coverage.
/// Uses the corp's policy budget to limit spending.
fn ftth_large_auto_deploy(
    world: &mut GameWorld,
    corp_id: EntityId,
    fin: &Financial,
    corp_nodes: &[EntityId],
    tick: Tick,
) {
    // Find city cells that have population but no NAP coverage from this corp
    let nap_cells: std::collections::HashSet<usize> = corp_nodes
        .iter()
        .filter_map(|&nid| {
            let node = world.infra_nodes.get(&nid)?;
            if node.node_type == NodeType::NetworkAccessPoint
                || node.node_type == NodeType::FiberDistributionHub
            {
                Some(node.cell_index)
            } else {
                None
            }
        })
        .collect();

    // Score candidate cells by population density and lack of FTTH coverage
    let mut candidates: Vec<(usize, f64)> = Vec::new();
    for (_, city) in &world.cities {
        for &ci in &city.cells {
            // Skip cells we already have FTTH nodes in
            if nap_cells.contains(&ci) {
                continue;
            }
            let cell_pop = city.population as f64 / city.cells.len().max(1) as f64;
            if cell_pop < 100.0 {
                continue; // Not enough population to justify FTTH
            }

            // Prefer cells close to existing corp infrastructure
            let dist_factor = if let Some(nearest) =
                helpers::find_nearest_node(world, corp_nodes, ci)
            {
                let nearest_cell = world
                    .infra_nodes
                    .get(&nearest)
                    .map(|n| n.cell_index)
                    .unwrap_or(0);
                let dist = world
                    .grid_cell_positions
                    .get(ci)
                    .and_then(|a| {
                        world
                            .grid_cell_positions
                            .get(nearest_cell)
                            .map(|b| helpers::sq_distance(a, b))
                    })
                    .unwrap_or(f64::MAX);
                1.0 / (dist + 1.0)
            } else {
                0.001
            };

            let score = cell_pop * dist_factor;
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

    // Budget: spend at most 5% of cash on FTTH auto-deployment per cycle
    let mut budget_remaining = fin.cash / 20;

    // Step 1: Place FDH if we don't have enough (need ~1 FDH per 10 NAPs)
    let fdh_count = corp_nodes
        .iter()
        .filter(|&&nid| {
            world
                .infra_nodes
                .get(&nid)
                .map(|n| n.node_type == NodeType::FiberDistributionHub)
                .unwrap_or(false)
        })
        .count();

    let nap_count = corp_nodes
        .iter()
        .filter(|&&nid| {
            world
                .infra_nodes
                .get(&nid)
                .map(|n| n.node_type == NodeType::NetworkAccessPoint)
                .unwrap_or(false)
        })
        .count();

    let target_fdh_count = (nap_count / 10).max(1);

    if fdh_count < target_fdh_count {
        if let Some(&(cell_index, _)) = candidates.first() {
            let fdh_cost = NodeType::FiberDistributionHub.construction_cost();
            if budget_remaining > fdh_cost * 2 {
                if let Some(fdh_id) =
                    helpers::build_node(world, corp_id, NodeType::FiberDistributionHub, cell_index, tick)
                {
                    // Connect FDH to nearest CentralOffice via FeederFiber
                    let co_nodes: Vec<EntityId> = corp_nodes
                        .iter()
                        .copied()
                        .filter(|&nid| {
                            world
                                .infra_nodes
                                .get(&nid)
                                .map(|n| n.node_type == NodeType::CentralOffice)
                                .unwrap_or(false)
                        })
                        .collect();

                    if let Some(nearest_co) =
                        helpers::find_nearest_node(world, &co_nodes, cell_index)
                    {
                        helpers::build_edge(world, corp_id, fdh_id, nearest_co, tick);
                    }
                }
                return; // One FDH per cycle to pace spending
            }
        }
    }

    // Step 2: Place NAPs in underserved cells and connect to nearest FDH
    let fdh_nodes: Vec<EntityId> = corp_nodes
        .iter()
        .copied()
        .filter(|&nid| {
            !world.constructions.contains_key(&nid)
                && world
                    .infra_nodes
                    .get(&nid)
                    .map(|n| n.node_type == NodeType::FiberDistributionHub)
                    .unwrap_or(false)
        })
        .collect();

    if fdh_nodes.is_empty() {
        return; // No FDHs to connect NAPs to
    }

    // Place up to 3 NAPs per cycle
    let mut placed = 0;
    for &(cell_index, _) in &candidates {
        if placed >= 3 {
            break;
        }
        let nap_cost = NodeType::NetworkAccessPoint.construction_cost();
        if budget_remaining < nap_cost * 2 {
            break;
        }

        if let Some(nap_id) =
            helpers::build_node(world, corp_id, NodeType::NetworkAccessPoint, cell_index, tick)
        {
            budget_remaining -= nap_cost;
            placed += 1;

            // Connect to nearest FDH via DistributionFiber
            if let Some(nearest_fdh) =
                helpers::find_nearest_node(world, &fdh_nodes, cell_index)
            {
                helpers::build_edge(world, corp_id, nap_id, nearest_fdh, tick);
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
