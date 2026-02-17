use gt_common::types::EntityId;
use serde::{Deserialize, Serialize};

use crate::grid::GeodesicGrid;
use crate::regions::Region;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct City {
    pub id: EntityId,
    pub name: String,
    pub region_id: EntityId,
    pub cell_index: usize,
    pub population: u64,
    pub growth_rate: f64,
    pub development: f64,
}

pub fn place_cities(
    grid: &GeodesicGrid,
    regions: &[Region],
    elevations: &[f64],
    seed: u64,
) -> Vec<City> {
    let mut cities = Vec::new();

    for (region_idx, region) in regions.iter().enumerate() {
        if region.cells.is_empty() {
            continue;
        }

        // Score cells by development potential: lower elevation (but land) + closer to center
        let mut scored_cells: Vec<(usize, f64)> = region
            .cells
            .iter()
            .map(|&ci| {
                let cell = &grid.cells[ci];
                let elev_score = 1.0 - (elevations[ci] - 0.1).abs(); // prefer moderate elevation
                let center_dist = dist_sq_3d(
                    cell.center,
                    crate::grid::latlon_to_xyz(region.center_lat, region.center_lon),
                );
                let proximity_score = 1.0 - center_dist.sqrt().min(1.0);
                (ci, elev_score * 0.6 + proximity_score * 0.4)
            })
            .collect();

        scored_cells.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // 1-3 cities per region based on size
        let city_count = if region.cells.len() > 100 {
            3
        } else if region.cells.len() > 30 {
            2
        } else {
            1
        };
        let city_count = city_count.min(scored_cells.len());

        for (city_idx, &(cell_index, _)) in scored_cells.iter().enumerate().take(city_count) {
            // Distribute population: first city gets most
            let pop_share = if city_idx == 0 {
                0.6
            } else if city_idx == 1 {
                0.25
            } else {
                0.15
            };
            let population = (region.population as f64 * pop_share) as u64;
            let development = region.development * (1.0 - city_idx as f64 * 0.15);
            let growth_rate = 0.001 + development * 0.005;

            cities.push(City {
                id: 0, // Will be assigned during entity creation
                name: generate_city_name(seed + region_idx as u64 * 100 + city_idx as u64),
                region_id: region.id,
                cell_index,
                population,
                growth_rate,
                development,
            });
        }
    }

    cities
}

fn dist_sq_3d(a: (f64, f64, f64), b: (f64, f64, f64)) -> f64 {
    let dx = a.0 - b.0;
    let dy = a.1 - b.1;
    let dz = a.2 - b.2;
    dx * dx + dy * dy + dz * dz
}

fn generate_city_name(seed: u64) -> String {
    let prefixes = [
        "San", "New", "Port", "Fort", "Saint", "El", "Las", "Lake", "Bay", "Old", "North", "South",
        "East", "West", "Grand", "Royal", "Iron", "Silver", "Gold", "Red", "Blue", "Green", "Dark",
        "Bright", "High", "Low", "Deep", "Far", "Near", "Long", "Stone", "River",
    ];
    let roots = [
        "haven", "bridge", "field", "ford", "dale", "wood", "gate", "holm", "wick", "stead",
        "cliff", "ridge", "brook", "creek", "lake", "springs", "falls", "mill", "cross", "well",
        "bury", "mond", "vale", "moor", "worth", "ham", "wick", "thorpe", "bourne", "crest",
        "peak", "mouth",
    ];

    let mut hash = seed;
    hash = hash
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    let use_prefix = !hash.is_multiple_of(3); // 2/3 chance of prefix
    hash = hash
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    let p = prefixes[(hash % prefixes.len() as u64) as usize];
    hash = hash
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    let r = roots[(hash % roots.len() as u64) as usize];

    if use_prefix {
        format!("{} {}", p, capitalize(r))
    } else {
        capitalize(r).to_string()
    }
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().to_string() + chars.as_str(),
    }
}
