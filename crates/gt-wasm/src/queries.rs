//! WASM-exported query methods that delegate to `gt_bridge::queries`.

use wasm_bindgen::prelude::*;

use crate::WasmBridge;

#[wasm_bindgen]
impl WasmBridge {
    pub fn get_world_info(&self) -> String {
        gt_bridge::queries::query_world_info(&self.world)
    }

    pub fn get_corporation_data(&self, corp_id: u64) -> String {
        gt_bridge::queries::query_corporation_data(&self.world, corp_id)
    }

    pub fn get_regions(&self) -> String {
        gt_bridge::queries::query_regions(&self.world)
    }

    pub fn is_real_earth(&self) -> bool {
        self.world.config().use_real_earth
    }

    pub fn get_cities(&self) -> String {
        gt_bridge::queries::query_cities(&self.world)
    }

    pub fn get_infrastructure_list(&self, corp_id: u64) -> String {
        gt_bridge::queries::query_infrastructure_list(&self.world, corp_id)
    }

    pub fn get_visible_entities(&self, min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> String {
        gt_bridge::queries::query_visible_entities(&self.world, min_x, min_y, max_x, max_y)
    }

    pub fn get_parcels_in_view(&self, min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> String {
        gt_bridge::queries::query_parcels_in_view(&self.world, min_x, min_y, max_x, max_y)
    }

    pub fn get_notifications(&mut self) -> String {
        gt_bridge::queries::query_notifications(&mut self.world)
    }

    pub fn get_player_corp_id(&self) -> u64 {
        self.world.player_corp_id().unwrap_or(0)
    }

    pub fn get_all_corporations(&self) -> String {
        gt_bridge::queries::query_all_corporations(&self.world)
    }

    pub fn get_contracts(&self, corp_id: u64) -> String {
        gt_bridge::queries::query_contracts(&self.world, corp_id)
    }

    pub fn get_debt_instruments(&self, corp_id: u64) -> String {
        gt_bridge::queries::query_debt_instruments(&self.world, corp_id)
    }

    pub fn get_research_state(&self) -> String {
        gt_bridge::queries::query_research_state(&self.world)
    }

    pub fn get_buildable_nodes(&self, lon: f64, lat: f64) -> String {
        gt_bridge::queries::query_buildable_nodes(&self.world, lon, lat)
    }

    pub fn get_buildable_edges(&self, source_id: u64) -> String {
        gt_bridge::queries::query_buildable_edges(&self.world, source_id)
    }

    pub fn save_game(&self) -> Result<String, JsValue> {
        self.world
            .save_game()
            .map_err(|e| JsValue::from_str(&e))
    }

    pub fn load_game(data: &str) -> Result<WasmBridge, JsValue> {
        let world =
            gt_simulation::world::GameWorld::load_game(data).map_err(|e| JsValue::from_str(&e))?;
        Ok(Self { world })
    }

    pub fn get_damaged_nodes(&self, corp_id: u64) -> String {
        gt_bridge::queries::query_damaged_nodes(&self.world, corp_id)
    }

    pub fn get_auctions(&self) -> String {
        gt_bridge::queries::query_auctions(&self.world)
    }

    pub fn get_acquisition_proposals(&self) -> String {
        gt_bridge::queries::query_acquisition_proposals(&self.world)
    }

    pub fn get_covert_ops(&self, corp_id: u64) -> String {
        gt_bridge::queries::query_covert_ops(&self.world, corp_id)
    }

    pub fn get_lobbying_campaigns(&self, corp_id: u64) -> String {
        gt_bridge::queries::query_lobbying_campaigns(&self.world, corp_id)
    }

    pub fn get_achievements(&self, corp_id: u64) -> String {
        gt_bridge::queries::query_achievements(&self.world, corp_id)
    }

    pub fn get_victory_state(&self) -> String {
        gt_bridge::queries::query_victory_state(&self.world)
    }

    pub fn get_cell_coverage(&self) -> String {
        gt_bridge::queries::query_cell_coverage(&self.world)
    }

    pub fn get_all_infrastructure(&self) -> String {
        gt_bridge::queries::query_all_infrastructure(&self.world)
    }

    pub fn get_traffic_flows(&self) -> String {
        gt_bridge::queries::query_traffic_flows(&self.world)
    }

    pub fn get_grid_cells(&self) -> String {
        gt_bridge::queries::query_grid_cells(&self.world)
    }

    pub fn create_world_preview(config_json: &str) -> Result<String, JsValue> {
        let config: gt_common::types::WorldConfig = serde_json::from_str(config_json)
            .map_err(|e| JsValue::from_str(&format!("Invalid config: {}", e)))?;

        if config.use_real_earth {
            return Ok(serde_json::json!({
                "is_real_earth": true,
                "cells": [],
                "width": 0,
                "height": 0,
            })
            .to_string());
        }

        let gen = gt_world::WorldGenerator::new(config);
        let world = gen.generate();

        let preview_cells: Vec<serde_json::Value> = world
            .grid
            .cells
            .iter()
            .enumerate()
            .map(|(i, cell)| {
                let terrain = &world.terrains[i];
                let elevation = world.elevations[i];
                serde_json::json!({
                    "lat": cell.lat,
                    "lon": cell.lon,
                    "terrain": format!("{:?}", terrain),
                    "elevation": elevation,
                })
            })
            .collect();

        let city_previews: Vec<serde_json::Value> = world
            .cities
            .iter()
            .map(|city| {
                let cell = &world.grid.cells[city.cell_index];
                serde_json::json!({
                    "name": city.name,
                    "lat": cell.lat,
                    "lon": cell.lon,
                    "population": city.population,
                })
            })
            .collect();

        let result = serde_json::json!({
            "is_real_earth": false,
            "cell_count": world.grid.cell_count(),
            "cells": preview_cells,
            "cities": city_previews,
            "region_count": world.regions.len(),
        });
        Ok(result.to_string())
    }

    pub fn get_world_geojson(&self) -> String {
        gt_bridge::queries::query_world_geojson(&self.world)
    }

    // ── Spectrum Queries ────────────────────────────────────────────────

    pub fn get_spectrum_licenses(&self) -> String {
        gt_bridge::queries::query_spectrum_licenses(&self.world)
    }

    pub fn get_spectrum_auctions(&self) -> String {
        gt_bridge::queries::query_spectrum_auctions(&self.world)
    }

    pub fn get_available_spectrum(&self, region_id: u64) -> String {
        gt_bridge::queries::query_available_spectrum(&self.world, region_id)
    }

    pub fn get_disaster_forecasts(&self) -> String {
        gt_bridge::queries::query_disaster_forecasts(&self.world)
    }

    pub fn get_weather_forecasts(&self) -> String {
        gt_bridge::queries::query_weather_forecasts(&self.world)
    }

    // ── Alliance / Legal / Stock Market Queries ─────────────────────────

    pub fn get_alliances(&self, corp_id: u64) -> String {
        gt_bridge::queries::query_alliances(&self.world, corp_id)
    }

    pub fn get_lawsuits(&self, corp_id: u64) -> String {
        gt_bridge::queries::query_lawsuits(&self.world, corp_id)
    }

    pub fn get_stock_market(&self, corp_id: u64) -> String {
        gt_bridge::queries::query_stock_market(&self.world, corp_id)
    }

    pub fn get_region_pricing(&self, corp_id: u64) -> String {
        gt_bridge::queries::query_region_pricing(&self.world, corp_id)
    }

    pub fn get_maintenance_priorities(&self, corp_id: u64) -> String {
        gt_bridge::queries::query_maintenance_priorities(&self.world, corp_id)
    }

    // ── Satellite Queries ───────────────────────────────────────────────

    pub fn get_constellation_data(&self, corp_id: u64) -> String {
        gt_bridge::queries::query_constellation_data(&self.world, corp_id)
    }

    pub fn get_orbital_view(&self) -> String {
        gt_bridge::queries::query_orbital_view(&self.world)
    }

    pub fn get_launch_schedule(&self, corp_id: u64) -> String {
        gt_bridge::queries::query_launch_schedule(&self.world, corp_id)
    }

    pub fn get_terminal_inventory(&self, corp_id: u64) -> String {
        gt_bridge::queries::query_terminal_inventory(&self.world, corp_id)
    }

    pub fn get_debris_status(&self) -> String {
        gt_bridge::queries::query_debris_status(&self.world)
    }

    // ── Road Network Queries ────────────────────────────────────────────

    pub fn road_pathfind(
        &self,
        from_lon: f64,
        from_lat: f64,
        to_lon: f64,
        to_lat: f64,
    ) -> String {
        gt_bridge::queries::query_road_pathfind(&self.world, from_lon, from_lat, to_lon, to_lat)
    }

    pub fn road_fiber_route_cost(
        &self,
        from_lon: f64,
        from_lat: f64,
        to_lon: f64,
        to_lat: f64,
    ) -> f64 {
        self.world
            .road_fiber_route_cost(from_lon, from_lat, to_lon, to_lat)
    }

    pub fn get_road_segments(&self) -> String {
        gt_bridge::queries::query_road_segments(&self.world)
    }
}
