use gt_common::types::WorldConfig;
use gt_simulation::world::GameWorld;
use wasm_bindgen::prelude::*;

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

    pub fn process_command(&mut self, command_json: &str) -> Result<(), JsValue> {
        let cmd: gt_common::commands::Command = serde_json::from_str(command_json)
            .map_err(|e| JsValue::from_str(&format!("Invalid command: {}", e)))?;
        self.world.process_command(cmd);
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
                })
            })
            .collect();
        serde_json::to_string(&regions).unwrap_or_default()
    }

    pub fn get_cities(&self) -> String {
        let cities: Vec<serde_json::Value> = self
            .world
            .cities
            .iter()
            .map(|(&id, c)| {
                let pos = self.world.positions.get(&id);
                serde_json::json!({
                    "id": id,
                    "name": c.name,
                    "region_id": c.region_id,
                    "cell_index": c.cell_index,
                    "population": c.population,
                    "growth_rate": c.growth_rate,
                    "development": c.development,
                    "telecom_demand": c.telecom_demand,
                    "infrastructure_satisfaction": c.infrastructure_satisfaction,
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

    pub fn get_grid_cells(&self) -> String {
        let cells: Vec<serde_json::Value> = self
            .world
            .grid_cell_positions
            .iter()
            .enumerate()
            .map(|(i, &(lat, lon))| {
                let terrain = self
                    .world
                    .land_parcels
                    .values()
                    .find(|p| p.cell_index == i)
                    .map(|p| format!("{:?}", p.terrain))
                    .unwrap_or_else(|| "Ocean".to_string());
                serde_json::json!({
                    "index": i,
                    "lat": lat,
                    "lon": lon,
                    "terrain": terrain,
                })
            })
            .collect();
        serde_json::to_string(&cells).unwrap_or_default()
    }
}
