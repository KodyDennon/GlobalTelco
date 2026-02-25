use gt_common::types::WorldConfig;
use gt_simulation::world::GameWorld;
use wasm_bindgen::prelude::*;
use js_sys::{Float64Array, Uint32Array, Uint8Array};

#[wasm_bindgen(start)]
pub fn init_panic_hook() {
    console_error_panic_hook::set_once();
}

#[wasm_bindgen]
pub struct WasmBridge {
    world: GameWorld,
}

impl Default for WasmBridge {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
impl WasmBridge {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            world: GameWorld::new(WorldConfig::default()),
        }
    }

    pub fn new_game(config_json: &str) -> Result<WasmBridge, JsValue> {
        let config: WorldConfig = serde_json::from_str(config_json)
            .map_err(|e| JsValue::from_str(&format!("Invalid config: {}", e)))?;
        Ok(Self {
            world: GameWorld::new(config),
        })
    }

    pub fn tick(&mut self) {
        self.world.tick();
    }

    pub fn current_tick(&self) -> u64 {
        self.world.current_tick()
    }

    pub fn process_command(&mut self, command_json: &str) -> Result<String, JsValue> {
        let cmd: gt_common::commands::Command = serde_json::from_str(command_json)
            .map_err(|e| JsValue::from_str(&format!("Invalid command: {}", e)))?;
        self.world.process_command(cmd);
        // Immediately drain any notifications so the frontend gets instant feedback
        // (e.g., "Insufficient funds") even when the game is paused.
        let events = self.world.event_queue.drain();
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

    /// Apply a batch of delta operations to the local world state.
    /// Used in multiplayer to incrementally update WASM state from
    /// CommandBroadcast messages without requiring a full snapshot reload.
    pub fn apply_batch(&mut self, ops_json: &str) -> Result<(), JsValue> {
        let ops: Vec<gt_common::protocol::DeltaOp> = serde_json::from_str(ops_json)
            .map_err(|e| JsValue::from_str(&format!("Invalid delta ops: {}", e)))?;
        self.world.apply_delta(&ops);
        Ok(())
    }

    pub fn get_world_info(&self) -> String {
        let info = serde_json::json!({
            "tick": self.world.current_tick(),
            "speed": format!("{:?}", self.world.speed()),
            "entity_count": self.world.entity_count(),
            "region_count": self.world.regions.len(),
            "city_count": self.world.cities.len(),
            "corporation_count": self.world.corporations.len(),
            "infra_node_count": self.world.infra_nodes.len(),
            "infra_edge_count": self.world.infra_edges.len(),
            "player_corp_id": self.world.player_corp_id(),
            "cell_spacing_km": self.world.cell_spacing_km,
        });
        serde_json::to_string(&info).unwrap_or_default()
    }

    pub fn get_corporation_data(&self, corp_id: u64) -> String {
        let corp = self.world.corporations.get(&corp_id);
        let fin = self.world.financials.get(&corp_id);
        let wf = self.world.workforces.get(&corp_id);
        let node_count = self
            .world
            .corp_infra_nodes
            .get(&corp_id)
            .map(|n| n.len())
            .unwrap_or(0);

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

    pub fn get_regions(&self) -> String {
        let regions: Vec<serde_json::Value> = self
            .world
            .regions
            .iter()
            .map(|(&id, r)| {
                let boundary: Vec<serde_json::Value> = r.boundary_polygon
                    .iter()
                    .map(|&(lat, lon)| serde_json::json!([lat, lon]))
                    .collect();
                serde_json::json!({
                    "id": id,
                    "name": r.name,
                    "center_lat": r.center_lat,
                    "center_lon": r.center_lon,
                    "population": r.population,
                    "gdp": r.gdp,
                    "development": r.development,
                    "tax_rate": r.tax_rate,
                    "regulatory_strictness": r.regulatory_strictness,
                    "disaster_risk": r.disaster_risk,
                    "cell_count": r.cells.len(),
                    "city_ids": r.city_ids,
                    "boundary_polygon": boundary,
                })
            })
            .collect();
        serde_json::to_string(&regions).unwrap_or_default()
    }

    pub fn is_real_earth(&self) -> bool {
        self.world.config().use_real_earth
    }

    pub fn get_cities(&self) -> String {
        let cities: Vec<serde_json::Value> = self
            .world
            .cities
            .iter()
            .map(|(&id, c)| {
                let pos = self.world.positions.get(&id);
                // Include cell positions for multi-cell rendering
                let cell_positions: Vec<serde_json::Value> = c.cells.iter().filter_map(|&ci| {
                    let (lat, lon) = self.world.grid_cell_positions.get(ci)?;
                    Some(serde_json::json!({"index": ci, "lat": lat, "lon": lon}))
                }).collect();
                serde_json::json!({
                    "id": id,
                    "name": c.name,
                    "region_id": c.region_id,
                    "cell_index": c.cell_index,
                    "cells": c.cells,
                    "cell_positions": cell_positions,
                    "population": c.population,
                    "growth_rate": c.growth_rate,
                    "development": c.development,
                    "telecom_demand": c.telecom_demand,
                    "infrastructure_satisfaction": c.infrastructure_satisfaction,
                    "employment_rate": c.employment_rate,
                    "jobs_available": c.jobs_available,
                    "birth_rate": c.birth_rate,
                    "death_rate": c.death_rate,
                    "migration_pressure": c.migration_pressure,
                    "x": pos.map(|p| p.x).unwrap_or(0.0),
                    "y": pos.map(|p| p.y).unwrap_or(0.0),
                })
            })
            .collect();
        serde_json::to_string(&cities).unwrap_or_default()
    }

    pub fn get_infrastructure_list(&self, corp_id: u64) -> String {
        let node_ids = self
            .world
            .corp_infra_nodes
            .get(&corp_id)
            .cloned()
            .unwrap_or_default();

        let nodes: Vec<serde_json::Value> = node_ids
            .iter()
            .filter_map(|&id| {
                let node = self.world.infra_nodes.get(&id)?;
                let pos = self.world.positions.get(&id);
                let health = self.world.healths.get(&id);
                let cap = self.world.capacities.get(&id);
                let under_construction = self.world.constructions.contains_key(&id);
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
                }))
            })
            .collect();

        let edges: Vec<serde_json::Value> = self
            .world
            .infra_edges
            .iter()
            .filter(|(_, e)| e.owner == corp_id)
            .map(|(&id, e)| {
                let src_pos = self.world.positions.get(&e.source);
                let dst_pos = self.world.positions.get(&e.target);
                let src_cell = self.world.infra_nodes.get(&e.source).map(|n| n.cell_index).unwrap_or(0);
                let dst_cell = self.world.infra_nodes.get(&e.target).map(|n| n.cell_index).unwrap_or(0);
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
                })
            })
            .collect();

        let result = serde_json::json!({
            "nodes": nodes,
            "edges": edges,
        });
        serde_json::to_string(&result).unwrap_or_default()
    }

    pub fn get_visible_entities(&self, min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> String {
        let nodes: Vec<serde_json::Value> = self
            .world
            .infra_nodes
            .iter()
            .filter_map(|(&id, node)| {
                let pos = self.world.positions.get(&id)?;
                if pos.x >= min_x && pos.x <= max_x && pos.y >= min_y && pos.y <= max_y {
                    let health = self.world.healths.get(&id);
                    let cap = self.world.capacities.get(&id);
                    Some(serde_json::json!({
                        "id": id,
                        "type": "node",
                        "node_type": format!("{:?}", node.node_type),
                        "owner": node.owner,
                        "x": pos.x,
                        "y": pos.y,
                        "health": health.map(|h| h.condition).unwrap_or(1.0),
                        "utilization": cap.map(|c| c.utilization()).unwrap_or(0.0),
                        "under_construction": self.world.constructions.contains_key(&id),
                    }))
                } else {
                    None
                }
            })
            .collect();

        let cities: Vec<serde_json::Value> = self
            .world
            .cities
            .iter()
            .filter_map(|(&id, city)| {
                let pos = self.world.positions.get(&id)?;
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

        let result = serde_json::json!({
            "nodes": nodes,
            "cities": cities,
        });
        serde_json::to_string(&result).unwrap_or_default()
    }

    pub fn get_parcels_in_view(&self, min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> String {
        let parcels: Vec<serde_json::Value> = self
            .world
            .land_parcels
            .iter()
            .filter_map(|(&id, parcel)| {
                let pos = self.world.positions.get(&id)?;
                if pos.x >= min_x && pos.x <= max_x && pos.y >= min_y && pos.y <= max_y {
                    Some(serde_json::json!({
                        "id": id,
                        "cell_index": parcel.cell_index,
                        "terrain": format!("{:?}", parcel.terrain),
                        "elevation": parcel.elevation,
                        "zoning": format!("{:?}", parcel.zoning),
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

    pub fn get_notifications(&mut self) -> String {
        let events = self.world.event_queue.drain();
        let notifications: Vec<serde_json::Value> = events
            .iter()
            .map(|(tick, event)| {
                serde_json::json!({
                    "tick": tick,
                    "event": serde_json::to_value(event).unwrap_or(serde_json::Value::Null),
                })
            })
            .collect();
        serde_json::to_string(&notifications).unwrap_or_default()
    }

    pub fn get_player_corp_id(&self) -> u64 {
        self.world.player_corp_id().unwrap_or(0)
    }

    pub fn get_all_corporations(&self) -> String {
        let corps: Vec<serde_json::Value> = self
            .world
            .corporations
            .iter()
            .map(|(&id, corp)| {
                let fin = self.world.financials.get(&id);
                serde_json::json!({
                    "id": id,
                    "name": corp.name,
                    "is_player": corp.is_player,
                    "credit_rating": format!("{:?}", corp.credit_rating),
                    "cash": fin.map(|f| f.cash).unwrap_or(0),
                    "revenue": fin.map(|f| f.revenue_per_tick).unwrap_or(0),
                    "cost": fin.map(|f| f.cost_per_tick).unwrap_or(0),
                })
            })
            .collect();
        serde_json::to_string(&corps).unwrap_or_default()
    }

    pub fn get_contracts(&self, corp_id: u64) -> String {
        let contracts: Vec<serde_json::Value> = self
            .world
            .contracts
            .iter()
            .filter(|(_, c)| c.from == corp_id || c.to == corp_id)
            .map(|(&id, c)| {
                let from_name = self
                    .world
                    .corporations
                    .get(&c.from)
                    .map(|corp| corp.name.as_str())
                    .unwrap_or("Unknown");
                let to_name = self
                    .world
                    .corporations
                    .get(&c.to)
                    .map(|corp| corp.name.as_str())
                    .unwrap_or("Unknown");
                serde_json::json!({
                    "id": id,
                    "contract_type": format!("{:?}", c.contract_type),
                    "from": c.from,
                    "to": c.to,
                    "from_name": from_name,
                    "to_name": to_name,
                    "capacity": c.capacity,
                    "price_per_tick": c.price_per_tick,
                    "start_tick": c.start_tick,
                    "end_tick": c.end_tick,
                    "status": format!("{:?}", c.status),
                    "penalty": c.penalty,
                })
            })
            .collect();
        serde_json::to_string(&contracts).unwrap_or_default()
    }

    pub fn get_debt_instruments(&self, corp_id: u64) -> String {
        let debts: Vec<serde_json::Value> = self
            .world
            .debt_instruments
            .iter()
            .filter(|(_, d)| d.holder == corp_id)
            .map(|(&id, d)| {
                serde_json::json!({
                    "id": id,
                    "principal": d.principal,
                    "interest_rate": d.interest_rate,
                    "remaining_ticks": d.remaining_ticks,
                    "payment_per_tick": d.payment_per_tick,
                    "is_paid_off": d.is_paid_off(),
                })
            })
            .collect();
        serde_json::to_string(&debts).unwrap_or_default()
    }

    pub fn get_research_state(&self) -> String {
        let research: Vec<serde_json::Value> = self
            .world
            .tech_research
            .iter()
            .map(|(&id, r)| {
                let researcher_name = r.researcher.and_then(|rid| {
                    self.world.corporations.get(&rid).map(|c| c.name.clone())
                });
                let patent_owner_name = r.patent_owner.and_then(|oid| {
                    self.world.corporations.get(&oid).map(|c| c.name.clone())
                });
                serde_json::json!({
                    "id": id,
                    "category": format!("{:?}", r.category),
                    "category_name": r.category.display_name(),
                    "name": r.name,
                    "description": r.description,
                    "progress": r.progress,
                    "total_cost": r.total_cost,
                    "progress_pct": r.progress_pct(),
                    "researcher": r.researcher,
                    "researcher_name": researcher_name,
                    "completed": r.completed,
                    "patent_status": format!("{:?}", r.patent_status),
                    "patent_owner": r.patent_owner,
                    "patent_owner_name": patent_owner_name,
                    "license_price": r.license_price,
                    "prerequisites": r.prerequisites,
                    "throughput_bonus": r.throughput_bonus,
                    "cost_reduction": r.cost_reduction,
                    "reliability_bonus": r.reliability_bonus,
                })
            })
            .collect();
        serde_json::to_string(&research).unwrap_or_default()
    }

    /// Get buildable node options for a given map coordinate.
    /// Uses nearest grid cell for terrain cost modifier lookup.
    pub fn get_buildable_nodes(&self, lon: f64, lat: f64) -> String {
        let player_id = self.world.player_corp_id().unwrap_or(0);
        let fin = self.world.financials.get(&player_id);
        let cash = fin.map(|f| f.cash).unwrap_or(0);

        // Find nearest cell for terrain cost modifier
        let terrain_mult = self.world.find_nearest_cell(lon, lat)
            .and_then(|(cell_idx, _)| {
                self.world.cell_to_parcel.get(&cell_idx)
                    .and_then(|&pid| self.world.land_parcels.get(&pid))
                    .map(|p| p.cost_modifier)
            })
            .unwrap_or(1.0);

        // (label, node_type, network_level, base_cost, build_ticks)
        let node_types = [
            ("Cell Tower", "CellTower", "Local", 200_000i64, 10),
            ("Wireless Relay", "WirelessRelay", "Local", 100_000, 10),
            ("Central Office", "CentralOffice", "Local", 500_000, 20),
            ("Exchange Point", "ExchangePoint", "Regional", 2_000_000, 30),
            ("Backbone Router", "BackboneRouter", "National", 3_000_000, 50),
            ("Data Center", "DataCenter", "National", 10_000_000, 50),
            ("Satellite Ground", "SatelliteGround", "Continental", 5_000_000, 40),
            ("Submarine Landing", "SubmarineLanding", "Global", 20_000_000, 60),
        ];

        let options: Vec<serde_json::Value> = node_types
            .iter()
            .map(|(label, node_type, level, base_cost, ticks)| {
                let cost = (*base_cost as f64 * terrain_mult) as i64;
                serde_json::json!({
                    "label": label,
                    "node_type": node_type,
                    "network_level": level,
                    "cost": cost,
                    "build_ticks": ticks,
                    "affordable": cash >= cost,
                })
            })
            .collect();

        serde_json::to_string(&options).unwrap_or_default()
    }

    pub fn get_buildable_edges(&self, source_id: u64) -> String {
        use gt_common::types::EdgeType;

        let player_id = self.world.player_corp_id().unwrap_or(0);
        let fin = self.world.financials.get(&player_id);
        let cash = fin.map(|f| f.cash).unwrap_or(0);

        // Find player-owned nodes that could be connected to source
        let player_nodes = self
            .world
            .corp_infra_nodes
            .get(&player_id)
            .cloned()
            .unwrap_or_default();

        // Use the cheapest common edge type (Copper) for the base cost estimate,
        // but also include per-type costs for accurate UI display.
        let edge_types_costs: &[(EdgeType, i64)] = &[
            (EdgeType::Copper, 10_000),
            (EdgeType::FiberLocal, 20_000),
            (EdgeType::FiberRegional, 40_000),
            (EdgeType::FiberNational, 80_000),
            (EdgeType::Microwave, 20_000),
            (EdgeType::Satellite, 0),     // flat cost
            (EdgeType::Submarine, 200_000),
        ];

        let targets: Vec<serde_json::Value> = player_nodes
            .iter()
            .filter(|&&nid| nid != source_id && !self.world.constructions.contains_key(&nid))
            .filter_map(|&nid| {
                let node = self.world.infra_nodes.get(&nid)?;
                let pos = self.world.positions.get(&nid)?;
                let src_pos = self.world.positions.get(&source_id)?;

                // Haversine distance (matches cmd_build_edge)
                let dlat = (src_pos.y - pos.y).to_radians();
                let dlon = (src_pos.x - pos.x).to_radians();
                let lat1 = src_pos.y.to_radians();
                let lat2 = pos.y.to_radians();
                let a = (dlat / 2.0).sin().powi(2)
                    + lat1.cos() * lat2.cos() * (dlon / 2.0).sin().powi(2);
                let c = 2.0 * a.sqrt().asin();
                let dist_km = 6371.0 * c;

                // Use cheapest applicable cost for the summary estimate
                let min_cost = edge_types_costs.iter().map(|(et, cpk)| {
                    if *et == EdgeType::Satellite { 5_000_000i64 }
                    else { (*cpk as f64 * dist_km) as i64 }
                }).min().unwrap_or(0);

                Some(serde_json::json!({
                    "target_id": nid,
                    "target_type": format!("{:?}", node.node_type),
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

    pub fn save_game(&self) -> Result<String, JsValue> {
        self.world
            .save_game()
            .map_err(|e| JsValue::from_str(&e))
    }

    pub fn load_game(data: &str) -> Result<WasmBridge, JsValue> {
        let world =
            GameWorld::load_game(data).map_err(|e| JsValue::from_str(&e))?;
        Ok(Self { world })
    }

    pub fn get_damaged_nodes(&self, corp_id: u64) -> String {
        let node_ids = self
            .world
            .corp_infra_nodes
            .get(&corp_id)
            .cloned()
            .unwrap_or_default();

        let damaged: Vec<serde_json::Value> = node_ids
            .iter()
            .filter_map(|&id| {
                let node = self.world.infra_nodes.get(&id)?;
                let health = self.world.healths.get(&id)?;
                if health.condition >= 0.95 {
                    return None;
                }
                let pos = self.world.positions.get(&id);
                let base_cost = node.construction_cost;
                let damage = 1.0 - health.condition;
                let repair_cost = (base_cost as f64 * damage * 0.2) as i64;
                let emergency_cost = (base_cost as f64 * damage * 0.6) as i64;
                Some(serde_json::json!({
                    "id": id,
                    "node_type": format!("{:?}", node.node_type),
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

    // === Phase 10 queries ===

    pub fn get_auctions(&self) -> String {
        let auctions: Vec<serde_json::Value> = self
            .world
            .auctions
            .iter()
            .map(|(&id, a)| {
                let seller_name = self
                    .world
                    .corporations
                    .get(&a.seller)
                    .map(|c| c.name.as_str())
                    .unwrap_or("Unknown");
                let highest = a.highest_bid();
                serde_json::json!({
                    "id": id,
                    "seller": a.seller,
                    "seller_name": seller_name,
                    "asset_count": a.assets.len(),
                    "bid_count": a.bids.len(),
                    "highest_bid": highest.map(|(_, amt)| amt).unwrap_or(0),
                    "highest_bidder": highest.map(|(id, _)| id).unwrap_or(0),
                    "start_tick": a.start_tick,
                    "end_tick": a.end_tick,
                    "status": format!("{:?}", a.status),
                })
            })
            .collect();
        serde_json::to_string(&auctions).unwrap_or_default()
    }

    pub fn get_acquisition_proposals(&self) -> String {
        let proposals: Vec<serde_json::Value> = self
            .world
            .acquisition_proposals
            .iter()
            .map(|(&id, p)| {
                let acquirer_name = self
                    .world
                    .corporations
                    .get(&p.acquirer)
                    .map(|c| c.name.as_str())
                    .unwrap_or("Unknown");
                let target_name = self
                    .world
                    .corporations
                    .get(&p.target)
                    .map(|c| c.name.as_str())
                    .unwrap_or("Unknown");
                serde_json::json!({
                    "id": id,
                    "acquirer": p.acquirer,
                    "acquirer_name": acquirer_name,
                    "target": p.target,
                    "target_name": target_name,
                    "offer": p.offer,
                    "status": format!("{:?}", p.status),
                    "tick": p.tick,
                })
            })
            .collect();
        serde_json::to_string(&proposals).unwrap_or_default()
    }

    pub fn get_covert_ops(&self, corp_id: u64) -> String {
        let ops = self.world.covert_ops.get(&corp_id);
        let data = serde_json::json!({
            "security_level": ops.map(|o| o.security_level).unwrap_or(0),
            "active_missions": ops.map(|o| o.active_missions.len()).unwrap_or(0),
            "detection_count": ops.map(|o| o.detection_history.len()).unwrap_or(0),
        });
        serde_json::to_string(&data).unwrap_or_default()
    }

    pub fn get_lobbying_campaigns(&self, corp_id: u64) -> String {
        let campaigns: Vec<serde_json::Value> = self
            .world
            .lobbying_campaigns
            .iter()
            .filter(|(_, c)| c.corporation == corp_id)
            .map(|(&id, c)| {
                let region_name = self
                    .world
                    .regions
                    .get(&c.region)
                    .map(|r| r.name.as_str())
                    .unwrap_or("Unknown");
                serde_json::json!({
                    "id": id,
                    "region": c.region,
                    "region_name": region_name,
                    "policy": format!("{:?}", c.policy),
                    "budget_spent": c.budget_spent,
                    "budget_total": c.budget_total,
                    "influence": c.influence,
                    "threshold": c.influence_threshold(),
                    "active": c.active,
                })
            })
            .collect();
        serde_json::to_string(&campaigns).unwrap_or_default()
    }

    pub fn get_achievements(&self, corp_id: u64) -> String {
        let tracker = self.world.achievements.get(&corp_id);
        let data = serde_json::json!({
            "unlocked": tracker.map(|t| t.unlocked.iter().cloned().collect::<Vec<_>>()).unwrap_or_default(),
            "progress": tracker.map(|t| t.progress.clone()).unwrap_or_default(),
        });
        serde_json::to_string(&data).unwrap_or_default()
    }

    pub fn get_victory_state(&self) -> String {
        let state = self.world.victory_state.as_ref();
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

    /// Get per-cell coverage data for the coverage heatmap overlay.
    /// Returns coverage for cells that have any signal.
    pub fn get_cell_coverage(&self) -> String {
        let coverage: Vec<serde_json::Value> = self
            .world
            .cell_coverage
            .iter()
            .filter_map(|(&cell_idx, cov)| {
                let (lat, lon) = self.world.grid_cell_positions.get(cell_idx)?;
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

    /// Get all infrastructure from all corporations (for rendering all nodes/edges on the map).
    pub fn get_all_infrastructure(&self) -> String {
        let nodes: Vec<serde_json::Value> = self
            .world
            .infra_nodes
            .iter()
            .filter_map(|(&id, node)| {
                let pos = self.world.positions.get(&id)?;
                let health = self.world.healths.get(&id);
                let cap = self.world.capacities.get(&id);
                let under_construction = self.world.constructions.contains_key(&id);
                let owner_name = self.world.corporations.get(&node.owner)
                    .map(|c| c.name.as_str()).unwrap_or("Unknown");
                Some(serde_json::json!({
                    "id": id,
                    "node_type": format!("{:?}", node.node_type),
                    "network_level": format!("{:?}", node.network_level),
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
                }))
            })
            .collect();

        let edges: Vec<serde_json::Value> = self
            .world
            .infra_edges
            .iter()
            .map(|(&id, e)| {
                let src_pos = self.world.positions.get(&e.source);
                let dst_pos = self.world.positions.get(&e.target);
                let owner_name = self.world.corporations.get(&e.owner)
                    .map(|c| c.name.as_str()).unwrap_or("Unknown");
                let src_cell = self.world.infra_nodes.get(&e.source).map(|n| n.cell_index).unwrap_or(0);
                let dst_cell = self.world.infra_nodes.get(&e.target).map(|n| n.cell_index).unwrap_or(0);
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
                    "owner": e.owner,
                    "owner_name": owner_name,
                    "src_x": src_pos.map(|p| p.x).unwrap_or(0.0),
                    "src_y": src_pos.map(|p| p.y).unwrap_or(0.0),
                    "dst_x": dst_pos.map(|p| p.x).unwrap_or(0.0),
                    "dst_y": dst_pos.map(|p| p.y).unwrap_or(0.0),
                    "src_cell": src_cell,
                    "dst_cell": dst_cell,
                    "waypoints": e.waypoints.iter().map(|&(lon, lat)| [lon, lat]).collect::<Vec<_>>(),
                    "deployment": format!("{:?}", e.deployment),
                })
            })
            .collect();

        let result = serde_json::json!({
            "nodes": nodes,
            "edges": edges,
        });
        serde_json::to_string(&result).unwrap_or_default()
    }

    /// Get traffic flow data for the traffic overlay and infrastructure panel.
    pub fn get_traffic_flows(&self) -> String {
        let tm = &self.world.traffic_matrix;

        // Per-edge traffic with utilization coloring
        let edge_flows: Vec<serde_json::Value> = self
            .world
            .infra_edges
            .iter()
            .map(|(&id, e)| {
                let traffic = tm.edge_traffic.get(&id).copied().unwrap_or(0.0);
                let utilization = if e.bandwidth > 0.0 {
                    traffic / e.bandwidth
                } else {
                    0.0
                };
                let src_pos = self.world.positions.get(&e.source);
                let dst_pos = self.world.positions.get(&e.target);
                serde_json::json!({
                    "id": id,
                    "traffic": traffic,
                    "bandwidth": e.bandwidth,
                    "utilization": utilization,
                    "health": e.health,
                    "edge_type": format!("{:?}", e.edge_type),
                    "owner": e.owner,
                    "src_x": src_pos.map(|p| p.x).unwrap_or(0.0),
                    "src_y": src_pos.map(|p| p.y).unwrap_or(0.0),
                    "dst_x": dst_pos.map(|p| p.x).unwrap_or(0.0),
                    "dst_y": dst_pos.map(|p| p.y).unwrap_or(0.0),
                })
            })
            .collect();

        // Per-node traffic
        let node_flows: Vec<serde_json::Value> = self
            .world
            .infra_nodes
            .iter()
            .filter_map(|(&id, node)| {
                let traffic = tm.node_traffic.get(&id).copied().unwrap_or(0.0);
                let pos = self.world.positions.get(&id)?;
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
                    "node_type": format!("{:?}", node.node_type),
                    "owner": node.owner,
                    "x": pos.x,
                    "y": pos.y,
                }))
            })
            .collect();

        // Summary stats
        let player_id = self.world.player_corp_id().unwrap_or(0);
        let player_served = tm.corp_traffic_served.get(&player_id).copied().unwrap_or(0.0);
        let player_dropped = tm.corp_traffic_dropped.get(&player_id).copied().unwrap_or(0.0);

        // Top congested edges (sorted by utilization, top 5)
        let mut congested: Vec<(u64, f64)> = self
            .world
            .infra_edges
            .iter()
            .map(|(&id, e)| {
                let traffic = tm.edge_traffic.get(&id).copied().unwrap_or(0.0);
                let util = if e.bandwidth > 0.0 { traffic / e.bandwidth } else { 0.0 };
                (id, util)
            })
            .collect();
        congested.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        let top_congested: Vec<serde_json::Value> = congested
            .iter()
            .take(5)
            .filter(|(_, util)| *util > 0.0)
            .map(|(id, util)| {
                let edge = self.world.infra_edges.get(id);
                serde_json::json!({
                    "id": id,
                    "utilization": util,
                    "edge_type": edge.map(|e| format!("{:?}", e.edge_type)).unwrap_or_default(),
                    "owner": edge.map(|e| e.owner).unwrap_or(0),
                })
            })
            .collect();

        let result = serde_json::json!({
            "edge_flows": edge_flows,
            "node_flows": node_flows,
            "total_served": tm.total_served,
            "total_dropped": tm.total_dropped,
            "total_demand": tm.total_served + tm.total_dropped,
            "player_served": player_served,
            "player_dropped": player_dropped,
            "top_congested": top_congested,
        });
        serde_json::to_string(&result).unwrap_or_default()
    }

    pub fn get_grid_cells(&self) -> String {
        // Build cell_index → terrain lookup once (O(parcels)) instead of scanning
        // all parcels for every cell (which was O(cells × parcels)).
        let mut cell_terrain: std::collections::HashMap<usize, String> =
            std::collections::HashMap::new();
        for parcel in self.world.land_parcels.values() {
            cell_terrain.insert(parcel.cell_index, format!("{:?}", parcel.terrain));
        }

        let cells: Vec<serde_json::Value> = self
            .world
            .grid_cell_positions
            .iter()
            .enumerate()
            .map(|(i, &(lat, lon))| {
                let terrain = cell_terrain
                    .get(&i)
                    .cloned()
                    .unwrap_or_else(|| "Ocean".to_string());
                let neighbors = self.world.grid_cell_neighbors.get(i)
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

    /// Generate a low-detail world preview for the new-game screen.
    /// Takes a WorldConfig JSON, runs a fast generation, returns preview data JSON.
    pub fn create_world_preview(config_json: &str) -> Result<String, JsValue> {
        let config: WorldConfig = serde_json::from_str(config_json)
            .map_err(|e| JsValue::from_str(&format!("Invalid config: {}", e)))?;

        if config.use_real_earth {
            // Real earth doesn't need a procgen preview
            return Ok(serde_json::json!({
                "is_real_earth": true,
                "cells": [],
                "width": 0,
                "height": 0,
            }).to_string());
        }

        let gen = gt_world::WorldGenerator::new(config);
        let world = gen.generate();

        // Build preview: cell terrain colors + positions
        let preview_cells: Vec<serde_json::Value> = world.grid.cells.iter().enumerate().map(|(i, cell)| {
            let terrain = &world.terrains[i];
            let elevation = world.elevations[i];
            serde_json::json!({
                "lat": cell.lat,
                "lon": cell.lon,
                "terrain": format!("{:?}", terrain),
                "elevation": elevation,
            })
        }).collect();

        let city_previews: Vec<serde_json::Value> = world.cities.iter().map(|city| {
            let cell = &world.grid.cells[city.cell_index];
            serde_json::json!({
                "name": city.name,
                "lat": cell.lat,
                "lon": cell.lon,
                "population": city.population,
            })
        }).collect();

        let result = serde_json::json!({
            "is_real_earth": false,
            "cell_count": world.grid.cell_count(),
            "cells": preview_cells,
            "cities": city_previews,
            "region_count": world.regions.len(),
        });
        Ok(result.to_string())
    }

    /// Export the current game world's geography as GeoJSON for MapLibre rendering.
    /// Returns a GeoJSON FeatureCollection with region polygons, city points, etc.
    pub fn get_world_geojson(&self) -> String {
        let mut features: Vec<serde_json::Value> = Vec::new();

        // Region polygons
        for (&id, region) in &self.world.regions {
            if region.boundary_polygon.is_empty() {
                continue;
            }
            // Convert boundary to GeoJSON coordinates [lon, lat]
            let coords: Vec<Vec<f64>> = region.boundary_polygon.iter()
                .map(|&(lat, lon)| vec![lon, lat])
                .collect();
            // Close the polygon ring
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

        // City points
        for (&id, city) in &self.world.cities {
            if let Some(pos) = self.world.positions.get(&id) {
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

        let geojson = serde_json::json!({
            "type": "FeatureCollection",
            "features": features,
        });
        serde_json::to_string(&geojson).unwrap_or_default()
    }

    // ── Typed Array Exports (zero-copy hot-path rendering) ──────────────

    /// Returns infrastructure node data as parallel typed arrays.
    /// Output: { count, ids: Uint32Array, owners: Uint32Array, positions: Float64Array,
    ///           stats: Float64Array, node_types: Uint32Array, network_levels: Uint32Array,
    ///           construction_flags: Uint8Array }
    /// positions layout: [lon0, lat0, lon1, lat1, ...] (2 floats per node)
    /// stats layout: [health0, utilization0, throughput0, ...] (3 floats per node)
    pub fn get_infra_nodes_typed(&self) -> JsValue {
        let w = &self.world;
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

        let obj = js_sys::Object::new();
        let _ = js_sys::Reflect::set(&obj, &"count".into(), &JsValue::from(count as u32));
        let _ = js_sys::Reflect::set(&obj, &"ids".into(), &Uint32Array::from(&ids[..]).into());
        let _ = js_sys::Reflect::set(&obj, &"owners".into(), &Uint32Array::from(&owners[..]).into());
        let _ = js_sys::Reflect::set(&obj, &"positions".into(), &Float64Array::from(&positions[..]).into());
        let _ = js_sys::Reflect::set(&obj, &"stats".into(), &Float64Array::from(&stats[..]).into());
        let _ = js_sys::Reflect::set(&obj, &"node_types".into(), &Uint32Array::from(&node_types[..]).into());
        let _ = js_sys::Reflect::set(&obj, &"network_levels".into(), &Uint32Array::from(&network_levels[..]).into());
        let _ = js_sys::Reflect::set(&obj, &"construction_flags".into(), &Uint8Array::from(&construction_flags[..]).into());
        obj.into()
    }

    /// Returns infrastructure edge data as parallel typed arrays.
    /// endpoints layout: [src_lon0, src_lat0, dst_lon0, dst_lat0, ...] (4 floats per edge)
    /// stats layout: [bandwidth0, utilization0, ...] (2 floats per edge)
    pub fn get_infra_edges_typed(&self) -> JsValue {
        let w = &self.world;
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

            let src_pos = w.positions.get(&edge.source);
            let dst_pos = w.positions.get(&edge.target);
            endpoints.push(src_pos.map(|p| p.x).unwrap_or(0.0));
            endpoints.push(src_pos.map(|p| p.y).unwrap_or(0.0));
            endpoints.push(dst_pos.map(|p| p.x).unwrap_or(0.0));
            endpoints.push(dst_pos.map(|p| p.y).unwrap_or(0.0));

            stats.push(edge.bandwidth);
            let utilization = w.capacities.get(&eid).map(|c| c.utilization()).unwrap_or(0.0);
            stats.push(utilization);

            edge_types.push(edge.edge_type as u32);
        }

        let obj = js_sys::Object::new();
        let _ = js_sys::Reflect::set(&obj, &"count".into(), &JsValue::from(count as u32));
        let _ = js_sys::Reflect::set(&obj, &"ids".into(), &Uint32Array::from(&ids[..]).into());
        let _ = js_sys::Reflect::set(&obj, &"owners".into(), &Uint32Array::from(&owners[..]).into());
        let _ = js_sys::Reflect::set(&obj, &"endpoints".into(), &Float64Array::from(&endpoints[..]).into());
        let _ = js_sys::Reflect::set(&obj, &"stats".into(), &Float64Array::from(&stats[..]).into());
        let _ = js_sys::Reflect::set(&obj, &"edge_types".into(), &Uint32Array::from(&edge_types[..]).into());
        obj.into()
    }

    /// Returns all corporation summary data as typed arrays for fast leaderboard rendering.
    /// ids: Uint32Array, cash/revenue/cost: Float64Array (3 floats per corp)
    pub fn get_corporations_typed(&self) -> JsValue {
        let w = &self.world;
        let count = w.corporations.len();
        let mut ids = Vec::with_capacity(count);
        let mut financials = Vec::with_capacity(count * 3);
        let mut names_packed = Vec::new();
        let mut name_offsets = Vec::with_capacity(count * 2); // [offset, length] pairs

        for (&cid, corp) in &w.corporations {
            ids.push(cid as u32);
            let fin = w.financials.get(&cid);
            financials.push(fin.map(|f| f.cash as f64).unwrap_or(0.0));
            financials.push(fin.map(|f| f.revenue_per_tick as f64).unwrap_or(0.0));
            financials.push(fin.map(|f| f.cost_per_tick as f64).unwrap_or(0.0));

            let name_bytes = corp.name.as_bytes();
            name_offsets.push(names_packed.len() as u32);
            name_offsets.push(name_bytes.len() as u32);
            names_packed.extend_from_slice(name_bytes);
        }

        let obj = js_sys::Object::new();
        let _ = js_sys::Reflect::set(&obj, &"count".into(), &JsValue::from(count as u32));
        let _ = js_sys::Reflect::set(&obj, &"ids".into(), &Uint32Array::from(&ids[..]).into());
        let _ = js_sys::Reflect::set(&obj, &"financials".into(), &Float64Array::from(&financials[..]).into());
        let _ = js_sys::Reflect::set(&obj, &"name_offsets".into(), &Uint32Array::from(&name_offsets[..]).into());
        let _ = js_sys::Reflect::set(&obj, &"names_packed".into(), &Uint8Array::from(&names_packed[..]).into());
        obj.into()
    }

    // ── Phase 8: Spectrum & Frequency Management queries ──────────────

    /// Get all spectrum licenses (active and expired).
    pub fn get_spectrum_licenses(&self) -> String {
        let tick = self.world.current_tick();
        let licenses: Vec<serde_json::Value> = self
            .world
            .spectrum_licenses
            .iter()
            .filter(|(_, l)| l.is_active(tick))
            .map(|(&id, l)| {
                let region_name = self
                    .world
                    .regions
                    .get(&l.region_id)
                    .map(|r| r.name.as_str())
                    .unwrap_or("Unknown");
                let owner_name = self
                    .world
                    .corporations
                    .get(&l.owner)
                    .map(|c| c.name.as_str())
                    .unwrap_or("Unknown");
                serde_json::json!({
                    "id": id,
                    "band": format!("{:?}", l.band),
                    "band_name": l.band.display_name(),
                    "band_category": l.band.category(),
                    "region_id": l.region_id,
                    "region_name": region_name,
                    "owner": l.owner,
                    "owner_name": owner_name,
                    "bandwidth_mhz": l.bandwidth_mhz,
                    "start_tick": l.start_tick,
                    "end_tick": l.end_tick(),
                    "cost_per_tick": l.cost_per_tick(),
                    "coverage_radius_km": l.band.coverage_radius_km(),
                })
            })
            .collect();
        serde_json::to_string(&licenses).unwrap_or_default()
    }

    /// Get all active spectrum auctions.
    pub fn get_spectrum_auctions(&self) -> String {
        let tick = self.world.current_tick();
        let auctions: Vec<serde_json::Value> = self
            .world
            .spectrum_auctions
            .iter()
            .filter(|(_, a)| !a.is_ended(tick))
            .map(|(&id, a)| {
                let region_name = self
                    .world
                    .regions
                    .get(&a.region_id)
                    .map(|r| r.name.as_str())
                    .unwrap_or("Unknown");
                let (highest_bidder, current_bid) = a.highest_bid().unwrap_or((0, 0));
                let bidder_name = self
                    .world
                    .corporations
                    .get(&highest_bidder)
                    .map(|c| c.name.as_str())
                    .unwrap_or("None");
                serde_json::json!({
                    "id": id,
                    "band": format!("{:?}", a.band),
                    "band_name": a.band.display_name(),
                    "band_category": a.band.category(),
                    "region_id": a.region_id,
                    "region_name": region_name,
                    "bandwidth_mhz": a.bandwidth_mhz,
                    "current_bid": current_bid,
                    "highest_bidder": highest_bidder,
                    "bidder_name": bidder_name,
                    "end_tick": a.end_tick,
                    "ticks_remaining": a.ticks_remaining(tick),
                    "coverage_radius_km": a.band.coverage_radius_km(),
                })
            })
            .collect();
        serde_json::to_string(&auctions).unwrap_or_default()
    }

    /// Get available frequency bands for a region (not currently licensed or in auction).
    pub fn get_available_spectrum(&self, region_id: u64) -> String {
        use gt_common::types::FrequencyBand;
        let tick = self.world.current_tick();

        let licensed_bands: std::collections::HashSet<String> = self
            .world
            .spectrum_licenses
            .values()
            .filter(|l| l.region_id == region_id && l.is_active(tick))
            .map(|l| format!("{:?}", l.band))
            .collect();

        let auction_bands: std::collections::HashSet<String> = self
            .world
            .spectrum_auctions
            .values()
            .filter(|a| a.region_id == region_id && !a.is_ended(tick))
            .map(|a| format!("{:?}", a.band))
            .collect();

        let available: Vec<serde_json::Value> = FrequencyBand::all()
            .iter()
            .filter(|b| {
                let name = format!("{:?}", b);
                !licensed_bands.contains(&name) && !auction_bands.contains(&name)
            })
            .map(|b| {
                serde_json::json!({
                    "band": format!("{:?}", b),
                    "band_name": b.display_name(),
                    "band_category": b.category(),
                    "coverage_radius_km": b.coverage_radius_km(),
                    "max_bandwidth_mhz": b.max_bandwidth_mhz(),
                    "min_bid": b.cost_per_mhz() * b.max_bandwidth_mhz() as i64,
                })
            })
            .collect();
        serde_json::to_string(&available).unwrap_or_default()
    }

    /// Get disaster forecasts: server-side prediction of upcoming disasters.
    /// Returns JSON array of { region_id, region_name, predicted_tick, probability, disaster_type }.
    pub fn get_disaster_forecasts(&self) -> String {
        let forecasts = self.world.get_disaster_forecasts();
        let json: Vec<serde_json::Value> = forecasts
            .iter()
            .map(|f| {
                serde_json::json!({
                    "region_id": f.region_id,
                    "region_name": f.region_name,
                    "predicted_tick": f.predicted_tick,
                    "probability": f.probability,
                    "disaster_type": f.disaster_type,
                })
            })
            .collect();
        serde_json::to_string(&json).unwrap_or_default()
    }

    // ── Road Network Queries (Fiber Auto-Routing) ────────────────────────

    /// A* pathfinding along the road network between two (lon, lat) points.
    /// Returns JSON array of [lon, lat] waypoints for fiber auto-routing.
    pub fn road_pathfind(
        &self,
        from_lon: f64,
        from_lat: f64,
        to_lon: f64,
        to_lat: f64,
    ) -> String {
        let waypoints = self.world.road_pathfind(from_lon, from_lat, to_lon, to_lat);
        let json: Vec<serde_json::Value> = waypoints
            .iter()
            .map(|(lon, lat)| serde_json::json!([lon, lat]))
            .collect();
        serde_json::to_string(&json).unwrap_or_default()
    }

    /// Cost of routing fiber along roads between two points.
    /// Returns a single f64 value representing the weighted km cost.
    pub fn road_fiber_route_cost(
        &self,
        from_lon: f64,
        from_lat: f64,
        to_lon: f64,
        to_lat: f64,
    ) -> f64 {
        self.world.road_fiber_route_cost(from_lon, from_lat, to_lon, to_lat)
    }

    /// Get all road segments for map rendering.
    /// Returns JSON array of { id, from: [lon, lat], to: [lon, lat], road_class, length_km, region_id }.
    pub fn get_road_segments(&self) -> String {
        let segments = self.world.get_road_segments();
        let json: Vec<serde_json::Value> = segments
            .iter()
            .map(|s| {
                serde_json::json!({
                    "id": s.id,
                    "from": [s.from.0, s.from.1],
                    "to": [s.to.0, s.to.1],
                    "road_class": format!("{:?}", s.road_class),
                    "length_km": s.length_km,
                    "region_id": s.region_id,
                })
            })
            .collect();
        serde_json::to_string(&json).unwrap_or_default()
    }
}

// ── BridgeQuery Trait Implementation ────────────────────────────────────

impl gt_bridge::BridgeQuery for WasmBridge {
    fn tick(&mut self) {
        self.world.tick();
    }

    fn current_tick(&self) -> u64 {
        self.world.current_tick()
    }

    fn process_command(&mut self, command_json: &str) -> Result<String, String> {
        let cmd: gt_common::commands::Command = serde_json::from_str(command_json)
            .map_err(|e| format!("Invalid command: {}", e))?;
        self.world.process_command(cmd);
        let events = self.world.event_queue.drain();
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
            .map_err(|e| format!("Invalid delta ops: {}", e))?;
        self.world.apply_delta(&ops);
        Ok(())
    }

    fn get_world_info(&self) -> String {
        WasmBridge::get_world_info(self)
    }

    fn get_corporation_data(&self, corp_id: gt_common::types::EntityId) -> String {
        WasmBridge::get_corporation_data(self, corp_id)
    }

    fn get_regions(&self) -> String {
        WasmBridge::get_regions(self)
    }

    fn get_cities(&self) -> String {
        WasmBridge::get_cities(self)
    }

    fn get_all_corporations(&self) -> String {
        WasmBridge::get_all_corporations(self)
    }

    fn get_research_state(&self) -> String {
        WasmBridge::get_research_state(self)
    }

    fn get_contracts(&self, corp_id: gt_common::types::EntityId) -> String {
        WasmBridge::get_contracts(self, corp_id)
    }

    fn get_debt_instruments(&self, corp_id: gt_common::types::EntityId) -> String {
        WasmBridge::get_debt_instruments(self, corp_id)
    }

    fn get_notifications(&mut self) -> String {
        WasmBridge::get_notifications(self)
    }

    fn get_buildable_nodes(&self, lon: f64, lat: f64) -> String {
        WasmBridge::get_buildable_nodes(self, lon, lat)
    }

    fn get_buildable_edges(&self, source_id: gt_common::types::EntityId) -> String {
        WasmBridge::get_buildable_edges(self, source_id)
    }

    fn get_damaged_nodes(&self, corp_id: gt_common::types::EntityId) -> String {
        WasmBridge::get_damaged_nodes(self, corp_id)
    }

    fn get_auctions(&self) -> String {
        WasmBridge::get_auctions(self)
    }

    fn get_covert_ops(&self, corp_id: gt_common::types::EntityId) -> String {
        WasmBridge::get_covert_ops(self, corp_id)
    }

    fn get_lobbying_campaigns(&self, corp_id: gt_common::types::EntityId) -> String {
        WasmBridge::get_lobbying_campaigns(self, corp_id)
    }

    fn get_achievements(&self, corp_id: gt_common::types::EntityId) -> String {
        WasmBridge::get_achievements(self, corp_id)
    }

    fn get_victory_state(&self) -> String {
        WasmBridge::get_victory_state(self)
    }

    fn get_traffic_flows(&self) -> String {
        WasmBridge::get_traffic_flows(self)
    }

    fn save_game(&self) -> Result<String, String> {
        self.world.save_game().map_err(|e| format!("Save failed: {}", e))
    }

    fn load_game(&mut self, data: &str) -> Result<(), String> {
        self.world = GameWorld::load_game(data).map_err(|e| format!("Load failed: {}", e))?;
        Ok(())
    }

    fn get_infra_arrays(&self) -> gt_bridge::InfraArrays {
        let w = &self.world;
        let count = w.infra_nodes.len();
        let mut ids = Vec::with_capacity(count);
        let mut owners_vec = Vec::with_capacity(count);
        let mut positions = Vec::with_capacity(count * 2);
        let mut stats = Vec::with_capacity(count * 3);
        let mut node_types_vec = Vec::with_capacity(count);
        let mut network_levels_vec = Vec::with_capacity(count);
        let mut construction_flags = Vec::with_capacity(count);

        for (&eid, node) in &w.infra_nodes {
            ids.push(eid as u32);
            let owner = w.ownerships.get(&eid).map(|o| o.owner).unwrap_or(0);
            owners_vec.push(owner as u32);
            let pos = w.positions.get(&eid);
            positions.push(pos.map(|p| p.x).unwrap_or(0.0));
            positions.push(pos.map(|p| p.y).unwrap_or(0.0));
            let health = w.healths.get(&eid).map(|h| h.condition).unwrap_or(1.0);
            let utilization = w.capacities.get(&eid).map(|c| c.utilization()).unwrap_or(0.0);
            stats.push(health);
            stats.push(utilization);
            stats.push(node.max_throughput);
            node_types_vec.push(node.node_type as u32);
            network_levels_vec.push(node.network_level as u32);
            construction_flags.push(if w.constructions.contains_key(&eid) { 1u8 } else { 0u8 });
        }

        gt_bridge::InfraArrays {
            ids,
            owners: owners_vec,
            positions,
            stats,
            node_types: node_types_vec,
            network_levels: network_levels_vec,
            construction_flags,
        }
    }

    fn get_edge_arrays(&self) -> gt_bridge::EdgeArrays {
        let w = &self.world;
        let count = w.infra_edges.len();
        let mut ids = Vec::with_capacity(count);
        let mut owners_vec = Vec::with_capacity(count);
        let mut endpoints = Vec::with_capacity(count * 4);
        let mut stats = Vec::with_capacity(count * 2);
        let mut edge_types_vec = Vec::with_capacity(count);

        for (&eid, edge) in &w.infra_edges {
            ids.push(eid as u32);
            let owner = w.ownerships.get(&eid).map(|o| o.owner).unwrap_or(0);
            owners_vec.push(owner as u32);
            let src_pos = w.positions.get(&edge.source);
            let dst_pos = w.positions.get(&edge.target);
            endpoints.push(src_pos.map(|p| p.x).unwrap_or(0.0));
            endpoints.push(src_pos.map(|p| p.y).unwrap_or(0.0));
            endpoints.push(dst_pos.map(|p| p.x).unwrap_or(0.0));
            endpoints.push(dst_pos.map(|p| p.y).unwrap_or(0.0));
            stats.push(edge.bandwidth);
            let utilization = w.capacities.get(&eid).map(|c| c.utilization()).unwrap_or(0.0);
            stats.push(utilization);
            edge_types_vec.push(edge.edge_type as u32);
        }

        gt_bridge::EdgeArrays {
            ids,
            owners: owners_vec,
            endpoints,
            stats,
            edge_types: edge_types_vec,
        }
    }
}
