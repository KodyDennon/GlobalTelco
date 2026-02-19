use crate::world::GameWorld;
use gt_common::types::NodeType;

pub fn run(world: &mut GameWorld) {
    // Phase 0: Reset capacities to base values before applying boosts
    // This prevents cumulative drift from DataCenter boosts and maintenance reductions
    reset_capacities_to_base(world);

    // Phase 1: DataCenter boost — DataCenters increase effective capacity of connected nodes
    apply_datacenter_boosts(world);

    // Phase 2: ExchangePoint latency reduction — ExchangePoints reduce latency on connected edges
    apply_exchange_point_latency(world);

    // Phase 3: Calculate node utilization from per-cell coverage demand
    // Each node's load comes from the cells it covers and the demand in those cells
    calculate_node_loads(world);

    // Phase 4: Calculate edge utilization from traffic between connected nodes
    calculate_edge_loads(world);
}

/// Reset capacity max_throughput to the base value from InfraNode each tick.
/// This prevents cumulative drift from DataCenter boosts and maintenance reductions.
fn reset_capacities_to_base(world: &mut GameWorld) {
    let mut updates: Vec<(u64, f64)> = world
        .infra_nodes
        .iter()
        .filter(|(id, _)| !world.constructions.contains_key(id))
        .map(|(&id, node)| (id, node.max_throughput))
        .collect();
    updates.sort_unstable_by_key(|t| t.0);

    for (node_id, base_throughput) in updates {
        if let Some(cap) = world.capacities.get_mut(&node_id) {
            // Apply health degradation: severely damaged nodes have reduced capacity
            let health = world.healths.get(&node_id).map(|h| h.condition).unwrap_or(1.0);
            if health < 0.5 {
                cap.max_throughput = base_throughput * health;
            } else {
                cap.max_throughput = base_throughput;
            }
        }
    }
}

/// DataCenters boost the effective capacity of all nodes they're connected to via edges.
/// A DataCenter acts as a capacity multiplier for its network cluster.
fn apply_datacenter_boosts(world: &mut GameWorld) {
    // Find all operational DataCenters and the nodes they connect to
    // Sort by ID for deterministic iteration order
    let mut datacenter_boosts: Vec<(u64, f64)> = world
        .infra_nodes
        .iter()
        .filter(|(id, node)| {
            node.node_type == NodeType::DataCenter && !world.constructions.contains_key(id)
        })
        .map(|(&id, node)| {
            let health = world.healths.get(&id).map(|h| h.condition).unwrap_or(1.0);
            // DataCenter boost scales with its throughput and health
            // A healthy 50K throughput DC provides a 20% capacity boost
            let boost_factor = (node.max_throughput / 50000.0).min(2.0) * 0.2 * health;
            (id, boost_factor)
        })
        .collect();
    datacenter_boosts.sort_unstable_by_key(|t| t.0);

    for (dc_id, boost_factor) in &datacenter_boosts {
        // Find all edges connecting to this DataCenter — sorted for determinism
        let mut connected_nodes: Vec<u64> = world
            .infra_edges
            .values()
            .filter_map(|edge| {
                if edge.source == *dc_id {
                    Some(edge.target)
                } else if edge.target == *dc_id {
                    Some(edge.source)
                } else {
                    None
                }
            })
            .collect();
        connected_nodes.sort_unstable();

        // Boost capacity of connected nodes
        for &node_id in &connected_nodes {
            if let Some(cap) = world.capacities.get_mut(&node_id) {
                // Apply boost additively (multiple DCs stack, but with diminishing returns)
                let boost_amount = cap.max_throughput * boost_factor;
                cap.max_throughput += boost_amount;
            }
        }
    }
}

/// Reset edge latency to base values before ExchangePoint reductions.
fn reset_edge_latency(world: &mut GameWorld) {
    use gt_common::types::EdgeType;
    let mut updates: Vec<(u64, f64)> = world
        .infra_edges
        .iter()
        .map(|(&id, edge)| {
            let latency_per_km = match edge.edge_type {
                EdgeType::FiberLocal | EdgeType::FiberRegional | EdgeType::FiberNational | EdgeType::Submarine => 0.005,
                EdgeType::Copper => 0.02,
                EdgeType::Microwave => 0.003,
                EdgeType::Satellite => 0.5,
            };
            (id, latency_per_km * edge.length_km)
        })
        .collect();
    updates.sort_unstable_by_key(|t| t.0);
    for (edge_id, base_latency) in updates {
        if let Some(edge) = world.infra_edges.get_mut(&edge_id) {
            edge.latency_ms = base_latency;
        }
    }
}

/// ExchangePoints reduce latency on all edges connecting through them.
fn apply_exchange_point_latency(world: &mut GameWorld) {
    let mut exchange_points: Vec<(u64, f64)> = world
        .infra_nodes
        .iter()
        .filter(|(id, node)| {
            node.node_type == NodeType::ExchangePoint && !world.constructions.contains_key(id)
        })
        .map(|(&id, node)| {
            let health = world.healths.get(&id).map(|h| h.condition).unwrap_or(1.0);
            // ExchangePoint reduces latency by up to 30% based on throughput and health
            let reduction = (node.max_throughput / 5000.0).min(1.0) * 0.3 * health;
            (id, reduction)
        })
        .collect();
    exchange_points.sort_unstable_by_key(|t| t.0);

    // Collect edge updates sorted for deterministic application
    let mut edge_latency_reductions: Vec<(u64, f64)> = Vec::new();

    for (ep_id, reduction) in &exchange_points {
        let mut edge_ids: Vec<u64> = world
            .infra_edges
            .iter()
            .filter(|(_, edge)| edge.source == *ep_id || edge.target == *ep_id)
            .map(|(&id, _)| id)
            .collect();
        edge_ids.sort_unstable();
        for edge_id in edge_ids {
            edge_latency_reductions.push((edge_id, *reduction));
        }
    }

    // Sort by edge_id for deterministic application order
    edge_latency_reductions.sort_unstable_by_key(|t| t.0);

    for (edge_id, reduction) in edge_latency_reductions {
        if let Some(edge) = world.infra_edges.get_mut(&edge_id) {
            // Multiple ExchangePoints have diminishing returns
            edge.latency_ms *= 1.0 - reduction;
            edge.latency_ms = edge.latency_ms.max(0.1); // Floor at 0.1ms
        }
    }
}

/// Calculate node load based on per-cell coverage demand.
/// Each node's load comes from the population demand in cells it covers,
/// weighted by how much of each cell's coverage this node provides.
fn calculate_node_loads(world: &mut GameWorld) {
    // Build a map: node_id -> total demand served
    // We derive this from cell_coverage and city demand data
    let mut node_demand: std::collections::HashMap<u64, f64> = std::collections::HashMap::new();

    // For each city, distribute its demand across covering nodes proportionally
    // SORT by city ID for deterministic floating-point accumulation
    let mut city_data: Vec<(u64, f64, Vec<usize>)> = world
        .cities
        .iter()
        .map(|(&id, city)| {
            let demand = world
                .demands
                .get(&id)
                .map(|d| d.current_demand)
                .unwrap_or(city.telecom_demand);
            (id, demand, city.cells.clone())
        })
        .collect();
    city_data.sort_unstable_by_key(|t| t.0);

    // Pre-collect operational nodes sorted by ID for deterministic inner iteration
    let cell_spacing = world.cell_spacing_km;
    let mut sorted_node_data: Vec<(u64, usize, f64, f64, f64, NodeType)> = world
        .infra_nodes
        .iter()
        .filter(|(id, _)| !world.constructions.contains_key(id))
        .map(|(&node_id, node)| {
            let health = world.healths.get(&node_id).map(|h| h.condition).unwrap_or(1.0);
            (node_id, node.cell_index, node.max_throughput, health, 0.0, node.node_type)
        })
        .collect();
    sorted_node_data.sort_unstable_by_key(|t| t.0);

    for (_city_id, demand, cells) in &city_data {
        if cells.is_empty() || *demand <= 0.0 {
            continue;
        }

        let demand_per_cell = demand / cells.len() as f64;

        for &ci in cells {
            let coverage = world.cell_coverage.get(&ci);
            if coverage.is_none() {
                continue;
            }

            // Find all nodes covering this cell and distribute demand proportionally
            // by each node's signal contribution
            let total_signal = coverage.map(|c| c.signal_strength).unwrap_or(0.0);
            if total_signal <= 0.0 {
                continue;
            }

            let cell_pos = world.grid_cell_positions.get(ci);
            if cell_pos.is_none() {
                continue;
            }
            let &(clat, clon) = cell_pos.unwrap();

            // Iterate sorted nodes for deterministic demand distribution
            for &(node_id, cell_index, max_throughput, health, _, node_type) in &sorted_node_data {
                let node_pos = world.grid_cell_positions.get(cell_index);

                if let Some(&(nlat, nlon)) = node_pos {
                    // Scale radius to grid resolution (same as coverage system)
                    let base_radius = node_type.coverage_radius_km();
                    let min_cells = match node_type {
                        NodeType::CellTower => 2.5,
                        NodeType::WirelessRelay => 1.5,
                        NodeType::CentralOffice => 1.5,
                        NodeType::SatelliteGround => 6.0,
                        NodeType::DataCenter => 1.0,
                        NodeType::ExchangePoint => 1.0,
                        NodeType::SubmarineLanding => 0.5,
                    };
                    let radius_km = base_radius.max(cell_spacing * min_cells);
                    let dist_km = haversine_km(nlat, nlon, clat, clon);

                    if dist_km <= radius_km && radius_km > 0.0 {
                        let distance_ratio = dist_km / radius_km;
                        let attenuation = (1.0 - distance_ratio).powi(2);
                        let node_signal = max_throughput * node_type.coverage_capacity_fraction() * health * attenuation;

                        if node_signal > 0.01 {
                            // This node's share of the cell's demand
                            let share = node_signal / total_signal;
                            let allocated_demand = demand_per_cell * share;
                            *node_demand.entry(node_id).or_insert(0.0) += allocated_demand;
                        }
                    }
                }
            }
        }
    }

    // Apply calculated demand as load to each node
    let mut node_ids: Vec<u64> = world.infra_nodes.keys().copied().collect();
    node_ids.sort_unstable();

    for &node_id in &node_ids {
        if world.constructions.contains_key(&node_id) {
            continue;
        }

        let demand = node_demand.get(&node_id).copied().unwrap_or(0.0);

        // Also add transit traffic for backbone/high-level nodes
        let transit_load = calculate_transit_load(world, node_id);
        let total_load = demand + transit_load;

        if let Some(cap) = world.capacities.get_mut(&node_id) {
            // Smooth load changes: blend 80% new, 20% old for stability
            cap.current_load = cap.current_load * 0.2 + total_load * 0.8;
            // Allow slight overload (up to 120%)
            cap.current_load = cap.current_load.min(cap.max_throughput * 1.2);
        }

        // Sync to InfraNode
        if let Some(node) = world.infra_nodes.get_mut(&node_id) {
            if let Some(cap) = world.capacities.get(&node_id) {
                node.current_load = cap.current_load;
            }
        }
    }
}

/// Calculate transit traffic for backbone-level nodes.
/// Higher-level nodes (ExchangePoints, DataCenters, SubmarineLandings) carry
/// inter-region traffic proportional to connected edge bandwidth and utilization.
fn calculate_transit_load(world: &GameWorld, node_id: u64) -> f64 {
    let node = match world.infra_nodes.get(&node_id) {
        Some(n) => n,
        None => return 0.0,
    };

    // Only higher-level nodes handle significant transit
    let transit_factor = match node.node_type {
        NodeType::SubmarineLanding => 0.4,
        NodeType::DataCenter => 0.3,
        NodeType::ExchangePoint => 0.5,
        NodeType::SatelliteGround => 0.2,
        NodeType::CentralOffice => 0.1,
        NodeType::CellTower | NodeType::WirelessRelay => 0.0,
    };

    if transit_factor <= 0.0 {
        return 0.0;
    }

    // Sum up traffic flowing through connected edges
    // Collect and sort by edge ID for deterministic summation
    let mut edge_traffic: Vec<(u64, f64)> = world
        .infra_edges
        .iter()
        .filter(|(_, edge)| edge.source == node_id || edge.target == node_id)
        .map(|(&eid, edge)| {
            // Traffic on edge = average utilization of endpoints × bandwidth
            let other_id = if edge.source == node_id {
                edge.target
            } else {
                edge.source
            };
            let other_util = world
                .capacities
                .get(&other_id)
                .map(|c| c.utilization())
                .unwrap_or(0.0);
            (eid, edge.bandwidth * other_util * 0.1) // 10% of edge traffic is transit
        })
        .collect();
    edge_traffic.sort_unstable_by_key(|t| t.0);
    let connected_edge_traffic: f64 = edge_traffic.iter().map(|t| t.1).sum();

    connected_edge_traffic * transit_factor
}

/// Calculate edge utilization from the load on connected nodes.
/// Edges carry traffic between their endpoint nodes.
fn calculate_edge_loads(world: &mut GameWorld) {
    let mut edge_updates: Vec<(u64, f64)> = world
        .infra_edges
        .iter()
        .map(|(&id, edge)| {
            let src_util = world
                .capacities
                .get(&edge.source)
                .map(|c| c.utilization())
                .unwrap_or(0.0);
            let dst_util = world
                .capacities
                .get(&edge.target)
                .map(|c| c.utilization())
                .unwrap_or(0.0);

            // Edge traffic is proportional to the average load of its endpoints
            // Higher-bandwidth edges carry proportionally more traffic
            let avg_util = (src_util + dst_util) / 2.0;

            // Also consider: traffic flows toward higher-utilized nodes (demand pull)
            let demand_pull = (src_util - dst_util).abs() * 0.2;
            let effective_util = (avg_util + demand_pull).min(1.0);

            let edge_load = effective_util * edge.bandwidth;
            (id, edge_load)
        })
        .collect();
    edge_updates.sort_unstable_by_key(|t| t.0);

    for (edge_id, load) in edge_updates {
        if let Some(edge) = world.infra_edges.get_mut(&edge_id) {
            // Smooth edge load changes
            edge.current_load = edge.current_load * 0.2 + load * 0.8;
        }
    }
}

/// Haversine distance between two lat/lon points in km.
fn haversine_km(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let dlat = (lat1 - lat2).to_radians();
    let dlon = (lon1 - lon2).to_radians();
    let a = (dlat / 2.0).sin().powi(2)
        + lat1.to_radians().cos() * lat2.to_radians().cos() * (dlon / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().asin();
    6371.0 * c
}
