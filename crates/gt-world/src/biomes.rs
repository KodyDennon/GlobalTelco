use gt_common::types::TerrainType;

use crate::voronoi::VoronoiGrid;

/// Assign biomes to each Voronoi cell using a Whittaker-diagram approach.
///
/// Inputs:
/// - Temperature derived from latitude and elevation
/// - Moisture from the river system (ocean proximity + river proximity + rain shadow)
///
/// Output: one TerrainType per cell.
pub fn assign_biomes(
    grid: &VoronoiGrid,
    elevations: &[f64],
    moisture: &[f64],
    climate_variation: f64,
) -> Vec<TerrainType> {
    let n = grid.cell_count();

    // Pre-compute which cells have mountainous neighbors (for rain shadow)
    let mountain_threshold = 3000.0; // meters
    let is_high: Vec<bool> = elevations.iter().map(|&e| e > mountain_threshold).collect();

    let mut terrains = Vec::with_capacity(n);

    for i in 0..n {
        let cell = &grid.cells[i];
        let elev = elevations[i];

        // Ocean cells
        if elev < 0.0 {
            if elev > -200.0 {
                terrains.push(TerrainType::OceanShallow);
            } else {
                terrains.push(TerrainType::OceanDeep);
            }
            continue;
        }

        // Temperature: decreases with latitude and altitude
        // Base temp: 30C at equator, -30C at poles
        let lat = cell.center_lat.abs();
        let lat_temp = 30.0 - (lat / 90.0) * 60.0 * (0.8 + climate_variation * 0.4);

        // Altitude lapse rate: ~6.5C per 1000m
        let altitude_cooling = elev / 1000.0 * 6.5;
        let temperature = lat_temp - altitude_cooling;

        // Moisture: base from river system + rain shadow effect
        let mut cell_moisture = moisture[i];

        // Rain shadow: if a cell is behind a mountain (relative to prevailing wind from west),
        // reduce moisture. Check if western neighbors are high mountains.
        let rain_shadow_factor = compute_rain_shadow(grid, &is_high, i);
        cell_moisture *= 1.0 - rain_shadow_factor * 0.6;
        cell_moisture = cell_moisture.clamp(0.0, 1.0);

        // High elevation overrides
        if elev > 4000.0 {
            terrains.push(TerrainType::Frozen);
            continue;
        }
        if elev > mountain_threshold {
            terrains.push(TerrainType::Mountainous);
            continue;
        }

        // Whittaker classification
        let terrain = classify_whittaker(temperature, cell_moisture, elev);
        terrains.push(terrain);
    }

    terrains
}

/// Classify terrain using temperature and moisture (Whittaker diagram).
fn classify_whittaker(temperature: f64, moisture: f64, elevation: f64) -> TerrainType {
    // Frozen: very cold
    if temperature < -15.0 {
        return TerrainType::Frozen;
    }

    // Tundra: cold
    if temperature < -5.0 {
        return TerrainType::Tundra;
    }

    // Cool regions
    if temperature < 5.0 {
        if moisture > 0.5 {
            return TerrainType::Tundra; // cold+wet = tundra/taiga
        }
        return TerrainType::Tundra; // cold+dry = also tundra
    }

    // Temperate (5-15C)
    if temperature < 15.0 {
        if moisture > 0.6 {
            return TerrainType::Rural; // temperate forest
        }
        if moisture > 0.3 {
            return TerrainType::Rural; // temperate grassland
        }
        return TerrainType::Desert; // cold desert (steppe)
    }

    // Warm (15-25C)
    if temperature < 25.0 {
        if moisture > 0.6 {
            return TerrainType::Coastal; // warm + wet near coast approximation
        }
        if moisture > 0.3 {
            return TerrainType::Rural; // savanna/woodland
        }
        return TerrainType::Desert;
    }

    // Hot (>25C)
    if moisture > 0.7 {
        return TerrainType::Coastal; // tropical wet
    }
    if moisture > 0.4 {
        return TerrainType::Rural; // tropical seasonal
    }
    if moisture > 0.2 {
        // Semi-arid with higher elevation tends to be rural
        if elevation > 500.0 {
            return TerrainType::Rural;
        }
        return TerrainType::Desert;
    }

    TerrainType::Desert
}

/// Compute rain shadow effect for a cell.
/// Returns 0.0 (no shadow) to 1.0 (full shadow).
/// Checks if mountains exist in the prevailing wind direction (assumed westerly).
fn compute_rain_shadow(grid: &VoronoiGrid, is_high: &[bool], cell_idx: usize) -> f64 {
    let cell = &grid.cells[cell_idx];

    if is_high[cell_idx] {
        return 0.0; // Mountains themselves are not in shadow
    }

    // Check neighbors to the west (lon decreasing = westward)
    // A cell is in rain shadow if it has high-elevation neighbors with lower longitude
    let cell_lon = cell.center_lon;
    let mut mountain_west_count = 0;
    let mut west_neighbor_count = 0;

    for &ni in &cell.neighbor_indices {
        let neighbor = &grid.cells[ni];
        // Normalize longitude difference
        let mut dlon = cell_lon - neighbor.center_lon;
        if dlon > 180.0 {
            dlon -= 360.0;
        }
        if dlon < -180.0 {
            dlon += 360.0;
        }

        // Neighbor is to the west if its longitude is less (dlon > 0)
        if dlon > 0.0 {
            west_neighbor_count += 1;
            if is_high[ni] {
                mountain_west_count += 1;
            }
        }
    }

    if west_neighbor_count == 0 {
        return 0.0;
    }

    mountain_west_count as f64 / west_neighbor_count as f64
}

/// Upgrade terrain types near cities to Urban/Suburban.
/// Called after city placement — cities get Urban, their neighbors get Suburban.
pub fn apply_city_terrain(
    grid: &VoronoiGrid,
    terrains: &mut [TerrainType],
    city_cells: &[usize],
) {
    for &ci in city_cells {
        if ci < terrains.len() && terrains[ci].is_land() {
            terrains[ci] = TerrainType::Urban;
        }
    }

    // Second pass: suburban around cities (avoid overwriting other urban cells)
    for &ci in city_cells {
        if ci >= grid.cells.len() {
            continue;
        }
        for &ni in &grid.cells[ci].neighbor_indices {
            if ni < terrains.len() && terrains[ni].is_land() && terrains[ni] != TerrainType::Urban {
                terrains[ni] = TerrainType::Suburban;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::elevation;
    use crate::rivers;
    use crate::voronoi::VoronoiGrid;
    use gt_common::types::MapSize;

    #[test]
    fn test_biome_assignment() {
        let grid = VoronoiGrid::generate(MapSize::Small, 42);
        let mut elev = elevation::generate_elevation(&grid, 42, 0.7, 0.5, 4);
        let river_system = rivers::generate_rivers(&grid, &mut elev, MapSize::Small, 42);
        let terrains = assign_biomes(&grid, &elev, &river_system.cell_moisture, 0.5);

        assert_eq!(terrains.len(), grid.cell_count());

        // Should have both land and ocean
        assert!(terrains.iter().any(|t| t.is_land()), "Should have land");
        assert!(terrains.iter().any(|t| !t.is_land()), "Should have ocean");

        // Should have terrain variety
        let unique: std::collections::HashSet<_> = terrains.iter().collect();
        assert!(unique.len() >= 3, "Should have at least 3 terrain types, got {}", unique.len());
    }

    #[test]
    fn test_whittaker_hot_dry() {
        assert_eq!(classify_whittaker(30.0, 0.1, 100.0), TerrainType::Desert);
    }

    #[test]
    fn test_whittaker_cold() {
        assert_eq!(classify_whittaker(-20.0, 0.5, 100.0), TerrainType::Frozen);
    }

    #[test]
    fn test_whittaker_temperate_wet() {
        assert_eq!(classify_whittaker(10.0, 0.8, 100.0), TerrainType::Rural);
    }

    #[test]
    fn test_city_terrain_upgrade() {
        let grid = VoronoiGrid::generate(MapSize::Small, 42);
        let mut elev = elevation::generate_elevation(&grid, 42, 0.7, 0.5, 4);
        let river_system = rivers::generate_rivers(&grid, &mut elev, MapSize::Small, 42);
        let mut terrains = assign_biomes(&grid, &elev, &river_system.cell_moisture, 0.5);

        // Find a land cell to use as a city
        let land_cell = (0..grid.cell_count())
            .find(|&i| terrains[i].is_land())
            .expect("Must have land");

        apply_city_terrain(&grid, &mut terrains, &[land_cell]);
        assert_eq!(terrains[land_cell], TerrainType::Urban);
    }
}
