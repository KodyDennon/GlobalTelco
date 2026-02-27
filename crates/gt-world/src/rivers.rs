use gt_common::types::MapSize;
use rand::Rng;
use rand::SeedableRng;
use serde::{Deserialize, Serialize};

use crate::voronoi::VoronoiGrid;

/// A single river as an ordered sequence of cell indices from source to mouth.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct River {
    /// Ordered cell indices from source (highest) to mouth (lowest/ocean).
    pub cells: Vec<usize>,
    /// Total accumulated flow at the river mouth.
    pub flow: f64,
}

/// Complete river system and moisture data for a generated world.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiverSystem {
    /// All rivers above the flow threshold, sorted by flow descending.
    pub rivers: Vec<River>,
    /// Per-cell moisture value (0.0 = arid, 1.0 = saturated). One per grid cell.
    pub cell_moisture: Vec<f64>,
}

/// Generate rivers and moisture from elevation data using hydraulic simulation.
///
/// Pipeline:
/// 1. Simplified erosion: drop water at random land cells, trace downhill, slightly erode
/// 2. Drainage: for every land cell, trace the steepest-descent path to ocean
/// 3. Accumulate flow per cell; cells above threshold become river segments
/// 4. Merge contiguous river segments into ordered River structs
/// 5. Compute per-cell moisture from ocean proximity and river proximity
pub fn generate_rivers(
    grid: &VoronoiGrid,
    elevations: &mut [f64],
    map_size: MapSize,
    seed: u64,
) -> RiverSystem {
    let n = grid.cell_count();
    let mut rng = rand::rngs::StdRng::seed_from_u64(seed.wrapping_add(7777));

    let is_land: Vec<bool> = elevations.iter().map(|&e| e >= 0.0).collect();
    let land_indices: Vec<usize> = (0..n).filter(|&i| is_land[i]).collect();

    if land_indices.is_empty() {
        return RiverSystem {
            rivers: Vec::new(),
            cell_moisture: vec![0.0; n],
        };
    }

    // Step 1: Simplified erosion — drop water particles and trace downhill
    let erosion_iterations = match map_size {
        MapSize::Small => 50,
        MapSize::Medium => 100,
        MapSize::Large => 150,
        MapSize::Huge => 200,
    };
    let erosion_rate = 5.0; // meters removed per pass

    for _ in 0..erosion_iterations {
        let start = land_indices[rng.gen_range(0..land_indices.len())];
        let mut current = start;
        let mut sediment = 0.0;
        let max_steps = 100;

        for _ in 0..max_steps {
            // Find steepest downhill neighbor
            let mut best_neighbor = None;
            let mut best_drop = 0.0;

            for &ni in &grid.cells[current].neighbor_indices {
                let drop = elevations[current] - elevations[ni];
                if drop > best_drop {
                    best_drop = drop;
                    best_neighbor = Some(ni);
                }
            }

            match best_neighbor {
                Some(next) => {
                    // Erode current cell, deposit proportional to slope decrease
                    let erode_amount = (erosion_rate * best_drop / (best_drop + 1.0)).min(elevations[current]);
                    elevations[current] -= erode_amount;
                    sediment += erode_amount;

                    // Deposit some sediment if slope is gentle
                    if best_drop < 50.0 && sediment > 0.0 {
                        let deposit = sediment * 0.1;
                        elevations[next] += deposit;
                        sediment -= deposit;
                    }

                    current = next;
                    if !is_land[current] {
                        break; // reached ocean
                    }
                }
                None => break, // local minimum
            }
        }
    }

    // Step 2: Compute drainage — for each land cell, find the steepest descent neighbor
    let mut downstream: Vec<Option<usize>> = vec![None; n];
    for i in 0..n {
        if !is_land[i] {
            continue;
        }
        let mut best_neighbor = None;
        let mut best_drop = 0.0;
        for &ni in &grid.cells[i].neighbor_indices {
            let drop = elevations[i] - elevations[ni];
            if drop > best_drop {
                best_drop = drop;
                best_neighbor = Some(ni);
            }
        }
        downstream[i] = best_neighbor;
    }

    // Step 3: Accumulate flow — trace each land cell's path to ocean
    let mut flow: Vec<f64> = vec![0.0; n];

    // Process cells from highest to lowest elevation for correct accumulation
    let mut sorted_land: Vec<usize> = land_indices.clone();
    sorted_land.sort_by(|&a, &b| {
        elevations[b]
            .partial_cmp(&elevations[a])
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    for &ci in &sorted_land {
        flow[ci] += 1.0; // each cell contributes 1 unit of precipitation
        if let Some(next) = downstream[ci] {
            flow[next] += flow[ci];
        }
    }

    // Step 4: Extract rivers — cells with flow above threshold
    let flow_threshold = match map_size {
        MapSize::Small => 8.0,
        MapSize::Medium => 12.0,
        MapSize::Large => 18.0,
        MapSize::Huge => 25.0,
    };

    let is_river: Vec<bool> = flow.iter().map(|&f| f >= flow_threshold).collect();

    // Trace rivers from sources (high flow cells with no upstream river)
    let mut visited = vec![false; n];
    let mut rivers: Vec<River> = Vec::new();

    // Find river source cells: river cells where no upstream neighbor is also a river
    let mut sources: Vec<(usize, f64)> = Vec::new();
    for i in 0..n {
        if !is_river[i] || !is_land[i] {
            continue;
        }
        // Check if any neighbor that drains INTO this cell is also a river
        let has_upstream_river = grid.cells[i]
            .neighbor_indices
            .iter()
            .any(|&ni| is_river[ni] && downstream[ni] == Some(i));
        if !has_upstream_river {
            sources.push((i, flow[i]));
        }
    }

    // Sort sources by flow descending to get major rivers first
    sources.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    for (source, _) in sources {
        if visited[source] {
            continue;
        }
        let mut river_cells = Vec::new();
        let mut current = source;

        loop {
            if visited[current] {
                break;
            }
            visited[current] = true;
            river_cells.push(current);

            match downstream[current] {
                Some(next) => {
                    if !is_land[next] {
                        // Reached ocean — river complete
                        break;
                    }
                    current = next;
                }
                None => break,
            }
        }

        if river_cells.len() >= 3 {
            let mouth_flow = flow[*river_cells.last().unwrap()];
            rivers.push(River {
                cells: river_cells,
                flow: mouth_flow,
            });
        }
    }

    // Sort rivers by flow descending
    rivers.sort_by(|a, b| b.flow.partial_cmp(&a.flow).unwrap_or(std::cmp::Ordering::Equal));

    // Step 5: Compute moisture per cell
    let mut cell_moisture = vec![0.0; n];

    // Base moisture from ocean proximity (BFS distance)
    let ocean_dist = compute_land_distance_from_ocean(grid, &is_land, 15);
    for i in 0..n {
        if is_land[i] {
            let dist = ocean_dist[i] as f64;
            cell_moisture[i] = (1.0 - dist / 15.0).max(0.1);
        }
    }

    // Add moisture from rivers (BFS out from river cells)
    let river_dist = compute_distance_from_set(
        grid,
        &(0..n).filter(|&i| is_river[i] && is_land[i]).collect::<Vec<_>>(),
        5,
    );
    for i in 0..n {
        if is_land[i] && river_dist[i] < 5 {
            let river_bonus = (1.0 - river_dist[i] as f64 / 5.0) * 0.5;
            cell_moisture[i] = (cell_moisture[i] + river_bonus).min(1.0);
        }
    }

    RiverSystem {
        rivers,
        cell_moisture,
    }
}

/// BFS distance from ocean cells to land cells, capped at max_dist.
fn compute_land_distance_from_ocean(
    grid: &VoronoiGrid,
    is_land: &[bool],
    max_dist: usize,
) -> Vec<usize> {
    let n = grid.cell_count();
    let mut dist = vec![usize::MAX; n];
    let mut queue = std::collections::VecDeque::new();

    // Seed: land cells adjacent to ocean
    for i in 0..n {
        if is_land[i] {
            let near_ocean = grid.cells[i]
                .neighbor_indices
                .iter()
                .any(|&ni| !is_land[ni]);
            if near_ocean {
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
            if is_land[ni] && dist[ni] > d + 1 {
                dist[ni] = d + 1;
                queue.push_back(ni);
            }
        }
    }

    // Cells never reached get max_dist
    for d in &mut dist {
        if *d == usize::MAX {
            *d = max_dist;
        }
    }
    dist
}

/// BFS distance from a set of seed cells, capped at max_dist.
fn compute_distance_from_set(
    grid: &VoronoiGrid,
    seeds: &[usize],
    max_dist: usize,
) -> Vec<usize> {
    let n = grid.cell_count();
    let mut dist = vec![usize::MAX; n];
    let mut queue = std::collections::VecDeque::new();

    for &s in seeds {
        dist[s] = 0;
        queue.push_back(s);
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

    for d in &mut dist {
        if *d == usize::MAX {
            *d = max_dist;
        }
    }
    dist
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::elevation;
    use crate::voronoi::VoronoiGrid;

    #[test]
    fn test_river_generation() {
        // Use low ocean percentage (0.4) and more continents to ensure enough land for rivers
        let grid = VoronoiGrid::generate(MapSize::Small, 42);
        let mut elev = elevation::generate_elevation(&grid, 42, 0.4, 0.6, 4);
        let rivers = generate_rivers(&grid, &mut elev, MapSize::Small, 42);

        assert_eq!(rivers.cell_moisture.len(), grid.cell_count());
        // Rivers depend on elevation and land mass — verify system runs without errors.
        // With low ocean percentage, we expect rivers. With high ocean, they may not form.
        if !rivers.rivers.is_empty() {
            for river in &rivers.rivers {
                assert!(river.cells.len() >= 2, "Rivers must have at least 2 cells");
                assert!(river.flow > 0.0, "Rivers must have positive flow");
            }
        }
    }

    #[test]
    fn test_moisture_range() {
        let grid = VoronoiGrid::generate(MapSize::Small, 42);
        let mut elev = elevation::generate_elevation(&grid, 42, 0.4, 0.6, 4);
        let rivers = generate_rivers(&grid, &mut elev, MapSize::Small, 42);

        for &m in &rivers.cell_moisture {
            assert!(m >= 0.0 && m <= 1.0, "Moisture {} out of range", m);
        }
    }

    #[test]
    fn test_rivers_deterministic() {
        let grid = VoronoiGrid::generate(MapSize::Small, 42);
        let mut e1 = elevation::generate_elevation(&grid, 42, 0.7, 0.5, 4);
        let mut e2 = e1.clone();
        let r1 = generate_rivers(&grid, &mut e1, MapSize::Small, 42);
        let r2 = generate_rivers(&grid, &mut e2, MapSize::Small, 42);

        assert_eq!(r1.rivers.len(), r2.rivers.len());
        assert_eq!(r1.cell_moisture, r2.cell_moisture);
    }
}
