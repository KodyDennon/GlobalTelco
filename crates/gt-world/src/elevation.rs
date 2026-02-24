use noise::{NoiseFn, SuperSimplex};
use rand::Rng;
use rand::SeedableRng;

use crate::voronoi::VoronoiGrid;

/// Generate elevation values for each Voronoi cell using tectonic-inspired simulation.
///
/// Pipeline:
/// 1. Multi-octave simplex noise for base continental shapes
/// 2. Tectonic plate regions via flood fill from random seeds
/// 3. Raised elevation along convergent plate boundaries (mountain ranges)
/// 4. Continental shelf gradual depth from coastline
///
/// Returns one elevation value per cell, range approximately -10000 to 9000 meters.
pub fn generate_elevation(
    grid: &VoronoiGrid,
    seed: u64,
    ocean_percentage: f64,
    terrain_roughness: f64,
    continent_count: u8,
) -> Vec<f64> {
    let n = grid.cell_count();
    let mut rng = rand::rngs::StdRng::seed_from_u64(seed);

    // Step 1: Multi-octave simplex noise for base elevation
    let simplex = SuperSimplex::new(seed as u32);
    let base_frequency = 1.5 + terrain_roughness * 1.5; // 1.5-3.0
    let octaves = 4 + (terrain_roughness * 3.0) as u32; // 4-7

    let mut elevations: Vec<f64> = grid
        .cells
        .iter()
        .map(|cell| {
            let (x, y, z) = latlon_to_xyz(cell.center_lat, cell.center_lon);
            sample_fractal_noise(&simplex, x, y, z, base_frequency, octaves, terrain_roughness)
        })
        .collect();

    // Step 2: Tectonic plates via flood fill
    let num_plates = continent_count.max(3) as usize + rng.gen_range(2..5);
    let plate_assignments = assign_tectonic_plates(grid, num_plates, &mut rng);

    // Assign plate velocity vectors (random direction on tangent plane)
    let mut plate_velocities: Vec<(f64, f64, f64)> = Vec::with_capacity(num_plates);
    for _ in 0..num_plates {
        let vx: f64 = rng.gen_range(-1.0..1.0);
        let vy: f64 = rng.gen_range(-1.0..1.0);
        let vz: f64 = rng.gen_range(-1.0..1.0);
        let len = (vx * vx + vy * vy + vz * vz).sqrt().max(0.01);
        plate_velocities.push((vx / len, vy / len, vz / len));
    }

    // Step 3: Raise elevation along convergent plate boundaries
    for i in 0..n {
        let my_plate = plate_assignments[i];
        let mut is_boundary = false;
        let mut convergent_strength: f64 = 0.0;

        for &ni in &grid.cells[i].neighbor_indices {
            let neighbor_plate = plate_assignments[ni];
            if neighbor_plate != my_plate {
                is_boundary = true;
                // Check convergence: dot product of velocity difference with boundary normal
                let v1 = plate_velocities[my_plate];
                let v2 = plate_velocities[neighbor_plate];
                let diff = (v1.0 - v2.0, v1.1 - v2.1, v1.2 - v2.2);
                let (cx, cy, cz) = latlon_to_xyz(
                    grid.cells[i].center_lat,
                    grid.cells[i].center_lon,
                );
                // Project velocity diff onto radial direction
                let convergence = -(diff.0 * cx + diff.1 * cy + diff.2 * cz);
                if convergence > 0.0 {
                    convergent_strength = convergent_strength.max(convergence);
                }
            }
        }

        if is_boundary {
            // Mountain ranges at convergent boundaries
            elevations[i] += convergent_strength * 5000.0;
            // Slight ridge at all plate boundaries
            elevations[i] += 500.0;
        }
    }

    // Step 4: Determine sea level from ocean_percentage
    let mut sorted_elev: Vec<f64> = elevations.clone();
    sorted_elev.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let sea_level_idx = (ocean_percentage.clamp(0.3, 0.9) * n as f64) as usize;
    let sea_level = sorted_elev[sea_level_idx.min(n - 1)];

    // Step 5: Scale to realistic meters
    // Land: 0 to ~9000m, Ocean: 0 to ~-10000m
    let max_elev = elevations
        .iter()
        .cloned()
        .fold(f64::NEG_INFINITY, f64::max);
    let min_elev = elevations
        .iter()
        .cloned()
        .fold(f64::INFINITY, f64::min);

    for i in 0..n {
        if elevations[i] >= sea_level {
            // Land: normalize to 0-9000m
            let range = (max_elev - sea_level).max(0.001);
            elevations[i] = ((elevations[i] - sea_level) / range) * 9000.0;
        } else {
            // Ocean: normalize to -10000-0m
            let range = (sea_level - min_elev).max(0.001);
            elevations[i] = -((sea_level - elevations[i]) / range) * 10000.0;
        }
    }

    // Step 6: Continental shelf — shallow water near coastlines
    let is_land: Vec<bool> = elevations.iter().map(|&e| e >= 0.0).collect();
    let coast_dist = compute_coast_distance(grid, &is_land, 5);

    for i in 0..n {
        if !is_land[i] && coast_dist[i] < 5 {
            // Gradually deepen from coast: shelf effect
            let depth_factor = coast_dist[i] as f64 / 5.0;
            let shelf_depth = -200.0 - depth_factor * 2000.0;
            elevations[i] = elevations[i].max(shelf_depth);
        }
    }

    elevations
}

fn sample_fractal_noise(
    noise: &SuperSimplex,
    x: f64,
    y: f64,
    z: f64,
    base_freq: f64,
    octaves: u32,
    roughness: f64,
) -> f64 {
    let mut value = 0.0;
    let mut amplitude = 1.0;
    let mut frequency = base_freq;
    let mut max_amplitude = 0.0;
    let persistence = 0.45 + roughness * 0.15; // 0.45-0.60

    for _ in 0..octaves {
        value += amplitude * noise.get([x * frequency, y * frequency, z * frequency]);
        max_amplitude += amplitude;
        amplitude *= persistence;
        frequency *= 2.0;
    }

    value / max_amplitude
}

/// Assign each cell to a tectonic plate using flood fill from random seed cells.
fn assign_tectonic_plates(
    grid: &VoronoiGrid,
    num_plates: usize,
    rng: &mut rand::rngs::StdRng,
) -> Vec<usize> {
    let n = grid.cell_count();
    let mut assignments = vec![usize::MAX; n];
    let mut queue = std::collections::VecDeque::new();

    // Seed one random cell per plate
    let mut seeded = std::collections::HashSet::new();
    for plate in 0..num_plates {
        loop {
            let idx = rng.gen_range(0..n);
            if seeded.insert(idx) {
                assignments[idx] = plate;
                queue.push_back(idx);
                break;
            }
        }
    }

    // BFS flood fill
    while let Some(ci) = queue.pop_front() {
        let plate = assignments[ci];
        for &ni in &grid.cells[ci].neighbor_indices {
            if assignments[ni] == usize::MAX {
                assignments[ni] = plate;
                queue.push_back(ni);
            }
        }
    }

    // Handle any unassigned cells (shouldn't happen with connected graph)
    for i in 0..n {
        if assignments[i] == usize::MAX {
            assignments[i] = 0;
        }
    }

    assignments
}

/// BFS to compute distance (in hops) from nearest coastline cell.
fn compute_coast_distance(grid: &VoronoiGrid, is_land: &[bool], max_dist: usize) -> Vec<usize> {
    let n = grid.cell_count();
    let mut dist = vec![usize::MAX; n];
    let mut queue = std::collections::VecDeque::new();

    // Find coastal cells: ocean cells adjacent to land
    for i in 0..n {
        if !is_land[i] {
            let near_land = grid.cells[i]
                .neighbor_indices
                .iter()
                .any(|&ni| is_land[ni]);
            if near_land {
                dist[i] = 0;
                queue.push_back(i);
            }
        }
    }

    while let Some(ci) = queue.pop_front() {
        let d = dist[ci];
        if d >= max_dist {
            continue;
        }
        for &ni in &grid.cells[ci].neighbor_indices {
            if dist[ni] > d + 1 {
                dist[ni] = d + 1;
                queue.push_back(ni);
            }
        }
    }

    dist
}

fn latlon_to_xyz(lat: f64, lon: f64) -> (f64, f64, f64) {
    let lat_r = lat.to_radians();
    let lon_r = lon.to_radians();
    (
        lat_r.cos() * lon_r.cos(),
        lat_r.cos() * lon_r.sin(),
        lat_r.sin(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use gt_common::types::MapSize;

    #[test]
    fn test_elevation_range() {
        let grid = VoronoiGrid::generate(MapSize::Small, 42);
        let elev = generate_elevation(&grid, 42, 0.7, 0.5, 4);
        assert_eq!(elev.len(), grid.cell_count());
        let min = elev.iter().cloned().fold(f64::INFINITY, f64::min);
        let max = elev.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        assert!(min < 0.0, "Should have ocean cells below 0");
        assert!(max > 0.0, "Should have land cells above 0");
        assert!(min >= -10001.0, "Min elevation too low: {}", min);
        assert!(max <= 9001.0, "Max elevation too high: {}", max);
    }

    #[test]
    fn test_elevation_deterministic() {
        let grid = VoronoiGrid::generate(MapSize::Small, 42);
        let e1 = generate_elevation(&grid, 42, 0.7, 0.5, 4);
        let e2 = generate_elevation(&grid, 42, 0.7, 0.5, 4);
        assert_eq!(e1, e2);
    }

    #[test]
    fn test_ocean_percentage() {
        let grid = VoronoiGrid::generate(MapSize::Small, 42);
        let elev = generate_elevation(&grid, 42, 0.7, 0.5, 4);
        let ocean_count = elev.iter().filter(|&&e| e < 0.0).count();
        let ocean_frac = ocean_count as f64 / elev.len() as f64;
        // Should be approximately 70% ocean (with some tolerance from post-processing)
        assert!(
            ocean_frac > 0.5 && ocean_frac < 0.9,
            "Ocean fraction {} out of expected range",
            ocean_frac
        );
    }
}
