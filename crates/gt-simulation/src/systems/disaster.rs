//! Disaster system with cascading failures.
//!
//! Disasters now damage both nodes AND edges. When critical infrastructure
//! is damaged, traffic reroutes through alternate paths. If no alternate
//! path exists, traffic is dropped and affected cities lose satisfaction.
//!
//! Deployment vulnerability: aerial, underground, and submarine edges take
//! different amounts of damage depending on the disaster type. See
//! `deployment_vulnerability_multiplier` for the full matrix.

use crate::components::infra_edge::DeploymentMethod;
use crate::world::GameWorld;
use gt_common::types::EdgeType;

const DISASTER_TYPES: &[(&str, f64)] = &[
    ("Earthquake", 0.15),
    ("Hurricane", 0.15),
    ("Flooding", 0.20),
    ("Landslide", 0.10),
    ("CyberAttack", 0.15),
    ("PoliticalUnrest", 0.10),
    ("RegulatoryChange", 0.10),
    ("EquipmentFailure", 0.05),
];

pub fn run(world: &mut GameWorld) {
    let tick = world.current_tick();

    if !tick.is_multiple_of(50) {
        return;
    }

    let region_data: Vec<(u64, f64, Vec<usize>)> = {
        let mut v: Vec<_> = world
            .regions
            .iter()
            .map(|(&id, r)| (id, r.disaster_risk, r.cells.clone()))
            .collect();
        v.sort_unstable_by_key(|t| t.0);
        v
    };

    for (region_id, disaster_risk, cells) in region_data {
        let roll = world.deterministic_random();

        if roll >= 0.02 * disaster_risk {
            continue;
        }

        let severity = world.deterministic_random() * 0.5 + 0.1;
        let disaster_name = pick_disaster_type(world);

        let (affected_node_count, affected_edge_count) =
            apply_disaster_damage(world, &cells, severity, tick, disaster_name);

        apply_population_displacement(world, &cells, severity);

        // Mark network dirty so routing recomputes (cascading reroute)
        mark_network_dirty(world, &cells);

        world.event_queue.push(
            tick,
            gt_common::events::GameEvent::DisasterStruck {
                region: region_id,
                severity,
                disaster_type: disaster_name.to_string(),
                affected_nodes: affected_node_count + affected_edge_count,
            },
        );
    }
}

// ─── Disaster Type Selection ──────────────────────────────────────────────────

fn pick_disaster_type(world: &mut GameWorld) -> &'static str {
    let type_roll = world.deterministic_random();
    let mut cumulative = 0.0;
    for &(name, weight) in DISASTER_TYPES {
        cumulative += weight;
        if type_roll < cumulative {
            return name;
        }
    }
    "Earthquake"
}

// ─── Deployment Vulnerability ─────────────────────────────────────────────────

/// Categorize a disaster type string into a canonical disaster category.
fn disaster_category(disaster_type: &str) -> &'static str {
    let lower = disaster_type.to_lowercase();
    if lower.contains("ice") || lower.contains("blizzard") {
        "ice_storm"
    } else if lower.contains("storm") || lower.contains("hurricane")
        || lower.contains("typhoon") || lower.contains("cyclone")
    {
        "storm"
    } else if lower.contains("earthquake") {
        "earthquake"
    } else if lower.contains("flood") {
        "flood"
    } else if lower.contains("landslide") {
        "landslide"
    } else if lower.contains("heat") {
        "heat_wave"
    } else {
        // CyberAttack, PoliticalUnrest, RegulatoryChange, EquipmentFailure, etc.
        // These don't have deployment-specific modifiers — use 1.0 (neutral).
        "other"
    }
}

/// Returns true if the edge type represents submarine/subsea infrastructure.
fn is_submarine_edge(edge_type: &EdgeType) -> bool {
    matches!(
        edge_type,
        EdgeType::Submarine | EdgeType::SubseaTelegraphCable | EdgeType::SubseaFiberCable
    )
}

/// Damage multiplier based on deployment method and disaster category.
///
/// Aerial edges are highly vulnerable to storms/ice but resilient to earthquakes.
/// Underground edges are resilient to weather but vulnerable to earthquakes/flooding.
/// Submarine edges are mainly vulnerable to earthquakes (seabed movement).
fn deployment_vulnerability_multiplier(
    deployment: DeploymentMethod,
    edge_type: &EdgeType,
    disaster_type: &str,
) -> f64 {
    let category = disaster_category(disaster_type);

    // Submarine edges override deployment method — cable ships, seabed environment
    if is_submarine_edge(edge_type) {
        return match category {
            "earthquake" => 1.5,
            "storm" => 0.3,
            "ice_storm" => 0.1,
            "flood" => 0.1,
            "landslide" => 0.1,
            "heat_wave" => 0.1,
            _ => 0.1,
        };
    }

    match deployment {
        DeploymentMethod::Aerial => match category {
            "storm" => 1.5,
            "ice_storm" => 1.8,
            "earthquake" => 0.3,
            "flood" => 0.7,
            "landslide" => 1.0,
            "heat_wave" => 0.2,
            _ => 1.0,
        },
        DeploymentMethod::Underground => match category {
            "storm" => 0.2,
            "ice_storm" => 0.1,
            "earthquake" => 1.5,
            "flood" => 1.5,
            "landslide" => 1.2,
            "heat_wave" => 0.1,
            _ => 1.0,
        },
    }
}

// ─── Damage Application ───────────────────────────────────────────────────────

fn apply_disaster_damage(
    world: &mut GameWorld,
    cells: &[usize],
    severity: f64,
    tick: u64,
    disaster_type: &str,
) -> (u32, u32) {
    // Damage nodes
    let mut affected_nodes: Vec<u64> = world
        .infra_nodes
        .iter()
        .filter(|(_, node)| cells.contains(&node.cell_index))
        .map(|(&id, _)| id)
        .collect();
    affected_nodes.sort_unstable();

    let node_count = affected_nodes.len() as u32;

    for &node_id in &affected_nodes {
        let damage = severity * 0.3;
        if let Some(health) = world.healths.get_mut(&node_id) {
            health.degrade(damage);
        }

        apply_insurance_payout(world, node_id, damage, tick);
    }

    // Damage edges in affected region (50% chance per edge, deployment-modified damage)
    let mut affected_edges: Vec<(u64, DeploymentMethod, EdgeType)> = world
        .infra_edges
        .iter()
        .filter(|(_, edge)| {
            let src_cell = world.infra_nodes.get(&edge.source).map(|n| n.cell_index);
            let dst_cell = world.infra_nodes.get(&edge.target).map(|n| n.cell_index);
            src_cell.map(|c| cells.contains(&c)).unwrap_or(false)
                || dst_cell.map(|c| cells.contains(&c)).unwrap_or(false)
        })
        .map(|(&id, edge)| (id, edge.deployment, edge.edge_type))
        .collect();
    affected_edges.sort_unstable_by_key(|t| t.0);

    let mut edge_count: u32 = 0;
    for &(edge_id, deployment, edge_type) in &affected_edges {
        // 50% chance to damage each edge
        let roll = world.deterministic_random();
        if roll > 0.5 {
            continue;
        }

        let vuln = deployment_vulnerability_multiplier(deployment, &edge_type, disaster_type);
        let damage = severity * 0.2 * vuln;

        if damage > 0.0 {
            if let Some(edge) = world.infra_edges.get_mut(&edge_id) {
                edge.health = (edge.health - damage).max(0.0);
                edge.last_damage_tick = Some(tick);
                edge_count += 1;
            }
        }
    }

    (node_count, edge_count)
}

fn apply_insurance_payout(world: &mut GameWorld, node_id: u64, damage: f64, tick: u64) {
    let is_insured = world
        .infra_nodes
        .get(&node_id)
        .map(|n| n.insured)
        .unwrap_or(false);

    if !is_insured {
        return;
    }

    let repair_cost = world
        .infra_nodes
        .get(&node_id)
        .map(|n| (n.construction_cost as f64 * damage * 0.2) as i64)
        .unwrap_or(0);
    let payout = (repair_cost as f64 * 0.6) as i64;
    let owner = world.infra_nodes.get(&node_id).map(|n| n.owner);

    if let Some(owner_id) = owner {
        if let Some(fin) = world.financials.get_mut(&owner_id) {
            fin.cash += payout;
        }
        world.event_queue.push(
            tick,
            gt_common::events::GameEvent::InsurancePayout {
                entity: node_id,
                amount: payout,
            },
        );
    }
}

// ─── Population Displacement ──────────────────────────────────────────────────

fn apply_population_displacement(world: &mut GameWorld, cells: &[usize], severity: f64) {
    if severity <= 0.3 {
        return;
    }

    let mut city_ids: Vec<u64> = world
        .cities
        .iter()
        .filter(|(_, c)| cells.contains(&c.cell_index))
        .map(|(&id, _)| id)
        .collect();
    city_ids.sort_unstable();

    for &city_id in &city_ids {
        if let Some(city) = world.cities.get_mut(&city_id) {
            let displaced = (city.population as f64 * severity * 0.05) as u64;
            city.population = city.population.saturating_sub(displaced);
            city.migration_pressure += severity * 0.2;
        }
    }
}

// ─── Network Dirty Marking (triggers reroute) ────────────────────────────────

fn mark_network_dirty(world: &mut GameWorld, cells: &[usize]) {
    let affected_node_ids: Vec<u64> = world
        .infra_nodes
        .iter()
        .filter(|(_, node)| cells.contains(&node.cell_index))
        .map(|(&id, _)| id)
        .collect();

    for &nid in &affected_node_ids {
        world.network.invalidate_node(nid);
    }
}
