//! Infrastructure list, node/edge queries, cell coverage, roads, and traffic.

use gt_common::types::EntityId;
use gt_simulation::world::GameWorld;

pub fn query_terrain_at(world: &GameWorld, lon: f64, lat: f64) -> String {
    let cell_idx = world.nearest_cell_latlon(lat, lon);
    let terrain = world.get_cell_terrain(cell_idx).unwrap_or(gt_common::types::TerrainType::Rural);
    serde_json::to_string(&terrain).unwrap_or_default()
}

pub fn query_node_metadata(world: &GameWorld, id: EntityId) -> String {
    let node = match world.infra_nodes.get(&id) {
        Some(n) => n,
        None => return String::new(),
    };
    let pos = world.positions.get(&id);
    let health = world.healths.get(&id);
    let cap = world.capacities.get(&id);
    let under_construction = world.constructions.contains_key(&id);

    let data = serde_json::json!({
        "id": id,
        "node_type": node.node_type,
        "x": pos.map(|p| p.x).unwrap_or(0.0),
        "y": pos.map(|p| p.y).unwrap_or(0.0),
        "health": health.map(|h| h.condition).unwrap_or(1.0),
        "utilization": cap.map(|c| c.utilization()).unwrap_or(0.0),
        "under_construction": under_construction,
        "owner": node.owner,
        "cell_index": node.cell_index,
    });
    serde_json::to_string(&data).unwrap_or_default()
}

pub fn query_edge_metadata(world: &GameWorld, id: EntityId) -> String {
    let e = match world.infra_edges.get(&id) {
        Some(e) => e,
        None => return String::new(),
    };
    let src_pos = world.positions.get(&e.source);
    let dst_pos = world.positions.get(&e.target);

    let data = serde_json::json!({
        "id": id,
        "edge_type": e.edge_type,
        "source": e.source,
        "target": e.target,
        "src_x": src_pos.map(|p| p.x).unwrap_or(0.0),
        "src_y": src_pos.map(|p| p.y).unwrap_or(0.0),
        "dst_x": dst_pos.map(|p| p.x).unwrap_or(0.0),
        "dst_y": dst_pos.map(|p| p.y).unwrap_or(0.0),
        "health": e.health,
        "owner": e.owner,
    });
    serde_json::to_string(&data).unwrap_or_default()
}

pub fn query_nodes_metadata(world: &GameWorld, ids: &[EntityId]) -> String {
    let nodes: Vec<serde_json::Value> = ids.iter().filter_map(|&id| {
        let node = world.infra_nodes.get(&id)?;
        let pos = world.positions.get(&id)?;
        Some(serde_json::json!({
            "id": id,
            "node_type": node.node_type,
            "x": pos.x,
            "y": pos.y,
            "owner": node.owner,
        }))
    }).collect();
    serde_json::to_string(&nodes).unwrap_or_default()
}

pub fn query_infrastructure_list(world: &GameWorld, corp_id: EntityId) -> String {
    let node_ids = world
        .corp_infra_nodes
        .get(&corp_id)
        .cloned()
        .unwrap_or_default();

    let nodes: Vec<serde_json::Value> = node_ids
        .iter()
        .filter_map(|&id| {
            let node = world.infra_nodes.get(&id)?;
            let pos = world.positions.get(&id);
            let health = world.healths.get(&id);
            let cap = world.capacities.get(&id);
            let under_construction = world.constructions.contains_key(&id);
            let util_history: Vec<f64> = world
                .utilization_history
                .get(&id)
                .map(|h| h.iter().copied().collect())
                .unwrap_or_default();
            Some(serde_json::json!({
                "id": id,
                "node_type": node.node_type,
                "network_level": node.network_level,
                "max_throughput": node.max_throughput,
                "current_load": node.current_load,
                "latency_ms": node.latency_ms,
                "reliability": node.reliability,
                "construction_cost": node.construction_cost,
                "maintenance_cost": node.maintenance_cost,
                "cell_index": node.cell_index,
                "x": pos.map(|p| p.x).unwrap_or(0.0),
                "y": pos.map(|p| p.y).unwrap_or(0.0),
                "health": health.map(|h| h.condition).unwrap_or(1.0),
                "utilization": cap.map(|c| c.utilization()).unwrap_or(0.0),
                "under_construction": under_construction,
                "repairing": node.repairing,
                "repair_ticks_left": node.repair_ticks_left,
                "repair_health_per_tick": node.repair_health_per_tick,
                "revenue_generated": node.revenue_generated,
                "utilization_history": util_history,
                "insured": node.insured,
                "maintenance_priority": world.maintenance_priorities.get(&id).map(|m| m.tier).unwrap_or_default(),
                "auto_repair": world.maintenance_priorities.get(&id).map(|m| m.auto_repair).unwrap_or(true),
            }))
        })
        .collect();

    let edges: Vec<serde_json::Value> = world
        .infra_edges
        .iter()
        .filter(|(_, e)| e.owner == corp_id)
        .map(|(&id, e)| {
            let src_pos = world.positions.get(&e.source);
            let dst_pos = world.positions.get(&e.target);
            let src_cell = world
                .infra_nodes
                .get(&e.source)
                .map(|n| n.cell_index)
                .unwrap_or(0);
            let dst_cell = world
                .infra_nodes
                .get(&e.target)
                .map(|n| n.cell_index)
                .unwrap_or(0);
            let util_history: Vec<f64> = world
                .utilization_history
                .get(&id)
                .map(|h| h.iter().copied().collect())
                .unwrap_or_default();
            serde_json::json!({
                "id": id,
                "edge_type": e.edge_type,
                "source": e.source,
                "target": e.target,
                "bandwidth": e.bandwidth,
                "current_load": e.current_load,
                "latency_ms": e.latency_ms,
                "length_km": e.length_km,
                "health": e.health,
                "utilization": e.utilization(),
                "src_x": src_pos.map(|p| p.x).unwrap_or(0.0),
                "src_y": src_pos.map(|p| p.y).unwrap_or(0.0),
                "dst_x": dst_pos.map(|p| p.x).unwrap_or(0.0),
                "dst_y": dst_pos.map(|p| p.y).unwrap_or(0.0),
                "src_cell": src_cell,
                "dst_cell": dst_cell,
                "waypoints": e.waypoints.iter().map(|&(lon, lat)| [lon, lat]).collect::<Vec<_>>(),
                "deployment": e.deployment,
                "maintenance_cost": e.maintenance_cost,
                "repairing": e.repairing,
                "repair_ticks_left": e.repair_ticks_left,
                "repair_health_per_tick": e.repair_health_per_tick,
                "revenue_generated": e.revenue_generated,
                "utilization_history": util_history,
            })
        })
        .collect();

    serde_json::json!({ "nodes": nodes, "edges": edges }).to_string()
}

pub fn query_visible_entities(
    world: &GameWorld,
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
) -> String {
    let nodes: Vec<serde_json::Value> = world
        .infra_nodes
        .iter()
        .filter_map(|(&id, node)| {
            let pos = world.positions.get(&id)?;
            if pos.x >= min_x && pos.x <= max_x && pos.y >= min_y && pos.y <= max_y {
                let health = world.healths.get(&id);
                let cap = world.capacities.get(&id);
                Some(serde_json::json!({
                    "id": id,
                    "type": "node",
                    "node_type": node.node_type,
                    "owner": node.owner,
                    "x": pos.x,
                    "y": pos.y,
                    "health": health.map(|h| h.condition).unwrap_or(1.0),
                    "utilization": cap.map(|c| c.utilization()).unwrap_or(0.0),
                    "under_construction": world.constructions.contains_key(&id),
                }))
            } else {
                None
            }
        })
        .collect();

    let cities: Vec<serde_json::Value> = world
        .cities
        .iter()
        .filter_map(|(&id, city)| {
            let pos = world.positions.get(&id)?;
            if pos.x >= min_x && pos.x <= max_x && pos.y >= min_y && pos.y <= max_y {
                Some(serde_json::json!({
                    "id": id,
                    "type": "city",
                    "name": city.name,
                    "population": city.population,
                    "x": pos.x,
                    "y": pos.y,
                }))
            } else {
                None
            }
        })
        .collect();

    serde_json::json!({ "nodes": nodes, "cities": cities }).to_string()
}

pub fn query_parcels_in_view(
    world: &GameWorld,
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
) -> String {
    let parcels: Vec<serde_json::Value> = world
        .land_parcels
        .iter()
        .filter_map(|(&id, parcel)| {
            let pos = world.positions.get(&id)?;
            if pos.x >= min_x && pos.x <= max_x && pos.y >= min_y && pos.y <= max_y {
                Some(serde_json::json!({
                    "id": id,
                    "cell_index": parcel.cell_index,
                    "terrain": parcel.terrain,
                    "elevation": parcel.elevation,
                    "zoning": parcel.zoning,
                    "cost_modifier": parcel.cost_modifier,
                    "x": pos.x,
                    "y": pos.y,
                }))
            } else {
                None
            }
        })
        .collect();
    serde_json::to_string(&parcels).unwrap_or_default()
}

pub fn query_cell_coverage(world: &GameWorld) -> String {
    let coverage: Vec<serde_json::Value> = world
        .cell_coverage
        .iter()
        .filter_map(|(&cell_idx, cov)| {
            let (lat, lon) = world.grid_cell_positions.get(cell_idx)?;
            Some(serde_json::json!({
                "cell_index": cell_idx,
                "lat": lat,
                "lon": lon,
                "signal_strength": cov.signal_strength,
                "bandwidth": cov.bandwidth,
                "node_count": cov.node_count,
                "best_signal": cov.best_signal,
                "dominant_owner": cov.dominant_owner,
            }))
        })
        .collect();
    serde_json::to_string(&coverage).unwrap_or_default()
}

pub fn query_all_infrastructure(world: &GameWorld) -> String {
    let nodes: Vec<serde_json::Value> = world
        .infra_nodes
        .iter()
        .filter_map(|(&id, node)| {
            let pos = world.positions.get(&id)?;
            let health = world.healths.get(&id);
            let cap = world.capacities.get(&id);
            let under_construction = world.constructions.contains_key(&id);
            let owner_name = world
                .corporations
                .get(&node.owner)
                .map(|c| c.name.as_str())
                .unwrap_or("Unknown");
            Some(serde_json::json!({
                "id": id,
                "node_type": node.node_type,
                "network_level": node.network_level,
                "max_throughput": node.max_throughput,
                "current_load": node.current_load,
                "latency_ms": node.latency_ms,
                "reliability": node.reliability,
                "cell_index": node.cell_index,
                "owner": node.owner,
                "owner_name": owner_name,
                "x": pos.x,
                "y": pos.y,
                "health": health.map(|h| h.condition).unwrap_or(1.0),
                "utilization": cap.map(|c| c.utilization()).unwrap_or(0.0),
                "under_construction": under_construction,
                "active_ftth": node.active_ftth,
            }))
        })
        .collect();

    let edges: Vec<serde_json::Value> = world
        .infra_edges
        .iter()
        .map(|(&id, e)| {
            let src_pos = world.positions.get(&e.source);
            let dst_pos = world.positions.get(&e.target);
            let owner_name = world
                .corporations
                .get(&e.owner)
                .map(|c| c.name.as_str())
                .unwrap_or("Unknown");
            let src_cell = world
                .infra_nodes
                .get(&e.source)
                .map(|n| n.cell_index)
                .unwrap_or(0);
            let dst_cell = world
                .infra_nodes
                .get(&e.target)
                .map(|n| n.cell_index)
                .unwrap_or(0);
            serde_json::json!({
                "id": id,
                "edge_type": e.edge_type,
                "source": e.source,
                "target": e.target,
                "bandwidth": e.bandwidth,
                "current_load": e.current_load,
                "latency_ms": e.latency_ms,
                "length_km": e.length_km,
                "health": e.health,
                "utilization": e.utilization(),
                "owner": e.owner,
                "owner_name": owner_name,
                "src_x": src_pos.map(|p| p.x).unwrap_or(0.0),
                "src_y": src_pos.map(|p| p.y).unwrap_or(0.0),
                "dst_x": dst_pos.map(|p| p.x).unwrap_or(0.0),
                "dst_y": dst_pos.map(|p| p.y).unwrap_or(0.0),
                "src_cell": src_cell,
                "dst_cell": dst_cell,
                "waypoints": e.waypoints.iter().map(|&(lon, lat)| [lon, lat]).collect::<Vec<_>>(),
                "deployment": e.deployment,
            })
        })
        .collect();

    serde_json::json!({ "nodes": nodes, "edges": edges }).to_string()
}

pub fn query_grid_cells(world: &GameWorld) -> String {
    let mut cell_terrain: std::collections::HashMap<usize, gt_common::types::TerrainType> =
        std::collections::HashMap::new();
    for parcel in world.land_parcels.values() {
        cell_terrain.insert(parcel.cell_index, parcel.terrain);
    }

    let cells: Vec<serde_json::Value> = world
        .grid_cell_positions
        .iter()
        .enumerate()
        .map(|(i, &(lat, lon))| {
            let terrain = cell_terrain
                .get(&i)
                .copied()
                .unwrap_or(gt_common::types::TerrainType::OceanShallow);
            let neighbors = world
                .grid_cell_neighbors
                .get(i)
                .cloned()
                .unwrap_or_default();
            serde_json::json!({
                "index": i,
                "lat": lat,
                "lon": lon,
                "terrain": terrain,
                "neighbors": neighbors,
            })
        })
        .collect();
    serde_json::to_string(&cells).unwrap_or_default()
}

pub fn query_world_geojson(world: &GameWorld) -> String {
    let mut features: Vec<serde_json::Value> = Vec::new();

    for (&id, region) in &world.regions {
        if region.boundary_polygon.is_empty() {
            continue;
        }
        let coords: Vec<Vec<f64>> = region
            .boundary_polygon
            .iter()
            .map(|&(lat, lon)| vec![lon, lat])
            .collect();
        let mut ring = coords;
        if let Some(first) = ring.first().cloned() {
            ring.push(first);
        }

        features.push(serde_json::json!({
            "type": "Feature",
            "geometry": {
                "type": "Polygon",
                "coordinates": [ring],
            },
            "properties": {
                "id": id,
                "name": region.name,
                "type": "region",
                "population": region.population,
                "gdp": region.gdp,
                "development": region.development,
            },
        }));
    }

    for (&id, city) in &world.cities {
        if let Some(pos) = world.positions.get(&id) {
            features.push(serde_json::json!({
                "type": "Feature",
                "geometry": {
                    "type": "Point",
                    "coordinates": [pos.x, pos.y],
                },
                "properties": {
                    "id": id,
                    "name": city.name,
                    "type": "city",
                    "population": city.population,
                    "development": city.development,
                },
            }));
        }
    }

    serde_json::json!({
        "type": "FeatureCollection",
        "features": features,
    })
    .to_string()
}

pub fn query_buildable_nodes(world: &GameWorld, lon: f64, lat: f64) -> String {
    use gt_common::types::NodeType;

    let player_id = world.player_corp_id().unwrap_or(0);
    let fin = world.financials.get(&player_id);
    let cash = fin.map(|f| f.cash).unwrap_or(0);

    let terrain_mult = world
        .find_nearest_cell(lon, lat)
        .and_then(|(cell_idx, _)| {
            world
                .cell_to_parcel
                .get(&cell_idx)
                .and_then(|&pid| world.land_parcels.get(&pid))
                .map(|p| p.cost_modifier)
        })
        .unwrap_or(1.0);

    let options: Vec<serde_json::Value> = NodeType::ALL
        .iter()
        .map(|nt| {
            let base_cost = nt.construction_cost();
            let cost = (base_cost as f64 * terrain_mult) as i64;
            let build_ticks = (base_cost / 10_000).max(5);
            serde_json::json!({
                "label": nt.display_name(),
                "node_type": nt,
                "network_level": nt.tier(),
                "tier": nt.tier().value(),
                "era": nt.era().display_name(),
                "cost": cost,
                "build_ticks": build_ticks,
                "affordable": cash >= cost,
            })
        })
        .collect();

    serde_json::to_string(&options).unwrap_or_default()
}

pub fn query_buildable_edges(world: &GameWorld, source_id: EntityId) -> String {
    use gt_common::types::EdgeType;

    let player_id = world.player_corp_id().unwrap_or(0);
    let fin = world.financials.get(&player_id);
    let cash = fin.map(|f| f.cash).unwrap_or(0);

    let player_nodes = world
        .corp_infra_nodes
        .get(&player_id)
        .cloned()
        .unwrap_or_default();

    let targets: Vec<serde_json::Value> = player_nodes
        .iter()
        .filter(|&&nid| nid != source_id && !world.constructions.contains_key(&nid))
        .filter_map(|&nid| {
            let node = world.infra_nodes.get(&nid)?;
            let pos = world.positions.get(&nid)?;
            let src_pos = world.positions.get(&source_id)?;

            let dlat = (src_pos.y - pos.y).to_radians();
            let dlon = (src_pos.x - pos.x).to_radians();
            let lat1 = src_pos.y.to_radians();
            let lat2 = pos.y.to_radians();
            let a = (dlat / 2.0).sin().powi(2)
                + lat1.cos() * lat2.cos() * (dlon / 2.0).sin().powi(2);
            let c = 2.0 * a.sqrt().asin();
            let dist_km = 6371.0 * c;

            let min_cost = EdgeType::ALL
                .iter()
                .map(|et| {
                    let cpk = et.cost_per_km();
                    if cpk == 0 {
                        5_000_000i64
                    } else {
                        (cpk as f64 * dist_km) as i64
                    }
                })
                .min()
                .unwrap_or(0);

            Some(serde_json::json!({
                "target_id": nid,
                "target_type": node.node_type,
                "x": pos.x,
                "y": pos.y,
                "distance_km": dist_km,
                "cost": min_cost,
                "affordable": cash >= min_cost,
            }))
        })
        .collect();

    serde_json::to_string(&targets).unwrap_or_default()
}

pub fn query_damaged_nodes(world: &GameWorld, corp_id: EntityId) -> String {
    let node_ids = world
        .corp_infra_nodes
        .get(&corp_id)
        .cloned()
        .unwrap_or_default();

    let damaged: Vec<serde_json::Value> = node_ids
        .iter()
        .filter_map(|&id| {
            let node = world.infra_nodes.get(&id)?;
            let health = world.healths.get(&id)?;
            if health.condition >= 0.95 {
                return None;
            }
            let pos = world.positions.get(&id);
            let base_cost = node.construction_cost;
            let damage = 1.0 - health.condition;
            let repair_cost = (base_cost as f64 * damage * 0.2) as i64;
            let emergency_cost = (base_cost as f64 * damage * 0.6) as i64;
            Some(serde_json::json!({
                "id": id,
                "node_type": node.node_type,
                "health": health.condition,
                "repair_cost": repair_cost,
                "emergency_cost": emergency_cost,
                "x": pos.map(|p| p.x).unwrap_or(0.0),
                "y": pos.map(|p| p.y).unwrap_or(0.0),
            }))
        })
        .collect();

    serde_json::to_string(&damaged).unwrap_or_default()
}

pub fn query_traffic_flows(world: &GameWorld) -> String {
    let tm = &world.traffic_matrix;

    let edge_flows: Vec<serde_json::Value> = world
        .infra_edges
        .iter()
        .map(|(&id, e)| {
            let traffic = tm.edge_traffic.get(&id).copied().unwrap_or(0.0);
            let utilization = if e.bandwidth > 0.0 {
                traffic / e.bandwidth
            } else {
                0.0
            };
            let src_pos = world.positions.get(&e.source);
            let dst_pos = world.positions.get(&e.target);
            serde_json::json!({
                "id": id,
                "traffic": traffic,
                "bandwidth": e.bandwidth,
                "utilization": utilization,
                "health": e.health,
                "edge_type": e.edge_type,
                "owner": e.owner,
                "src_x": src_pos.map(|p| p.x).unwrap_or(0.0),
                "src_y": src_pos.map(|p| p.y).unwrap_or(0.0),
                "dst_x": dst_pos.map(|p| p.x).unwrap_or(0.0),
                "dst_y": dst_pos.map(|p| p.y).unwrap_or(0.0),
            })
        })
        .collect();

    let node_flows: Vec<serde_json::Value> = world
        .infra_nodes
        .iter()
        .filter_map(|(&id, node)| {
            let traffic = tm.node_traffic.get(&id).copied().unwrap_or(0.0);
            let pos = world.positions.get(&id)?;
            let utilization = if node.max_throughput > 0.0 {
                traffic / node.max_throughput
            } else {
                0.0
            };
            Some(serde_json::json!({
                "id": id,
                "traffic": traffic,
                "max_throughput": node.max_throughput,
                "utilization": utilization,
                "node_type": node.node_type,
                "owner": node.owner,
                "x": pos.x,
                "y": pos.y,
            }))
        })
        .collect();

    let player_id = world.player_corp_id().unwrap_or(0);
    let player_served = tm
        .corp_traffic_served
        .get(&player_id)
        .copied()
        .unwrap_or(0.0);
    let player_dropped = tm
        .corp_traffic_dropped
        .get(&player_id)
        .copied()
        .unwrap_or(0.0);

    let mut congested: Vec<(u64, f64)> = world
        .infra_edges
        .iter()
        .map(|(&id, e)| {
            let traffic = tm.edge_traffic.get(&id).copied().unwrap_or(0.0);
            let util = if e.bandwidth > 0.0 {
                traffic / e.bandwidth
            } else {
                0.0
            };
            (id, util)
        })
        .collect();
    congested.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    let top_congested: Vec<serde_json::Value> = congested
        .iter()
        .take(5)
        .filter(|(_, util)| *util > 0.0)
        .map(|(id, util)| {
            let edge = world.infra_edges.get(id);
            serde_json::json!({
                "id": id,
                "utilization": util,
                "edge_type": edge.map(|e| e.edge_type),
                "owner": edge.map(|e| e.owner).unwrap_or(0),
            })
        })
        .collect();

    serde_json::json!({
        "edge_flows": edge_flows,
        "node_flows": node_flows,
        "total_served": tm.total_served,
        "total_dropped": tm.total_dropped,
        "total_demand": tm.total_served + tm.total_dropped,
        "player_served": player_served,
        "player_dropped": player_dropped,
        "top_congested": top_congested,
    })
    .to_string()
}

pub fn query_road_pathfind(
    world: &GameWorld,
    from_lon: f64,
    from_lat: f64,
    to_lon: f64,
    to_lat: f64,
) -> String {
    let waypoints = world.road_pathfind(from_lon, from_lat, to_lon, to_lat);
    let json: Vec<serde_json::Value> = waypoints
        .iter()
        .map(|(lon, lat)| serde_json::json!([lon, lat]))
        .collect();
    serde_json::to_string(&json).unwrap_or_default()
}

pub fn query_road_segments(world: &GameWorld) -> String {
    let segments = world.get_road_segments();
    let json: Vec<serde_json::Value> = segments
        .iter()
        .map(|s| {
            serde_json::json!({
                "id": s.id,
                "from": [s.from.0, s.from.1],
                "to": [s.to.0, s.to.1],
                "road_class": s.road_class,
                "length_km": s.length_km,
                "region_id": s.region_id,
            })
        })
        .collect();
    serde_json::to_string(&json).unwrap_or_default()
}
