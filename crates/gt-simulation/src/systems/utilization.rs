//! Traffic Flow Engine — OD-matrix based utilization system.
//!
//! Replaces the old proximity-based utilization with actual traffic routing:
//! 1. Cities generate traffic demand (OD matrix)
//! 2. Traffic is routed through the network graph via cached shortest paths
//! 3. Node/edge loads accumulate from actual traffic flowing through them
//! 4. Congestion occurs when links exceed capacity; excess traffic is dropped

use crate::components::ContractStatus;
use crate::world::GameWorld;
use gt_common::types::{
    EntityId, NetworkTier, NodeType, PathAttribution, TrafficDemand, TrafficMatrix,
    TransitPermission,
};
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
        .filter(|(id, _)| !world.constructions.contains_key(*id))
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
            .filter(|(id, _)| !world.constructions.contains_key(*id))
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
                    // Dynamic satellite edges
                    EdgeType::SatelliteDownlink => 0.01,
                    EdgeType::IntraplaneISL => 0.002,
                    EdgeType::CrossplaneISL => 0.003,
                    EdgeType::GEO_GroundLink => 0.3,
                    EdgeType::MEO_GroundLink => 0.05,
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
    // Build transit permissions to check for active peering contracts
    let transit_perms = build_transit_permissions(world);

    // Collect all exchange point and IXP nodes
    let exchange_points: Vec<(u64, f64, bool)> = {
        let mut v: Vec<_> = world
            .infra_nodes
            .iter()
            .filter(|(id, node)| {
                (node.node_type == NodeType::ExchangePoint
                    || node.node_type == NodeType::InternetExchangePoint)
                    && !world.constructions.contains_key(*id)
            })
            .map(|(&id, node)| {
                let health = world.healths.get(&id).map(|h| h.condition).unwrap_or(1.0);
                let is_ixp = node.node_type == NodeType::InternetExchangePoint;
                // IXPs provide 40% base reduction vs ExchangePoints' 30%
                let base_reduction = if is_ixp { 0.4 } else { 0.3 };
                let reduction = (node.max_throughput / 5000.0).min(1.0) * base_reduction * health;
                (id, reduction, is_ixp)
            })
            .collect();
        v.sort_unstable_by_key(|t| t.0);
        v
    };

    // Collect which corps are connected to each exchange point
    let mut ep_connected_corps: std::collections::HashMap<u64, Vec<u64>> =
        std::collections::HashMap::new();
    for &(ep_id, _, _) in &exchange_points {
        let ep_owner = world
            .infra_nodes
            .get(&ep_id)
            .map(|n| n.owner)
            .unwrap_or(0);
        let mut connected: Vec<u64> = vec![ep_owner];

        // Find all corps that have edges connecting to this EP
        for edge in world.infra_edges.values() {
            let other_node = if edge.source == ep_id {
                edge.target
            } else if edge.target == ep_id {
                edge.source
            } else {
                continue;
            };
            if let Some(other) = world.infra_nodes.get(&other_node) {
                if !connected.contains(&other.owner) {
                    connected.push(other.owner);
                }
            }
        }
        connected.sort_unstable();
        connected.dedup();
        ep_connected_corps.insert(ep_id, connected);
    }

    let mut reductions: Vec<(u64, f64)> = Vec::new();
    for &(ep_id, base_reduction, _is_ixp) in &exchange_points {
        let connected = ep_connected_corps.get(&ep_id).cloned().unwrap_or_default();

        let mut eids: Vec<u64> = world
            .infra_edges
            .iter()
            .filter(|(_, edge)| edge.source == ep_id || edge.target == ep_id)
            .map(|(&id, _)| id)
            .collect();
        eids.sort_unstable();

        for eid in eids {
            let edge = match world.infra_edges.get(&eid) {
                Some(e) => e,
                None => continue,
            };

            // Determine the two corps involved in this edge
            let other_node = if edge.source == ep_id {
                edge.target
            } else {
                edge.source
            };
            let ep_owner = world
                .infra_nodes
                .get(&ep_id)
                .map(|n| n.owner)
                .unwrap_or(0);
            let other_owner = world
                .infra_nodes
                .get(&other_node)
                .map(|n| n.owner)
                .unwrap_or(0);

            let mut reduction = base_reduction;

            // Peering bonus: if both corps connected to this EP have a peering contract,
            // apply full reduction. Otherwise reduce the bonus.
            if ep_owner != other_owner && ep_owner != 0 && other_owner != 0 {
                let both_connected = connected.contains(&ep_owner)
                    && connected.contains(&other_owner);
                let (perm, _) =
                    lookup_permission(&transit_perms, ep_owner, other_owner);
                let has_peering = matches!(
                    perm,
                    gt_common::types::TransitPermission::PeeringContract
                );

                if both_connected && has_peering {
                    // Full reduction — this is the ideal peering scenario
                    // (already set to base_reduction)
                } else {
                    // Without a peering contract at this EP, reduced benefit
                    reduction *= 0.5;
                }
            }

            reductions.push((eid, reduction));
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

// ─── Transit Permission Cache ─────────────────────────────────────────────────

/// Lookup cache: (corp_a, corp_b) → permission level (ordered so a < b).
/// Also maps contract_id for Transit contracts to enable per-contract traffic tracking.
pub type TransitPermissionCache = HashMap<(EntityId, EntityId), (TransitPermission, Option<EntityId>)>;

/// Build a lookup of transit permissions between every pair of corporations.
/// Scans active contracts, alliances, and co-ownership to determine what traffic
/// is allowed across corporate boundaries.
pub fn build_transit_permissions(world: &GameWorld) -> TransitPermissionCache {
    let mut cache = TransitPermissionCache::new();

    // 1. Active contracts (Peering and Transit)
    for (&contract_id, contract) in &world.contracts {
        if contract.status != ContractStatus::Active {
            continue;
        }
        let (a, b) = ordered_pair(contract.from, contract.to);
        match contract.contract_type {
            crate::components::ContractType::Peering => {
                // Peering is settlement-free, prefer it over alliance
                cache.insert((a, b), (TransitPermission::PeeringContract, Some(contract_id)));
            }
            crate::components::ContractType::Transit => {
                // Transit: paid, price_per_unit derived from contract
                let price_per_unit = if contract.capacity > 0.0 {
                    contract.price_per_tick as f64 / contract.capacity
                } else {
                    0.0
                };
                // Only upgrade: Transit is better than Alliance but worse than Peering
                let existing = cache.get(&(a, b));
                let dominated = matches!(
                    existing,
                    Some((TransitPermission::PeeringContract, _))
                );
                if !dominated {
                    cache.insert(
                        (a, b),
                        (TransitPermission::TransitContract { price_per_unit }, Some(contract_id)),
                    );
                }
            }
            crate::components::ContractType::SLA => {
                // SLA contracts also enable transit (they're premium transit)
                let price_per_unit = if contract.capacity > 0.0 {
                    contract.price_per_tick as f64 / contract.capacity
                } else {
                    0.0
                };
                let existing = cache.get(&(a, b));
                let dominated = matches!(
                    existing,
                    Some((TransitPermission::PeeringContract, _))
                        | Some((TransitPermission::TransitContract { .. }, _))
                );
                if !dominated {
                    cache.insert(
                        (a, b),
                        (TransitPermission::TransitContract { price_per_unit }, Some(contract_id)),
                    );
                }
            }
        }
    }

    // 2. Alliances (only if no better contract exists)
    for alliance in world.alliances.values() {
        let members = &alliance.member_corp_ids;
        for i in 0..members.len() {
            for j in (i + 1)..members.len() {
                let (a, b) = ordered_pair(members[i], members[j]);
                cache.entry((a, b)).or_insert((
                    TransitPermission::Alliance {
                        revenue_share_pct: alliance.revenue_share_pct,
                    },
                    None,
                ));
            }
        }
    }

    // 3. Co-ownership: if node A is co-owned by corps X and Y, they get free transit
    for ownership in world.ownerships.values() {
        let primary = ownership.owner;
        for &(co_owner, _share) in &ownership.co_owners {
            let (a, b) = ordered_pair(primary, co_owner);
            // Co-owned is best — override everything except own network
            cache.insert((a, b), (TransitPermission::CoOwned, None));
        }
        // Also handle co-owner pairs
        for i in 0..ownership.co_owners.len() {
            for j in (i + 1)..ownership.co_owners.len() {
                let (a, b) = ordered_pair(ownership.co_owners[i].0, ownership.co_owners[j].0);
                cache.insert((a, b), (TransitPermission::CoOwned, None));
            }
        }
    }

    cache
}

/// Order a pair of entity IDs so the smaller one is first.
pub fn ordered_pair(a: EntityId, b: EntityId) -> (EntityId, EntityId) {
    if a <= b { (a, b) } else { (b, a) }
}

/// Look up transit permission between two corporations.
pub fn lookup_permission(
    cache: &TransitPermissionCache,
    owner_a: EntityId,
    owner_b: EntityId,
) -> (TransitPermission, Option<EntityId>) {
    if owner_a == owner_b {
        return (TransitPermission::OwnNetwork, None);
    }
    let (a, b) = ordered_pair(owner_a, owner_b);
    cache
        .get(&(a, b))
        .copied()
        .unwrap_or((TransitPermission::Blocked, None))
}

// ─── OD Traffic Matrix ────────────────────────────────────────────────────────

fn recompute_traffic_matrix_if_needed(world: &mut GameWorld) {
    let tick = world.current_tick();
    // Recompute matrix every 100 ticks or if empty
    let needs_recompute = world.traffic_matrix.od_pairs.is_empty()
        || tick.saturating_sub(world.traffic_matrix.last_computed_tick) >= 100;

    if !needs_recompute {
        return;
    }

    // Identify Global Hubs (Population > 500k)
    let hub_ids: Vec<EntityId> = world.cities.iter()
        .filter(|(_, c)| c.population > 500_000)
        .map(|(&id, _)| id)
        .collect();

    let mut od_pairs: Vec<TrafficDemand> = Vec::new();
    let mut external_traffic: Vec<(EntityId, f64)> = Vec::new();
    let mut total_demand = 0.0;

    // Build regional mapping for fast neighbor lookups
    let mut region_cities: HashMap<EntityId, Vec<EntityId>> = HashMap::new();
    for (&id, city) in &world.cities {
        region_cities.entry(city.region_id).or_default().push(id);
    }

    for (&src_id, src_city) in &world.cities {
        let src_demand = world.demands.get(&src_id).map(|d| d.current_demand).unwrap_or(src_city.telecom_demand);
        if src_demand <= 0.0 { continue; }
        
        // 40% of traffic is "Internet-bound" (external)
        external_traffic.push((src_id, src_demand * 0.4));
        total_demand += src_demand;

        // Peer-to-peer traffic targets (Gravity Model)
        // D_ij = G * (P_i * P_j) / dist^2
        
        // 1. Regional targets (all cities in same region)
        if let Some(neighbors) = region_cities.get(&src_city.region_id) {
            for &dst_id in neighbors {
                if src_id == dst_id { continue; }
                let dst_city = &world.cities[&dst_id];
                let dst_demand = world.demands.get(&dst_id).map(|d| d.current_demand).unwrap_or(dst_city.telecom_demand);
                
                // Simplified gravity: shared demand factor
                let traffic = (src_demand * dst_demand * 0.3) / total_demand.max(1.0);
                if traffic > 0.05 {
                    od_pairs.push(TrafficDemand { source_city: src_id, dest_city: dst_id, demand: traffic });
                }
            }
        }

        // 2. Global Hub targets (long-distance high-fidelity traffic)
        for &hub_id in &hub_ids {
            if src_id == hub_id || src_city.region_id == world.cities[&hub_id].region_id { continue; }
            let hub_city = &world.cities[&hub_id];
            let hub_demand = world.demands.get(&hub_id).map(|d| d.current_demand).unwrap_or(hub_city.telecom_demand);
            
            let traffic = (src_demand * hub_demand * 0.2) / total_demand.max(1.0);
            if traffic > 0.1 {
                od_pairs.push(TrafficDemand { source_city: src_id, dest_city: hub_id, demand: traffic });
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
        contract_traffic: HashMap::new(),
        path_attribution: Vec::new(),
    };
}

// ─── Traffic Flow Accumulation ────────────────────────────────────────────────

/// Shared lookup data for traffic routing functions.
struct RoutingContext {
    city_access: HashMap<EntityId, (EntityId, EntityId)>,
    backbone_nodes: Vec<u64>,
    node_cap: HashMap<u64, f64>,
    edge_cap: HashMap<u64, f64>,
    /// Node ID → owner corp ID for ownership boundary detection
    node_owners: HashMap<u64, u64>,
    /// Transit permission cache for cross-corp routing
    transit_permissions: TransitPermissionCache,
    /// Set of entity IDs currently under construction
    constructions: std::collections::HashSet<EntityId>,
}

fn accumulate_traffic_flows(world: &mut GameWorld) {
    // Reset loads
    reset_all_loads(world);

    // Build transit permission cache before routing
    let transit_permissions = build_transit_permissions(world);

    // Build node ownership map
    let node_owners: HashMap<u64, u64> = world
        .infra_nodes
        .iter()
        .filter(|(id, _)| !world.constructions.contains_key(*id))
        .map(|(&id, node)| (id, node.owner))
        .collect();

    // Build lookup structures
    let ctx = RoutingContext {
        city_access: find_city_access_nodes(world),
        node_cap: collect_node_capacities(world),
        edge_cap: collect_edge_capacities(world),
        backbone_nodes: find_backbone_nodes(world),
        node_owners,
        transit_permissions,
        constructions: world.constructions.keys().copied().collect(),
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
    /// Per-contract traffic flow (contract_id → traffic units routed through it)
    contract_traffic: HashMap<u64, f64>,
    /// Per-path attribution for alliance revenue splitting.
    path_attribution: Vec<PathAttribution>,
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
            contract_traffic: HashMap::new(),
            path_attribution: Vec::new(),
        }
    }

    fn record_served(&mut self, amount: f64, owner: u64) {
        self.total_served += amount;
        *self.corp_served.entry(owner).or_insert(0.0) += amount;
    }

    fn record_path_attribution(&mut self, src: EntityId, dst: EntityId, traffic: f64, corp_hops: HashMap<u64, u32>) {
        self.path_attribution.push(PathAttribution {
            source_city: src,
            dest_city: dst,
            traffic,
            corp_hops,
        });
    }

    fn record_dropped(&mut self, amount: f64, owner: Option<u64>) {
        self.total_dropped += amount;
        if let Some(o) = owner {
            *self.corp_dropped.entry(o).or_insert(0.0) += amount;
        }
    }

    fn record_contract_traffic(&mut self, contract_id: u64, amount: f64) {
        *self.contract_traffic.entry(contract_id).or_insert(0.0) += amount;
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
        .filter(|(id, _)| !world.constructions.contains_key(*id))
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
    let cell_spacing = world.cell_spacing_km;

    for (&city_id, city) in &world.cities {
        let city_pos = match world.grid_cell_positions.get(city.cell_index) {
            Some(p) => *p,
            None => continue,
        };

        // Optimization: Use spatial index to find nearby candidate nodes
        // Max coverage radius is ~6.0 * cell_spacing (SatelliteGround)
        let search_radius_km = cell_spacing * 7.0;
        let deg_range = search_radius_km / 111.0;
        let envelope = rstar::AABB::from_corners(
            [city_pos.1 - deg_range * 2.0, city_pos.0 - deg_range],
            [city_pos.1 + deg_range * 2.0, city_pos.0 + deg_range]
        );

        let mut best: Option<(u64, u64, f64)> = None;

        for spatial_node in world.spatial_index.locate_in_envelope(&envelope) {
            let nid = spatial_node.id;
            
            // Skip nodes under construction
            if world.constructions.contains_key(&nid) {
                continue;
            }

            if let Some(node) = world.infra_nodes.get(&nid) {
                let radius_km = scaled_coverage_radius(node.node_type, cell_spacing);
                let dist = haversine_km(city_pos.0, city_pos.1, spatial_node.pos[1], spatial_node.pos[0]);
                
                if dist <= radius_km {
                    // Prefer access-tier nodes (lower tier penalty)
                    let tier_penalty = node.node_type.tier().value() as f64 * 10.0;
                    let effective_dist = dist + tier_penalty;

                    if best.map_or(true, |b: (u64, u64, f64)| effective_dist < b.2) {
                        best = Some((nid, node.owner, effective_dist));
                    }
                }
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
        .filter(|(id, _)| !world.constructions.contains_key(*id))
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
    world: &mut GameWorld,
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

    // Optimization: Lazy compute and cache paths
    let path = world.network.get_or_compute_path(src_node, dst_node, &ctx.edge_cap);

    match path {
        Some(ref p) if p.len() >= 2 => {
            let (served, corp_hops) = push_traffic_on_path(p, od.demand, ctx, accum, &world.network);
            accum.record_served(served, src_owner);
            if served > 0.0 {
                accum.record_path_attribution(od.source_city, od.dest_city, served, corp_hops);
            }
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
    world: &mut GameWorld,
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

    // External traffic goes to nearest backbone node
    let mut best_path: Option<Vec<u64>> = None;

    for &bb in &ctx.backbone_nodes {
        if let Some(path) = world.network.get_or_compute_path(access_node, bb, &ctx.edge_cap) {
            if best_path.as_ref().map_or(true, |bp| path.len() < bp.len()) {
                best_path = Some(path);
            }
        }
    }

    match best_path {
        Some(ref p) if p.len() >= 2 => {
            let (served, corp_hops) = push_traffic_on_path(p, demand, ctx, accum, &world.network);
            accum.record_served(served, owner);
            if served > 0.0 {
                accum.record_path_attribution(city_id, 0, served, corp_hops); // 0 = external
            }
            if served < demand {
                accum.record_dropped(demand - served, Some(owner));
            }
        }
        Some(_) => {
            // Access node is a backbone node
            *accum.node_load.entry(access_node).or_insert(0.0) += demand;
            accum.record_served(demand, owner);
            
            let mut corp_hops = HashMap::new();
            corp_hops.insert(owner, 1);
            accum.record_path_attribution(city_id, 0, demand, corp_hops);
        }
        None => {
            accum.record_dropped(demand, Some(owner));
        }
    }
}

/// Push traffic along a path, respecting capacity and tracking cross-corp contract usage.
/// Returns (traffic actually served, corp_hops).
fn push_traffic_on_path(
    path: &[u64],
    demand: f64,
    ctx: &RoutingContext,
    accum: &mut TrafficAccumulator,
    network: &gt_infrastructure::NetworkGraph,
) -> (f64, HashMap<u64, u32>) {
    // Find bottleneck
    let mut min_remaining = demand;
    let mut corp_hops = HashMap::new();

    // Single pass to find bottleneck and track hops
    for i in 0..path.len() {
        let nid = path[i];
        
        // Node capacity check
        let cap = ctx.node_cap.get(&nid).copied().unwrap_or(0.0);
        let current = accum.node_load.get(&nid).copied().unwrap_or(0.0);
        let remaining = (cap * 1.2 - current).max(0.0);
        min_remaining = min_remaining.min(remaining);

        // Track hops per corporation
        if let Some(&owner) = ctx.node_owners.get(&nid) {
            *corp_hops.entry(owner).or_insert(0) += 1;
        }

        // Edge capacity check (for all but last node)
        if i < path.len() - 1 {
            if let Some(eid) = network.get_edge_id(nid, path[i + 1]) {
                // Block traffic if edge is under construction
                if ctx.constructions.contains(&eid) {
                    return (0.0, corp_hops);
                }
                let cap = ctx.edge_cap.get(&eid).copied().unwrap_or(0.0);
                let current = accum.edge_load.get(&eid).copied().unwrap_or(0.0);
                let remaining = (cap * 1.2 - current).max(0.0);
                min_remaining = min_remaining.min(remaining);
            }
        }

        if min_remaining <= 0.0 { break; }
    }

    let served = min_remaining.min(demand).max(0.0);
    if served <= 0.0 {
        return (0.0, corp_hops);
    }

    // Apply load and track cross-corp boundary crossings
    for i in 0..path.len() {
        let nid = path[i];
        *accum.node_load.entry(nid).or_insert(0.0) += served;

        if i < path.len() - 1 {
            if let Some(eid) = network.get_edge_id(nid, path[i + 1]) {
                *accum.edge_load.entry(eid).or_insert(0.0) += served;
            }

            // Track per-contract traffic when crossing corporate boundaries
            let owner_a = ctx.node_owners.get(&nid).copied().unwrap_or(0);
            let owner_b = ctx.node_owners.get(&path[i + 1]).copied().unwrap_or(0);
            if owner_a != owner_b && owner_a != 0 && owner_b != 0 {
                let (_, contract_id) = lookup_permission(&ctx.transit_permissions, owner_a, owner_b);
                if let Some(cid) = contract_id {
                    accum.record_contract_traffic(cid, served);
                }
            }
        }
    }

    (served, corp_hops)
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
    world.traffic_matrix.contract_traffic = accum.contract_traffic.clone();
    world.traffic_matrix.path_attribution = accum.path_attribution.clone();
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
        // Satellite nodes — orbital coverage
        NodeType::LEO_Satellite
        | NodeType::MEO_Satellite
        | NodeType::GEO_Satellite
        | NodeType::HEO_Satellite => 10.0,
        // Satellite ground stations
        NodeType::LEO_GroundStation
        | NodeType::MEO_GroundStation => 4.0,
        // Satellite infrastructure (no coverage)
        NodeType::SatelliteFactory
        | NodeType::TerminalFactory
        | NodeType::SatelliteWarehouse
        | NodeType::LaunchPad
        | NodeType::Building => 0.0,
    };
    base_radius.max(cell_spacing * min_cells)
}

use gt_common::geo::haversine_km;
