use gt_common::types::TerrainType;
use noise::{NoiseFn, Perlin};

use crate::grid::GeodesicGrid;

pub struct TerrainGenerator {
    perlin: Perlin,
    sea_level: f64,
    frequency: f64,
    octaves: u32,
}

impl TerrainGenerator {
    pub fn new(seed: u32, sea_level: f64) -> Self {
        Self {
            perlin: Perlin::new(seed),
            sea_level,
            frequency: 2.0,
            octaves: 4,
        }
    }

    pub fn generate_elevation(&self, grid: &GeodesicGrid) -> Vec<f64> {
        grid.cells
            .iter()
            .map(|cell| self.sample_elevation(cell.center.0, cell.center.1, cell.center.2))
            .collect()
    }

    fn sample_elevation(&self, x: f64, y: f64, z: f64) -> f64 {
        let mut value = 0.0;
        let mut amplitude = 1.0;
        let mut frequency = self.frequency;
        let mut max_amplitude = 0.0;

        for _ in 0..self.octaves {
            value += amplitude
                * self
                    .perlin
                    .get([x * frequency, y * frequency, z * frequency]);
            max_amplitude += amplitude;
            amplitude *= 0.5;
            frequency *= 2.0;
        }

        value / max_amplitude
    }

    pub fn classify_terrain(&self, grid: &GeodesicGrid, elevations: &[f64]) -> Vec<TerrainType> {
        let is_land: Vec<bool> = elevations.iter().map(|&e| e > self.sea_level).collect();

        grid.cells
            .iter()
            .enumerate()
            .map(|(i, cell)| {
                let elevation = elevations[i];

                if elevation <= self.sea_level {
                    if elevation > self.sea_level - 0.15 {
                        return TerrainType::OceanShallow;
                    }
                    // Extreme depths: ocean trenches (subduction zones, abyssal chasms)
                    if elevation < self.sea_level - 0.65 {
                        return TerrainType::OceanTrench;
                    }
                    return TerrainType::OceanDeep;
                }

                // Check coastal: any ocean neighbor
                let is_coastal = cell.neighbors.iter().any(|&n| !is_land[n]);
                if is_coastal && elevation < self.sea_level + 0.1 {
                    return TerrainType::Coastal;
                }

                // High elevation
                if elevation > self.sea_level + 0.5 {
                    return TerrainType::Mountainous;
                }

                // High latitude
                let abs_lat = cell.lat.abs();
                if abs_lat > 70.0 {
                    return TerrainType::Frozen;
                }
                if abs_lat > 55.0 {
                    return TerrainType::Tundra;
                }

                // Desert: low-mid latitude, moderate elevation, dry band (~15-35 degrees)
                if abs_lat > 15.0 && abs_lat < 35.0 && elevation < self.sea_level + 0.3 {
                    // Use noise to create patchy deserts
                    let desert_noise = self.perlin.get([
                        cell.center.0 * 3.0 + 100.0,
                        cell.center.1 * 3.0 + 100.0,
                        cell.center.2 * 3.0 + 100.0,
                    ]);
                    if desert_noise > 0.0 {
                        return TerrainType::Desert;
                    }
                }

                // Default land categories based on elevation
                if elevation > self.sea_level + 0.35 {
                    TerrainType::Rural
                } else if elevation > self.sea_level + 0.2 {
                    TerrainType::Suburban
                } else {
                    TerrainType::Rural
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_elevation_generation() {
        let grid = GeodesicGrid::new(2);
        let terrain_gen = TerrainGenerator::new(42, 0.0);
        let elevations = terrain_gen.generate_elevation(&grid);
        assert_eq!(elevations.len(), grid.cell_count());
        // Should have both positive and negative values
        assert!(elevations.iter().any(|&e| e > 0.0));
        assert!(elevations.iter().any(|&e| e < 0.0));
    }

    #[test]
    fn test_terrain_classification() {
        let grid = GeodesicGrid::new(2);
        let terrain_gen = TerrainGenerator::new(42, 0.0);
        let elevations = terrain_gen.generate_elevation(&grid);
        let terrains = terrain_gen.classify_terrain(&grid, &elevations);
        assert_eq!(terrains.len(), grid.cell_count());
        // Should have ocean and land
        assert!(terrains.iter().any(|t| !t.is_land()));
        assert!(terrains.iter().any(|t| t.is_land()));
    }

    #[test]
    fn test_deterministic() {
        let grid = GeodesicGrid::new(2);
        let gen1 = TerrainGenerator::new(42, 0.0);
        let gen2 = TerrainGenerator::new(42, 0.0);
        let e1 = gen1.generate_elevation(&grid);
        let e2 = gen2.generate_elevation(&grid);
        assert_eq!(e1, e2);
    }
}
