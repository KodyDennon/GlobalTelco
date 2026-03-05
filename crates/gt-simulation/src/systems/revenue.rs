//! Traffic-based revenue system.
//!
//! Revenue is now tied to actual traffic flowing through infrastructure:
//! - Nodes earn revenue based on traffic routed through them
//! - Edges earn transit fees based on traffic on the link
//! - Coverage alone provides reduced subscription revenue
//! - Empty/disconnected nodes earn nothing
//! - FTTH building revenue: active NAPs generate per-building subscriber revenue
//!   - Buildings with direct DropCable connections get 100% revenue
//!   - Auto-covered buildings (within NAP radius, no DropCable) get 85% revenue (15% overhead)

use std::collections::HashSet;

use crate::components::ContractStatus;
use crate::world::GameWorld;
use gt_common::types::{EdgeType, EntityId, NodeType};

// ─── Public entry point ───────────────────────────────────────────────────────

pub fn run(world: &mut GameWorld) {
    let tick = world.current_tick();

    // Reset per-node and per-edge revenue tracking before recalculating
    for node in world.infra_nodes.values_mut() {
        node.revenue_generated = 0;
    }
    for edge in world.infra_edges.values_mut() {
        edge.revenue_generated = 0;
    }

    // Temporary map to accumulate revenue per corporation
    let mut corp_revenues: std::collections::HashMap<u64, i64> = std::collections::HashMap::new();
    let mut node_revenue_updates: Vec<(u64, i64)> = Vec::new();
    let mut edge_revenue_updates: Vec<(u64, i64)> = Vec::new();

    // 1. Calculate Node Traffic Revenue
    for (&node_id, node) in &world.infra_nodes {
        let health = world.healths.get(&node_id).map(|h| h.condition).unwrap_or(1.0);
        if world.constructions.contains_key(&node_id) || health < 0.1 {
            continue;
        }

        let traffic = world.traffic_matrix.node_traffic.get(&node_id).copied().unwrap_or(0.0);
        if traffic <= 0.0 { continue; }

        let rate = node.node_type.traffic_revenue_rate();
        let quality = quality_multiplier(world, node_id, health);
        let total_node_revenue = (traffic * rate * quality) as i64;

        if total_node_revenue > 0 {
            // Track total generated for display
            node_revenue_updates.push((node_id, total_node_revenue));

            // Distribute to co-owners
            if let Some(ownership) = world.ownerships.get(&node_id) {
                let co_owner_total_share: f64 = ownership.co_owners.iter().map(|(_, s)| *s).sum();
                let primary_share = (1.0 - co_owner_total_share).max(0.0);
                
                *corp_revenues.entry(ownership.owner).or_insert(0) += (total_node_revenue as f64 * primary_share) as i64;
                for &(co_owner_id, share_pct) in &ownership.co_owners {
                    *corp_revenues.entry(co_owner_id).or_insert(0) += (total_node_revenue as f64 * share_pct) as i64;
                }
            }
        }
    }

    // 2. Calculate Edge Traffic Revenue (Transit Fees)
    for (&edge_id, edge) in &world.infra_edges {
        let traffic = world.traffic_matrix.edge_traffic.get(&edge_id).copied().unwrap_or(0.0);
        if traffic <= 0.0 { continue; }

        let rate = edge.edge_type.traffic_revenue_rate();
        let total_edge_revenue = (traffic * rate * edge.health) as i64;

        if total_edge_revenue > 0 {
            edge_revenue_updates.push((edge_id, total_edge_revenue));

            if let Some(ownership) = world.ownerships.get(&edge_id) {
                let co_owner_total_share: f64 = ownership.co_owners.iter().map(|(_, s)| *s).sum();
                let primary_share = (1.0 - co_owner_total_share).max(0.0);
                
                *corp_revenues.entry(ownership.owner).or_insert(0) += (total_edge_revenue as f64 * primary_share) as i64;
                for &(co_owner_id, share_pct) in &ownership.co_owners {
                    *corp_revenues.entry(co_owner_id).or_insert(0) += (total_edge_revenue as f64 * share_pct) as i64;
                }
            }
        }
    }

    // Apply accumulated updates
    for (node_id, rev) in node_revenue_updates {
        if let Some(n) = world.infra_nodes.get_mut(&node_id) {
            n.revenue_generated += rev;
        }
    }
    for (edge_id, rev) in edge_revenue_updates {
        if let Some(e) = world.infra_edges.get_mut(&edge_id) {
            e.revenue_generated += rev;
        }
    }

    // 3. Subscription & Contract Revenue (Iteration per corp is fine here as they are corp-centric)
    let mut corp_ids: Vec<u64> = world.corporations.keys().copied().collect();
    corp_ids.sort_unstable();

    for &corp_id in &corp_ids {
        let mut extra_revenue = 0;
        extra_revenue += calculate_contract_revenue(world, corp_id);
        extra_revenue += calculate_coverage_revenue(world, corp_id);
        extra_revenue += calculate_building_revenue(world, corp_id);
        
        *corp_revenues.entry(corp_id).or_insert(0) += extra_revenue;
    }

    // 4. Apply all collected revenue to financials
    for (&corp_id, &amount) in &corp_revenues {
        if let Some(fin) = world.financials.get_mut(&corp_id) {
            fin.revenue_per_tick = amount;
            fin.cash += amount;
        }

        if amount > 0 {
            world.event_queue.push(
                tick,
                gt_common::events::GameEvent::RevenueEarned {
                    corporation: corp_id,
                    amount,
                },
            );
        }
    }

    // Process transit settlements (paid transit) and alliance revenue sharing
    calculate_transit_settlements(world, tick);
    calculate_alliance_traffic_revenue(world, tick);
}

// ─── Contract Revenue ─────────────────────────────────────────────────────────

fn calculate_contract_revenue(world: &GameWorld, corp_id: EntityId) -> i64 {
    let mut contract_revs: Vec<(u64, i64)> = world
        .contracts
        .iter()
        .filter(|(_, c)| {
            c.from == corp_id && c.status == crate::components::ContractStatus::Active
        })
        .map(|(&cid, c)| (cid, c.price_per_tick))
        .collect();
    contract_revs.sort_unstable_by_key(|t| t.0);
    contract_revs.iter().map(|t| t.1).sum()
}

// ─── Coverage Revenue (reduced weight) ────────────────────────────────────────

fn calculate_coverage_revenue(world: &GameWorld, corp_id: EntityId) -> i64 {
    let mut covered_population: u64 = 0;

    let mut coverage_cells: Vec<usize> = world.cell_coverage.keys().copied().collect();
    coverage_cells.sort_unstable();

    for cell_idx in coverage_cells {
        let coverage = match world.cell_coverage.get(&cell_idx) {
            Some(c) => c,
            None => continue,
        };
        if coverage.dominant_owner != Some(corp_id) || coverage.bandwidth <= 0.0 {
            continue;
        }

        if let Some(&city_id) = world.cell_to_city.get(&cell_idx) {
            if let Some(city) = world.cities.get(&city_id) {
                let cell_pop = city.population / city.cells.len().max(1) as u64;
                let satisfaction = city.infrastructure_satisfaction;
                covered_population += (cell_pop as f64 * satisfaction) as u64;
            }
        }
    }

    // Coverage subscription revenue: represents monthly fees from covered population.
    // At $0.15/pop/tick, covering 100K people at 50% satisfaction = $7,500/tick.
    (covered_population as f64 * 0.15) as i64
}

// ─── FTTH Building Revenue (per-building subscriber fees) ────────────────────

/// Base rate per tick per demand unit for FTTH building subscribers.
const BUILDING_BASE_RATE: f64 = 50.0;

/// Overhead deduction for auto-covered buildings (no dedicated DropCable).
/// Buildings covered by NAP radius alone get 85% of full revenue.
const AUTO_COVERAGE_FACTOR: f64 = 0.85;

/// Calculate per-building subscriber revenue for a corporation.
///
/// Each building footprint in the simulation serves as an individual demand point.
/// Revenue per building = BUILDING_BASE_RATE * demand_value * service_quality * connection_factor * competition_share.
///
/// **Connection factor:**
/// - Buildings with a direct DropCable from one of the corp's active NAPs get 100% (full rate).
/// - Buildings auto-covered by NAP radius (no DropCable) get 85% (15% overhead deduction).
///
/// **Competition:**
/// - If multiple corporations cover the same cell, subscriber revenue is split
///   proportionally by bandwidth share at that cell (from `CellCoverage::per_corp_bandwidth`).
/// - A monopoly corp in a cell captures 100% of building revenue in that cell.
///
/// **Fallback (no buildings seeded):**
/// - If no building footprints exist yet, falls back to the legacy cell-based approximation
///   to keep revenue flowing during early game or un-migrated saves.
fn calculate_building_revenue(world: &GameWorld, corp_id: EntityId) -> i64 {
    // Collect this corporation's active FTTH NAPs
    let corp_nodes = world
        .corp_infra_nodes
        .get(&corp_id)
        .cloned()
        .unwrap_or_default();

    let mut active_naps: Vec<(EntityId, usize, f64, f64)> = Vec::new(); // (id, cell_index, health, utilization)

    for &node_id in &corp_nodes {
        let node = match world.infra_nodes.get(&node_id) {
            Some(n) => n,
            None => continue,
        };

        if node.node_type != NodeType::NetworkAccessPoint || !node.active_ftth {
            continue;
        }

        // Skip NAPs still under construction
        if world.constructions.contains_key(&node_id) {
            continue;
        }

        let health = world
            .healths
            .get(&node_id)
            .map(|h| h.condition)
            .unwrap_or(1.0);
        let utilization = node.utilization();

        active_naps.push((node_id, node.cell_index, health, utilization));
    }

    if active_naps.is_empty() {
        return 0;
    }

    // Sort for deterministic processing
    active_naps.sort_unstable_by_key(|t| t.0);

    // Build a set of building entity IDs that have a direct DropCable from one of our NAPs.
    // Also build a set of cells that our NAPs serve with DropCables.
    let mut drop_cable_targets: HashSet<EntityId> = HashSet::new();
    let mut drop_cable_cells: HashSet<usize> = HashSet::new();
    let nap_ids: HashSet<EntityId> = active_naps.iter().map(|t| t.0).collect();

    for edge in world.infra_edges.values() {
        if edge.edge_type != EdgeType::DropCable {
            continue;
        }
        // A DropCable connects a NAP to a building endpoint (or vice versa).
        let nap_end = if nap_ids.contains(&edge.source) {
            Some(edge.target)
        } else if nap_ids.contains(&edge.target) {
            Some(edge.source)
        } else {
            None
        };
        if let Some(target) = nap_end {
            drop_cable_targets.insert(target);
            // Also mark the cell of the target node (if it's an infra node) as DropCable-covered
            if let Some(node) = world.infra_nodes.get(&target) {
                drop_cable_cells.insert(node.cell_index);
            }
        }
    }

    let cell_spacing = world.cell_spacing_km;

    // Collect cells covered by our NAPs (with service quality per cell)
    let mut nap_covered_cells: std::collections::HashMap<usize, f64> =
        std::collections::HashMap::new();

    for &(_nap_id, nap_cell, health, utilization) in &active_naps {
        let utilization_headroom = (1.0 - utilization).max(0.0);
        let service_quality = health * utilization_headroom;

        if service_quality <= 0.0 {
            continue;
        }

        let base_radius_km = NodeType::NetworkAccessPoint.coverage_radius_km();
        let radius_km = base_radius_km.max(cell_spacing * 0.8);

        let nap_pos = match world.grid_cell_positions.get(nap_cell) {
            Some(p) => *p,
            None => continue,
        };
        let (nap_lat, nap_lon) = nap_pos;
        let lat_range = radius_km / 111.0;
        let cos_lat = (nap_lat.to_radians()).cos().max(0.1);
        let lon_range = radius_km / (111.0 * cos_lat);

        for (cell_idx, &(cell_lat, cell_lon)) in world.grid_cell_positions.iter().enumerate() {
            if (cell_lat - nap_lat).abs() > lat_range || (cell_lon - nap_lon).abs() > lon_range {
                continue;
            }
            // Only cells that belong to a city have buildings
            if !world.cell_to_city.contains_key(&cell_idx) {
                continue;
            }
            // Use the best service quality from any covering NAP for this cell
            let entry = nap_covered_cells
                .entry(cell_idx)
                .or_insert(0.0_f64);
            if service_quality > *entry {
                *entry = service_quality;
            }
        }
    }

    if nap_covered_cells.is_empty() {
        return 0;
    }

    // ── Per-building revenue calculation ─────────────────────────────────────

    // Check if building footprints exist for any of the covered cities
    let has_buildings = !world.building_footprints.is_empty();

    if has_buildings {
        // New per-building model: iterate over individual building footprints in covered cells
        let mut revenue: f64 = 0.0;

        // Collect and sort building IDs for deterministic processing
        let mut building_entries: Vec<(EntityId, usize, f64)> = world
            .building_footprints
            .iter()
            .filter_map(|(&bldg_id, bldg)| {
                if bldg.effective_demand() <= 0.0 {
                    return None;
                }
                let service_quality = nap_covered_cells.get(&bldg.cell_index)?;
                Some((bldg_id, bldg.cell_index, *service_quality))
            })
            .collect();
        building_entries.sort_unstable_by_key(|t| t.0);

        for (bldg_id, cell_idx, service_quality) in building_entries {
            let bldg = match world.building_footprints.get(&bldg_id) {
                Some(b) => b,
                None => continue,
            };

            let demand_value = bldg.effective_demand();

            // Connection factor: DropCable cells get 100%, auto-covered get 85%
            let connection_factor = if drop_cable_cells.contains(&cell_idx) {
                1.0
            } else {
                AUTO_COVERAGE_FACTOR
            };

            // Competition share: if multiple corps cover this cell, split by bandwidth
            let competition_share = world
                .cell_coverage
                .get(&cell_idx)
                .map(|cov| {
                    if cov.competitor_count() <= 1 {
                        // Monopoly or no other coverage — full revenue
                        1.0
                    } else {
                        // Split by bandwidth share; minimum 10% even with tiny share
                        cov.corp_bandwidth_share(corp_id).max(0.1)
                    }
                })
                .unwrap_or(1.0);

            let bldg_revenue = BUILDING_BASE_RATE
                * demand_value
                * service_quality
                * connection_factor
                * competition_share;

            revenue += bldg_revenue;
        }

        revenue as i64
    } else {
        // Legacy fallback: cell-based approximation for worlds without building footprints.
        // This preserves backward compatibility with old save files.
        calculate_building_revenue_legacy(world, corp_id, &active_naps, &drop_cable_cells, &nap_covered_cells)
    }
}

/// Legacy cell-based building revenue approximation.
/// Used as fallback when no BuildingFootprint entities have been seeded.
fn calculate_building_revenue_legacy(
    world: &GameWorld,
    _corp_id: EntityId,
    active_naps: &[(EntityId, usize, f64, f64)],
    _drop_cable_cells: &HashSet<usize>,
    _nap_covered_cells: &std::collections::HashMap<usize, f64>,
) -> i64 {
    let cell_spacing = world.cell_spacing_km;
    let mut revenue: f64 = 0.0;

    // Count total DropCable connections across all active NAPs for proportional allocation
    let _nap_ids: HashSet<EntityId> = active_naps.iter().map(|t| t.0).collect();
    let mut drop_cable_count: std::collections::HashMap<EntityId, u32> =
        std::collections::HashMap::new();
    for edge in world.infra_edges.values() {
        if edge.edge_type != EdgeType::DropCable {
            continue;
        }
        for &(nap_id, _, _, _) in active_naps {
            if edge.source == nap_id || edge.target == nap_id {
                *drop_cable_count.entry(nap_id).or_insert(0) += 1;
            }
        }
    }

    for &(nap_id, nap_cell, health, utilization) in active_naps {
        let utilization_headroom = (1.0 - utilization).max(0.0);
        let service_quality = health * utilization_headroom;
        if service_quality <= 0.0 {
            continue;
        }

        let base_radius_km = NodeType::NetworkAccessPoint.coverage_radius_km();
        let radius_km = base_radius_km.max(cell_spacing * 0.8);

        let nap_pos = match world.grid_cell_positions.get(nap_cell) {
            Some(p) => *p,
            None => continue,
        };
        let (nap_lat, nap_lon) = nap_pos;
        let lat_range = radius_km / 111.0;
        let cos_lat = (nap_lat.to_radians()).cos().max(0.1);
        let lon_range = radius_km / (111.0 * cos_lat);

        let drops = drop_cable_count.get(&nap_id).copied().unwrap_or(0);
        let has_drops = drops > 0;

        let mut covered_demand: f64 = 0.0;
        let mut covered_cell_count: u32 = 0;

        for (cell_idx, &(cell_lat, cell_lon)) in world.grid_cell_positions.iter().enumerate() {
            if (cell_lat - nap_lat).abs() > lat_range || (cell_lon - nap_lon).abs() > lon_range {
                continue;
            }
            let city = match world
                .cell_to_city
                .get(&cell_idx)
                .and_then(|&cid| world.cities.get(&cid))
            {
                Some(c) => c,
                None => continue,
            };
            let cell_pop = city.population / city.cells.len().max(1) as u64;
            let demand_value = city.telecom_demand * (cell_pop as f64 / 1000.0);
            if demand_value <= 0.0 {
                continue;
            }
            covered_demand += demand_value;
            covered_cell_count += 1;
        }

        if covered_demand <= 0.0 {
            continue;
        }

        let full_rate_cells = if has_drops {
            drops.min(covered_cell_count)
        } else {
            0
        };
        let auto_rate_cells = covered_cell_count.saturating_sub(full_rate_cells);

        let demand_per_cell = if covered_cell_count > 0 {
            covered_demand / covered_cell_count as f64
        } else {
            0.0
        };

        let full_rate_revenue =
            full_rate_cells as f64 * demand_per_cell * BUILDING_BASE_RATE * service_quality;
        let auto_rate_revenue = auto_rate_cells as f64
            * demand_per_cell
            * BUILDING_BASE_RATE
            * service_quality
            * AUTO_COVERAGE_FACTOR;

        revenue += full_rate_revenue + auto_rate_revenue;
    }

    revenue as i64
}

// ─── Quality Multiplier ───────────────────────────────────────────────────────

fn quality_multiplier(world: &GameWorld, node_id: EntityId, health: f64) -> f64 {
    let health_factor = if health < 0.8 { health / 0.8 } else { 1.0 };

    let node_cell = world
        .infra_nodes
        .get(&node_id)
        .map(|n| n.cell_index)
        .unwrap_or(0);

    let satisfaction_bonus = world
        .cell_to_city
        .get(&node_cell)
        .and_then(|&cid| world.cities.get(&cid))
        .map(|city| {
            if city.infrastructure_satisfaction > 0.8 {
                0.1
            } else {
                0.0
            }
        })
        .unwrap_or(0.0);

    health_factor * (1.0 + satisfaction_bonus)
}

// ─── Transit Settlements (paid Transit contracts) ────────────────────────────

/// For each active Transit contract with traffic flowing through it,
/// credit the transit provider and debit the originator.
/// Peering contracts are settlement-free (no payment exchanged).
fn calculate_transit_settlements(world: &mut GameWorld, tick: u64) {
    // Collect contract traffic data
    let contract_traffic = world.traffic_matrix.contract_traffic.clone();

    // Process each contract with traffic
    let mut contract_ids: Vec<u64> = contract_traffic.keys().copied().collect();
    contract_ids.sort_unstable();

    for contract_id in contract_ids {
        let traffic = match contract_traffic.get(&contract_id) {
            Some(&t) if t > 0.0 => t,
            _ => continue,
        };

        let (contract_type, provider, consumer, price_per_tick, capacity) =
            match world.contracts.get(&contract_id) {
                Some(c) if c.status == ContractStatus::Active => {
                    (c.contract_type, c.from, c.to, c.price_per_tick, c.capacity)
                }
                _ => continue,
            };

        // Only Transit and SLA contracts generate settlement payments
        // Peering is settlement-free
        if contract_type == crate::components::ContractType::Peering {
            continue;
        }

        // Transit revenue = traffic * (price_per_tick / capacity)
        // This proportionally charges based on how much of the contract capacity is used
        let price_per_unit = if capacity > 0.0 {
            price_per_tick as f64 / capacity
        } else {
            0.0
        };
        let payment = (traffic * price_per_unit) as i64;
        if payment <= 0 {
            continue;
        }

        // Credit provider, debit consumer
        if let Some(fin) = world.financials.get_mut(&provider) {
            fin.cash += payment;
        }
        if let Some(fin) = world.financials.get_mut(&consumer) {
            fin.cash -= payment;
        }

        world.event_queue.push(
            tick,
            gt_common::events::GameEvent::TransitPayment {
                provider,
                consumer,
                contract: contract_id,
                amount: payment,
            },
        );
    }
}

// ─── Alliance Traffic Revenue Sharing ────────────────────────────────────────

/// When alliance members' traffic flows through each other's network,
/// the transit provider earns a share of the total path revenue
/// based on their node hop contribution.
fn calculate_alliance_traffic_revenue(world: &mut GameWorld, _tick: u64) {
    let attributions = world.traffic_matrix.path_attribution.clone();
    
    // Group alliances by member for fast lookup
    let mut corp_to_alliance: std::collections::HashMap<u64, &crate::components::alliance::Alliance> = 
        std::collections::HashMap::new();
    for alliance in world.alliances.values() {
        for &member_id in &alliance.member_corp_ids {
            corp_to_alliance.insert(member_id, alliance);
        }
    }

    let mut alliance_settlements: std::collections::HashMap<u64, i64> = std::collections::HashMap::new();

    for attr in attributions {
        if attr.traffic <= 0.0 || attr.corp_hops.len() < 2 {
            continue;
        }

        // Find the \"Originator\" (primary owner of the source city)
        let originator = world.cities.get(&attr.source_city)
            .and_then(|_c| world.ownerships.get(&attr.source_city).map(|o| o.owner))
            .unwrap_or(0);

        
        if originator == 0 { continue; }

        let alliance = match corp_to_alliance.get(&originator) {
            Some(a) => a,
            None => continue, // Originator not in an alliance
        };

        // Total hops in path
        let total_hops: u32 = attr.corp_hops.values().sum();
        if total_hops == 0 { continue; }

        // Calculate theoretical total revenue for this path
        // (Simplified: using a blended path rate)
        let path_rate = 0.5; // $0.5 per unit per hop (average)
        let total_path_revenue = (attr.traffic * path_rate * total_hops as f64) as i64;

        for (&corp_id, &hops) in &attr.corp_hops {
            if corp_id == originator { continue; }

            // Check if this contributor is an ally of the originator
            if alliance.member_corp_ids.contains(&corp_id) {
                // Proportional share based on hops
                let share = (total_path_revenue as f64 * (hops as f64 / total_hops as f64)) as i64;
                
                if share > 0 {
                    *alliance_settlements.entry(corp_id).or_insert(0) += share;
                    *alliance_settlements.entry(originator).or_insert(0) -= share;
                }
            }
        }
    }

    // Apply settlements
    for (&corp_id, &amount) in &alliance_settlements {
        if let Some(fin) = world.financials.get_mut(&corp_id) {
            fin.cash += amount;
            // Also adjust revenue_per_tick for transparency
            fin.revenue_per_tick += amount;
        }
    }
}
