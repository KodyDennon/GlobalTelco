use crate::world::GameWorld;

/// Per-cell coverage data calculated each tick.
#[derive(Debug, Clone, Default)]
pub struct CellCoverage {
    /// Total signal strength at this cell (sum of all covering nodes' contributions)
    pub signal_strength: f64,
    /// Total bandwidth capacity available to this cell
    pub bandwidth: f64,
    /// Number of nodes covering this cell
    pub node_count: u32,
    /// Best single-node signal (for quality indicator)
    pub best_signal: f64,
    /// Owner of the strongest signal source (for ownership overlay)
    pub dominant_owner: Option<u64>,
    /// Per-corporation bandwidth at this cell. Used by the revenue system to
    /// detect market competition and split subscriber revenue proportionally.
    /// Sorted by corp_id for deterministic iteration.
    pub per_corp_bandwidth: Vec<(u64, f64)>,
}

impl CellCoverage {
    /// Add bandwidth contribution from a specific corporation.
    /// Merges into existing entry if present, otherwise appends and re-sorts.
    pub fn add_corp_bandwidth(&mut self, corp_id: u64, bandwidth: f64) {
        if bandwidth <= 0.0 {
            return;
        }
        if let Some(entry) = self.per_corp_bandwidth.iter_mut().find(|(c, _)| *c == corp_id) {
            entry.1 += bandwidth;
        } else {
            self.per_corp_bandwidth.push((corp_id, bandwidth));
            self.per_corp_bandwidth.sort_unstable_by_key(|t| t.0);
        }
    }

    /// Returns the number of distinct corporations providing coverage at this cell.
    pub fn competitor_count(&self) -> usize {
        self.per_corp_bandwidth.len()
    }

    /// Returns the bandwidth share (0.0-1.0) that a specific corporation holds at this cell.
    /// If the corp has no coverage, returns 0.0.
    pub fn corp_bandwidth_share(&self, corp_id: u64) -> f64 {
        let total: f64 = self.per_corp_bandwidth.iter().map(|(_, bw)| *bw).sum();
        if total <= 0.0 {
            return 0.0;
        }
        self.per_corp_bandwidth
            .iter()
            .find(|(c, _)| *c == corp_id)
            .map(|(_, bw)| *bw / total)
            .unwrap_or(0.0)
    }
}

/// Run coverage calculation: determine which cells are covered by which infrastructure.
///
/// Coverage model:
/// - Each operational infrastructure node provides coverage to cells within its radius
/// - Signal strength attenuates with distance: strength = capacity * (1 - dist/radius)^2
/// - Terrain modifies coverage (mountains reduce, flat terrain is neutral)
/// - Wireless nodes (CellTower, WirelessRelay, SatelliteGround) cover a radius
/// - Wired nodes (CentralOffice, etc.) cover only their own cell plus edge-connected nodes' cells
/// - Connected network: nodes connected via edges extend wired coverage along the backbone
pub fn run(world: &mut GameWorld) {
    let cell_count = world.grid_cell_count;
    if cell_count == 0 {
        return;
    }

    // Clear previous coverage
    world.cell_coverage.clear();

    // Backhaul check: a node only provides coverage if it has a network path to a backbone.
    // Backbone nodes = CentralOffice, ExchangePoint, DataCenter, SubmarineLanding.
    // CellTowers and WirelessRelays need to be connected via edges to a backbone node.
    // SatelliteGround nodes are self-backhaul (satellite uplink).
    use std::collections::HashSet;
    let backbone_types: HashSet<gt_common::types::NodeType> = [
        gt_common::types::NodeType::CentralOffice,
        gt_common::types::NodeType::ExchangePoint,
        gt_common::types::NodeType::DataCenter,
        gt_common::types::NodeType::SubmarineLanding,
        gt_common::types::NodeType::BackboneRouter,
    ]
    .into_iter()
    .collect();

    // Find all operational backbone nodes
    let mut backhauled_nodes: HashSet<u64> = HashSet::new();
    for (&id, node) in &world.infra_nodes {
        if world.constructions.contains_key(&id) {
            continue;
        }
        if backbone_types.contains(&node.node_type)
            || node.node_type == gt_common::types::NodeType::SatelliteGround
        {
            // Backbone nodes and satellite ground stations are always backhauled
            backhauled_nodes.insert(id);
        }
    }

    // Flood-fill: any node connected (via edges) to a backbone node is backhauled
    let mut changed = true;
    while changed {
        changed = false;
        for edge in world.infra_edges.values() {
            let src_ok = backhauled_nodes.contains(&edge.source);
            let dst_ok = backhauled_nodes.contains(&edge.target);
            if src_ok && !dst_ok {
                // Check target is operational
                if world.infra_nodes.contains_key(&edge.target)
                    && !world.constructions.contains_key(&edge.target)
                {
                    backhauled_nodes.insert(edge.target);
                    changed = true;
                }
            } else if dst_ok && !src_ok
                && world.infra_nodes.contains_key(&edge.source)
                    && !world.constructions.contains_key(&edge.source)
                {
                    backhauled_nodes.insert(edge.source);
                    changed = true;
                }
        }
    }

    // Calculate per-node backhaul bandwidth: the max bandwidth of edges connected to each node.
    // A node's effective coverage is min(its own throughput, its backhaul bandwidth).
    // Backbone nodes (CO, EP, DC) aren't limited — they ARE the backbone.
    let mut node_backhaul_bw: std::collections::HashMap<u64, f64> =
        std::collections::HashMap::new();
    for edge in world.infra_edges.values() {
        let bw = edge.bandwidth;
        node_backhaul_bw
            .entry(edge.source)
            .and_modify(|existing| *existing = existing.max(bw))
            .or_insert(bw);
        node_backhaul_bw
            .entry(edge.target)
            .and_modify(|existing| *existing = existing.max(bw))
            .or_insert(bw);
    }

    // Collect operational node data — only nodes with backhaul provide coverage
    // SORT by node_id for deterministic coverage accumulation order
    let mut nodes: Vec<(u64, usize, gt_common::types::NodeType, f64, u64, f64)> = world
        .infra_nodes
        .iter()
        .filter(|(id, _)| !world.constructions.contains_key(id))
        .filter(|(id, _)| backhauled_nodes.contains(id))
        .map(|(&id, node)| {
            let health = world.healths.get(&id).map(|h| h.condition).unwrap_or(1.0);
            let raw_throughput = world
                .capacities
                .get(&id)
                .map(|c| c.max_throughput)
                .unwrap_or(node.max_throughput);
            // Limit throughput by backhaul bandwidth for non-backbone nodes.
            // Backbone nodes (CO/EP/DC) aren't bottlenecked by their own edges.
            let effective_throughput = if backbone_types.contains(&node.node_type) {
                raw_throughput
            } else {
                let backhaul = node_backhaul_bw.get(&id).copied().unwrap_or(0.0);
                raw_throughput.min(backhaul.max(raw_throughput * 0.1)) // At least 10% base capacity
            };
            (
                id,
                node.cell_index,
                node.node_type,
                effective_throughput,
                node.owner,
                health,
            )
        })
        .collect();
    nodes.sort_unstable_by_key(|t| t.0);

    if nodes.is_empty() {
        return;
    }

    // Scale coverage radius to be meaningful at this grid resolution.
    // On small maps cells are ~500km apart but CellTower "real" radius is 15km.
    // We ensure each node covers at least a few grid cells so the game is playable.
    let cell_spacing = world.cell_spacing_km;

    // For each node, calculate coverage to nearby cells
    for &(_node_id, node_cell, node_type, throughput, owner, health) in &nodes {
        let base_radius_km = node_type.coverage_radius_km();
        let coverage_fraction = node_type.coverage_capacity_fraction();
        let coverage_capacity = throughput * coverage_fraction * health;

        if coverage_capacity <= 0.0 || base_radius_km <= 0.0 {
            continue;
        }

        // Scale radius: ensure minimum coverage of ~2-3 grid cells for gameplay
        let min_cells = match node_type {
            // Wireless access nodes
            gt_common::types::NodeType::CellTower
            | gt_common::types::NodeType::MacroCell => 2.5,
            gt_common::types::NodeType::WirelessRelay
            | gt_common::types::NodeType::MeshDroneRelay => 1.5,
            // Wired access with some coverage
            gt_common::types::NodeType::CentralOffice
            | gt_common::types::NodeType::ManualExchange
            | gt_common::types::NodeType::AutomaticExchange
            | gt_common::types::NodeType::DigitalSwitch
            | gt_common::types::NodeType::ISPGateway
            | gt_common::types::NodeType::FiberPOP
            | gt_common::types::NodeType::CoaxHub
            | gt_common::types::NodeType::ContentDeliveryNode
            | gt_common::types::NodeType::TelegraphOffice => 1.5,
            // Satellite — very wide
            gt_common::types::NodeType::SatelliteGround
            | gt_common::types::NodeType::SatelliteGroundStation
            | gt_common::types::NodeType::LEO_SatelliteGateway => 6.0,
            // Core/backbone nodes
            gt_common::types::NodeType::DataCenter
            | gt_common::types::NodeType::BackboneRouter
            | gt_common::types::NodeType::EarlyDataCenter
            | gt_common::types::NodeType::ColocationFacility
            | gt_common::types::NodeType::EdgeDataCenter
            | gt_common::types::NodeType::HyperscaleDataCenter
            | gt_common::types::NodeType::CloudOnRamp
            | gt_common::types::NodeType::DWDM_Terminal
            | gt_common::types::NodeType::NeuromorphicEdgeNode
            | gt_common::types::NodeType::InternetExchangePoint => 1.0,
            // Exchange/aggregation
            gt_common::types::NodeType::ExchangePoint
            | gt_common::types::NodeType::MicrowaveTower
            | gt_common::types::NodeType::LongDistanceRelay
            | gt_common::types::NodeType::QuantumRepeater => 1.0,
            // Small access
            gt_common::types::NodeType::SmallCell
            | gt_common::types::NodeType::TerahertzRelay
            | gt_common::types::NodeType::TelephonePole
            | gt_common::types::NodeType::NetworkAccessPoint
            | gt_common::types::NodeType::FiberDistributionHub
            | gt_common::types::NodeType::TelegraphRelay => 0.8,
            // Passive/landing
            gt_common::types::NodeType::SubmarineLanding
            | gt_common::types::NodeType::SubseaLandingStation
            | gt_common::types::NodeType::CableHut
            | gt_common::types::NodeType::UnderwaterDataCenter
            | gt_common::types::NodeType::FiberSplicePoint => 0.5,
            // Satellite nodes — coverage handled by orbital system
            gt_common::types::NodeType::LEO_Satellite
            | gt_common::types::NodeType::MEO_Satellite
            | gt_common::types::NodeType::GEO_Satellite
            | gt_common::types::NodeType::HEO_Satellite => 10.0,
            // Satellite ground stations
            gt_common::types::NodeType::LEO_GroundStation
            | gt_common::types::NodeType::MEO_GroundStation => 4.0,
            // Satellite infrastructure (no coverage)
            gt_common::types::NodeType::SatelliteFactory
            | gt_common::types::NodeType::TerminalFactory
            | gt_common::types::NodeType::SatelliteWarehouse
            | gt_common::types::NodeType::LaunchPad => 0.0,
        };
        let radius_km = base_radius_km.max(cell_spacing * min_cells);

        // Convert radius from km to approximate degrees for the spatial search
        // 1 degree latitude ≈ 111 km, longitude varies by cos(lat)
        let node_pos = match world.grid_cell_positions.get(node_cell) {
            Some(p) => *p,
            None => continue,
        };
        let (node_lat, node_lon) = node_pos;
        let lat_range = radius_km / 111.0;
        let cos_lat = (node_lat.to_radians()).cos().max(0.1);
        let lon_range = radius_km / (111.0 * cos_lat);

        // Scan cells within bounding box
        for (cell_idx, &(cell_lat, cell_lon)) in world.grid_cell_positions.iter().enumerate() {
            // Quick bounding box check
            if (cell_lat - node_lat).abs() > lat_range || (cell_lon - node_lon).abs() > lon_range {
                continue;
            }

            // Calculate actual distance using haversine
            let dist_km = haversine_km(node_lat, node_lon, cell_lat, cell_lon);
            if dist_km > radius_km {
                continue;
            }

            // Signal attenuation: inverse square falloff
            let distance_ratio = dist_km / radius_km;
            let attenuation = (1.0 - distance_ratio).powi(2);
            let signal = coverage_capacity * attenuation;

            if signal < 0.01 {
                continue;
            }

            // Apply terrain modifier for wireless coverage
            let terrain_mod = if node_type.is_wireless() {
                world
                    .land_parcels
                    .values()
                    .find(|p| p.cell_index == cell_idx)
                    .map(|p| terrain_coverage_modifier(p.terrain))
                    .unwrap_or(1.0)
            } else {
                1.0
            };

            let final_signal = signal * terrain_mod;
            let final_bandwidth = coverage_capacity * attenuation * terrain_mod;

            let entry = world
                .cell_coverage
                .entry(cell_idx)
                .or_default();
            entry.signal_strength += final_signal;
            entry.bandwidth += final_bandwidth;
            entry.node_count += 1;
            entry.add_corp_bandwidth(owner, final_bandwidth);
            if final_signal > entry.best_signal {
                entry.best_signal = final_signal;
                entry.dominant_owner = Some(owner);
            }
        }
    }

    // Inter-city connectivity: fiber edges bridge coverage between their endpoints.
    // When tower A (city X) is connected to tower B (city Y), the edge:
    // 1. Adds backbone bandwidth to cells at both endpoints (25% of edge BW)
    // 2. Adds intermediate coverage along the edge path (interpolated cells)
    // This simulates traffic flowing between connected cities.
    let mut sorted_edge_ids: Vec<u64> = world.infra_edges.keys().copied().collect();
    sorted_edge_ids.sort_unstable();
    for edge_id in sorted_edge_ids {
        let edge = match world.infra_edges.get(&edge_id) {
            Some(e) => e,
            None => continue,
        };
        // Only edges between backhauled nodes contribute
        if !backhauled_nodes.contains(&edge.source) || !backhauled_nodes.contains(&edge.target) {
            continue;
        }
        let src_node = world.infra_nodes.get(&edge.source);
        let dst_node = world.infra_nodes.get(&edge.target);
        let (src_ci, dst_ci) = match (src_node, dst_node) {
            (Some(s), Some(d)) => (s.cell_index, d.cell_index),
            _ => continue,
        };
        let owner = edge.owner;
        let backbone_bw = edge.bandwidth * 0.25; // 25% of edge bandwidth serves local coverage

        // Add coverage at both endpoint cells
        for &ci in &[src_ci, dst_ci] {
            let entry = world
                .cell_coverage
                .entry(ci)
                .or_default();
            entry.bandwidth += backbone_bw;
            entry.signal_strength += backbone_bw * 0.5;
            entry.add_corp_bandwidth(owner, backbone_bw);
            if entry.dominant_owner.is_none() {
                entry.dominant_owner = Some(owner);
            }
        }

        // Also add coverage to cells along the edge path (interpolated).
        // This simulates wired infrastructure covering the corridor between two cities.
        let src_pos = world.grid_cell_positions.get(src_ci);
        let dst_pos = world.grid_cell_positions.get(dst_ci);
        if let (Some(&(slat, slon)), Some(&(dlat, dlon))) = (src_pos, dst_pos) {
            // Sample a few points along the edge and cover nearby cells
            let steps = 3;
            let corridor_bw = backbone_bw * 0.3; // 30% of backbone BW along corridor
            for step in 1..steps {
                let t = step as f64 / steps as f64;
                let mid_lat = slat + (dlat - slat) * t;
                let mid_lon = slon + (dlon - slon) * t;
                // Find nearest cell to this interpolated point
                let nearest = world.grid_cell_positions.iter().enumerate().min_by(
                    |(_, &(a_lat, a_lon)), (_, &(b_lat, b_lon))| {
                        let da = (a_lat - mid_lat).powi(2) + (a_lon - mid_lon).powi(2);
                        let db = (b_lat - mid_lat).powi(2) + (b_lon - mid_lon).powi(2);
                        da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
                    },
                );
                if let Some((ci, _)) = nearest {
                    if ci != src_ci && ci != dst_ci {
                        let entry = world
                            .cell_coverage
                            .entry(ci)
                            .or_default();
                        entry.bandwidth += corridor_bw;
                        entry.signal_strength += corridor_bw * 0.3;
                        entry.add_corp_bandwidth(owner, corridor_bw);
                        if entry.dominant_owner.is_none() {
                            entry.dominant_owner = Some(owner);
                        }
                    }
                }
            }
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

/// Terrain modifies wireless signal propagation.
fn terrain_coverage_modifier(terrain: gt_common::types::TerrainType) -> f64 {
    use gt_common::types::TerrainType;
    match terrain {
        TerrainType::Urban => 0.7,        // buildings attenuate
        TerrainType::Suburban => 0.85,    // some attenuation
        TerrainType::Rural => 1.0,        // clear, best propagation
        TerrainType::Mountainous => 0.4,  // mountains block signal
        TerrainType::Desert => 0.95,      // flat, minor dust
        TerrainType::Coastal => 0.9,      // good propagation
        TerrainType::OceanShallow => 0.3,  // poor over water
        TerrainType::OceanDeep => 0.2,     // very poor
        TerrainType::OceanTrench => 0.1,   // extreme depth, negligible
        TerrainType::Tundra => 0.85,       // flat, cold
        TerrainType::Frozen => 0.6,        // ice interference
    }
}
