use gt_common::types::{TerrainType, WorldConfig};
use serde::{Deserialize, Serialize};

use crate::biomes;
use crate::cities::City;
use crate::economics::{self, EconomicData};
use crate::elevation;
use crate::grid::GeodesicGrid;
use crate::parcels::{self, LandParcel};
use crate::politics;
use crate::regions::Region;
use crate::rivers;
use crate::voronoi::VoronoiGrid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedWorld {
    pub grid: GeodesicGrid,
    pub elevations: Vec<f64>,
    pub terrains: Vec<TerrainType>,
    pub parcels: Vec<LandParcel>,
    pub regions: Vec<Region>,
    pub cities: Vec<City>,
    pub economics: EconomicData,
}

pub struct WorldGenerator {
    config: WorldConfig,
}

impl WorldGenerator {
    pub fn new(config: WorldConfig) -> Self {
        Self { config }
    }

    pub fn config(&self) -> &WorldConfig {
        &self.config
    }

    pub fn generate(&self) -> GeneratedWorld {
        // Real Earth mode: use actual geography data (unchanged)
        if self.config.use_real_earth {
            let subdivisions = self.config.map_size.grid_subdivisions();
            let grid = GeodesicGrid::new(subdivisions);
            return crate::real_earth::generate_real_earth(&grid, self.config.seed);
        }

        // Procedural generation pipeline using Voronoi tessellation
        self.generate_procgen()
    }

    /// New procedural generation pipeline:
    /// 1. Create VoronoiGrid
    /// 2. Generate elevation (tectonic-inspired)
    /// 3. Run hydraulic erosion + rivers
    /// 4. Assign biomes/terrain (Whittaker diagram)
    /// 5. Generate political boundaries
    /// 6. Place cities with river/coast preference
    /// 7. Seed economics
    fn generate_procgen(&self) -> GeneratedWorld {
        let seed = self.config.seed;

        // Step 1: Create Voronoi grid
        let voronoi = VoronoiGrid::generate(self.config.map_size, seed);

        // Step 2: Generate elevation
        let mut elevations = elevation::generate_elevation(
            &voronoi,
            seed,
            self.config.ocean_percentage,
            self.config.terrain_roughness,
            self.config.continent_count,
        );

        // Step 3: Hydraulic erosion + rivers
        let river_system = rivers::generate_rivers(
            &voronoi,
            &mut elevations,
            self.config.map_size,
            seed,
        );

        // Step 4: Assign biomes
        let mut terrains = biomes::assign_biomes(
            &voronoi,
            &elevations,
            &river_system.cell_moisture,
            self.config.climate_variation,
        );

        // Step 5: Generate political boundaries
        let (regions, _countries) = politics::generate_politics(
            &voronoi,
            &terrains,
            &elevations,
            &river_system.cell_moisture,
            self.config.map_size,
            seed,
        );

        // Step 6: Convert VoronoiGrid to GeodesicGrid for backward compatibility
        let grid = voronoi_to_geodesic(&voronoi);

        // Step 7: Place cities with river/coast preference
        let mut all_cities = place_cities_enhanced(
            &voronoi,
            &regions,
            &elevations,
            &terrains,
            &river_system,
            seed,
            self.config.city_density,
        );

        // Set city region_ids
        for city in &mut all_cities {
            for region in &regions {
                if region.cells.contains(&city.cell_index) {
                    city.region_id = region.id;
                    break;
                }
            }
        }

        // Step 8: Upgrade terrain near cities to Urban/Suburban
        let city_cells: Vec<usize> = all_cities.iter().map(|c| c.cell_index).collect();
        biomes::apply_city_terrain(&voronoi, &mut terrains, &city_cells);

        // Step 9: Create land parcels
        let parcels = parcels::create_parcels(&terrains, &elevations);

        // Step 10: Seed economics
        let economics = economics::seed_economics(
            &regions,
            &all_cities,
            &terrains,
            self.config.starting_era,
        );

        GeneratedWorld {
            grid,
            elevations,
            terrains,
            parcels,
            regions,
            cities: all_cities,
            economics,
        }
    }
}

/// Convert a VoronoiGrid to a GeodesicGrid, preserving cell indices and adjacency.
/// This maintains backward compatibility with the frontend and all downstream systems.
fn voronoi_to_geodesic(voronoi: &VoronoiGrid) -> GeodesicGrid {
    use crate::grid::GridCell;

    let cells: Vec<GridCell> = voronoi
        .cells
        .iter()
        .map(|vc| {
            let lat_r = vc.center_lat.to_radians();
            let lon_r = vc.center_lon.to_radians();
            let x = lat_r.cos() * lon_r.cos();
            let y = lat_r.cos() * lon_r.sin();
            let z = lat_r.sin();

            GridCell {
                index: vc.index,
                center: (x, y, z),
                neighbors: vc.neighbor_indices.clone(),
                lat: vc.center_lat,
                lon: vc.center_lon,
            }
        })
        .collect();

    GeodesicGrid::from_cells(cells)
}

/// Enhanced city placement that prefers river confluences and coastal locations.
fn place_cities_enhanced(
    grid: &VoronoiGrid,
    regions: &[Region],
    elevations: &[f64],
    terrains: &[TerrainType],
    river_system: &rivers::RiverSystem,
    seed: u64,
    city_density: f64,
) -> Vec<City> {
    use rand::SeedableRng;

    let mut rng = rand::rngs::StdRng::seed_from_u64(seed.wrapping_add(5555));
    let mut all_cities = Vec::new();

    // Pre-compute river presence and flow per cell
    let n = grid.cell_count();
    let mut river_flow: Vec<f64> = vec![0.0; n];
    for river in &river_system.rivers {
        for &ci in &river.cells {
            river_flow[ci] = river_flow[ci].max(river.flow);
        }
    }

    // Track occupied cells to prevent duplicate city placement
    let mut occupied = std::collections::HashSet::new();

    for (region_idx, region) in regions.iter().enumerate() {
        if region.cells.is_empty() {
            continue;
        }

        // Score each cell for city suitability
        let mut scored_cells: Vec<(usize, f64)> = region
            .cells
            .iter()
            .filter(|&&ci| terrains[ci].is_land() && !occupied.contains(&ci))
            .map(|&ci| {
                let mut score = 0.5;

                // Elevation preference: moderate elevation (50-500m) is best
                let elev = elevations[ci];
                if elev > 50.0 && elev < 500.0 {
                    score += 0.3;
                } else if elev > 500.0 && elev < 1500.0 {
                    score += 0.1;
                }

                // River bonus: higher flow = better
                if river_flow[ci] > 0.0 {
                    let river_bonus = (river_flow[ci].ln() / 10.0).clamp(0.0, 0.4);
                    score += river_bonus;
                }

                // Coastal bonus
                let near_ocean = grid.cells[ci]
                    .neighbor_indices
                    .iter()
                    .any(|&ni| !terrains[ni].is_land());
                if near_ocean {
                    score += 0.35;
                }

                // Moisture bonus
                score += river_system.cell_moisture[ci] * 0.15;

                // Terrain penalty
                match terrains[ci] {
                    TerrainType::Desert => score -= 0.3,
                    TerrainType::Mountainous => score -= 0.4,
                    TerrainType::Tundra => score -= 0.3,
                    TerrainType::Frozen => score -= 0.5,
                    _ => {}
                }

                // Center proximity: slight preference for region center
                let dlat = grid.cells[ci].center_lat - region.center_lat;
                let dlon = grid.cells[ci].center_lon - region.center_lon;
                let dist = (dlat * dlat + dlon * dlon).sqrt();
                score += (1.0 - (dist / 90.0).min(1.0)) * 0.1;

                (ci, score.max(0.01))
            })
            .collect();

        scored_cells.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Number of cities per region based on region size and density setting
        let base_count = if region.cells.len() > 80 {
            3
        } else if region.cells.len() > 30 {
            2
        } else {
            1
        };
        let city_count = ((base_count as f64 * (0.5 + city_density)) as usize)
            .max(1)
            .min(scored_cells.len());

        for (city_idx, &(cell_index, _)) in scored_cells.iter().enumerate().take(city_count) {
            // Minimum distance between cities (at least 2 hops apart)
            if occupied.contains(&cell_index) {
                continue;
            }
            let too_close = grid.cells[cell_index]
                .neighbor_indices
                .iter()
                .any(|&ni| occupied.contains(&ni));
            if too_close && city_idx > 0 {
                continue; // skip if too close, but always place first city
            }

            occupied.insert(cell_index);

            let pop_share = if city_idx == 0 {
                0.55
            } else if city_idx == 1 {
                0.28
            } else {
                0.17
            };
            let population = (region.population as f64 * pop_share) as u64;
            let development = region.development * (1.0 - city_idx as f64 * 0.12);
            let growth_rate = 0.001 + development * 0.005;

            // Bonus growth for river/coastal cities
            let location_growth_bonus = if river_flow[cell_index] > 0.0 { 0.002 } else { 0.0 }
                + if grid.cells[cell_index]
                    .neighbor_indices
                    .iter()
                    .any(|&ni| !terrains[ni].is_land())
                {
                    0.001
                } else {
                    0.0
                };

            all_cities.push(City {
                id: 0,
                name: generate_city_name(seed + region_idx as u64 * 100 + city_idx as u64, &mut rng),
                region_id: region.id,
                cell_index,
                population,
                growth_rate: growth_rate + location_growth_bonus,
                development,
            });
        }
    }

    all_cities
}

/// Generate a city name with some variation.
fn generate_city_name(seed: u64, rng: &mut rand::rngs::StdRng) -> String {
    use rand::Rng;

    let prefixes = [
        "San", "New", "Port", "Fort", "Saint", "El", "Las", "Lake", "Bay", "Old",
        "North", "South", "East", "West", "Grand", "Royal", "Iron", "Silver", "Gold",
        "Red", "Blue", "Green", "Dark", "Bright", "High", "Low", "Deep", "Far",
        "Near", "Long", "Stone", "River",
    ];
    let roots = [
        "haven", "bridge", "field", "ford", "dale", "wood", "gate", "holm", "wick",
        "stead", "cliff", "ridge", "brook", "creek", "lake", "springs", "falls",
        "mill", "cross", "well", "bury", "mond", "vale", "moor", "worth", "ham",
        "thorpe", "bourne", "crest", "peak", "mouth",
    ];

    let _ = seed; // determinism comes from rng state
    let use_prefix = rng.gen_range(0..3) != 0; // 2/3 chance
    let p = prefixes[rng.gen_range(0..prefixes.len())];
    let r = roots[rng.gen_range(0..roots.len())];

    if use_prefix {
        format!("{} {}", p, capitalize(r))
    } else {
        capitalize(r)
    }
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().to_string() + chars.as_str(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_world_generation() {
        let config = WorldConfig {
            seed: 42,
            map_size: gt_common::types::MapSize::Small,
            ..WorldConfig::default()
        };
        let gen = WorldGenerator::new(config);
        let world = gen.generate();

        assert!(world.grid.cell_count() > 100);
        assert!(!world.parcels.is_empty());
        assert!(!world.regions.is_empty());
        assert!(!world.cities.is_empty());
        assert!(world.economics.global_demand > 0.0);
    }

    #[test]
    fn test_determinism() {
        let config1 = WorldConfig {
            seed: 12345,
            map_size: gt_common::types::MapSize::Small,
            ..WorldConfig::default()
        };
        let config2 = config1.clone();

        let w1 = WorldGenerator::new(config1).generate();
        let w2 = WorldGenerator::new(config2).generate();

        assert_eq!(w1.grid.cell_count(), w2.grid.cell_count());
        assert_eq!(w1.parcels.len(), w2.parcels.len());
        assert_eq!(w1.regions.len(), w2.regions.len());
        assert_eq!(w1.cities.len(), w2.cities.len());
        assert_eq!(w1.elevations, w2.elevations);
    }

    #[test]
    fn test_different_seeds() {
        let w1 = WorldGenerator::new(WorldConfig {
            seed: 1,
            map_size: gt_common::types::MapSize::Small,
            ..WorldConfig::default()
        })
        .generate();
        let w2 = WorldGenerator::new(WorldConfig {
            seed: 999,
            map_size: gt_common::types::MapSize::Small,
            ..WorldConfig::default()
        })
        .generate();

        // Different seeds should produce different elevations
        assert_ne!(w1.elevations, w2.elevations);
    }

    #[test]
    fn test_cities_have_urban_terrain() {
        let w = WorldGenerator::new(WorldConfig {
            seed: 42,
            map_size: gt_common::types::MapSize::Small,
            ..WorldConfig::default()
        })
        .generate();

        for city in &w.cities {
            assert_eq!(w.terrains[city.cell_index], TerrainType::Urban);
        }
    }

    #[test]
    fn test_real_earth_still_works() {
        let config = WorldConfig {
            seed: 42,
            map_size: gt_common::types::MapSize::Small,
            use_real_earth: true,
            ..WorldConfig::default()
        };
        let gen = WorldGenerator::new(config);
        let world = gen.generate();

        assert!(!world.regions.is_empty());
        assert!(!world.cities.is_empty());
    }

    #[test]
    fn test_voronoi_grid_backward_compat() {
        let w = WorldGenerator::new(WorldConfig {
            seed: 42,
            map_size: gt_common::types::MapSize::Small,
            ..WorldConfig::default()
        })
        .generate();

        // Grid cells should have valid lat/lon and neighbors
        for cell in &w.grid.cells {
            assert!(cell.lat >= -90.0 && cell.lat <= 90.0);
            assert!(cell.lon >= -180.0 && cell.lon <= 180.0);
            assert!(!cell.neighbors.is_empty());
            // Center should be on unit sphere
            let len = (cell.center.0.powi(2) + cell.center.1.powi(2) + cell.center.2.powi(2)).sqrt();
            assert!(
                (len - 1.0).abs() < 0.01,
                "Cell {} center not on unit sphere: len={}",
                cell.index,
                len
            );
        }
    }

    #[test]
    fn test_terrain_variety() {
        let w = WorldGenerator::new(WorldConfig {
            seed: 42,
            map_size: gt_common::types::MapSize::Small,
            ..WorldConfig::default()
        })
        .generate();

        let has_ocean = w.terrains.iter().any(|t| !t.is_land());
        let has_land = w.terrains.iter().any(|t| t.is_land());
        let has_urban = w.terrains.iter().any(|t| *t == TerrainType::Urban);

        assert!(has_ocean, "World should have ocean");
        assert!(has_land, "World should have land");
        assert!(has_urban, "World should have urban terrain from cities");
    }

    #[test]
    fn test_procgen_with_presets() {
        use crate::config::{apply_preset, WorldPreset};

        // Test Pangaea
        let mut config = WorldConfig {
            seed: 42,
            map_size: gt_common::types::MapSize::Small,
            ..WorldConfig::default()
        };
        apply_preset(&mut config, WorldPreset::Pangaea);
        let world = WorldGenerator::new(config).generate();
        assert!(!world.regions.is_empty());

        // Test Archipelago
        let mut config = WorldConfig {
            seed: 42,
            map_size: gt_common::types::MapSize::Small,
            ..WorldConfig::default()
        };
        apply_preset(&mut config, WorldPreset::Archipelago);
        let world = WorldGenerator::new(config).generate();
        assert!(!world.regions.is_empty());
    }
}
