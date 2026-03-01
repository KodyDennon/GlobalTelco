//! Tauri-native simulation bridge for desktop builds.
//!
//! Wraps gt-simulation's `GameWorld` and implements `gt_bridge::BridgeQuery`
//! for native Rust execution — no WASM overhead. The `SimThread` module
//! provides a dedicated background thread that owns the `GameWorld`.
//!
//! Frontend calls Tauri `invoke()` instead of calling WASM when running
//! in the desktop environment.

pub mod binary;
pub mod sim_thread;

use std::sync::Mutex;

use gt_bridge::{BridgeQuery, EdgeArrays, InfraArrays, SatelliteArrays};
use gt_common::types::{EntityId, WorldConfig};
use gt_simulation::world::GameWorld;

/// Native simulation bridge for desktop.
/// Holds a `GameWorld` behind a `Mutex` so it can be shared via Tauri state.
pub struct TauriBridge {
    pub world: Mutex<GameWorld>,
}

impl TauriBridge {
    pub fn new(config: WorldConfig) -> Self {
        Self {
            world: Mutex::new(GameWorld::new(config)),
        }
    }

    pub fn from_save(data: &str) -> Result<Self, String> {
        let world = GameWorld::load_game(data).map_err(|e| format!("Load failed: {e}"))?;
        Ok(Self {
            world: Mutex::new(world),
        })
    }
}

// ── Non-BridgeQuery methods (extra queries not in the trait) ───────────

impl TauriBridge {
    pub fn get_player_corp_id(&self) -> u64 {
        self.world.lock().unwrap().player_corp_id().unwrap_or(0)
    }

    pub fn is_real_earth(&self) -> bool {
        self.world.lock().unwrap().config().use_real_earth
    }

    pub fn get_infrastructure_list(&self, corp_id: EntityId) -> String {
        let w = self.world.lock().unwrap();
        let node_ids = w.corp_infra_nodes.get(&corp_id).cloned().unwrap_or_default();

        let nodes: Vec<serde_json::Value> = node_ids
            .iter()
            .filter_map(|&id| {
                let node = w.infra_nodes.get(&id)?;
                let pos = w.positions.get(&id);
                let health = w.healths.get(&id);
                let cap = w.capacities.get(&id);
                let under_construction = w.constructions.contains_key(&id);
                let util_history: Vec<f64> = w.utilization_history
                    .get(&id)
                    .map(|h| h.iter().copied().collect())
                    .unwrap_or_default();
                Some(serde_json::json!({
                    "id": id,
                    "node_type": format!("{:?}", node.node_type),
                    "network_level": format!("{:?}", node.network_level),
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
                    "maintenance_priority": format!("{:?}", w.maintenance_priorities.get(&id).map(|m| m.tier).unwrap_or_default()),
                    "auto_repair": w.maintenance_priorities.get(&id).map(|m| m.auto_repair).unwrap_or(true),
                }))
            })
            .collect();

        let edges: Vec<serde_json::Value> = w.infra_edges
            .iter()
            .filter(|(_, e)| e.owner == corp_id)
            .map(|(&id, e)| {
                let src_pos = w.positions.get(&e.source);
                let dst_pos = w.positions.get(&e.target);
                let src_cell = w.infra_nodes.get(&e.source).map(|n| n.cell_index).unwrap_or(0);
                let dst_cell = w.infra_nodes.get(&e.target).map(|n| n.cell_index).unwrap_or(0);
                let util_history: Vec<f64> = w.utilization_history
                    .get(&id)
                    .map(|h| h.iter().copied().collect())
                    .unwrap_or_default();
                serde_json::json!({
                    "id": id,
                    "edge_type": format!("{:?}", e.edge_type),
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
                    "deployment": format!("{:?}", e.deployment),
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

    pub fn get_visible_entities(&self, min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> String {
        let w = self.world.lock().unwrap();
        let nodes: Vec<serde_json::Value> = w.infra_nodes
            .iter()
            .filter_map(|(&id, node)| {
                let pos = w.positions.get(&id)?;
                if pos.x >= min_x && pos.x <= max_x && pos.y >= min_y && pos.y <= max_y {
                    let health = w.healths.get(&id);
                    let cap = w.capacities.get(&id);
                    Some(serde_json::json!({
                        "id": id,
                        "type": "node",
                        "node_type": format!("{:?}", node.node_type),
                        "owner": node.owner,
                        "x": pos.x,
                        "y": pos.y,
                        "health": health.map(|h| h.condition).unwrap_or(1.0),
                        "utilization": cap.map(|c| c.utilization()).unwrap_or(0.0),
                        "under_construction": w.constructions.contains_key(&id),
                    }))
                } else {
                    None
                }
            })
            .collect();

        let cities: Vec<serde_json::Value> = w.cities
            .iter()
            .filter_map(|(&id, city)| {
                let pos = w.positions.get(&id)?;
                if pos.x >= min_x && pos.x <= max_x && pos.y >= min_y && pos.y <= max_y {
                    Some(serde_json::json!({
                        "id": id, "type": "city", "name": city.name,
                        "population": city.population, "x": pos.x, "y": pos.y,
                    }))
                } else {
                    None
                }
            })
            .collect();

        serde_json::json!({ "nodes": nodes, "cities": cities }).to_string()
    }

    pub fn get_parcels_in_view(&self, min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> String {
        let w = self.world.lock().unwrap();
        let parcels: Vec<serde_json::Value> = w.land_parcels
            .iter()
            .filter_map(|(&id, parcel)| {
                let pos = w.positions.get(&id)?;
                if pos.x >= min_x && pos.x <= max_x && pos.y >= min_y && pos.y <= max_y {
                    Some(serde_json::json!({
                        "id": id, "cell_index": parcel.cell_index,
                        "terrain": format!("{:?}", parcel.terrain),
                        "elevation": parcel.elevation,
                        "zoning": format!("{:?}", parcel.zoning),
                        "cost_modifier": parcel.cost_modifier,
                        "x": pos.x, "y": pos.y,
                    }))
                } else {
                    None
                }
            })
            .collect();
        serde_json::to_string(&parcels).unwrap_or_default()
    }

    pub fn get_cell_coverage(&self) -> String {
        let w = self.world.lock().unwrap();
        let coverage: Vec<serde_json::Value> = w.cell_coverage
            .iter()
            .filter_map(|(&cell_idx, cov)| {
                let (lat, lon) = w.grid_cell_positions.get(cell_idx)?;
                Some(serde_json::json!({
                    "cell_index": cell_idx, "lat": lat, "lon": lon,
                    "signal_strength": cov.signal_strength, "bandwidth": cov.bandwidth,
                    "node_count": cov.node_count, "best_signal": cov.best_signal,
                    "dominant_owner": cov.dominant_owner,
                }))
            })
            .collect();
        serde_json::to_string(&coverage).unwrap_or_default()
    }

    pub fn get_all_infrastructure(&self) -> String {
        let w = self.world.lock().unwrap();
        let nodes: Vec<serde_json::Value> = w.infra_nodes
            .iter()
            .filter_map(|(&id, node)| {
                let pos = w.positions.get(&id)?;
                let health = w.healths.get(&id);
                let cap = w.capacities.get(&id);
                let owner_name = w.corporations.get(&node.owner)
                    .map(|c| c.name.as_str()).unwrap_or("Unknown");
                Some(serde_json::json!({
                    "id": id, "node_type": format!("{:?}", node.node_type),
                    "network_level": format!("{:?}", node.network_level),
                    "max_throughput": node.max_throughput, "current_load": node.current_load,
                    "latency_ms": node.latency_ms, "reliability": node.reliability,
                    "cell_index": node.cell_index, "owner": node.owner,
                    "owner_name": owner_name, "x": pos.x, "y": pos.y,
                    "health": health.map(|h| h.condition).unwrap_or(1.0),
                    "utilization": cap.map(|c| c.utilization()).unwrap_or(0.0),
                    "under_construction": w.constructions.contains_key(&id),
                    "active_ftth": node.active_ftth,
                }))
            })
            .collect();

        let edges: Vec<serde_json::Value> = w.infra_edges
            .iter()
            .map(|(&id, e)| {
                let src_pos = w.positions.get(&e.source);
                let dst_pos = w.positions.get(&e.target);
                let owner_name = w.corporations.get(&e.owner)
                    .map(|c| c.name.as_str()).unwrap_or("Unknown");
                let src_cell = w.infra_nodes.get(&e.source).map(|n| n.cell_index).unwrap_or(0);
                let dst_cell = w.infra_nodes.get(&e.target).map(|n| n.cell_index).unwrap_or(0);
                serde_json::json!({
                    "id": id, "edge_type": format!("{:?}", e.edge_type),
                    "source": e.source, "target": e.target,
                    "bandwidth": e.bandwidth, "current_load": e.current_load,
                    "latency_ms": e.latency_ms, "length_km": e.length_km,
                    "health": e.health, "utilization": e.utilization(),
                    "owner": e.owner, "owner_name": owner_name,
                    "src_x": src_pos.map(|p| p.x).unwrap_or(0.0),
                    "src_y": src_pos.map(|p| p.y).unwrap_or(0.0),
                    "dst_x": dst_pos.map(|p| p.x).unwrap_or(0.0),
                    "dst_y": dst_pos.map(|p| p.y).unwrap_or(0.0),
                    "src_cell": src_cell, "dst_cell": dst_cell,
                    "waypoints": e.waypoints.iter().map(|&(lon, lat)| [lon, lat]).collect::<Vec<_>>(),
                    "deployment": format!("{:?}", e.deployment),
                })
            })
            .collect();

        serde_json::json!({ "nodes": nodes, "edges": edges }).to_string()
    }

    pub fn get_grid_cells(&self) -> String {
        let w = self.world.lock().unwrap();
        let mut cell_terrain: std::collections::HashMap<usize, String> = std::collections::HashMap::new();
        for parcel in w.land_parcels.values() {
            cell_terrain.insert(parcel.cell_index, format!("{:?}", parcel.terrain));
        }
        let cells: Vec<serde_json::Value> = w.grid_cell_positions
            .iter()
            .enumerate()
            .map(|(i, &(lat, lon))| {
                let terrain = cell_terrain.get(&i).cloned().unwrap_or_else(|| "Ocean".to_string());
                let neighbors = w.grid_cell_neighbors.get(i).cloned().unwrap_or_default();
                serde_json::json!({ "index": i, "lat": lat, "lon": lon, "terrain": terrain, "neighbors": neighbors })
            })
            .collect();
        serde_json::to_string(&cells).unwrap_or_default()
    }

    pub fn get_world_geojson(&self) -> String {
        let w = self.world.lock().unwrap();
        let mut features: Vec<serde_json::Value> = Vec::new();

        for (&id, region) in &w.regions {
            if region.boundary_polygon.is_empty() { continue; }
            let coords: Vec<Vec<f64>> = region.boundary_polygon.iter()
                .map(|&(lat, lon)| vec![lon, lat]).collect();
            let mut ring = coords;
            if let Some(first) = ring.first().cloned() { ring.push(first); }
            features.push(serde_json::json!({
                "type": "Feature",
                "geometry": { "type": "Polygon", "coordinates": [ring] },
                "properties": { "id": id, "name": region.name, "type": "region",
                    "population": region.population, "gdp": region.gdp, "development": region.development },
            }));
        }

        for (&id, city) in &w.cities {
            if let Some(pos) = w.positions.get(&id) {
                features.push(serde_json::json!({
                    "type": "Feature",
                    "geometry": { "type": "Point", "coordinates": [pos.x, pos.y] },
                    "properties": { "id": id, "name": city.name, "type": "city",
                        "population": city.population, "development": city.development },
                }));
            }
        }

        serde_json::json!({ "type": "FeatureCollection", "features": features }).to_string()
    }

    pub fn get_spectrum_licenses(&self) -> String {
        let w = self.world.lock().unwrap();
        let tick = w.current_tick();
        let licenses: Vec<serde_json::Value> = w.spectrum_licenses
            .iter()
            .filter(|(_, l)| l.is_active(tick))
            .map(|(&id, l)| {
                let region_name = w.regions.get(&l.region_id).map(|r| r.name.as_str()).unwrap_or("Unknown");
                let owner_name = w.corporations.get(&l.owner).map(|c| c.name.as_str()).unwrap_or("Unknown");
                serde_json::json!({
                    "id": id, "band": format!("{:?}", l.band), "band_name": l.band.display_name(),
                    "band_category": l.band.category(), "region_id": l.region_id,
                    "region_name": region_name, "owner": l.owner, "owner_name": owner_name,
                    "bandwidth_mhz": l.bandwidth_mhz, "start_tick": l.start_tick,
                    "end_tick": l.end_tick(), "cost_per_tick": l.cost_per_tick(),
                    "coverage_radius_km": l.band.coverage_radius_km(),
                })
            })
            .collect();
        serde_json::to_string(&licenses).unwrap_or_default()
    }

    pub fn get_spectrum_auctions(&self) -> String {
        let w = self.world.lock().unwrap();
        let tick = w.current_tick();
        let auctions: Vec<serde_json::Value> = w.spectrum_auctions
            .iter()
            .filter(|(_, a)| !a.is_ended(tick))
            .map(|(&id, a)| {
                let region_name = w.regions.get(&a.region_id).map(|r| r.name.as_str()).unwrap_or("Unknown");
                let (highest_bidder, current_bid) = a.highest_bid().unwrap_or((0, 0));
                let bidder_name = w.corporations.get(&highest_bidder).map(|c| c.name.as_str()).unwrap_or("None");
                serde_json::json!({
                    "id": id, "band": format!("{:?}", a.band), "band_name": a.band.display_name(),
                    "band_category": a.band.category(), "region_id": a.region_id,
                    "region_name": region_name, "bandwidth_mhz": a.bandwidth_mhz,
                    "current_bid": current_bid, "highest_bidder": highest_bidder,
                    "bidder_name": bidder_name, "end_tick": a.end_tick,
                    "ticks_remaining": a.ticks_remaining(tick),
                    "coverage_radius_km": a.band.coverage_radius_km(),
                })
            })
            .collect();
        serde_json::to_string(&auctions).unwrap_or_default()
    }

    pub fn get_available_spectrum(&self, region_id: EntityId) -> String {
        use gt_common::types::FrequencyBand;
        let w = self.world.lock().unwrap();
        let tick = w.current_tick();
        let licensed_bands: std::collections::HashSet<String> = w.spectrum_licenses.values()
            .filter(|l| l.region_id == region_id && l.is_active(tick))
            .map(|l| format!("{:?}", l.band)).collect();
        let auction_bands: std::collections::HashSet<String> = w.spectrum_auctions.values()
            .filter(|a| a.region_id == region_id && !a.is_ended(tick))
            .map(|a| format!("{:?}", a.band)).collect();
        let available: Vec<serde_json::Value> = FrequencyBand::all()
            .iter()
            .filter(|b| {
                let name = format!("{:?}", b);
                !licensed_bands.contains(&name) && !auction_bands.contains(&name)
            })
            .map(|b| serde_json::json!({
                "band": format!("{:?}", b), "band_name": b.display_name(),
                "band_category": b.category(), "coverage_radius_km": b.coverage_radius_km(),
                "max_bandwidth_mhz": b.max_bandwidth_mhz(),
                "min_bid": b.cost_per_mhz() * b.max_bandwidth_mhz() as i64,
            }))
            .collect();
        serde_json::to_string(&available).unwrap_or_default()
    }

    pub fn get_disaster_forecasts(&self) -> String {
        let w = self.world.lock().unwrap();
        let forecasts = w.get_disaster_forecasts();
        let json: Vec<serde_json::Value> = forecasts.iter().map(|f| serde_json::json!({
            "region_id": f.region_id, "region_name": f.region_name,
            "predicted_tick": f.predicted_tick, "probability": f.probability,
            "disaster_type": f.disaster_type,
        })).collect();
        serde_json::to_string(&json).unwrap_or_default()
    }

    pub fn get_acquisition_proposals(&self) -> String {
        let w = self.world.lock().unwrap();
        let proposals: Vec<serde_json::Value> = w.acquisition_proposals
            .iter()
            .map(|(&id, p)| {
                let acquirer_name = w.corporations.get(&p.acquirer).map(|c| c.name.as_str()).unwrap_or("Unknown");
                let target_name = w.corporations.get(&p.target).map(|c| c.name.as_str()).unwrap_or("Unknown");
                serde_json::json!({
                    "id": id, "acquirer": p.acquirer, "acquirer_name": acquirer_name,
                    "target": p.target, "target_name": target_name,
                    "offer": p.offer, "status": format!("{:?}", p.status), "tick": p.tick,
                })
            })
            .collect();
        serde_json::to_string(&proposals).unwrap_or_default()
    }

    pub fn road_pathfind(&self, from_lon: f64, from_lat: f64, to_lon: f64, to_lat: f64) -> String {
        let w = self.world.lock().unwrap();
        let waypoints = w.road_pathfind(from_lon, from_lat, to_lon, to_lat);
        let json: Vec<serde_json::Value> = waypoints.iter()
            .map(|(lon, lat)| serde_json::json!([lon, lat])).collect();
        serde_json::to_string(&json).unwrap_or_default()
    }

    pub fn road_fiber_route_cost(&self, from_lon: f64, from_lat: f64, to_lon: f64, to_lat: f64) -> String {
        let w = self.world.lock().unwrap();
        let cost = w.road_fiber_route_cost(from_lon, from_lat, to_lon, to_lat);
        serde_json::to_string(&cost).unwrap_or_default()
    }

    pub fn get_road_segments(&self) -> String {
        let w = self.world.lock().unwrap();
        let segments = w.get_road_segments();
        let json: Vec<serde_json::Value> = segments.iter().map(|s| serde_json::json!({
            "id": s.id, "from": [s.from.0, s.from.1], "to": [s.to.0, s.to.1],
            "road_class": format!("{:?}", s.road_class), "length_km": s.length_km,
            "region_id": s.region_id,
        })).collect();
        serde_json::to_string(&json).unwrap_or_default()
    }

    pub fn create_world_preview(config_json: &str) -> Result<String, String> {
        let config: WorldConfig = serde_json::from_str(config_json)
            .map_err(|e| format!("Invalid config: {e}"))?;
        if config.use_real_earth {
            return Ok(serde_json::json!({ "is_real_earth": true, "cells": [], "width": 0, "height": 0 }).to_string());
        }
        let gen = gt_world::WorldGenerator::new(config);
        let world = gen.generate();
        let preview_cells: Vec<serde_json::Value> = world.grid.cells.iter().enumerate().map(|(i, cell)| {
            let terrain = &world.terrains[i];
            let elevation = world.elevations[i];
            serde_json::json!({ "lat": cell.lat, "lon": cell.lon, "terrain": format!("{:?}", terrain), "elevation": elevation })
        }).collect();
        let city_previews: Vec<serde_json::Value> = world.cities.iter().map(|city| {
            let cell = &world.grid.cells[city.cell_index];
            serde_json::json!({ "name": city.name, "lat": cell.lat, "lon": cell.lon, "population": city.population })
        }).collect();
        Ok(serde_json::json!({
            "is_real_earth": false, "cell_count": world.grid.cell_count(),
            "cells": preview_cells, "cities": city_previews, "region_count": world.regions.len(),
        }).to_string())
    }
}

// ── BridgeQuery implementation ──────────────────────────────────────────

impl BridgeQuery for TauriBridge {
    fn tick(&mut self) {
        self.world.lock().unwrap().tick();
    }

    fn current_tick(&self) -> u64 {
        self.world.lock().unwrap().current_tick()
    }

    fn process_command(&mut self, command_json: &str) -> Result<String, String> {
        let cmd: gt_common::commands::Command = serde_json::from_str(command_json)
            .map_err(|e| format!("Invalid command: {e}"))?;
        let mut w = self.world.lock().unwrap();
        w.process_command(cmd);
        let events = w.event_queue.drain();
        if events.is_empty() {
            Ok(String::new())
        } else {
            let notifications: Vec<serde_json::Value> = events
                .iter()
                .map(|(tick, event)| {
                    serde_json::json!({
                        "tick": tick,
                        "event": serde_json::to_value(event).unwrap_or(serde_json::Value::Null),
                    })
                })
                .collect();
            Ok(serde_json::to_string(&notifications).unwrap_or_default())
        }
    }

    fn apply_batch(&mut self, ops_json: &str) -> Result<(), String> {
        let ops: Vec<gt_common::protocol::DeltaOp> = serde_json::from_str(ops_json)
            .map_err(|e| format!("Invalid delta ops: {e}"))?;
        self.world.lock().unwrap().apply_delta(&ops);
        Ok(())
    }

    fn get_world_info(&self) -> String {
        let w = self.world.lock().unwrap();
        let info = serde_json::json!({
            "tick": w.current_tick(),
            "speed": format!("{:?}", w.speed()),
            "entity_count": w.entity_count(),
            "region_count": w.regions.len(),
            "city_count": w.cities.len(),
            "corporation_count": w.corporations.len(),
            "infra_node_count": w.infra_nodes.len(),
            "infra_edge_count": w.infra_edges.len(),
            "player_corp_id": w.player_corp_id(),
            "cell_spacing_km": w.cell_spacing_km,
            "sandbox": w.config().sandbox,
        });
        serde_json::to_string(&info).unwrap_or_default()
    }

    fn get_corporation_data(&self, corp_id: EntityId) -> String {
        let w = self.world.lock().unwrap();
        let corp = w.corporations.get(&corp_id);
        let fin = w.financials.get(&corp_id);
        let wf = w.workforces.get(&corp_id);
        let node_count = w.corp_infra_nodes.get(&corp_id).map(|n| n.len()).unwrap_or(0);
        let data = serde_json::json!({
            "id": corp_id,
            "name": corp.map(|c| c.name.as_str()).unwrap_or("Unknown"),
            "is_player": corp.map(|c| c.is_player).unwrap_or(false),
            "credit_rating": corp.map(|c| format!("{:?}", c.credit_rating)).unwrap_or_default(),
            "cash": fin.map(|f| f.cash).unwrap_or(0),
            "revenue_per_tick": fin.map(|f| f.revenue_per_tick).unwrap_or(0),
            "cost_per_tick": fin.map(|f| f.cost_per_tick).unwrap_or(0),
            "debt": fin.map(|f| f.debt).unwrap_or(0),
            "profit_per_tick": fin.map(|f| f.revenue_per_tick - f.cost_per_tick).unwrap_or(0),
            "employee_count": wf.map(|w| w.employee_count).unwrap_or(0),
            "morale": wf.map(|w| w.morale).unwrap_or(0.0),
            "infrastructure_count": node_count,
        });
        serde_json::to_string(&data).unwrap_or_default()
    }

    fn get_regions(&self) -> String {
        let w = self.world.lock().unwrap();
        let regions: Vec<serde_json::Value> = w.regions.iter().map(|(&id, r)| {
            let boundary: Vec<serde_json::Value> = r.boundary_polygon
                .iter()
                .map(|&(lat, lon)| serde_json::json!([lat, lon]))
                .collect();
            serde_json::json!({
                "id": id, "name": r.name, "center_lat": r.center_lat, "center_lon": r.center_lon,
                "population": r.population, "gdp": r.gdp, "development": r.development,
                "tax_rate": r.tax_rate, "regulatory_strictness": r.regulatory_strictness,
                "disaster_risk": r.disaster_risk, "cell_count": r.cells.len(),
                "city_ids": r.city_ids, "boundary_polygon": boundary,
            })
        }).collect();
        serde_json::to_string(&regions).unwrap_or_default()
    }

    fn get_cities(&self) -> String {
        let w = self.world.lock().unwrap();
        let cities: Vec<serde_json::Value> = w.cities.iter().map(|(&id, c)| {
            let pos = w.positions.get(&id);
            let cell_positions: Vec<serde_json::Value> = c.cells.iter().filter_map(|&ci| {
                let (lat, lon) = w.grid_cell_positions.get(ci)?;
                Some(serde_json::json!({"index": ci, "lat": lat, "lon": lon}))
            }).collect();
            serde_json::json!({
                "id": id, "name": c.name, "region_id": c.region_id,
                "cell_index": c.cell_index, "cells": c.cells, "cell_positions": cell_positions,
                "population": c.population, "growth_rate": c.growth_rate,
                "development": c.development, "telecom_demand": c.telecom_demand,
                "infrastructure_satisfaction": c.infrastructure_satisfaction,
                "employment_rate": c.employment_rate, "jobs_available": c.jobs_available,
                "birth_rate": c.birth_rate, "death_rate": c.death_rate,
                "migration_pressure": c.migration_pressure,
                "x": pos.map(|p| p.x).unwrap_or(0.0),
                "y": pos.map(|p| p.y).unwrap_or(0.0),
            })
        }).collect();
        serde_json::to_string(&cities).unwrap_or_default()
    }

    fn get_all_corporations(&self) -> String {
        let w = self.world.lock().unwrap();
        let corps: Vec<serde_json::Value> = w.corporations.iter().map(|(&id, corp)| {
            let fin = w.financials.get(&id);
            serde_json::json!({
                "id": id, "name": corp.name, "is_player": corp.is_player,
                "credit_rating": format!("{:?}", corp.credit_rating),
                "cash": fin.map(|f| f.cash).unwrap_or(0),
                "revenue": fin.map(|f| f.revenue_per_tick).unwrap_or(0),
                "cost": fin.map(|f| f.cost_per_tick).unwrap_or(0),
            })
        }).collect();
        serde_json::to_string(&corps).unwrap_or_default()
    }

    fn get_research_state(&self) -> String {
        let w = self.world.lock().unwrap();
        let research: Vec<serde_json::Value> = w.tech_research
            .iter()
            .map(|(&id, r)| {
                let researcher_name = r.researcher.and_then(|rid| w.corporations.get(&rid).map(|c| c.name.clone()));
                let patent_owner_name = r.patent_owner.and_then(|oid| w.corporations.get(&oid).map(|c| c.name.clone()));
                let patent_data = w.patents.values().find(|p| p.tech_id == id);
                let per_unit_price = patent_data.map(|p| p.per_unit_price).unwrap_or(0);
                let lease_duration = patent_data.map(|p| p.lease_duration).unwrap_or(0);
                let patent_license_type = patent_data.map(|p| format!("{:?}", p.license_type)).unwrap_or_default();
                serde_json::json!({
                    "id": id, "category": format!("{:?}", r.category),
                    "category_name": r.category.display_name(), "name": r.name,
                    "description": r.description, "progress": r.progress,
                    "total_cost": r.total_cost, "progress_pct": r.progress_pct(),
                    "researcher": r.researcher, "researcher_name": researcher_name,
                    "completed": r.completed, "patent_status": format!("{:?}", r.patent_status),
                    "patent_owner": r.patent_owner, "patent_owner_name": patent_owner_name,
                    "license_price": r.license_price, "prerequisites": r.prerequisites,
                    "throughput_bonus": r.throughput_bonus, "cost_reduction": r.cost_reduction,
                    "reliability_bonus": r.reliability_bonus,
                    "independent_tier": format!("{:?}", r.independent_tier),
                    "per_unit_price": per_unit_price, "lease_duration": lease_duration,
                    "patent_license_type": patent_license_type,
                })
            })
            .collect();
        serde_json::to_string(&research).unwrap_or_default()
    }

    fn get_contracts(&self, corp_id: EntityId) -> String {
        let w = self.world.lock().unwrap();
        let contracts: Vec<serde_json::Value> = w.contracts
            .iter()
            .filter(|(_, c)| c.from == corp_id || c.to == corp_id)
            .map(|(&id, c)| {
                let from_name = w.corporations.get(&c.from).map(|corp| corp.name.as_str()).unwrap_or("Unknown");
                let to_name = w.corporations.get(&c.to).map(|corp| corp.name.as_str()).unwrap_or("Unknown");
                let sla_status = if c.sla_current_performance >= c.sla_target { "ok" }
                    else if c.sla_current_performance >= c.sla_target - 5.0 { "at_risk" }
                    else { "breach" };
                let traffic_current = w.traffic_matrix.contract_traffic.get(&id).copied().unwrap_or(0.0);
                let traffic_capacity_pct = if c.capacity > 0.0 { (traffic_current / c.capacity * 100.0).min(100.0) } else { 0.0 };
                let price_per_unit = if c.capacity > 0.0 { c.price_per_tick as f64 / c.capacity } else { 0.0 };
                let transit_amount = (traffic_current * price_per_unit) as i64;
                let (transit_revenue, transit_cost) = if c.from == corp_id { (transit_amount, 0i64) } else { (0i64, transit_amount) };
                serde_json::json!({
                    "id": id, "contract_type": format!("{:?}", c.contract_type),
                    "from": c.from, "to": c.to, "from_name": from_name, "to_name": to_name,
                    "capacity": c.capacity, "price_per_tick": c.price_per_tick,
                    "start_tick": c.start_tick, "end_tick": c.end_tick,
                    "status": format!("{:?}", c.status), "penalty": c.penalty,
                    "sla_target": c.sla_target, "sla_current_performance": c.sla_current_performance,
                    "sla_status": sla_status, "sla_penalty_accrued": c.sla_penalty_accrued,
                    "traffic_current": traffic_current, "traffic_capacity_pct": traffic_capacity_pct,
                    "transit_revenue": transit_revenue, "transit_cost": transit_cost,
                })
            })
            .collect();
        serde_json::to_string(&contracts).unwrap_or_default()
    }

    fn get_debt_instruments(&self, corp_id: EntityId) -> String {
        let w = self.world.lock().unwrap();
        let debts: Vec<serde_json::Value> = w.debt_instruments
            .iter()
            .filter(|(_, d)| d.holder == corp_id)
            .map(|(&id, d)| serde_json::json!({
                "id": id, "principal": d.principal, "interest_rate": d.interest_rate,
                "remaining_ticks": d.remaining_ticks, "payment_per_tick": d.payment_per_tick,
                "is_paid_off": d.is_paid_off(),
            }))
            .collect();
        serde_json::to_string(&debts).unwrap_or_default()
    }

    fn get_notifications(&mut self) -> String {
        let mut w = self.world.lock().unwrap();
        let events = w.event_queue.drain();
        let notifications: Vec<serde_json::Value> = events.iter().map(|(tick, event)| {
            serde_json::json!({
                "tick": tick,
                "event": serde_json::to_value(event).unwrap_or(serde_json::Value::Null),
            })
        }).collect();
        serde_json::to_string(&notifications).unwrap_or_default()
    }

    fn get_buildable_nodes(&self, lon: f64, lat: f64) -> String {
        use gt_common::types::NodeType;
        let w = self.world.lock().unwrap();
        let player_id = w.player_corp_id().unwrap_or(0);
        let cash = w.financials.get(&player_id).map(|f| f.cash).unwrap_or(0);
        let terrain_mult = w.find_nearest_cell(lon, lat)
            .and_then(|(cell_idx, _)| {
                w.cell_to_parcel.get(&cell_idx)
                    .and_then(|&pid| w.land_parcels.get(&pid))
                    .map(|p| p.cost_modifier)
            })
            .unwrap_or(1.0);
        let options: Vec<serde_json::Value> = NodeType::ALL.iter().map(|nt| {
            let base_cost = nt.construction_cost();
            let cost = (base_cost as f64 * terrain_mult) as i64;
            let build_ticks = (base_cost / 10_000).max(5);
            serde_json::json!({
                "label": nt.display_name(), "node_type": format!("{:?}", nt),
                "network_level": format!("{:?}", nt.tier()), "tier": nt.tier().value(),
                "era": nt.era().display_name(), "cost": cost, "build_ticks": build_ticks,
                "affordable": cash >= cost,
            })
        }).collect();
        serde_json::to_string(&options).unwrap_or_default()
    }

    fn get_buildable_edges(&self, source_id: EntityId) -> String {
        use gt_common::types::EdgeType;
        let w = self.world.lock().unwrap();
        let player_id = w.player_corp_id().unwrap_or(0);
        let cash = w.financials.get(&player_id).map(|f| f.cash).unwrap_or(0);
        let player_nodes = w.corp_infra_nodes.get(&player_id).cloned().unwrap_or_default();
        let targets: Vec<serde_json::Value> = player_nodes
            .iter()
            .filter(|&&nid| nid != source_id && !w.constructions.contains_key(&nid))
            .filter_map(|&nid| {
                let node = w.infra_nodes.get(&nid)?;
                let pos = w.positions.get(&nid)?;
                let src_pos = w.positions.get(&source_id)?;
                let dlat = (src_pos.y - pos.y).to_radians();
                let dlon = (src_pos.x - pos.x).to_radians();
                let lat1 = src_pos.y.to_radians();
                let lat2 = pos.y.to_radians();
                let a = (dlat / 2.0).sin().powi(2) + lat1.cos() * lat2.cos() * (dlon / 2.0).sin().powi(2);
                let c = 2.0 * a.sqrt().asin();
                let dist_km = 6371.0 * c;
                let min_cost = EdgeType::ALL.iter().map(|et| {
                    let cpk = et.cost_per_km();
                    if cpk == 0 { 5_000_000i64 } else { (cpk as f64 * dist_km) as i64 }
                }).min().unwrap_or(0);
                Some(serde_json::json!({
                    "target_id": nid, "target_type": format!("{:?}", node.node_type),
                    "x": pos.x, "y": pos.y, "distance_km": dist_km,
                    "cost": min_cost, "affordable": cash >= min_cost,
                }))
            })
            .collect();
        serde_json::to_string(&targets).unwrap_or_default()
    }

    fn get_damaged_nodes(&self, corp_id: EntityId) -> String {
        let w = self.world.lock().unwrap();
        let node_ids = w.corp_infra_nodes.get(&corp_id).cloned().unwrap_or_default();
        let damaged: Vec<serde_json::Value> = node_ids
            .iter()
            .filter_map(|&id| {
                let node = w.infra_nodes.get(&id)?;
                let health = w.healths.get(&id)?;
                if health.condition >= 0.95 { return None; }
                let pos = w.positions.get(&id);
                let damage = 1.0 - health.condition;
                let repair_cost = (node.construction_cost as f64 * damage * 0.2) as i64;
                let emergency_cost = (node.construction_cost as f64 * damage * 0.6) as i64;
                Some(serde_json::json!({
                    "id": id, "node_type": format!("{:?}", node.node_type),
                    "health": health.condition, "repair_cost": repair_cost,
                    "emergency_cost": emergency_cost,
                    "x": pos.map(|p| p.x).unwrap_or(0.0),
                    "y": pos.map(|p| p.y).unwrap_or(0.0),
                }))
            })
            .collect();
        serde_json::to_string(&damaged).unwrap_or_default()
    }

    fn get_auctions(&self) -> String {
        let w = self.world.lock().unwrap();
        let auctions: Vec<serde_json::Value> = w.auctions
            .iter()
            .map(|(&id, a)| {
                let seller_name = w.corporations.get(&a.seller).map(|c| c.name.as_str()).unwrap_or("Unknown");
                let highest = a.highest_bid();
                serde_json::json!({
                    "id": id, "seller": a.seller, "seller_name": seller_name,
                    "asset_count": a.assets.len(), "bid_count": a.bids.len(),
                    "highest_bid": highest.map(|(_, amt)| amt).unwrap_or(0),
                    "highest_bidder": highest.map(|(id, _)| id).unwrap_or(0),
                    "start_tick": a.start_tick, "end_tick": a.end_tick,
                    "status": format!("{:?}", a.status),
                })
            })
            .collect();
        serde_json::to_string(&auctions).unwrap_or_default()
    }

    fn get_covert_ops(&self, corp_id: EntityId) -> String {
        let w = self.world.lock().unwrap();
        let ops = w.covert_ops.get(&corp_id);
        let data = serde_json::json!({
            "security_level": ops.map(|o| o.security_level).unwrap_or(0),
            "active_missions": ops.map(|o| o.active_missions.len()).unwrap_or(0),
            "detection_count": ops.map(|o| o.detection_history.len()).unwrap_or(0),
        });
        serde_json::to_string(&data).unwrap_or_default()
    }

    fn get_lobbying_campaigns(&self, corp_id: EntityId) -> String {
        let w = self.world.lock().unwrap();
        let campaigns: Vec<serde_json::Value> = w.lobbying_campaigns
            .iter()
            .filter(|(_, c)| c.corporation == corp_id)
            .map(|(&id, c)| {
                let region_name = w.regions.get(&c.region).map(|r| r.name.as_str()).unwrap_or("Unknown");
                serde_json::json!({
                    "id": id, "region": c.region, "region_name": region_name,
                    "policy": format!("{:?}", c.policy), "budget_spent": c.budget_spent,
                    "budget_total": c.budget_total, "influence": c.influence,
                    "threshold": c.influence_threshold(), "active": c.active,
                })
            })
            .collect();
        serde_json::to_string(&campaigns).unwrap_or_default()
    }

    fn get_achievements(&self, corp_id: EntityId) -> String {
        let w = self.world.lock().unwrap();
        let tracker = w.achievements.get(&corp_id);
        let data = serde_json::json!({
            "unlocked": tracker.map(|t| t.unlocked.iter().cloned().collect::<Vec<_>>()).unwrap_or_default(),
            "progress": tracker.map(|t| t.progress.clone()).unwrap_or_default(),
        });
        serde_json::to_string(&data).unwrap_or_default()
    }

    fn get_victory_state(&self) -> String {
        let w = self.world.lock().unwrap();
        let state = w.victory_state.as_ref();
        let data = serde_json::json!({
            "domination_score": state.map(|s| s.domination_score).unwrap_or(0.0),
            "tech_score": state.map(|s| s.tech_score).unwrap_or(0.0),
            "wealth_score": state.map(|s| s.wealth_score).unwrap_or(0.0),
            "infrastructure_score": state.map(|s| s.infrastructure_score).unwrap_or(0.0),
            "total_score": state.map(|s| s.total_score).unwrap_or(0.0),
            "victory_type": state.and_then(|s| s.victory_type.clone()),
        });
        serde_json::to_string(&data).unwrap_or_default()
    }

    fn get_traffic_flows(&self) -> String {
        let w = self.world.lock().unwrap();
        let tm = &w.traffic_matrix;

        let edge_flows: Vec<serde_json::Value> = w.infra_edges
            .iter()
            .map(|(&id, e)| {
                let traffic = tm.edge_traffic.get(&id).copied().unwrap_or(0.0);
                let utilization = if e.bandwidth > 0.0 { traffic / e.bandwidth } else { 0.0 };
                let src_pos = w.positions.get(&e.source);
                let dst_pos = w.positions.get(&e.target);
                serde_json::json!({
                    "id": id, "traffic": traffic, "bandwidth": e.bandwidth,
                    "utilization": utilization, "health": e.health,
                    "edge_type": format!("{:?}", e.edge_type), "owner": e.owner,
                    "src_x": src_pos.map(|p| p.x).unwrap_or(0.0),
                    "src_y": src_pos.map(|p| p.y).unwrap_or(0.0),
                    "dst_x": dst_pos.map(|p| p.x).unwrap_or(0.0),
                    "dst_y": dst_pos.map(|p| p.y).unwrap_or(0.0),
                })
            })
            .collect();

        let node_flows: Vec<serde_json::Value> = w.infra_nodes
            .iter()
            .filter_map(|(&id, node)| {
                let traffic = tm.node_traffic.get(&id).copied().unwrap_or(0.0);
                let pos = w.positions.get(&id)?;
                let utilization = if node.max_throughput > 0.0 { traffic / node.max_throughput } else { 0.0 };
                Some(serde_json::json!({
                    "id": id, "traffic": traffic, "max_throughput": node.max_throughput,
                    "utilization": utilization, "node_type": format!("{:?}", node.node_type),
                    "owner": node.owner, "x": pos.x, "y": pos.y,
                }))
            })
            .collect();

        let player_id = w.player_corp_id().unwrap_or(0);
        let player_served = tm.corp_traffic_served.get(&player_id).copied().unwrap_or(0.0);
        let player_dropped = tm.corp_traffic_dropped.get(&player_id).copied().unwrap_or(0.0);

        let mut congested: Vec<(u64, f64)> = w.infra_edges
            .iter()
            .map(|(&id, e)| {
                let traffic = tm.edge_traffic.get(&id).copied().unwrap_or(0.0);
                let util = if e.bandwidth > 0.0 { traffic / e.bandwidth } else { 0.0 };
                (id, util)
            })
            .collect();
        congested.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        let top_congested: Vec<serde_json::Value> = congested.iter().take(5)
            .filter(|(_, util)| *util > 0.0)
            .map(|(id, util)| {
                let edge = w.infra_edges.get(id);
                serde_json::json!({
                    "id": id, "utilization": util,
                    "edge_type": edge.map(|e| format!("{:?}", e.edge_type)).unwrap_or_default(),
                    "owner": edge.map(|e| e.owner).unwrap_or(0),
                })
            })
            .collect();

        serde_json::json!({
            "edge_flows": edge_flows, "node_flows": node_flows,
            "total_served": tm.total_served, "total_dropped": tm.total_dropped,
            "total_demand": tm.total_served + tm.total_dropped,
            "player_served": player_served, "player_dropped": player_dropped,
            "top_congested": top_congested,
        }).to_string()
    }

    fn get_weather_forecasts(&self) -> String {
        let w = self.world.lock().unwrap();
        let forecasts = w.get_weather_forecasts();
        let json: Vec<serde_json::Value> = forecasts.iter().map(|f| serde_json::json!({
            "region_id": f.region_id, "region_name": f.region_name,
            "predicted_type": f.predicted_type.display_name(), "probability": f.probability,
            "eta_ticks": f.eta_ticks, "predicted_severity": f.predicted_severity,
        })).collect();
        serde_json::to_string(&json).unwrap_or_default()
    }

    fn save_game(&self) -> Result<String, String> {
        self.world.lock().unwrap().save_game().map_err(|e| format!("Save failed: {e}"))
    }

    fn load_game(&mut self, data: &str) -> Result<(), String> {
        let world = GameWorld::load_game(data).map_err(|e| format!("Load failed: {e}"))?;
        *self.world.lock().unwrap() = world;
        Ok(())
    }

    fn get_alliances(&self, corp_id: EntityId) -> String {
        let w = self.world.lock().unwrap();
        let alliances: Vec<serde_json::Value> = w.alliances
            .iter()
            .filter(|(_, a)| a.member_corp_ids.contains(&corp_id))
            .map(|(&id, a)| {
                let member_names: Vec<String> = a.member_corp_ids.iter()
                    .filter_map(|cid| w.corporations.get(cid).map(|c| c.name.clone()))
                    .collect();
                let trust_map: std::collections::HashMap<String, f64> = a.trust_scores.iter()
                    .map(|(cid, &score)| (cid.to_string(), score))
                    .collect();
                serde_json::json!({
                    "id": id,
                    "name": a.name,
                    "member_corp_ids": a.member_corp_ids,
                    "member_names": member_names,
                    "trust_scores": trust_map,
                    "revenue_share_pct": a.revenue_share_pct,
                    "formed_tick": a.formed_tick,
                })
            })
            .collect();
        serde_json::to_string(&alliances).unwrap_or_default()
    }

    fn get_lawsuits(&self, corp_id: EntityId) -> String {
        let w = self.world.lock().unwrap();
        let lawsuits: Vec<serde_json::Value> = w.lawsuits
            .iter()
            .filter(|(_, l)| l.plaintiff == corp_id || l.defendant == corp_id)
            .map(|(&id, l)| {
                let plaintiff_name = w.corporations.get(&l.plaintiff).map(|c| c.name.as_str()).unwrap_or("Unknown");
                let defendant_name = w.corporations.get(&l.defendant).map(|c| c.name.as_str()).unwrap_or("Unknown");
                serde_json::json!({
                    "id": id,
                    "plaintiff": l.plaintiff,
                    "plaintiff_name": plaintiff_name,
                    "defendant": l.defendant,
                    "defendant_name": defendant_name,
                    "lawsuit_type": format!("{:?}", l.lawsuit_type),
                    "damages_claimed": l.damages_claimed,
                    "filing_cost": l.filing_cost,
                    "filed_tick": l.filed_tick,
                    "resolution_tick": l.resolution_tick,
                    "status": format!("{:?}", l.status),
                    "outcome": l.outcome.as_ref().map(|o| format!("{:?}", o)),
                })
            })
            .collect();
        serde_json::to_string(&lawsuits).unwrap_or_default()
    }

    fn get_stock_market(&self, corp_id: EntityId) -> String {
        let w = self.world.lock().unwrap();
        let sm = w.stock_market.get(&corp_id);
        let data = serde_json::json!({
            "public": sm.map(|s| s.public).unwrap_or(false),
            "total_shares": sm.map(|s| s.total_shares).unwrap_or(0),
            "share_price": sm.map(|s| s.share_price).unwrap_or(0),
            "dividends_per_share": sm.map(|s| s.dividends_per_share).unwrap_or(0),
            "ipo_tick": sm.and_then(|s| s.ipo_tick),
            "shareholder_satisfaction": sm.map(|s| s.shareholder_satisfaction).unwrap_or(0.0),
            "board_votes": sm.map(|s| {
                s.board_votes.iter().map(|v| serde_json::json!({
                    "proposal": v.proposal,
                    "votes_for": v.votes_for,
                    "votes_against": v.votes_against,
                    "deadline_tick": v.deadline_tick,
                })).collect::<Vec<_>>()
            }).unwrap_or_default(),
        });
        serde_json::to_string(&data).unwrap_or_default()
    }

    fn get_region_pricing(&self, corp_id: EntityId) -> String {
        let w = self.world.lock().unwrap();
        let pricing: Vec<serde_json::Value> = w.region_pricing
            .iter()
            .filter(|((cid, _), _)| *cid == corp_id)
            .map(|((_, region_id), rp)| {
                let region_name = w.regions.get(region_id).map(|r| r.name.as_str()).unwrap_or("Unknown");
                serde_json::json!({
                    "region_id": region_id,
                    "region_name": region_name,
                    "tier": format!("{:?}", rp.tier),
                    "price_per_unit": rp.price_per_unit,
                })
            })
            .collect();
        serde_json::to_string(&pricing).unwrap_or_default()
    }

    fn get_maintenance_priorities(&self, corp_id: EntityId) -> String {
        let w = self.world.lock().unwrap();
        let node_ids = w.corp_infra_nodes
            .get(&corp_id)
            .cloned()
            .unwrap_or_default();
        let priorities: Vec<serde_json::Value> = node_ids
            .iter()
            .filter_map(|&id| {
                let mp = w.maintenance_priorities.get(&id)?;
                Some(serde_json::json!({
                    "node_id": id,
                    "priority": format!("{:?}", mp.tier),
                    "auto_repair": mp.auto_repair,
                }))
            })
            .collect();
        serde_json::to_string(&priorities).unwrap_or_default()
    }

    fn get_constellation_data(&self, corp_id: EntityId) -> String {
        let w = self.world.lock().unwrap();
        let constellations: Vec<serde_json::Value> = w.constellations
            .iter()
            .filter(|(_, c)| c.owner == corp_id)
            .map(|(&id, c)| serde_json::json!({
                "id": id, "name": c.name, "orbit_type": format!("{:?}", c.orbit_type),
                "target_altitude_km": c.target_altitude_km,
                "target_inclination_deg": c.target_inclination_deg,
                "num_planes": c.num_planes, "sats_per_plane": c.sats_per_plane,
                "total_target": c.num_planes * c.sats_per_plane,
                "operational_count": c.operational_count, "satellite_ids": c.satellite_ids,
            }))
            .collect();
        serde_json::to_string(&constellations).unwrap_or_default()
    }

    fn get_orbital_view(&self) -> String {
        let w = self.world.lock().unwrap();
        let sats: Vec<serde_json::Value> = w.satellites
            .iter()
            .map(|(&id, sat)| {
                let pos = w.positions.get(&id);
                let owner = w.ownerships.get(&id).map(|o| o.owner).unwrap_or(0);
                serde_json::json!({
                    "id": id, "owner": owner,
                    "lon": pos.map(|p| p.x).unwrap_or(0.0),
                    "lat": pos.map(|p| p.y).unwrap_or(0.0),
                    "altitude_km": sat.altitude_km,
                    "orbit_type": format!("{:?}", sat.orbit_type),
                    "status": format!("{:?}", sat.status),
                    "fuel_remaining": sat.fuel_remaining,
                    "fuel_capacity": sat.fuel_capacity,
                    "constellation_id": sat.constellation_id,
                })
            })
            .collect();
        serde_json::to_string(&sats).unwrap_or_default()
    }

    fn get_launch_schedule(&self, corp_id: EntityId) -> String {
        let w = self.world.lock().unwrap();
        let launches: Vec<serde_json::Value> = w.launch_pads
            .iter()
            .filter(|(_, lp)| lp.owner == corp_id)
            .map(|(&id, lp)| serde_json::json!({
                "launch_pad_id": id, "cooldown_remaining": lp.cooldown_remaining,
                "reusable": lp.reusable,
                "queue": lp.launch_queue.iter().map(|(rt, sats)| serde_json::json!({
                    "rocket_type": rt, "satellite_count": sats.len(),
                })).collect::<Vec<_>>(),
            }))
            .collect();
        serde_json::to_string(&launches).unwrap_or_default()
    }

    fn get_terminal_inventory(&self, corp_id: EntityId) -> String {
        let w = self.world.lock().unwrap();
        let factories: Vec<serde_json::Value> = w.terminal_factories
            .iter()
            .filter(|(_, tf)| tf.owner == corp_id)
            .map(|(&id, tf)| serde_json::json!({
                "factory_id": id, "tier": format!("{:?}", tf.tier),
                "produced_stored": tf.produced_stored,
                "production_progress": tf.production_progress,
            }))
            .collect();
        let warehouses: Vec<serde_json::Value> = w.warehouses
            .iter()
            .filter(|(_, wh)| wh.owner == corp_id)
            .map(|(&id, wh)| serde_json::json!({
                "warehouse_id": id, "region_id": wh.region_id,
                "terminal_inventory": wh.terminal_inventory,
                "distribution_rate": wh.distribution_rate,
            }))
            .collect();
        serde_json::json!({ "factories": factories, "warehouses": warehouses }).to_string()
    }

    fn get_debris_status(&self) -> String {
        let w = self.world.lock().unwrap();
        let shells: Vec<serde_json::Value> = w.orbital_shells
            .iter()
            .enumerate()
            .map(|(i, shell)| serde_json::json!({
                "index": i, "min_altitude_km": shell.min_altitude_km,
                "max_altitude_km": shell.max_altitude_km,
                "debris_count": shell.debris_count,
                "collision_probability": shell.collision_probability,
                "kessler_threshold": shell.kessler_threshold,
                "cascade_active": shell.cascade_active,
            }))
            .collect();
        serde_json::to_string(&shells).unwrap_or_default()
    }

    fn get_infra_arrays(&self) -> InfraArrays {
        let w = self.world.lock().unwrap();
        let count = w.infra_nodes.len();
        let mut ids = Vec::with_capacity(count);
        let mut owners = Vec::with_capacity(count);
        let mut positions = Vec::with_capacity(count * 2);
        let mut stats = Vec::with_capacity(count * 3);
        let mut node_types = Vec::with_capacity(count);
        let mut network_levels = Vec::with_capacity(count);
        let mut construction_flags = Vec::with_capacity(count);

        for (&eid, node) in &w.infra_nodes {
            ids.push(eid as u32);
            let owner = w.ownerships.get(&eid).map(|o| o.owner).unwrap_or(0);
            owners.push(owner as u32);
            let pos = w.positions.get(&eid);
            positions.push(pos.map(|p| p.x).unwrap_or(0.0));
            positions.push(pos.map(|p| p.y).unwrap_or(0.0));
            let health = w.healths.get(&eid).map(|h| h.condition).unwrap_or(1.0);
            let utilization = w.capacities.get(&eid).map(|c| c.utilization()).unwrap_or(0.0);
            stats.push(health);
            stats.push(utilization);
            stats.push(node.max_throughput);
            node_types.push(node.node_type as u32);
            network_levels.push(node.network_level as u32);
            construction_flags.push(if w.constructions.contains_key(&eid) { 1u8 } else { 0u8 });
        }

        InfraArrays { ids, owners, positions, stats, node_types, network_levels, construction_flags }
    }

    fn get_edge_arrays(&self) -> EdgeArrays {
        let w = self.world.lock().unwrap();
        let count = w.infra_edges.len();
        let mut ids = Vec::with_capacity(count);
        let mut owners = Vec::with_capacity(count);
        let mut endpoints = Vec::with_capacity(count * 4);
        let mut stats = Vec::with_capacity(count * 2);
        let mut edge_types = Vec::with_capacity(count);

        for (&eid, edge) in &w.infra_edges {
            ids.push(eid as u32);
            let owner = w.ownerships.get(&eid).map(|o| o.owner).unwrap_or(0);
            owners.push(owner as u32);
            let src = w.positions.get(&edge.source);
            let dst = w.positions.get(&edge.target);
            endpoints.push(src.map(|p| p.x).unwrap_or(0.0));
            endpoints.push(src.map(|p| p.y).unwrap_or(0.0));
            endpoints.push(dst.map(|p| p.x).unwrap_or(0.0));
            endpoints.push(dst.map(|p| p.y).unwrap_or(0.0));
            stats.push(edge.bandwidth);
            let utilization = w.capacities.get(&eid).map(|c| c.utilization()).unwrap_or(0.0);
            stats.push(utilization);
            edge_types.push(edge.edge_type as u32);
        }

        EdgeArrays { ids, owners, endpoints, stats, edge_types }
    }

    fn get_satellite_arrays(&self) -> SatelliteArrays {
        let w = self.world.lock().unwrap();
        let count = w.satellites.len();
        let mut ids = Vec::with_capacity(count);
        let mut owners = Vec::with_capacity(count);
        let mut positions = Vec::with_capacity(count * 2);
        let mut altitudes = Vec::with_capacity(count);
        let mut orbit_types = Vec::with_capacity(count);
        let mut statuses = Vec::with_capacity(count);
        let mut fuel_levels = Vec::with_capacity(count);

        for (&eid, sat) in &w.satellites {
            ids.push(eid as u32);
            let owner = w.ownerships.get(&eid).map(|o| o.owner).unwrap_or(0);
            owners.push(owner as u32);
            let pos = w.positions.get(&eid);
            positions.push(pos.map(|p| p.x).unwrap_or(0.0));
            positions.push(pos.map(|p| p.y).unwrap_or(0.0));
            altitudes.push(sat.altitude_km);
            orbit_types.push(sat.orbit_type as u32);
            statuses.push(sat.status as u32);
            let fuel_frac = if sat.fuel_capacity > 0.0 { sat.fuel_remaining / sat.fuel_capacity } else { 0.0 };
            fuel_levels.push(fuel_frac);
        }

        SatelliteArrays { ids, owners, positions, altitudes, orbit_types, statuses, fuel_levels }
    }
}

/// Create a new game world with the given config JSON.
pub fn cmd_new_game(config_json: &str) -> Result<TauriBridge, String> {
    let config: WorldConfig = serde_json::from_str(config_json)
        .map_err(|e| format!("Invalid config: {e}"))?;
    Ok(TauriBridge::new(config))
}
