//! `BridgeQuery` trait implementation for `WasmBridge`.

use gt_bridge::{BridgeQuery, EdgeArrays, InfraArrays, SatelliteArrays};
use gt_simulation::world::GameWorld;

use crate::WasmBridge;

impl BridgeQuery for WasmBridge {
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
        gt_bridge::queries::query_world_info(&self.world)
    }

    fn get_static_definitions(&self) -> String {
        gt_bridge::queries::query_static_definitions()
    }

    fn get_corporation_data(&self, corp_id: gt_common::types::EntityId) -> String {
        gt_bridge::queries::query_corporation_data(&self.world, corp_id)
    }

    fn get_regions(&self) -> String {
        gt_bridge::queries::query_regions(&self.world)
    }

    fn get_cities(&self) -> String {
        gt_bridge::queries::query_cities(&self.world)
    }

    fn get_all_corporations(&self) -> String {
        gt_bridge::queries::query_all_corporations(&self.world)
    }

    fn get_research_state(&self) -> String {
        gt_bridge::queries::query_research_state(&self.world)
    }

    fn get_contracts(&self, corp_id: gt_common::types::EntityId) -> String {
        gt_bridge::queries::query_contracts(&self.world, corp_id)
    }

    fn get_debt_instruments(&self, corp_id: gt_common::types::EntityId) -> String {
        gt_bridge::queries::query_debt_instruments(&self.world, corp_id)
    }

    fn get_notifications(&mut self) -> String {
        gt_bridge::queries::query_notifications(&mut self.world)
    }

    fn get_buildable_nodes(&self, lon: f64, lat: f64) -> String {
        gt_bridge::queries::query_buildable_nodes(&self.world, lon, lat)
    }

    fn get_buildable_edges(&self, source_id: gt_common::types::EntityId) -> String {
        gt_bridge::queries::query_buildable_edges(&self.world, source_id)
    }

    fn get_damaged_nodes(&self, corp_id: gt_common::types::EntityId) -> String {
        gt_bridge::queries::query_damaged_nodes(&self.world, corp_id)
    }

    fn get_auctions(&self) -> String {
        gt_bridge::queries::query_auctions(&self.world)
    }

    fn get_covert_ops(&self, corp_id: gt_common::types::EntityId) -> String {
        gt_bridge::queries::query_covert_ops(&self.world, corp_id)
    }

    fn get_lobbying_campaigns(&self, corp_id: gt_common::types::EntityId) -> String {
        gt_bridge::queries::query_lobbying_campaigns(&self.world, corp_id)
    }

    fn get_achievements(&self, corp_id: gt_common::types::EntityId) -> String {
        gt_bridge::queries::query_achievements(&self.world, corp_id)
    }

    fn get_victory_state(&self) -> String {
        gt_bridge::queries::query_victory_state(&self.world)
    }

    fn get_traffic_flows(&self) -> String {
        gt_bridge::queries::query_traffic_flows(&self.world)
    }

    fn get_weather_forecasts(&self) -> String {
        gt_bridge::queries::query_weather_forecasts(&self.world)
    }

    fn save_game(&self) -> Result<String, String> {
        self.world
            .save_game()
            .map_err(|e| format!("Save failed: {}", e))
    }

    fn load_game(&mut self, data: &str) -> Result<(), String> {
        self.world = GameWorld::load_game(data).map_err(|e| format!("Load failed: {}", e))?;
        Ok(())
    }

    fn get_alliances(&self, corp_id: gt_common::types::EntityId) -> String {
        gt_bridge::queries::query_alliances(&self.world, corp_id)
    }

    fn get_lawsuits(&self, corp_id: gt_common::types::EntityId) -> String {
        gt_bridge::queries::query_lawsuits(&self.world, corp_id)
    }

    fn get_stock_market(&self, corp_id: gt_common::types::EntityId) -> String {
        gt_bridge::queries::query_stock_market(&self.world, corp_id)
    }

    fn get_region_pricing(&self, corp_id: gt_common::types::EntityId) -> String {
        gt_bridge::queries::query_region_pricing(&self.world, corp_id)
    }

    fn get_maintenance_priorities(&self, corp_id: gt_common::types::EntityId) -> String {
        gt_bridge::queries::query_maintenance_priorities(&self.world, corp_id)
    }

    fn get_node_metadata(&self, id: gt_common::types::EntityId) -> String {
        gt_bridge::queries::query_node_metadata(&self.world, id)
    }

    fn get_nodes_metadata(&self, ids: &[gt_common::types::EntityId]) -> String {
        gt_bridge::queries::query_nodes_metadata(&self.world, ids)
    }

    fn get_edge_metadata(&self, id: gt_common::types::EntityId) -> String {
        gt_bridge::queries::query_edge_metadata(&self.world, id)
    }

    fn get_constellation_data(&self, corp_id: gt_common::types::EntityId) -> String {
        gt_bridge::queries::query_constellation_data(&self.world, corp_id)
    }

    fn get_orbital_view(&self) -> String {
        gt_bridge::queries::query_orbital_view(&self.world)
    }

    fn get_launch_schedule(&self, corp_id: gt_common::types::EntityId) -> String {
        gt_bridge::queries::query_launch_schedule(&self.world, corp_id)
    }

    fn get_terminal_inventory(&self, corp_id: gt_common::types::EntityId) -> String {
        gt_bridge::queries::query_terminal_inventory(&self.world, corp_id)
    }

    fn get_debris_status(&self) -> String {
        gt_bridge::queries::query_debris_status(&self.world)
    }

    fn get_infra_arrays(&self) -> InfraArrays {
        gt_bridge::queries::build_infra_arrays(&self.world)
    }

    fn get_infra_arrays_viewport(&self, west: f64, south: f64, east: f64, north: f64) -> InfraArrays {
        gt_bridge::queries::build_infra_arrays_viewport(&self.world, west, south, east, north)
    }

    fn get_edge_arrays(&self) -> EdgeArrays {
        gt_bridge::queries::build_edge_arrays(&self.world)
    }

    fn get_edge_arrays_viewport(&self, west: f64, south: f64, east: f64, north: f64) -> EdgeArrays {
        gt_bridge::queries::build_edge_arrays_viewport(&self.world, west, south, east, north)
    }

    fn get_satellite_arrays(&self) -> SatelliteArrays {
        gt_bridge::queries::build_satellite_arrays(&self.world)
    }
}
