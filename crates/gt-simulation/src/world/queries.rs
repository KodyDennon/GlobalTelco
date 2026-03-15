use std::collections::HashMap;

use gt_common::types::*;

use super::GameWorld;

impl GameWorld {
    pub fn config(&self) -> &WorldConfig {
        &self.config
    }

    pub fn current_tick(&self) -> Tick {
        self.tick
    }

    /// Get the intel level that `spy_corp` has against `target_corp`.
    /// Returns 0 (no intel) by default.
    pub fn get_intel_level(&self, spy_corp: EntityId, target_corp: EntityId) -> u8 {
        self.intel_levels
            .get(&(spy_corp, target_corp))
            .copied()
            .unwrap_or(0)
    }

    /// Get a snapshot of all intel levels for a specific spy corp.
    /// Returns a map of target_corp -> intel_level for all targets where level > 0.
    pub fn get_intel_levels_for_corp(&self, spy_corp: EntityId) -> HashMap<EntityId, u8> {
        self.intel_levels
            .iter()
            .filter(|&(&(spy, _), _)| spy == spy_corp)
            .map(|(&(_, target), &level)| (target, level))
            .collect()
    }

    pub fn speed(&self) -> GameSpeed {
        self.speed
    }

    pub fn player_corp_id(&self) -> Option<EntityId> {
        self.player_corp_id
    }

    pub fn entity_count(&self) -> usize {
        (self.next_entity_id - 1) as usize
    }

    /// Find the nearest grid cell to the given (lon, lat) coordinates.
    /// Returns (cell_index, distance_degrees) or None if no cells exist.
    pub fn find_nearest_cell(&self, lon: f64, lat: f64) -> Option<(usize, f64)> {
        if self.grid_cell_positions.is_empty() {
            return None;
        }
        let mut best_idx = 0;
        let mut best_dist_sq = f64::MAX;
        for (idx, &(cell_lat, cell_lon)) in self.grid_cell_positions.iter().enumerate() {
            let dlat = cell_lat - lat;
            let dlon = cell_lon - lon;
            let dist_sq = dlat * dlat + dlon * dlon;
            if dist_sq < best_dist_sq {
                best_dist_sq = dist_sq;
                best_idx = idx;
            }
        }
        Some((best_idx, best_dist_sq.sqrt()))
    }

    pub fn nearest_cell_latlon(&self, lat: f64, lon: f64) -> usize {
        self.find_nearest_cell(lon, lat).map(|(idx, _)| idx).unwrap_or(0)
    }

    /// Find all cell indices within a bounding box.
    pub fn cells_in_range(&self, lat: f64, lon: f64, lat_range: f64, lon_range: f64) -> Vec<usize> {
        let mut result = Vec::new();
        // Optimization: still a scan, but we could use a spatial hash here too if needed.
        // Given GameWorld doesn't have the GeodesicGrid spatial hash, we do a bounding box scan.
        for (i, &(clat, clon)) in self.grid_cell_positions.iter().enumerate() {
            if (clat - lat).abs() <= lat_range && (clon - lon).abs() <= lon_range {
                result.push(i);
            }
        }
        result
    }

    /// Get the terrain for a given cell index by looking up the land parcel.
    pub fn get_cell_terrain(&self, cell_index: usize) -> Option<TerrainType> {
        self.cell_to_parcel
            .get(&cell_index)
            .and_then(|pid| self.land_parcels.get(pid))
            .map(|p| p.terrain)
    }

    /// Query the road network for A* pathfinding between two (lon, lat) points.
    /// Returns a list of waypoints along the road network.
    /// If no road path exists, falls back to a direct line [from, to].
    pub fn road_pathfind(
        &self,
        from_lon: f64,
        from_lat: f64,
        to_lon: f64,
        to_lat: f64,
    ) -> Vec<(f64, f64)> {
        self.road_network
            .pathfind((from_lon, from_lat), (to_lon, to_lat))
    }

    /// Query the cost of routing fiber along the road network between two points.
    /// Uses A* pathfinding to find the road route, then sums segment costs
    /// with road class fiber cost multipliers applied.
    /// Returns the cost in km-equivalents (weighted by road class).
    pub fn road_fiber_route_cost(
        &self,
        from_lon: f64,
        from_lat: f64,
        to_lon: f64,
        to_lat: f64,
    ) -> f64 {
        self.road_network
            .fiber_route_cost((from_lon, from_lat), (to_lon, to_lat))
    }

    /// Get all road segments for rendering on the map.
    /// Returns a serializable representation of the road network.
    pub fn get_road_segments(&self) -> &[crate::components::RoadSegment] {
        &self.road_network.segments
    }
}
