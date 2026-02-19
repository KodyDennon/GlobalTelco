use crate::world::GameWorld;
use gt_common::types::NodeType;

pub fn run(world: &mut GameWorld) {
    let tick = world.current_tick();

    let mut corp_ids: Vec<u64> = world.corporations.keys().copied().collect();
    corp_ids.sort_unstable();

    for &corp_id in &corp_ids {
        let corp_nodes = world
            .corp_infra_nodes
            .get(&corp_id)
            .cloned()
            .unwrap_or_default();

        let mut total_revenue: i64 = 0;

        for &node_id in &corp_nodes {
            // Skip nodes under construction
            if world.constructions.contains_key(&node_id) {
                continue;
            }

            let utilization = world
                .capacities
                .get(&node_id)
                .map(|c| c.utilization())
                .unwrap_or(0.0);

            let health_factor = world
                .healths
                .get(&node_id)
                .map(|h| h.condition)
                .unwrap_or(1.0);

            let node_revenue = world
                .infra_nodes
                .get(&node_id)
                .map(|n| calculate_node_revenue(n.node_type, n.max_throughput, utilization, health_factor))
                .unwrap_or(0);

            total_revenue += node_revenue;
        }

        // Revenue from edges (transit/backbone revenue for fiber and submarine cables)
        // Collect and sort by edge ID for deterministic summation
        let mut edge_revs: Vec<(u64, i64)> = world
            .infra_edges
            .iter()
            .filter(|(_, e)| e.owner == corp_id)
            .map(|(&eid, edge)| {
                let util = edge.utilization();
                let health = 1.0; // Edges don't have health component currently
                (eid, calculate_edge_revenue(edge.edge_type, edge.bandwidth, util, health))
            })
            .collect();
        edge_revs.sort_unstable_by_key(|t| t.0);
        let edge_revenue: i64 = edge_revs.iter().map(|t| t.1).sum();

        total_revenue += edge_revenue;

        // Revenue from active contracts (where this corp provides service)
        // Sort by contract key for deterministic summation
        let mut contract_revs: Vec<(u64, i64)> = world
            .contracts
            .iter()
            .filter(|(_, c)| c.from == corp_id && c.status == crate::components::ContractStatus::Active)
            .map(|(&cid, c)| (cid, c.price_per_tick))
            .collect();
        contract_revs.sort_unstable_by_key(|t| t.0);
        let contract_revenue: i64 = contract_revs.iter().map(|t| t.1).sum();

        total_revenue += contract_revenue;

        // Coverage bonus: additional revenue per population covered
        // Corporations that cover more population cells earn subscription revenue
        let coverage_revenue = calculate_coverage_revenue(world, corp_id);
        total_revenue += coverage_revenue;

        if let Some(fin) = world.financials.get_mut(&corp_id) {
            fin.revenue_per_tick = total_revenue;
            fin.cash += total_revenue;
        }

        if total_revenue > 0 {
            world.event_queue.push(
                tick,
                gt_common::events::GameEvent::RevenueEarned {
                    corporation: corp_id,
                    amount: total_revenue,
                },
            );
        }
    }
}

/// Calculate revenue for a single node based on its type, capacity, utilization, and health.
/// Different node types generate revenue through different mechanisms.
fn calculate_node_revenue(
    node_type: NodeType,
    max_throughput: f64,
    utilization: f64,
    health: f64,
) -> i64 {
    // Revenue rate per unit of throughput served
    let (base_rate, revenue_model) = match node_type {
        // CellTowers: subscriber revenue, scales well with utilization
        NodeType::CellTower => (25.0, RevenueModel::Subscriber),
        // WirelessRelay: lower subscriber revenue, relay traffic
        NodeType::WirelessRelay => (15.0, RevenueModel::Subscriber),
        // CentralOffice: business connections, steady enterprise revenue
        NodeType::CentralOffice => (30.0, RevenueModel::Enterprise),
        // ExchangePoint: peering fees, revenue from interconnection
        NodeType::ExchangePoint => (8.0, RevenueModel::Peering),
        // DataCenter: hosting/colocation, high per-unit revenue
        NodeType::DataCenter => (5.0, RevenueModel::Hosting),
        // SatelliteGround: premium satellite service
        NodeType::SatelliteGround => (40.0, RevenueModel::Premium),
        // SubmarineLanding: wholesale transit, huge volume low margin
        NodeType::SubmarineLanding => (3.0, RevenueModel::Transit),
    };

    let effective_throughput = max_throughput * utilization * health;

    match revenue_model {
        // Subscriber: linear with utilization, bonus for high utilization
        RevenueModel::Subscriber => {
            let bonus = if utilization > 0.7 { 1.2 } else { 1.0 };
            (effective_throughput * base_rate * bonus) as i64
        }
        // Enterprise: steadier, less dependent on peak utilization
        RevenueModel::Enterprise => {
            let stability = 0.3 + utilization * 0.7; // Minimum 30% revenue even at low util
            (max_throughput * stability * base_rate * health) as i64
        }
        // Peering: revenue from number of connections (edges), not just throughput
        RevenueModel::Peering => {
            (effective_throughput * base_rate * 1.5) as i64
        }
        // Hosting: flat-rate + utilization bonus
        RevenueModel::Hosting => {
            let flat_component = max_throughput * base_rate * 0.3 * health;
            let usage_component = effective_throughput * base_rate * 0.7;
            (flat_component + usage_component) as i64
        }
        // Premium: high margin, full utilization dependence
        RevenueModel::Premium => {
            (effective_throughput * base_rate) as i64
        }
        // Transit: thin margin, massive volume
        RevenueModel::Transit => {
            (effective_throughput * base_rate) as i64
        }
    }
}

/// Calculate revenue from edge infrastructure (backbone/transit).
fn calculate_edge_revenue(
    edge_type: gt_common::types::EdgeType,
    bandwidth: f64,
    utilization: f64,
    health: f64,
) -> i64 {
    use gt_common::types::EdgeType;

    let rate = match edge_type {
        EdgeType::Submarine => 2.0,       // Huge bandwidth, low per-unit rate
        EdgeType::FiberNational => 1.5,
        EdgeType::FiberRegional => 1.0,
        EdgeType::FiberLocal => 0.5,
        EdgeType::Microwave => 0.8,
        EdgeType::Satellite => 3.0,       // Premium per-unit
        EdgeType::Copper => 0.3,          // Legacy, low margin
    };

    (bandwidth * utilization * rate * health) as i64
}

/// Calculate subscription revenue from population coverage.
/// Each population unit in a covered cell generates a small per-tick subscription fee.
fn calculate_coverage_revenue(world: &GameWorld, corp_id: u64) -> i64 {
    let mut covered_population: u64 = 0;

    // Find cells where this corporation is the dominant owner
    // Sort by cell_idx for deterministic iteration
    let mut coverage_cells: Vec<usize> = world
        .cell_coverage
        .keys()
        .copied()
        .collect();
    coverage_cells.sort_unstable();

    for cell_idx in coverage_cells {
        let coverage = match world.cell_coverage.get(&cell_idx) {
            Some(c) => c,
            None => continue,
        };
        if coverage.dominant_owner == Some(corp_id) && coverage.bandwidth > 0.0 {
            // Check if this cell belongs to a city (population center)
            if let Some(&city_id) = world.cell_to_city.get(&cell_idx) {
                if let Some(city) = world.cities.get(&city_id) {
                    // Population per cell = city population / number of cells
                    let cell_pop = city.population / city.cells.len().max(1) as u64;
                    // Scale by satisfaction (happy customers pay more)
                    let satisfaction = city.infrastructure_satisfaction;
                    covered_population += (cell_pop as f64 * satisfaction) as u64;
                }
            }
        }
    }

    // Revenue per population unit per tick (small but adds up)
    (covered_population as f64 * 0.1) as i64
}

enum RevenueModel {
    Subscriber,
    Enterprise,
    Peering,
    Hosting,
    Premium,
    Transit,
}
