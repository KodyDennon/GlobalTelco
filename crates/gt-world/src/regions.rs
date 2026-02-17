use gt_common::types::{EntityId, MapSize, TerrainType};
use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::grid::GeodesicGrid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Region {
    pub id: EntityId,
    pub name: String,
    pub cells: Vec<usize>,
    pub center_lat: f64,
    pub center_lon: f64,
    pub gdp: f64,
    pub population: u64,
    pub development: f64,
}

pub fn cluster_regions(
    grid: &GeodesicGrid,
    terrains: &[TerrainType],
    elevations: &[f64],
    map_size: MapSize,
    seed: u64,
) -> Vec<Region> {
    let k = match map_size {
        MapSize::Small => 8,
        MapSize::Medium => 16,
        MapSize::Large => 24,
        MapSize::Huge => 32,
    };

    let land_indices: Vec<usize> = (0..grid.cells.len())
        .filter(|&i| terrains[i].is_land())
        .collect();

    if land_indices.is_empty() {
        return Vec::new();
    }

    let k = k.min(land_indices.len());
    let mut rng = rand::rngs::StdRng::seed_from_u64(seed);

    // Seed initial centers on land cells
    let mut centers: Vec<(f64, f64, f64)> = Vec::with_capacity(k);
    let mut used = std::collections::HashSet::new();
    while centers.len() < k {
        let idx = land_indices[rng.gen_range(0..land_indices.len())];
        if used.insert(idx) {
            centers.push(grid.cells[idx].center);
        }
    }

    // K-means iterations
    let mut assignments = vec![0usize; grid.cells.len()];
    for _ in 0..20 {
        // Assign each land cell to nearest center
        for &li in &land_indices {
            let cell = &grid.cells[li];
            let mut best = 0;
            let mut best_dist = f64::MAX;
            for (ci, center) in centers.iter().enumerate() {
                let d = dist_sq_3d(cell.center, *center);
                if d < best_dist {
                    best_dist = d;
                    best = ci;
                }
            }
            assignments[li] = best;
        }

        // Recompute centers
        let mut sums = vec![(0.0, 0.0, 0.0); k];
        let mut counts = vec![0usize; k];
        for &li in &land_indices {
            let ci = assignments[li];
            let c = grid.cells[li].center;
            sums[ci].0 += c.0;
            sums[ci].1 += c.1;
            sums[ci].2 += c.2;
            counts[ci] += 1;
        }
        for i in 0..k {
            if counts[i] > 0 {
                let n = counts[i] as f64;
                let (x, y, z) = (sums[i].0 / n, sums[i].1 / n, sums[i].2 / n);
                // Project back to unit sphere
                let len = (x * x + y * y + z * z).sqrt();
                if len > 0.0 {
                    centers[i] = (x / len, y / len, z / len);
                }
            }
        }
    }

    // Build regions from assignments
    let mut region_cells: Vec<Vec<usize>> = vec![Vec::new(); k];
    for &li in &land_indices {
        region_cells[assignments[li]].push(li);
    }

    region_cells
        .into_iter()
        .enumerate()
        .filter(|(_, cells)| !cells.is_empty())
        .enumerate()
        .map(|(region_idx, (_, cells))| {
            let center = centers[region_idx.min(centers.len() - 1)];
            let center_lat = center.2.asin().to_degrees();
            let center_lon = center.1.atan2(center.0).to_degrees();

            // Development based on terrain quality and elevation
            let avg_elevation: f64 =
                cells.iter().map(|&c| elevations[c]).sum::<f64>() / cells.len() as f64;
            let coastal_count = cells
                .iter()
                .filter(|&&c| terrains[c] == TerrainType::Coastal)
                .count();
            let coastal_bonus = (coastal_count as f64 / cells.len() as f64) * 0.3;
            let development =
                (0.3 + (1.0 - avg_elevation.abs()) * 0.5 + coastal_bonus).clamp(0.1, 1.0);

            let population = (cells.len() as f64 * development * 500_000.0) as u64;
            let gdp = population as f64 * development * 30_000.0;

            Region {
                id: 0, // Will be assigned during entity creation
                name: generate_region_name(region_idx as u64 + seed),
                cells,
                center_lat,
                center_lon,
                gdp,
                population,
                development,
            }
        })
        .collect()
}

fn dist_sq_3d(a: (f64, f64, f64), b: (f64, f64, f64)) -> f64 {
    let dx = a.0 - b.0;
    let dy = a.1 - b.1;
    let dz = a.2 - b.2;
    dx * dx + dy * dy + dz * dz
}

fn generate_region_name(seed: u64) -> String {
    let prefixes = [
        "Al", "Bor", "Cor", "Del", "Er", "Fal", "Gar", "Hel", "Im", "Jar", "Kal", "Lor", "Mar",
        "Nor", "Or", "Pal", "Rav", "Sal", "Tar", "Val", "Wen", "Xar", "Yor", "Zan", "Aur", "Bel",
        "Cel", "Dor", "Eth", "Fen", "Gil", "Hal",
    ];
    let middles = [
        "an", "en", "in", "on", "un", "ar", "er", "ir", "or", "ur", "al", "el", "il", "ol", "ul",
        "as", "es", "is",
    ];
    let suffixes = [
        "ia", "ica", "istan", "land", "mar", "heim", "grad", "burg", "ford", "dale", "vale",
        "shire", "ton", "den", "berg", "haven", "port", "ven", "gard", "oth",
    ];

    let mut hash = seed;
    hash = hash
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    let p = prefixes[(hash % prefixes.len() as u64) as usize];
    hash = hash
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    let m = middles[(hash % middles.len() as u64) as usize];
    hash = hash
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    let s = suffixes[(hash % suffixes.len() as u64) as usize];

    format!("{}{}{}", p, m, s)
}

use rand::SeedableRng;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_region_naming() {
        let n1 = generate_region_name(1);
        let n2 = generate_region_name(2);
        assert_ne!(n1, n2);
        assert!(!n1.is_empty());
    }
}
