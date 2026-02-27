//! Traffic Flow Engine — OD-matrix based utilization system.
//!
//! Replaces the old proximity-based utilization with actual traffic routing:
//! 1. Cities generate traffic demand (OD matrix)
//! 2. Traffic is routed through the network graph via cached shortest paths
//! 3. Node/edge loads accumulate from actual traffic flowing through them
//! 4. Congestion occurs when links exceed capacity; excess traffic is dropped

use crate::world::GameWorld;
use gt_common::types::{EntityId, NetworkTier, NodeType, TrafficDemand, TrafficMatrix};
use std::collections::HashMap;

// ─── Public entry point ───────────────────────────────────────────────────────

pub fn run(world: &mut GameWorld) {
    reset_node_effective_throughput(world);
    reset_edge_latency(world);
    apply_exchange_point_latency(world);
    recompute_traffic_matrix_if_needed(world);
    accumulate_traffic_flows(world);
    record_utilization_history(world);
}

/// Maximum number of historical utilization snapshots to keep per entity.
const UTILIZATION_HISTORY_MAX: usize = 100;

/// Record current utilization values for all nodes and edges into history ring buffers.
fn record_utilization_history(world: &mut GameWorld) {
    // Record node utilization
    let node_utils: Vec<(u64, f64)> = world
        .infra_nodes
        .iter()
        .filter(|(id, _)| !world.constructions.contains_key(id))
        .map(|(&id, _)| {
            let util = world
                .capacities
                .get(&id)
                .map(|c| c.utilization())
                .unwrap_or(0.0);
            (id, util)
        })
        .collect();

    for (id, util) in node_utils {
        let history = world
            .utilization_history
            .entry(id)
            .or_default();
        if history.len() >= UTILIZATION_HISTORY_MAX {
            history.pop_front();
        }
        history.push_back(util);
    }

    // Record edge utilization
    let edge_utils: Vec<(u64, f64)> = world
        .infra_edges
        .iter()
        .map(|(&id, edge)| (id, edge.utilization()))
        .collect();

    for (id, util) in edge_utils {
        let history = world
            .utilization_history
            .entry(id)
            .or_default();
        if history.len() >= UTILIZATION_HISTORY_MAX {
            history.pop_front();
        }
        history.push_back(util);
    }
}

// ─── Capacity & Latency Reset ─────────────────────────────────────────────────

fn reset_node_effective_throughput(world: &mut GameWorld) {
    let updates: Vec<(u64, f64)> = {
        let mut v: Vec<_> = world
            .infra_nodes
            .iter()
            .filter(|(id, _)| !world.constructions.contains_key(id))
            .map(|(&id, node)| {
                let health = world.healths.get(&id).map(|h| h.condition).unwrap_or(1.0);
                let effective = if health < 0.5 {
                    node.max_throughput * health
                } else {
                    node.max_throughput
                };
                (id, effective)
            })
            .collect();
        v.sort_unstable_by_key(|t| t.0);
        v
    };

    for (node_id, effective_throughput) in updates {
        if let Some(cap) = world.capacities.get_mut(&node_id) {
            cap.max_throughput = effective_throughput;
        }
    }
}

fn reset_edge_latency(world: &mut GameWorld) {
    use gt_common::types::EdgeType;
    let updates: Vec<(u64, f64)> = {
        let mut v: Vec<_> = world
            .infra_edges
            .iter()
            .map(|(&id, edge)| {
                let latency_per_km = match edge.edge_type {
                    // Fiber-based: 0.005 ms/km
                    EdgeType::FiberLocal
                    | EdgeType::FiberRegional
                    | EdgeType::FiberNational
                    | EdgeType::Submarine
                    | EdgeType::SubseaFiberCable
                    | EdgeType::FeederFiber
                    | EdgeType::DistributionFiber
                    | EdgeType::DropCable => 0.005,
                    // Modern fiber with DWDM: slightly better
                    EdgeType::FiberMetro
                    | EdgeType::FiberLongHaul => 0.004,
                    EdgeType::DWDM_Backbone => 0.003,
                    // Copper: 0.02 ms/km
                    EdgeType::Copper
                    | EdgeType::CopperTrunkLine
                    | EdgeType::LongDistanceCopper => 0.02,
                    // Coaxial: 0.015
                    EdgeType::CoaxialCable => 0.015,
                    // Microwave: 0.003 ms/km
                    EdgeType::Microwave | EdgeType::MicrowaveLink => 0.003,
                    // Satellite: high latency
                    EdgeType::Satellite => 0.5,
                    EdgeType::EarlySatelliteLink => 0.6,
                    EdgeType::SatelliteLEOLink => 0.01,
                    // Telegraph: very slow
                    EdgeType::TelegraphWire => 0.1,
                    EdgeType::SubseaTelegraphCable => 0.2,
                    // Quantum: ultra-low latency
                    EdgeType::QuantumFiberLink => 0.001,
                    // Terahertz: ultra-low
                    EdgeType::TerahertzBeam => 0.001,
                    // Laser inter-satellite
                    EdgeType::LaserInterSatelliteLink => 0.0005,
                };
                (id, latency_per_km * edge.length_km)
            })
            .collect();
        v.sort_unstable_by_key(|t| t.0);
        v
    };
    for (edge_id, base_latency) in updates {
        if let Some(edge) = world.infra_edges.get_mut(&edge_id) {
            edge.latency_ms = base_latency;
        }
    }
}

fn apply_exchange_point_latency(world: &mut GameWorld) {
    let exchange_points: Vec<(u64, f64)> = {
        let mut v: Vec<_> = world
            .infra_nodes
            .iter()
            .filter(|(id, node)| {
                node.node_type == NodeType::ExchangePoint
                    && !world.constructions.contains_key(id)
            })
            .map(|(&id, node)| {
                let health = world.healths.get(&id).map(|h| h.condition).unwrap_or(1.0);
                let reduction = (node.max_throughput / 5000.0).min(1.0) * 0.3 * health;
                (id, reduction)
            })
            .collect();
        v.sort_unstable_by_key(|t| t.0);
        v
    };

    let mut reductions: Vec<(u64, f64)> = Vec::new();
    for (ep_id, reduction) in &exchange_points {
        let mut eids: Vec<u64> = world
            .infra_edges
            .iter()
            .filter(|(_, edge)| edge.source == *ep_id || edge.target == *ep_id)
            .map(|(&id, _)| id)
            .collect();
        eids.sort_unstable();
        for eid in eids {
            reductions.push((eid, *reduction));
        }
    }
    reductions.sort_unstable_by_key(|t| t.0);

    for (edge_id, reduction) in reductions {
        if let Some(edge) = world.infra_edges.get_mut(&edge_id) {
            edge.latency_ms *= 1.0 - reduction;
            edge.latency_ms = edge.latency_ms.max(0.1);
        }
    }
}

// ─── OD Traffic Matrix ────────────────────────────────────────────────────────

fn recompute_traffic_matrix_if_needed(world: &mut GameWorld) {
    let tick = world.current_tick();
    let needs_recompute = world.traffic_matrix.od_pairs.is_empty()
        || tick.saturating_sub(world.traffic_matrix.last_computed_tick) >= 50;

    if !needs_recompute {
        return;
    }

    let mut city_demands: Vec<(EntityId, f64)> = world
        .cities
        .iter()
        .map(|(&id, city)| {
            let demand = world
                .demands
                .get(&id)
                .map(|d| d.current_demand)
                .unwrap_or(city.telecom_demand);
            (id, demand)
        })
        .collect();
    city_demands.sort_unstable_by_key(|t| t.0);

    let total_demand: f64 = city_demands.iter().map(|(_, d)| d).sum();
    if total_demand <= 0.0 {
        world.traffic_matrix = TrafficMatrix {
            last_computed_tick: tick,
            ..TrafficMatrix::default()
        };
        return;
    }

    let mut od_pairs: Vec<TrafficDemand> = Vec::new();
    let mut external_traffic: Vec<(EntityId, f64)> = Vec::new();

    for i in 0..city_demands.len() {
        let (src_id, src_demand) = city_demands[i];
        external_traffic.push((src_id, src_demand * 0.4));

        for &(dst_id, dst_demand) in &city_demands[(i + 1)..] {
            let traffic = src_demand * dst_demand / total_demand;
            if traffic > 0.1 {
                od_pairs.push(TrafficDemand {
                    source_city: src_id,
                    dest_city: dst_id,
                    demand: traffic,
                });
            }
        }
    }

    world.traffic_matrix = TrafficMatrix {
        od_pairs,
        external_traffic,
        total_demand,
        last_computed_tick: tick,
        node_traffic: HashMap::new(),
        edge_traffic: HashMap::new(),
        total_served: 0.0,
        total_dropped: 0.0,
        corp_traffic_served: HashMap::new(),
        corp_traffic_dropped: HashMap::new(),
    };
}

// ─── Traffic Flow Accumulation ────────────────────────────────────────────────

/// Shared lookup data for traffic routing functions.
struct RoutingContext {
    city_access: HashMap<EntityId, (EntityId, EntityId)>,
    backbone_nodes: Vec<u64>,
    node_cap: HashMap<u64, f64>,
    edge_cap: HashMap<u64, f64>,
}

fn accumulate_traffic_flows(world: &mut GameWorld) {
    // Reset loads
    reset_all_loads(world);

    // Build lookup structures
    let ctx = RoutingContext {
        city_access: find_city_access_nodes(world),
        node_cap: collect_node_capacities(world),
        edge_cap: collect_edge_capacities(world),
        backbone_nodes: find_backbone_nodes(world),
    };

    let mut accum = TrafficAccumulator::new();

    // Route inter-city OD traffic
    let od_pairs = world.traffic_matrix.od_pairs.clone();
    for od in &od_pairs {
        route_od_pair(world, od, &ctx, &mut accum);
    }

    // Route external (internet-bound) traffic
    let external = world.traffic_matrix.external_traffic.clone();
    for &(city_id, ext_demand) in &external {
        route_external_traffic(world, city_id, ext_demand, &ctx, &mut accum);
    }

    // Apply accumulated loads to world state
    apply_loads(world, &accum);
}

/// Accumulated traffic state during flow computation.
struct TrafficAccumulator {
    node_load: HashMap<u64, f64>,
    edge_load: HashMap<u64, f64>,
    total_served: f64,
    total_dropped: f64,
    corp_served: HashMap<u64, f64>,
    corp_dropped: HashMap<u64, f64>,
}

impl TrafficAccumulator {
    fn new() -> Self {
        Self {
            node_load: HashMap::new(),
            edge_load: HashMap::new(),
            total_served: 0.0,
            total_dropped: 0.0,
            corp_served: HashMap::new(),
            corp_dropped: HashMap::new(),
        }
    }

    fn record_served(&mut self, amount: f64, owner: u64) {
        self.total_served += amount;
        *self.corp_served.entry(owner).or_insert(0.0) += amount;
    }

    fn record_dropped(&mut self, amount: f64, owner: Option<u64>) {
        self.total_dropped += amount;
        if let Some(o) = owner {
            *self.corp_dropped.entry(o).or_insert(0.0) += amount;
        }
    }
}

fn reset_all_loads(world: &mut GameWorld) {
    for cap in world.capacities.values_mut() {
        cap.current_load = 0.0;
    }
    for edge in world.infra_edges.values_mut() {
        edge.current_load = 0.0;
    }
}

fn collect_node_capacities(world: &GameWorld) -> HashMap<u64, f64> {
    world
        .infra_nodes
        .iter()
        .filter(|(id, _)| !world.constructions.contains_key(id))
        .map(|(&id, _)| {
            let cap = world.capacities.get(&id).map(|c| c.max_throughput).unwrap_or(0.0);
            (id, cap)
        })
        .collect()
}

fn collect_edge_capacities(world: &GameWorld) -> HashMap<u64, f64> {
    world
        .infra_edges
        .iter()
        .map(|(&id, edge)| (id, edge.effective_bandwidth()))
        .collect()
}

// ─── City → Access Node Mapping ───────────────────────────────────────────────

/// For each city, find the nearest operational node within coverage range.
/// Returns map: city_id → (node_id, owner_corp_id).
fn find_city_access_nodes(world: &GameWorld) -> HashMap<EntityId, (EntityId, EntityId)> {
    let mut result: HashMap<EntityId, (EntityId, EntityId)> = HashMap::new();

    let mut node_data: Vec<(u64, usize, NodeType, u64)> = world
        .infra_nodes
        .iter()
        .filter(|(id, _)| !world.constructions.contains_key(id))
        .map(|(&id, node)| (id, node.cell_index, node.node_type, node.owner))
        .collect();
    node_data.sort_unstable_by_key(|t| t.0);

    let cell_spacing = world.cell_spacing_km;

    for (&city_id, city) in &world.cities {
        let city_pos = match world.grid_cell_positions.get(city.cell_index) {
            Some(p) => *p,
            None => continue,
        };

        let mut best: Option<(u64, u64, f64)> = None;

        for &(nid, cell_index, node_type, owner) in &node_data {
            let node_pos = match world.grid_cell_positions.get(cell_index) {
                Some(p) => *p,
                None => continue,
            };

            let radius_km = scaled_coverage_radius(node_type, cell_spacing);
            let dist = haversine_km(city_pos.0, city_pos.1, node_pos.0, node_pos.1);
            if dist > radius_km {
                continue;
            }

            // Prefer access-tier nodes (lower tier penalty)
            let tier_penalty = node_type.tier().value() as f64 * 10.0;
            let effective_dist = dist + tier_penalty;

            if best.is_none() || effective_dist < best.unwrap().2 {
                best = Some((nid, owner, effective_dist));
            }
        }

        if let Some((nid, owner, _)) = best {
            result.insert(city_id, (nid, owner));
        }
    }

    result
}

fn find_backbone_nodes(world: &GameWorld) -> Vec<u64> {
    let mut nodes: Vec<u64> = world
        .infra_nodes
        .iter()
        .filter(|(id, _)| !world.constructions.contains_key(id))
        .filter(|(_, node)| {
            matches!(
                node.node_type.tier(),
                NetworkTier::Backbone | NetworkTier::Global | NetworkTier::Core
            )
        })
        .map(|(&id, _)| id)
        .collect();
    nodes.sort_unstable();
    nodes
}

// ─── Traffic Routing ──────────────────────────────────────────────────────────

fn route_od_pair(
    world: &GameWorld,
    od: &TrafficDemand,
    ctx: &RoutingContext,
    accum: &mut TrafficAccumulator,
) {
    let (src_node, src_owner) = match ctx.city_access.get(&od.source_city) {
        Some(&v) => v,
        None => {
            accum.record_dropped(od.demand, None);
            return;
        }
    };
    let (dst_node, _) = match ctx.city_access.get(&od.dest_city) {
        Some(&v) => v,
        None => {
            accum.record_dropped(od.demand, Some(src_owner));
            return;
        }
    };

    if src_node == dst_node {
        *accum.node_load.entry(src_node).or_insert(0.0) += od.demand;
        accum.record_served(od.demand, src_owner);
        return;
    }

    let path = world
        .network
        .get_cached_path(src_node, dst_node)
        .or_else(|| world.network.get_cached_path(dst_node, src_node))
        .cloned();

    match path {
        Some(ref p) if p.len() >= 2 => {
            let served = push_traffic_on_path(
                p, od.demand, &ctx.node_cap, &ctx.edge_cap,
                &mut accum.node_load, &mut accum.edge_load, &world.network,
            );
            accum.record_served(served, src_owner);
            if served < od.demand {
                accum.record_dropped(od.demand - served, Some(src_owner));
            }
        }
        _ => {
            accum.record_dropped(od.demand, Some(src_owner));
        }
    }
}

fn route_external_traffic(
    world: &GameWorld,
    city_id: EntityId,
    demand: f64,
    ctx: &RoutingContext,
    accum: &mut TrafficAccumulator,
) {
    let (access_node, owner) = match ctx.city_access.get(&city_id) {
        Some(&v) => v,
        None => {
            accum.record_dropped(demand, None);
            return;
        }
    };

    // Find shortest path to any backbone node
    let mut best_path: Option<Vec<u64>> = None;
    for &bb in &ctx.backbone_nodes {
        if let Some(path) = world.network.get_cached_path(access_node, bb) {
            if best_path.is_none() || path.len() < best_path.as_ref().unwrap().len() {
                best_path = Some(path.clone());
            }
        }
    }

    match best_path {
        Some(ref p) if p.len() >= 2 => {
            let served = push_traffic_on_path(
                p, demand, &ctx.node_cap, &ctx.edge_cap,
                &mut accum.node_load, &mut accum.edge_load, &world.network,
            );
            accum.record_served(served, owner);
            if served < demand {
                accum.record_dropped(demand - served, Some(owner));
            }
        }
        Some(_) => {
            // Access node is a backbone node
            *accum.node_load.entry(access_node).or_insert(0.0) += demand;
            accum.record_served(demand, owner);
        }
        None => {
            accum.record_dropped(demand, Some(owner));
        }
    }
}

/// Push traffic along a path, respecting capacity. Returns traffic actually served.
fn push_traffic_on_path(
    path: &[u64],
    demand: f64,
    node_cap: &HashMap<u64, f64>,
    edge_cap: &HashMap<u64, f64>,
    node_load: &mut HashMap<u64, f64>,
    edge_load: &mut HashMap<u64, f64>,
    network: &gt_infrastructure::NetworkGraph,
) -> f64 {
    // Find bottleneck
    let mut min_remaining = demand;

    for &nid in path {
        let cap = node_cap.get(&nid).copied().unwrap_or(0.0);
        let current = node_load.get(&nid).copied().unwrap_or(0.0);
        let remaining = (cap * 1.2 - current).max(0.0);
        min_remaining = min_remaining.min(remaining);
    }

    for i in 0..path.len() - 1 {
        if let Some(eid) = network.get_edge_id(path[i], path[i + 1]) {
            let cap = edge_cap.get(&eid).copied().unwrap_or(0.0);
            let current = edge_load.get(&eid).copied().unwrap_or(0.0);
            let remaining = (cap * 1.2 - current).max(0.0);
            min_remaining = min_remaining.min(remaining);
        }
    }

    let served = min_remaining.min(demand).max(0.0);
    if served <= 0.0 {
        return 0.0;
    }

    // Apply load
    for &nid in path {
        *node_load.entry(nid).or_insert(0.0) += served;
    }
    for i in 0..path.len() - 1 {
        if let Some(eid) = network.get_edge_id(path[i], path[i + 1]) {
            *edge_load.entry(eid).or_insert(0.0) += served;
        }
    }

    served
}

// ─── Apply Results ────────────────────────────────────────────────────────────

fn apply_loads(world: &mut GameWorld, accum: &TrafficAccumulator) {
    for (&nid, &load) in &accum.node_load {
        if let Some(cap) = world.capacities.get_mut(&nid) {
            cap.current_load = load;
        }
        if let Some(node) = world.infra_nodes.get_mut(&nid) {
            node.current_load = world.capacities.get(&nid).map(|c| c.current_load).unwrap_or(0.0);
        }
    }

    for (&eid, &load) in &accum.edge_load {
        if let Some(edge) = world.infra_edges.get_mut(&eid) {
            edge.current_load = load;
        }
    }

    world.traffic_matrix.node_traffic = accum.node_load.clone();
    world.traffic_matrix.edge_traffic = accum.edge_load.clone();
    world.traffic_matrix.total_served = accum.total_served;
    world.traffic_matrix.total_dropped = accum.total_dropped;
    world.traffic_matrix.corp_traffic_served = accum.corp_served.clone();
    world.traffic_matrix.corp_traffic_dropped = accum.corp_dropped.clone();
}

// ─── Utilities ────────────────────────────────────────────────────────────────

fn scaled_coverage_radius(node_type: NodeType, cell_spacing: f64) -> f64 {
    let base_radius = node_type.coverage_radius_km();
    let min_cells = match node_type {
        // Wireless access nodes — wide coverage needed
        NodeType::CellTower | NodeType::MacroCell => 2.5,
        NodeType::WirelessRelay | NodeType::MeshDroneRelay => 1.5,
        // Wired access with some coverage
        NodeType::CentralOffice
        | NodeType::ManualExchange
        | NodeType::AutomaticExchange
        | NodeType::DigitalSwitch
        | NodeType::ISPGateway
        | NodeType::FiberPOP
        | NodeType::CoaxHub
        | NodeType::ContentDeliveryNode => 1.5,
        // Satellite — very wide
        NodeType::SatelliteGround
        | NodeType::SatelliteGroundStation
        | NodeType::LEO_SatelliteGateway => 6.0,
        // Core/backbone — small footprint
        NodeType::DataCenter
        | NodeType::BackboneRouter
        | NodeType::EarlyDataCenter
        | NodeType::ColocationFacility
        | NodeType::EdgeDataCenter
        | NodeType::HyperscaleDataCenter
        | NodeType::CloudOnRamp
        | NodeType::DWDM_Terminal
        | NodeType::NeuromorphicEdgeNode
        | NodeType::InternetExchangePoint => 1.0,
        // Exchange/aggregation
        NodeType::ExchangePoint
        | NodeType::MicrowaveTower
        | NodeType::LongDistanceRelay
        | NodeType::QuantumRepeater => 1.0,
        // Small access nodes
        NodeType::SmallCell
        | NodeType::TerahertzRelay
        | NodeType::TelephonePole
        | NodeType::NetworkAccessPoint
        | NodeType::FiberDistributionHub => 0.8,
        // Passive/landing points
        NodeType::SubmarineLanding
        | NodeType::SubseaLandingStation
        | NodeType::CableHut
        | NodeType::UnderwaterDataCenter
        | NodeType::FiberSplicePoint => 0.5,
        // Telegraph
        NodeType::TelegraphOffice => 1.5,
        NodeType::TelegraphRelay => 0.8,
    };
    base_radius.max(cell_spacing * min_cells)
}

fn haversine_km(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let dlat = (lat1 - lat2).to_radians();
    let dlon = (lon1 - lon2).to_radians();
    let a = (dlat / 2.0).sin().powi(2)
        + lat1.to_radians().cos() * lat2.to_radians().cos() * (dlon / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().asin();
    6371.0 * c
}
