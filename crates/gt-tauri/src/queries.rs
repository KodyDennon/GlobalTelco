//! Non-BridgeQuery query methods for `TauriBridge` that delegate to
//! `gt_bridge::queries`. These are additional queries not part of the
//! `BridgeQuery` trait but exposed via Tauri commands.

use gt_common::types::EntityId;

use crate::TauriBridge;

impl TauriBridge {
    pub fn get_player_corp_id(&self) -> u64 {
        self.world.lock().expect("GameWorld mutex poisoned").player_corp_id().unwrap_or(0)
    }

    pub fn is_real_earth(&self) -> bool {
        self.world.lock().expect("GameWorld mutex poisoned").config().use_real_earth
    }

    pub fn get_infrastructure_list(&self, corp_id: EntityId) -> String {
        let w = self.world.lock().expect("GameWorld mutex poisoned");
        gt_bridge::queries::query_infrastructure_list(&w, corp_id)
    }

    pub fn get_visible_entities(&self, min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> String {
        let w = self.world.lock().expect("GameWorld mutex poisoned");
        gt_bridge::queries::query_visible_entities(&w, min_x, min_y, max_x, max_y)
    }

    pub fn get_parcels_in_view(&self, min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> String {
        let w = self.world.lock().expect("GameWorld mutex poisoned");
        gt_bridge::queries::query_parcels_in_view(&w, min_x, min_y, max_x, max_y)
    }

    pub fn get_cell_coverage(&self) -> String {
        let w = self.world.lock().expect("GameWorld mutex poisoned");
        gt_bridge::queries::query_cell_coverage(&w)
    }

    pub fn get_all_infrastructure(&self) -> String {
        let w = self.world.lock().expect("GameWorld mutex poisoned");
        gt_bridge::queries::query_all_infrastructure(&w)
    }

    pub fn get_grid_cells(&self) -> String {
        let w = self.world.lock().expect("GameWorld mutex poisoned");
        gt_bridge::queries::query_grid_cells(&w)
    }

    pub fn get_world_geojson(&self) -> String {
        let w = self.world.lock().expect("GameWorld mutex poisoned");
        gt_bridge::queries::query_world_geojson(&w)
    }

    pub fn get_spectrum_licenses(&self) -> String {
        let w = self.world.lock().expect("GameWorld mutex poisoned");
        gt_bridge::queries::query_spectrum_licenses(&w)
    }

    pub fn get_spectrum_auctions(&self) -> String {
        let w = self.world.lock().expect("GameWorld mutex poisoned");
        gt_bridge::queries::query_spectrum_auctions(&w)
    }

    pub fn get_available_spectrum(&self, region_id: EntityId) -> String {
        let w = self.world.lock().expect("GameWorld mutex poisoned");
        gt_bridge::queries::query_available_spectrum(&w, region_id)
    }

    pub fn get_acquisition_proposals(&self) -> String {
        let w = self.world.lock().expect("GameWorld mutex poisoned");
        gt_bridge::queries::query_acquisition_proposals(&w)
    }

    pub fn road_pathfind(&self, from_lon: f64, from_lat: f64, to_lon: f64, to_lat: f64) -> String {
        let w = self.world.lock().expect("GameWorld mutex poisoned");
        gt_bridge::queries::query_road_pathfind(&w, from_lon, from_lat, to_lon, to_lat)
    }

    pub fn road_fiber_route_cost(
        &self,
        from_lon: f64,
        from_lat: f64,
        to_lon: f64,
        to_lat: f64,
    ) -> String {
        let w = self.world.lock().expect("GameWorld mutex poisoned");
        let cost = w.road_fiber_route_cost(from_lon, from_lat, to_lon, to_lat);
        serde_json::to_string(&cost).unwrap_or_default()
    }

    pub fn get_road_segments(&self) -> String {
        let w = self.world.lock().expect("GameWorld mutex poisoned");
        gt_bridge::queries::query_road_segments(&w)
    }

    pub fn create_world_preview(config_json: &str) -> Result<String, String> {
        let config: gt_common::types::WorldConfig = serde_json::from_str(config_json)
            .map_err(|e| format!("Invalid config: {e}"))?;
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
        Ok(serde_json::json!({
            "is_real_earth": false,
            "cell_count": world.grid.cell_count(),
            "cells": preview_cells,
            "cities": city_previews,
            "region_count": world.regions.len(),
        })
        .to_string())
    }
}
