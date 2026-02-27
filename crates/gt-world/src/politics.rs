use gt_common::types::{MapSize, TerrainType};
use rand::Rng;
use rand::SeedableRng;
use serde::{Deserialize, Serialize};
use std::collections::BinaryHeap;

use crate::regions::Region;
use crate::voronoi::VoronoiGrid;

/// A country composed of multiple political regions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Country {
    pub name: String,
    pub capital_region_idx: usize,
    pub region_indices: Vec<usize>,
}

/// Generate political regions and countries via terrain-aware flood fill from capitals.
///
/// Pipeline:
/// 1. Score cells for capital suitability (coastal, moderate elevation, not extreme terrain)
/// 2. Pick N capital locations using greedy furthest-point sampling
/// 3. Terrain-aware Dijkstra flood fill: expansion cost varies by terrain
/// 4. Each resulting region gets name, GDP, population
/// 5. Group regions into countries (2-8 regions per country)
///
/// Returns (regions, countries).
pub fn generate_politics(
    grid: &VoronoiGrid,
    terrains: &[TerrainType],
    elevations: &[f64],
    moisture: &[f64],
    map_size: MapSize,
    seed: u64,
) -> (Vec<Region>, Vec<Country>) {
    let n = grid.cell_count();
    let mut rng = rand::rngs::StdRng::seed_from_u64(seed.wrapping_add(9999));

    let land_indices: Vec<usize> = (0..n).filter(|&i| terrains[i].is_land()).collect();
    if land_indices.is_empty() {
        return (Vec::new(), Vec::new());
    }

    // Number of regions based on map size
    let num_regions = match map_size {
        MapSize::Small => 8,
        MapSize::Medium => 16,
        MapSize::Large => 24,
        MapSize::Huge => 32,
    };
    let num_regions = num_regions.min(land_indices.len());

    // Step 1: Score cells for capital suitability
    let suitability: Vec<f64> = (0..n)
        .map(|i| {
            if !terrains[i].is_land() {
                return 0.0;
            }
            let mut score = 0.5;
            // Prefer coastal cells
            if terrains[i] == TerrainType::Coastal {
                score += 0.3;
            }
            let near_coast = grid.cells[i]
                .neighbor_indices
                .iter()
                .any(|&ni| !terrains[ni].is_land());
            if near_coast {
                score += 0.15;
            }
            // Prefer moderate elevation (100-500m)
            if elevations[i] > 100.0 && elevations[i] < 500.0 {
                score += 0.2;
            }
            // Prefer moist areas
            score += moisture[i] * 0.15;
            // Penalize extreme terrain
            match terrains[i] {
                TerrainType::Desert => score -= 0.2,
                TerrainType::Mountainous => score -= 0.3,
                TerrainType::Tundra | TerrainType::Frozen => score -= 0.4,
                _ => {}
            }
            score.max(0.01)
        })
        .collect();

    // Step 2: Greedy furthest-point sampling for capital placement
    let capitals = select_capitals(grid, &suitability, &land_indices, num_regions, &mut rng);

    // Step 3: Terrain-aware Dijkstra flood fill
    let assignments = terrain_flood_fill(grid, terrains, elevations, &capitals, n);

    // Step 4: Build regions
    let mut region_cells: Vec<Vec<usize>> = vec![Vec::new(); num_regions];
    for i in 0..n {
        if assignments[i] < num_regions && terrains[i].is_land() {
            region_cells[assignments[i]].push(i);
        }
    }

    let mut regions: Vec<Region> = region_cells
        .into_iter()
        .enumerate()
        .filter(|(_, cells)| !cells.is_empty())
        .map(|(idx, cells)| {
            let capital = capitals[idx.min(capitals.len() - 1)];
            let center_lat = grid.cells[capital].center_lat;
            let center_lon = grid.cells[capital].center_lon;

            // Development based on terrain quality
            let good_terrain_count = cells
                .iter()
                .filter(|&&c| {
                    matches!(
                        terrains[c],
                        TerrainType::Coastal | TerrainType::Rural | TerrainType::Suburban
                    )
                })
                .count();
            let development = (good_terrain_count as f64 / cells.len().max(1) as f64 * 0.7 + 0.2)
                .clamp(0.1, 1.0);

            let area_factor = cells.len() as f64;
            let population = (area_factor * development * 300_000.0) as u64;
            let gdp = population as f64 * development * 25_000.0;

            Region {
                id: 0,
                name: generate_region_name(seed + idx as u64, &mut rng),
                cells,
                center_lat,
                center_lon,
                gdp,
                population,
                development,
                boundary_polygon: Vec::new(),
            }
        })
        .collect();

    // Compute boundary polygons
    {
        let mut full_assignments = vec![0usize; n];
        for (ri, region) in regions.iter().enumerate() {
            for &ci in &region.cells {
                full_assignments[ci] = ri;
            }
        }
        crate::regions::compute_region_boundaries(
            &mut regions,
            &convert_voronoi_to_boundary_grid(grid),
            &full_assignments,
            &land_indices,
        );
    }

    // Assign temporary IDs
    for (i, region) in regions.iter_mut().enumerate() {
        region.id = (i + 1) as u64;
    }

    // Step 5: Group regions into countries
    let countries = group_into_countries(&regions, grid, &mut rng);

    (regions, countries)
}

/// Select capital positions using greedy furthest-point sampling,
/// weighted by suitability scores.
fn select_capitals(
    grid: &VoronoiGrid,
    suitability: &[f64],
    land_indices: &[usize],
    count: usize,
    rng: &mut rand::rngs::StdRng,
) -> Vec<usize> {
    let mut capitals = Vec::with_capacity(count);

    // First capital: highest suitability cell (with random tiebreaking)
    let first = *land_indices
        .iter()
        .max_by(|&&a, &&b| {
            let sa = suitability[a] + rng.gen::<f64>() * 0.01;
            let sb = suitability[b] + rng.gen::<f64>() * 0.01;
            sa.partial_cmp(&sb).unwrap_or(std::cmp::Ordering::Equal)
        })
        .unwrap();
    capitals.push(first);

    // Subsequent capitals: maximize minimum distance to existing capitals, weighted by suitability
    for _ in 1..count {
        let mut best_cell = land_indices[0];
        let mut best_score = f64::NEG_INFINITY;

        for &li in land_indices {
            let min_dist = capitals
                .iter()
                .map(|&c| {
                    let ci = &grid.cells[c];
                    let li_cell = &grid.cells[li];
                    let dlat = ci.center_lat - li_cell.center_lat;
                    let dlon = ci.center_lon - li_cell.center_lon;
                    dlat * dlat + dlon * dlon
                })
                .fold(f64::INFINITY, f64::min);

            let score = min_dist.sqrt() * suitability[li];
            if score > best_score {
                best_score = score;
                best_cell = li;
            }
        }

        capitals.push(best_cell);
    }

    capitals
}

#[derive(Clone, PartialEq)]
struct FloodEntry {
    cost: f64,
    cell: usize,
    region: usize,
}

impl Eq for FloodEntry {}

impl PartialOrd for FloodEntry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for FloodEntry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Reverse for min-heap behavior with BinaryHeap (max-heap)
        other
            .cost
            .partial_cmp(&self.cost)
            .unwrap_or(std::cmp::Ordering::Equal)
    }
}

/// Dijkstra-based flood fill with terrain-dependent costs.
fn terrain_flood_fill(
    grid: &VoronoiGrid,
    terrains: &[TerrainType],
    elevations: &[f64],
    capitals: &[usize],
    n: usize,
) -> Vec<usize> {
    let mut assignments = vec![usize::MAX; n];
    let mut costs = vec![f64::INFINITY; n];
    let mut heap = BinaryHeap::new();

    for (region_idx, &capital) in capitals.iter().enumerate() {
        assignments[capital] = region_idx;
        costs[capital] = 0.0;
        heap.push(FloodEntry {
            cost: 0.0,
            cell: capital,
            region: region_idx,
        });
    }

    while let Some(entry) = heap.pop() {
        if entry.cost > costs[entry.cell] {
            continue;
        }

        for &ni in &grid.cells[entry.cell].neighbor_indices {
            if !terrains[ni].is_land() {
                continue;
            }

            // Cost depends on terrain type and elevation difference
            let terrain_cost = expansion_cost(terrains[ni]);
            let elev_diff = (elevations[ni] - elevations[entry.cell]).abs();
            let elevation_cost = (elev_diff / 1000.0) * 2.0; // steepness penalty
            let total_cost = entry.cost + terrain_cost + elevation_cost;

            if total_cost < costs[ni] {
                costs[ni] = total_cost;
                assignments[ni] = entry.region;
                heap.push(FloodEntry {
                    cost: total_cost,
                    cell: ni,
                    region: entry.region,
                });
            }
        }
    }

    assignments
}

/// Expansion cost per terrain type. Natural barriers slow expansion.
fn expansion_cost(terrain: TerrainType) -> f64 {
    match terrain {
        TerrainType::Rural => 1.0,
        TerrainType::Suburban => 1.0,
        TerrainType::Coastal => 1.5,     // coastlines form natural borders
        TerrainType::Desert => 2.5,       // deserts hard to cross
        TerrainType::Mountainous => 5.0,  // mountains are major barriers
        TerrainType::Tundra => 3.0,
        TerrainType::Frozen => 4.0,
        TerrainType::Urban => 1.0,
        TerrainType::OceanShallow | TerrainType::OceanDeep | TerrainType::OceanTrench => f64::INFINITY,
    }
}

/// Group regions into countries of 2-8 regions based on adjacency.
fn group_into_countries(
    regions: &[Region],
    grid: &VoronoiGrid,
    rng: &mut rand::rngs::StdRng,
) -> Vec<Country> {
    if regions.is_empty() {
        return Vec::new();
    }

    let num_regions = regions.len();
    // Target: roughly num_regions / 4 countries (clamped to at least 1)
    let target_countries = (num_regions / 4).max(1).min(num_regions);
    let target_per_country = (num_regions / target_countries).max(2);

    // Build region adjacency: two regions are adjacent if they share boundary cells
    let mut region_adj: Vec<Vec<usize>> = vec![Vec::new(); num_regions];
    for i in 0..num_regions {
        let region_set: std::collections::HashSet<usize> =
            regions[i].cells.iter().copied().collect();
        for (j, other) in regions.iter().enumerate().skip(i + 1) {
            let adjacent = other.cells.iter().any(|&ci| {
                grid.cells[ci]
                    .neighbor_indices
                    .iter()
                    .any(|&ni| region_set.contains(&ni))
            });
            if adjacent {
                region_adj[i].push(j);
                region_adj[j].push(i);
            }
        }
    }

    // Greedy grouping
    let mut assigned = vec![false; num_regions];
    let mut countries = Vec::new();

    // Start with the region with fewest neighbors (tends to be isolated/corner)
    let mut order: Vec<usize> = (0..num_regions).collect();
    order.sort_by_key(|&i| region_adj[i].len());

    for &start in &order {
        if assigned[start] {
            continue;
        }
        let mut group = vec![start];
        assigned[start] = true;

        // Expand via adjacent unassigned regions
        let max_size = rng.gen_range(2..=target_per_country.min(8));
        let mut frontier: Vec<usize> = region_adj[start]
            .iter()
            .filter(|&&r| !assigned[r])
            .copied()
            .collect();

        while group.len() < max_size && !frontier.is_empty() {
            let pick = rng.gen_range(0..frontier.len());
            let next = frontier[pick];
            frontier.swap_remove(pick);

            if assigned[next] {
                continue;
            }

            assigned[next] = true;
            group.push(next);

            // Add new neighbors to frontier
            for &adj in &region_adj[next] {
                if !assigned[adj] && !frontier.contains(&adj) {
                    frontier.push(adj);
                }
            }
        }

        let capital_idx = group[0];
        let name = generate_country_name(
            regions[capital_idx].name.as_str(),
            rng,
        );

        countries.push(Country {
            name,
            capital_region_idx: capital_idx,
            region_indices: group,
        });
    }

    // Handle any unassigned regions
    for i in 0..num_regions {
        if !assigned[i] {
            let name = generate_country_name(regions[i].name.as_str(), rng);
            countries.push(Country {
                name,
                capital_region_idx: i,
                region_indices: vec![i],
            });
        }
    }

    countries
}

/// Generate a region name using word-part combination.
fn generate_region_name(seed: u64, rng: &mut rand::rngs::StdRng) -> String {
    let prefixes = [
        "Al", "Bor", "Cor", "Del", "Er", "Fal", "Gar", "Hel", "Im", "Jar",
        "Kal", "Lor", "Mar", "Nor", "Or", "Pal", "Rav", "Sal", "Tar", "Val",
        "Wen", "Xar", "Yor", "Zan", "Aur", "Bel", "Cel", "Dor", "Eth", "Fen",
        "Gil", "Hal",
    ];
    let middles = [
        "an", "en", "in", "on", "un", "ar", "er", "ir", "or", "ur",
        "al", "el", "il", "ol", "ul", "as", "es", "is",
    ];
    let suffixes = [
        "ia", "ica", "istan", "land", "mar", "heim", "grad", "burg", "ford",
        "dale", "vale", "shire", "ton", "den", "berg", "haven", "port", "ven",
        "gard", "oth",
    ];

    let _ = seed; // used for determinism via rng state
    let p = prefixes[rng.gen_range(0..prefixes.len())];
    let m = middles[rng.gen_range(0..middles.len())];
    let s = suffixes[rng.gen_range(0..suffixes.len())];

    format!("{}{}{}", p, m, s)
}

/// Generate a country name from the capital region name.
fn generate_country_name(
    capital_region: &str,
    rng: &mut rand::rngs::StdRng,
) -> String {
    let forms = [
        "Republic of {}",
        "Kingdom of {}",
        "Federation of {}",
        "{}",
        "United {}",
        "Greater {}",
        "Democratic {}",
        "Empire of {}",
    ];
    let form = forms[rng.gen_range(0..forms.len())];
    form.replace("{}", capital_region)
}

/// A lightweight adapter to pass VoronoiGrid data to compute_region_boundaries
/// which expects a GeodesicGrid-compatible interface.
/// We create a temporary GeodesicGrid with matching cell count and neighbors.
fn convert_voronoi_to_boundary_grid(vgrid: &VoronoiGrid) -> crate::grid::GeodesicGrid {
    let cells: Vec<crate::grid::GridCell> = vgrid
        .cells
        .iter()
        .map(|vc| {
            let (x, y, z) = latlon_to_xyz(vc.center_lat, vc.center_lon);
            crate::grid::GridCell {
                index: vc.index,
                center: (x, y, z),
                neighbors: vc.neighbor_indices.clone(),
                lat: vc.center_lat,
                lon: vc.center_lon,
            }
        })
        .collect();

    crate::grid::GeodesicGrid::from_cells(cells)
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
    use crate::elevation;
    use crate::rivers;
    use crate::voronoi::VoronoiGrid;
    use crate::biomes;

    #[test]
    fn test_politics_generation() {
        let grid = VoronoiGrid::generate(MapSize::Small, 42);
        let mut elev = elevation::generate_elevation(&grid, 42, 0.7, 0.5, 4);
        let river_system = rivers::generate_rivers(&grid, &mut elev, MapSize::Small, 42);
        let terrains = biomes::assign_biomes(&grid, &elev, &river_system.cell_moisture, 0.5);

        let (regions, countries) = generate_politics(
            &grid,
            &terrains,
            &elev,
            &river_system.cell_moisture,
            MapSize::Small,
            42,
        );

        assert!(!regions.is_empty(), "Should have regions");
        assert!(!countries.is_empty(), "Should have countries");

        // Most land cells should be assigned to a region.
        // Some small isolated islands may not get assigned depending on generation.
        let assigned_cells: std::collections::HashSet<usize> = regions
            .iter()
            .flat_map(|r| r.cells.iter().copied())
            .collect();
        let total_land = (0..grid.cell_count()).filter(|&i| terrains[i].is_land()).count();
        let assigned_land = assigned_cells.iter().filter(|&&i| terrains[i].is_land()).count();
        let coverage = if total_land > 0 { assigned_land as f64 / total_land as f64 } else { 1.0 };
        assert!(
            coverage > 0.80,
            "At least 80% of land cells should be in a region, got {:.1}% ({}/{})",
            coverage * 100.0,
            assigned_land,
            total_land,
        );

        // Each region should have a name
        for region in &regions {
            assert!(!region.name.is_empty());
            assert!(region.population > 0);
        }

        // Each country should reference valid regions
        for country in &countries {
            assert!(!country.region_indices.is_empty());
            assert!(!country.name.is_empty());
        }
    }

    #[test]
    fn test_politics_deterministic() {
        let grid = VoronoiGrid::generate(MapSize::Small, 42);
        let mut e1 = elevation::generate_elevation(&grid, 42, 0.7, 0.5, 4);
        let mut e2 = e1.clone();
        let r1 = rivers::generate_rivers(&grid, &mut e1, MapSize::Small, 42);
        let r2 = rivers::generate_rivers(&grid, &mut e2, MapSize::Small, 42);
        let t1 = biomes::assign_biomes(&grid, &e1, &r1.cell_moisture, 0.5);
        let t2 = biomes::assign_biomes(&grid, &e2, &r2.cell_moisture, 0.5);

        let (reg1, _) = generate_politics(&grid, &t1, &e1, &r1.cell_moisture, MapSize::Small, 42);
        let (reg2, _) = generate_politics(&grid, &t2, &e2, &r2.cell_moisture, MapSize::Small, 42);

        assert_eq!(reg1.len(), reg2.len());
        for i in 0..reg1.len() {
            assert_eq!(reg1[i].cells.len(), reg2[i].cells.len());
        }
    }
}
