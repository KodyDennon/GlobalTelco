//! Tauri-native simulation bridge for desktop builds.
//!
//! Wraps gt-simulation's `GameWorld` and implements `gt_bridge::BridgeQuery`
//! for native Rust execution — no WASM overhead. Exposes functions intended
//! to be registered as `#[tauri::command]` handlers in the desktop app.
//!
//! The desktop app holds a `TauriBridge` inside Tauri's managed state.
//! Frontend calls Tauri `invoke()` instead of calling WASM when running
//! in the desktop environment.

use std::sync::Mutex;

use gt_bridge::{BridgeQuery, EdgeArrays, InfraArrays};
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
        });
        serde_json::to_string(&info).unwrap_or_default()
    }

    fn get_corporation_data(&self, corp_id: EntityId) -> String {
        let w = self.world.lock().unwrap();
        if let Some(corp) = w.corporations.get(&corp_id) {
            let fin = w.financials.get(&corp_id);
            let employees = w.workforces.get(&corp_id).map(|wf| wf.employee_count).unwrap_or(0);
            let nodes = w.corp_infra_nodes.get(&corp_id).map(|n| n.len()).unwrap_or(0);
            let morale = w.workforces.get(&corp_id).map(|wf| wf.morale).unwrap_or(0.5);
            let data = serde_json::json!({
                "id": corp_id,
                "name": corp.name,
                "is_player": w.player_corp_id() == Some(corp_id),
                "credit_rating": format!("{:?}", corp.credit_rating),
                "cash": fin.map(|f| f.cash).unwrap_or(0),
                "revenue_per_tick": fin.map(|f| f.revenue_per_tick).unwrap_or(0),
                "cost_per_tick": fin.map(|f| f.cost_per_tick).unwrap_or(0),
                "debt": fin.map(|f| f.debt).unwrap_or(0),
                "profit_per_tick": fin.map(|f| f.revenue_per_tick - f.cost_per_tick).unwrap_or(0),
                "employee_count": employees,
                "morale": morale,
                "infrastructure_count": nodes,
            });
            serde_json::to_string(&data).unwrap_or_default()
        } else {
            "{}".to_string()
        }
    }

    fn get_regions(&self) -> String {
        let w = self.world.lock().unwrap();
        let regions: Vec<serde_json::Value> = w.regions.iter().map(|(&id, region)| {
            let city_ids: Vec<EntityId> = w.cities.keys()
                .filter(|&&cid| w.positions.get(&cid).and_then(|p| p.region_id) == Some(id))
                .copied().collect();
            serde_json::json!({
                "id": id,
                "name": region.name,
                "center_lat": region.center_lat,
                "center_lon": region.center_lon,
                "population": region.population,
                "gdp": region.gdp,
                "development": region.development,
                "tax_rate": region.tax_rate,
                "regulatory_strictness": region.regulatory_strictness,
                "disaster_risk": region.disaster_risk,
                "cell_count": region.cells.len(),
                "city_ids": city_ids,
                "boundary_polygon": region.boundary_polygon,
            })
        }).collect();
        serde_json::to_string(&regions).unwrap_or_default()
    }

    fn get_cities(&self) -> String {
        let w = self.world.lock().unwrap();
        let cities: Vec<serde_json::Value> = w.cities.iter().map(|(&id, city)| {
            let pos = w.positions.get(&id);
            serde_json::json!({
                "id": id,
                "name": city.name,
                "population": city.population,
                "growth_rate": city.growth_rate,
                "development": city.development,
                "telecom_demand": city.telecom_demand,
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
                "id": id,
                "name": corp.name,
                "is_player": w.player_corp_id() == Some(id),
                "credit_rating": format!("{:?}", corp.credit_rating),
                "cash": fin.map(|f| f.cash).unwrap_or(0),
                "revenue": fin.map(|f| f.revenue_per_tick).unwrap_or(0),
                "cost": fin.map(|f| f.cost_per_tick).unwrap_or(0),
            })
        }).collect();
        serde_json::to_string(&corps).unwrap_or_default()
    }

    fn get_research_state(&self) -> String {
        // Delegate to world's research serialization
        serde_json::to_string(&serde_json::json!([])).unwrap_or_default()
    }

    fn get_contracts(&self, _corp_id: EntityId) -> String {
        "[]".to_string()
    }

    fn get_debt_instruments(&self, _corp_id: EntityId) -> String {
        "[]".to_string()
    }

    fn get_notifications(&mut self) -> String {
        let mut w = self.world.lock().unwrap();
        let events = w.event_queue.drain();
        if events.is_empty() {
            "[]".to_string()
        } else {
            let notifications: Vec<serde_json::Value> = events.iter().map(|(tick, event)| {
                serde_json::json!({
                    "tick": tick,
                    "event": serde_json::to_value(event).unwrap_or(serde_json::Value::Null),
                })
            }).collect();
            serde_json::to_string(&notifications).unwrap_or_default()
        }
    }

    fn get_buildable_nodes(&self, _lon: f64, _lat: f64) -> String {
        "[]".to_string()
    }

    fn get_buildable_edges(&self, _source_id: EntityId) -> String {
        "[]".to_string()
    }

    fn get_damaged_nodes(&self, _corp_id: EntityId) -> String {
        "[]".to_string()
    }

    fn get_auctions(&self) -> String {
        "[]".to_string()
    }

    fn get_covert_ops(&self, _corp_id: EntityId) -> String {
        r#"{"security_level":0,"active_missions":0,"detection_count":0}"#.to_string()
    }

    fn get_lobbying_campaigns(&self, _corp_id: EntityId) -> String {
        "[]".to_string()
    }

    fn get_achievements(&self, _corp_id: EntityId) -> String {
        r#"{"unlocked":[],"progress":{}}"#.to_string()
    }

    fn get_victory_state(&self) -> String {
        "{}".to_string()
    }

    fn get_traffic_flows(&self) -> String {
        r#"{"edge_flows":[],"node_flows":[],"total_served":0,"total_dropped":0,"total_demand":0,"player_served":0,"player_dropped":0,"top_congested":[]}"#.to_string()
    }

    fn get_weather_forecasts(&self) -> String {
        let w = self.world.lock().unwrap();
        let forecasts = w.get_weather_forecasts();
        let json: Vec<serde_json::Value> = forecasts
            .iter()
            .map(|f| {
                serde_json::json!({
                    "region_id": f.region_id,
                    "region_name": f.region_name,
                    "predicted_type": f.predicted_type.display_name(),
                    "probability": f.probability,
                    "eta_ticks": f.eta_ticks,
                    "predicted_severity": f.predicted_severity,
                })
            })
            .collect();
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
}

// ── Tauri Command Wrappers ──────────────────────────────────────────────
// These are standalone functions that can be registered with
// `tauri::generate_handler![]`. They accept `tauri::State<TauriBridge>`.

/// Create a new game world with the given config JSON.
pub fn cmd_new_game(config_json: &str) -> Result<TauriBridge, String> {
    let config: WorldConfig = serde_json::from_str(config_json)
        .map_err(|e| format!("Invalid config: {e}"))?;
    Ok(TauriBridge::new(config))
}
