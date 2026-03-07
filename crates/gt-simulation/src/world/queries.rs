use std::collections::HashMap;

use gt_common::types::*;

use super::{DisasterForecast, GameWorld};

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

    /// Compute disaster forecasts by peeking ahead using the deterministic RNG.
    ///
    /// Simulates the disaster roll logic for the next `lookahead_windows` disaster
    /// check windows (each window is 50 ticks) without mutating state. Returns a
    /// list of forecasts for regions where the RNG roll would trigger a disaster.
    ///
    /// This gives the player early warning of upcoming disasters with authoritative
    /// server-side prediction using the actual RNG seed.
    pub fn get_disaster_forecasts(&self) -> Vec<DisasterForecast> {
        let mut forecasts = Vec::new();
        let current_tick = self.tick;
        let lookahead_windows = 10; // peek ahead 10 disaster windows (500 ticks)

        // Snapshot RNG state so we can simulate without mutation
        let rng_seed = self.rng_seed;

        // Helper: deterministic random without mutating self
        let peek_random = |counter: &mut u64| -> f64 {
            *counter = counter
                .wrapping_mul(6364136223846793005)
                .wrapping_add(rng_seed);
            (*counter >> 33) as f64 / (1u64 << 31) as f64
        };

        // Collect region data sorted by ID for deterministic iteration
        let mut region_data: Vec<(u64, f64, String)> = self
            .regions
            .iter()
            .map(|(&id, r)| (id, r.disaster_risk, r.name.clone()))
            .collect();
        region_data.sort_unstable_by_key(|t| t.0);

        // Disaster type names and weights (mirrors disaster.rs DISASTER_TYPES)
        let disaster_types: &[(&str, f64)] = &[
            ("Earthquake", 0.15),
            ("Hurricane", 0.15),
            ("Flooding", 0.20),
            ("Landslide", 0.10),
            ("CyberAttack", 0.15),
            ("PoliticalUnrest", 0.10),
            ("RegulatoryChange", 0.10),
            ("EquipmentFailure", 0.05),
        ];

        for window in 1..=lookahead_windows {
            let predicted_tick = current_tick + (window * 50);

            // Reset RNG counter for each window to match what the disaster system
            // would compute at that tick. The tick() method sets:
            //   tick_rng_counter = tick.wrapping_mul(rng_seed.wrapping_add(1))
            // then each system call to deterministic_random() advances the counter.
            // We approximate by using the same initial state the real tick would have,
            // then advancing through the same number of system RNG calls that precede
            // the disaster system (systems 1-12 run before disaster).
            let mut rng_counter = predicted_tick.wrapping_mul(rng_seed.wrapping_add(1));

            // Skip some RNG calls to approximate the state at disaster system entry.
            // The exact count depends on world state, but a fixed skip provides a
            // reasonable approximation for forecasting purposes.
            for _ in 0..20 {
                peek_random(&mut rng_counter);
            }

            // Run through the same disaster check logic for each region
            for (region_id, disaster_risk, ref name) in &region_data {
                let roll = peek_random(&mut rng_counter);

                // Same threshold as disaster::run: roll < 0.02 * disaster_risk
                let threshold = 0.02 * disaster_risk;
                if roll >= threshold {
                    continue;
                }

                // This region would get a disaster at this tick
                let _severity = peek_random(&mut rng_counter);

                // Pick disaster type using same logic as disaster::pick_disaster_type
                let type_roll = peek_random(&mut rng_counter);
                let mut cumulative = 0.0;
                let mut disaster_name = "Earthquake";
                for &(dtype, weight) in disaster_types {
                    cumulative += weight;
                    if type_roll < cumulative {
                        disaster_name = dtype;
                        break;
                    }
                }

                // Probability is the base disaster_risk scaled by how close the
                // roll was to the threshold (higher risk = more likely)
                let probability = (threshold - roll) / threshold;

                forecasts.push(DisasterForecast {
                    region_id: *region_id,
                    region_name: name.clone(),
                    predicted_tick,
                    probability: probability.clamp(0.0, 1.0),
                    disaster_type: disaster_name.to_string(),
                });
            }
        }

        // Sort by predicted tick, then probability descending
        forecasts.sort_by(|a, b| {
            a.predicted_tick.cmp(&b.predicted_tick).then_with(|| {
                b.probability
                    .partial_cmp(&a.probability)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
        });

        forecasts
    }

    /// Get weather forecasts by combining active weather conditions with
    /// disaster forecast look-ahead. Forecasts predict weather events 5-10
    /// disaster windows (250-500 ticks) ahead, using the same deterministic
    /// RNG peek technique as `get_disaster_forecasts`.
    ///
    /// Returns a list of `WeatherForecast` structs that the frontend can
    /// display as early warnings, allowing the player to reinforce or
    /// reroute infrastructure before weather events hit.
    pub fn get_weather_forecasts(&self) -> Vec<crate::components::weather::WeatherForecast> {
        use crate::components::weather::{WeatherForecast, WeatherType};

        let mut forecasts = Vec::new();
        let current_tick = self.tick;

        // Map disaster type names to WeatherType variants
        let map_disaster_to_weather = |name: &str| -> WeatherType {
            match name {
                "Earthquake" => WeatherType::Earthquake,
                "Hurricane" => WeatherType::Hurricane,
                "Flooding" => WeatherType::Flooding,
                "Landslide" => WeatherType::Storms, // landslides accompany storms
                _ => WeatherType::Clear, // non-weather events (cyber, political, etc.)
            }
        };

        // 1. Include active weather conditions as "now" forecasts (eta_ticks = 0, probability = 1.0)
        for wc in &self.weather_conditions {
            if wc.is_expired(current_tick) {
                continue;
            }
            let region_name = self
                .regions
                .get(&wc.region_id)
                .map(|r| r.name.clone())
                .unwrap_or_default();
            forecasts.push(WeatherForecast {
                region_id: wc.region_id,
                region_name,
                predicted_type: wc.condition,
                probability: 1.0,
                eta_ticks: 0,
                predicted_severity: wc.severity,
            });
        }

        // 2. Convert disaster forecasts into weather forecasts (look-ahead)
        let disaster_forecasts = self.get_disaster_forecasts();
        for df in &disaster_forecasts {
            let weather_type = map_disaster_to_weather(&df.disaster_type);
            if matches!(weather_type, WeatherType::Clear) {
                continue; // skip non-weather disasters (cyber, political, etc.)
            }
            let eta = if df.predicted_tick > current_tick {
                (df.predicted_tick - current_tick) as u32
            } else {
                0
            };
            forecasts.push(WeatherForecast {
                region_id: df.region_id,
                region_name: df.region_name.clone(),
                predicted_type: weather_type,
                probability: df.probability,
                eta_ticks: eta,
                predicted_severity: df.probability * 0.5 + 0.1, // estimate
            });
        }

        // 3. Generate terrain-based weather forecasts from region characteristics.
        // Regions with specific terrain profiles get additional weather forecasts
        // that aren't tied to the disaster RNG (e.g., persistent heat in deserts).
        let rng_seed = self.rng_seed;
        let peek_random = |counter: &mut u64| -> f64 {
            *counter = counter
                .wrapping_mul(6364136223846793005)
                .wrapping_add(rng_seed);
            (*counter >> 33) as f64 / (1u64 << 31) as f64
        };

        let weather_types = [
            WeatherType::Storms,
            WeatherType::IceStorm,
            WeatherType::Flooding,
            WeatherType::ExtremeHeat,
            WeatherType::Hurricane,
        ];

        let mut region_data: Vec<_> = self
            .regions
            .iter()
            .map(|(&id, r)| (id, r.name.clone(), r.disaster_risk, r.cells.clone()))
            .collect();
        region_data.sort_unstable_by_key(|t| t.0);

        for (region_id, region_name, disaster_risk, cells) in &region_data {
            // Determine dominant terrain for this region
            let dominant_terrain = self.dominant_terrain_for_cells(cells);

            // For each weather type, compute a terrain-weighted probability
            let mut rng_counter =
                current_tick.wrapping_mul(rng_seed.wrapping_add(3)).wrapping_add(*region_id);
            for _ in 0..5 {
                peek_random(&mut rng_counter);
            }

            for &wtype in &weather_types {
                let affinity = wtype.terrain_affinity(&dominant_terrain);
                if affinity < 0.3 {
                    continue; // too unlikely for this terrain
                }

                let roll = peek_random(&mut rng_counter);
                let threshold = 0.03 * disaster_risk * affinity;
                if roll >= threshold {
                    continue;
                }

                // Predict ETA: 5-10 disaster windows (250-500 ticks)
                let eta_roll = peek_random(&mut rng_counter);
                let eta_ticks = 250 + (eta_roll * 250.0) as u32;

                let prob = ((threshold - roll) / threshold).clamp(0.0, 1.0);

                // Don't duplicate if we already have a forecast for this region + type
                let already_forecast = forecasts.iter().any(|f| {
                    f.region_id == *region_id && f.predicted_type == wtype
                });
                if already_forecast {
                    continue;
                }

                forecasts.push(WeatherForecast {
                    region_id: *region_id,
                    region_name: region_name.clone(),
                    predicted_type: wtype,
                    probability: prob,
                    eta_ticks,
                    predicted_severity: prob * 0.4 + 0.1,
                });
            }
        }

        // Sort by eta ascending, then probability descending
        forecasts.sort_by(|a, b| {
            a.eta_ticks.cmp(&b.eta_ticks).then_with(|| {
                b.probability
                    .partial_cmp(&a.probability)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
        });

        forecasts
    }

    /// Determine the most common terrain type across a set of cell indices.
    fn dominant_terrain_for_cells(&self, cells: &[usize]) -> gt_common::types::TerrainType {
        use gt_common::types::TerrainType;
        let mut counts: HashMap<u8, u32> = HashMap::new();
        for &cell_idx in cells {
            let terrain = self.get_cell_terrain(cell_idx).unwrap_or(TerrainType::Rural);
            *counts.entry(terrain as u8).or_insert(0) += 1;
        }
        // Find the terrain with the highest count
        let best = counts.into_iter().max_by_key(|&(_, c)| c).map(|(t, _)| t);
        match best {
            Some(0) => TerrainType::Urban,
            Some(1) => TerrainType::Suburban,
            Some(2) => TerrainType::Rural,
            Some(3) => TerrainType::Mountainous,
            Some(4) => TerrainType::Desert,
            Some(5) => TerrainType::Coastal,
            Some(6) => TerrainType::OceanShallow,
            Some(7) => TerrainType::OceanDeep,
            Some(8) => TerrainType::OceanTrench,
            Some(9) => TerrainType::Tundra,
            Some(10) => TerrainType::Frozen,
            _ => TerrainType::Rural,
        }
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
