use gt_common::types::{TerrainType, WorldConfig};
use serde::{Deserialize, Serialize};

use crate::cities::{self, City};
use crate::economics::{self, EconomicData};
use crate::grid::GeodesicGrid;
use crate::parcels::{self, LandParcel};
use crate::regions::{self, Region};
use crate::terrain::TerrainGenerator;

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
        let subdivisions = self.config.map_size.grid_subdivisions();

        // Real Earth mode: use actual geography data
        if self.config.use_real_earth {
            let grid = GeodesicGrid::new(subdivisions);
            return crate::real_earth::generate_real_earth(&grid, self.config.seed);
        }

        // 1. Build geodesic grid
        let grid = GeodesicGrid::new(subdivisions);

        // 2. Generate elevation using fractal noise
        let terrain_gen = TerrainGenerator::new(self.config.seed as u32, 0.0);
        let elevations = terrain_gen.generate_elevation(&grid);

        // 3. Classify terrain
        let terrains = terrain_gen.classify_terrain(&grid, &elevations);

        // 4. Create land parcels
        let parcels = parcels::create_parcels(&terrains, &elevations);

        // 5. Cluster into regions
        let mut regions = regions::cluster_regions(
            &grid,
            &terrains,
            &elevations,
            self.config.map_size,
            self.config.seed,
        );

        // Assign temporary IDs to regions so cities can reference them
        for (i, region) in regions.iter_mut().enumerate() {
            region.id = (i + 1) as u64; // Temporary, will be reassigned in GameWorld
        }

        // 6. Place cities
        let mut all_cities = cities::place_cities(&grid, &regions, &elevations, self.config.seed);

        // Set city region_ids to match the temp region IDs
        for city in &mut all_cities {
            // Find which region contains this city's cell
            for region in &regions {
                if region.cells.contains(&city.cell_index) {
                    city.region_id = region.id;
                    break;
                }
            }
        }

        // 7. Seed economics
        let economics =
            economics::seed_economics(&regions, &all_cities, &terrains, self.config.starting_era);

        // 8. Upgrade terrain near cities to Urban/Suburban
        let mut terrains = terrains;
        for city in &all_cities {
            terrains[city.cell_index] = TerrainType::Urban;
            // Mark neighbors as suburban
            let neighbors: Vec<usize> = grid.cells[city.cell_index].neighbors.clone();
            for &n in &neighbors {
                if terrains[n].is_land() && terrains[n] != TerrainType::Urban {
                    terrains[n] = TerrainType::Suburban;
                }
            }
        }

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
}
