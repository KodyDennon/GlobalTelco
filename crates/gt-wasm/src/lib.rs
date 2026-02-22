use gt_common::types::WorldConfig;
use gt_simulation::world::GameWorld;
use wasm_bindgen::prelude::*;

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
                        "event": format!("{:?}", event),
                    })
                })
                .collect();
            Ok(serde_json::to_string(&notifications).unwrap_or_default())
        }
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
                    "utilization": e.utilization(),
                    "src_x": src_pos.map(|p| p.x).unwrap_or(0.0),
                    "src_y": src_pos.map(|p| p.y).unwrap_or(0.0),
                    "dst_x": dst_pos.map(|p| p.x).unwrap_or(0.0),
                    "dst_y": dst_pos.map(|p| p.y).unwrap_or(0.0),
                    "src_cell": src_cell,
                    "dst_cell": dst_cell,
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
                    "event": format!("{:?}", event),
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

        let targets: Vec<serde_json::Value> = player_nodes
            .iter()
            .filter(|&&nid| nid != source_id && !self.world.constructions.contains_key(&nid))
            .filter_map(|&nid| {
                let node = self.world.infra_nodes.get(&nid)?;
                let pos = self.world.positions.get(&nid)?;
                let src_pos = self.world.positions.get(&source_id)?;
                let dx = pos.x - src_pos.x;
                let dy = pos.y - src_pos.y;
                let dist_km = (dx * dx + dy * dy).sqrt() * 111.0; // rough deg to km
                let cost = (dist_km * 10_000.0) as i64;
                Some(serde_json::json!({
                    "target_id": nid,
                    "target_type": format!("{:?}", node.node_type),
                    "x": pos.x,
                    "y": pos.y,
                    "distance_km": dist_km,
                    "cost": cost,
                    "affordable": cash >= cost,
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
                    "utilization": e.utilization(),
                    "owner": e.owner,
                    "owner_name": owner_name,
                    "src_x": src_pos.map(|p| p.x).unwrap_or(0.0),
                    "src_y": src_pos.map(|p| p.y).unwrap_or(0.0),
                    "dst_x": dst_pos.map(|p| p.x).unwrap_or(0.0),
                    "dst_y": dst_pos.map(|p| p.y).unwrap_or(0.0),
                    "src_cell": src_cell,
                    "dst_cell": dst_cell,
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
}
