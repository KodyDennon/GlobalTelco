//! `BridgeQuery` trait implementation for `TauriBridge`.

use gt_bridge::{BridgeQuery, EdgeArrays, InfraArrays, SatelliteArrays};
use gt_simulation::world::GameWorld;

use crate::TauriBridge;

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
        gt_bridge::queries::query_world_info(&w)
    }

    fn get_static_definitions(&self) -> String {
        gt_bridge::queries::query_static_definitions()
    }

    fn get_corporation_data(&self, corp_id: gt_common::types::EntityId) -> String {
        let w = self.world.lock().unwrap();
        gt_bridge::queries::query_corporation_data(&w, corp_id)
    }

    fn get_regions(&self) -> String {
        let w = self.world.lock().unwrap();
        gt_bridge::queries::query_regions(&w)
    }

    fn get_cities(&self) -> String {
        let w = self.world.lock().unwrap();
        gt_bridge::queries::query_cities(&w)
    }

    fn get_all_corporations(&self) -> String {
        let w = self.world.lock().unwrap();
        gt_bridge::queries::query_all_corporations(&w)
    }

    fn get_research_state(&self) -> String {
        let w = self.world.lock().unwrap();
        gt_bridge::queries::query_research_state(&w)
    }

    fn get_contracts(&self, corp_id: gt_common::types::EntityId) -> String {
        let w = self.world.lock().unwrap();
        gt_bridge::queries::query_contracts(&w, corp_id)
    }

    fn get_debt_instruments(&self, corp_id: gt_common::types::EntityId) -> String {
        let w = self.world.lock().unwrap();
        gt_bridge::queries::query_debt_instruments(&w, corp_id)
    }

    fn get_notifications(&mut self) -> String {
        let mut w = self.world.lock().unwrap();
        gt_bridge::queries::query_notifications(&mut w)
    }

    fn get_buildable_nodes(&self, lon: f64, lat: f64) -> String {
        let w = self.world.lock().unwrap();
        gt_bridge::queries::query_buildable_nodes(&w, lon, lat)
    }

    fn get_buildable_edges(&self, source_id: gt_common::types::EntityId) -> String {
        let w = self.world.lock().unwrap();
        gt_bridge::queries::query_buildable_edges(&w, source_id)
    }

    fn get_damaged_nodes(&self, corp_id: gt_common::types::EntityId) -> String {
        let w = self.world.lock().unwrap();
        gt_bridge::queries::query_damaged_nodes(&w, corp_id)
    }

    fn get_auctions(&self) -> String {
        let w = self.world.lock().unwrap();
        gt_bridge::queries::query_auctions(&w)
    }

    fn get_covert_ops(&self, corp_id: gt_common::types::EntityId) -> String {
        let w = self.world.lock().unwrap();
        gt_bridge::queries::query_covert_ops(&w, corp_id)
    }

    fn get_lobbying_campaigns(&self, corp_id: gt_common::types::EntityId) -> String {
        let w = self.world.lock().unwrap();
        gt_bridge::queries::query_lobbying_campaigns(&w, corp_id)
    }

    fn get_achievements(&self, corp_id: gt_common::types::EntityId) -> String {
        let w = self.world.lock().unwrap();
        gt_bridge::queries::query_achievements(&w, corp_id)
    }

    fn get_victory_state(&self) -> String {
        let w = self.world.lock().unwrap();
        gt_bridge::queries::query_victory_state(&w)
    }

    fn get_traffic_flows(&self) -> String {
        let w = self.world.lock().unwrap();
        gt_bridge::queries::query_traffic_flows(&w)
    }

    fn get_weather_forecasts(&self) -> String {
        let w = self.world.lock().unwrap();
        gt_bridge::queries::query_weather_forecasts(&w)
    }

    fn save_game(&self) -> Result<String, String> {
        self.world
            .lock()
            .unwrap()
            .save_game()
            .map_err(|e| format!("Save failed: {e}"))
    }

    fn load_game(&mut self, data: &str) -> Result<(), String> {
        let world = GameWorld::load_game(data).map_err(|e| format!("Load failed: {e}"))?;
        *self.world.lock().unwrap() = world;
        Ok(())
    }

    fn get_alliances(&self, corp_id: gt_common::types::EntityId) -> String {
        let w = self.world.lock().unwrap();
        gt_bridge::queries::query_alliances(&w, corp_id)
    }

    fn get_lawsuits(&self, corp_id: gt_common::types::EntityId) -> String {
        let w = self.world.lock().unwrap();
        gt_bridge::queries::query_lawsuits(&w, corp_id)
    }

    fn get_stock_market(&self, corp_id: gt_common::types::EntityId) -> String {
        let w = self.world.lock().unwrap();
        gt_bridge::queries::query_stock_market(&w, corp_id)
    }

    fn get_region_pricing(&self, corp_id: gt_common::types::EntityId) -> String {
        let w = self.world.lock().unwrap();
        gt_bridge::queries::query_region_pricing(&w, corp_id)
    }

    fn get_maintenance_priorities(&self, corp_id: gt_common::types::EntityId) -> String {
        let w = self.world.lock().unwrap();
        gt_bridge::queries::query_maintenance_priorities(&w, corp_id)
    }

    fn get_constellation_data(&self, corp_id: gt_common::types::EntityId) -> String {
        let w = self.world.lock().unwrap();
        gt_bridge::queries::query_constellation_data(&w, corp_id)
    }

    fn get_orbital_view(&self) -> String {
        let w = self.world.lock().unwrap();
        gt_bridge::queries::query_orbital_view(&w)
    }

    fn get_launch_schedule(&self, corp_id: gt_common::types::EntityId) -> String {
        let w = self.world.lock().unwrap();
        gt_bridge::queries::query_launch_schedule(&w, corp_id)
    }

    fn get_terminal_inventory(&self, corp_id: gt_common::types::EntityId) -> String {
        let w = self.world.lock().unwrap();
        gt_bridge::queries::query_terminal_inventory(&w, corp_id)
    }

    fn get_debris_status(&self) -> String {
        let w = self.world.lock().unwrap();
        gt_bridge::queries::query_debris_status(&w)
    }

    fn get_infra_arrays(&self) -> InfraArrays {
        let w = self.world.lock().unwrap();
        gt_bridge::queries::build_infra_arrays(&w)
    }

    fn get_infra_arrays_viewport(&self, west: f64, south: f64, east: f64, north: f64) -> InfraArrays {
        let w = self.world.lock().unwrap();
        gt_bridge::queries::build_infra_arrays_viewport(&w, west, south, east, north)
    }

    fn get_edge_arrays(&self) -> EdgeArrays {
        let w = self.world.lock().unwrap();
        gt_bridge::queries::build_edge_arrays(&w)
    }

    fn get_edge_arrays_viewport(&self, west: f64, south: f64, east: f64, north: f64) -> EdgeArrays {
        let w = self.world.lock().unwrap();
        gt_bridge::queries::build_edge_arrays_viewport(&w, west, south, east, north)
    }

    fn get_satellite_arrays(&self) -> SatelliteArrays {
        let w = self.world.lock().unwrap();
        gt_bridge::queries::build_satellite_arrays(&w)
    }
}
