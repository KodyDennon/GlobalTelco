use gt_common::types::TerrainType;
use serde::Deserialize;

use crate::cities::City;
use crate::generation::GeneratedWorld;
use crate::grid::GeodesicGrid;
use crate::parcels::{LandParcel, ZoningType};
use crate::regions::Region;

const EARTH_DATA: &str = include_str!("../../../data/earth.json");

#[derive(Deserialize)]
struct EarthData {
    countries: Vec<CountryData>,
    cities: Vec<CityData>,
}

#[derive(Deserialize)]
struct CountryData {
    name: String,
    code: String,
    lat: f64,
    lon: f64,
    population: u64,
    gdp_billions: u64,
    #[allow(dead_code)]
    area_km2: u64,
    continent: String,
}

#[derive(Deserialize)]
struct CityData {
    name: String,
    country: String,
    lat: f64,
    lon: f64,
    population: u64,
}

/// Generate a world based on real Earth geography.
///
/// Uses the geodesic grid as the spatial substrate, but assigns cells to real
/// countries based on nearest-centroid (Voronoi tessellation). Cities are
/// placed at their real lat/lon (snapped to nearest grid cell). Economic
/// data comes from real GDP and population figures.
pub fn generate_real_earth(grid: &GeodesicGrid, seed: u64) -> GeneratedWorld {
    let data: EarthData =
        serde_json::from_str(EARTH_DATA).expect("embedded earth.json must be valid");

    // Pre-compute country centroids as (x, y, z) on unit sphere for distance calculations
    let country_xyz: Vec<(f64, f64, f64)> = data
        .countries
        .iter()
        .map(|c| latlon_to_xyz(c.lat, c.lon))
        .collect();

    // 1. Assign each grid cell to the nearest country (Voronoi on sphere)
    let cell_country: Vec<Option<usize>> = grid
        .cells
        .iter()
        .map(|cell| {
            let p = (cell.center.0, cell.center.1, cell.center.2);
            nearest_country(&country_xyz, p)
        })
        .collect();

    // 2. Determine land vs ocean: cells assigned to a country are land
    let is_land: Vec<bool> = cell_country.iter().map(|c| c.is_some()).collect();

    // 3. Generate elevations: land cells get positive elevation based on latitude,
    //    ocean cells get negative
    let elevations: Vec<f64> = grid
        .cells
        .iter()
        .enumerate()
        .map(|(i, cell)| {
            if is_land[i] {
                // Vary elevation by latitude for terrain diversity
                let lat_factor = (cell.lat.abs() / 90.0).powi(2);
                let hash = simple_hash(seed, i as u64);
                0.1 + lat_factor * 0.3 + hash * 0.2
            } else {
                -0.3 - simple_hash(seed.wrapping_add(1), i as u64) * 0.5
            }
        })
        .collect();

    // 4. Classify terrain
    let terrains: Vec<TerrainType> = grid
        .cells
        .iter()
        .enumerate()
        .map(|(i, cell)| {
            if !is_land[i] {
                if elevations[i] < -0.5 {
                    TerrainType::OceanDeep
                } else {
                    TerrainType::OceanShallow
                }
            } else {
                classify_real_terrain(cell.lat, elevations[i], &data.countries, cell_country[i])
            }
        })
        .collect();

    // 5. Create land parcels
    let parcels: Vec<LandParcel> = terrains
        .iter()
        .enumerate()
        .filter(|(_, t)| t.is_land())
        .map(|(i, t)| LandParcel {
            cell_index: i,
            terrain: *t,
            elevation: elevations[i],
            zoning: default_zoning(t),
            cost_modifier: t.construction_cost_multiplier(),
            disaster_risk: disaster_risk_for_terrain(t, grid.cells[i].lat),
            owner: None,
        })
        .collect();

    // 6. Create regions from countries
    let mut regions: Vec<Region> = Vec::new();
    for (country_idx, country) in data.countries.iter().enumerate() {
        let cells: Vec<usize> = cell_country
            .iter()
            .enumerate()
            .filter(|(_, &c)| c == Some(country_idx))
            .map(|(i, _)| i)
            .collect();

        if cells.is_empty() {
            continue;
        }

        let development = (country.gdp_billions as f64 / country.population.max(1) as f64 * 20.0)
            .clamp(0.05, 1.0);

        regions.push(Region {
            id: (country_idx + 1) as u64,
            name: country.name.clone(),
            cells,
            center_lat: country.lat,
            center_lon: country.lon,
            population: country.population,
            gdp: country.gdp_billions as f64 * 1_000_000_000.0,
            development,
        });
    }

    // 7. Place cities from real data, snapped to nearest grid cell
    let mut all_cities: Vec<City> = Vec::new();
    for city_data in &data.cities {
        let xyz = latlon_to_xyz(city_data.lat, city_data.lon);
        let cell_index = grid.nearest_cell(xyz.0, xyz.1, xyz.2);

        // Find the region this city belongs to (by country code)
        let region_id = data
            .countries
            .iter()
            .position(|c| c.code == city_data.country)
            .map(|idx| (idx + 1) as u64)
            .unwrap_or(0);

        let development = data
            .countries
            .iter()
            .find(|c| c.code == city_data.country)
            .map(|c| (c.gdp_billions as f64 / c.population.max(1) as f64 * 20.0).clamp(0.05, 1.0))
            .unwrap_or(0.5);

        all_cities.push(City {
            id: 0, // assigned during entity creation
            name: city_data.name.clone(),
            region_id,
            cell_index,
            population: city_data.population,
            growth_rate: 0.002 + development * 0.003,
            development,
        });
    }

    // (Region city_ids are managed by GameWorld during entity creation)

    // 8. Upgrade terrain near cities
    let mut terrains = terrains;
    for city in &all_cities {
        if city.cell_index < terrains.len() {
            terrains[city.cell_index] = TerrainType::Urban;
            let neighbors: Vec<usize> = grid.cells[city.cell_index].neighbors.clone();
            for &n in &neighbors {
                if n < terrains.len() && terrains[n].is_land() && terrains[n] != TerrainType::Urban
                {
                    terrains[n] = TerrainType::Suburban;
                }
            }
        }
    }

    // 9. Seed economics from real data
    // 9. Seed economics from real data using the existing function
    let economics = crate::economics::seed_economics(
        &regions,
        &all_cities,
        &terrains,
        gt_common::types::Era::Modern,
    );

    GeneratedWorld {
        grid: grid.clone(),
        elevations,
        terrains,
        parcels,
        regions,
        cities: all_cities,
        economics,
    }
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

fn nearest_country(centroids: &[(f64, f64, f64)], point: (f64, f64, f64)) -> Option<usize> {
    // Only assign to a country if reasonably close (within ~30 degrees arc)
    // This leaves ocean cells unassigned
    let threshold = 0.87; // cos(30 degrees) ≈ 0.866
    let mut best_idx = None;
    let mut best_dot = -1.0_f64;

    for (i, c) in centroids.iter().enumerate() {
        let dot = point.0 * c.0 + point.1 * c.1 + point.2 * c.2;
        if dot > best_dot {
            best_dot = dot;
            best_idx = Some(i);
        }
    }

    if best_dot > threshold {
        best_idx
    } else {
        None // ocean cell
    }
}

fn classify_real_terrain(
    lat: f64,
    elevation: f64,
    countries: &[CountryData],
    country_idx: Option<usize>,
) -> TerrainType {
    let abs_lat = lat.abs();

    if abs_lat > 70.0 {
        return TerrainType::Frozen;
    }
    if abs_lat > 60.0 {
        return TerrainType::Tundra;
    }
    if elevation > 0.5 {
        return TerrainType::Mountainous;
    }

    // Desert regions based on latitude and continent
    let is_desert_region = country_idx
        .and_then(|idx| countries.get(idx))
        .map(|c| {
            let is_arid_continent = c.continent == "Africa" || c.continent == "Asia";
            is_arid_continent && (20.0..35.0).contains(&abs_lat)
        })
        .unwrap_or(false);

    if is_desert_region {
        return TerrainType::Desert;
    }

    // Coastal if near edge of land mass (low elevation land)
    if elevation < 0.15 {
        return TerrainType::Coastal;
    }

    TerrainType::Rural
}

fn default_zoning(terrain: &TerrainType) -> ZoningType {
    match terrain {
        TerrainType::Urban => ZoningType::Mixed,
        TerrainType::Suburban => ZoningType::Residential,
        TerrainType::Mountainous | TerrainType::Frozen | TerrainType::Tundra => {
            ZoningType::Protected
        }
        _ => ZoningType::Unzoned,
    }
}

fn disaster_risk_for_terrain(terrain: &TerrainType, lat: f64) -> f64 {
    let base = match terrain {
        TerrainType::Coastal => 0.6,
        TerrainType::Mountainous => 0.5,
        TerrainType::Desert => 0.2,
        TerrainType::Tundra | TerrainType::Frozen => 0.3,
        _ => 0.3,
    };
    // Tropical regions have higher disaster risk
    let tropical_bonus = if lat.abs() < 25.0 { 0.2 } else { 0.0 };
    let result: f64 = base + tropical_bonus;
    result.min(1.0)
}

#[allow(dead_code)]
fn continent_disaster_risk(continent: &str) -> f64 {
    match continent {
        "Asia" => 0.5,
        "Africa" => 0.4,
        "SouthAmerica" => 0.4,
        "NorthAmerica" => 0.35,
        "Europe" => 0.2,
        "Oceania" => 0.35,
        _ => 0.3,
    }
}

#[allow(dead_code)]
fn continent_regulatory_strictness(continent: &str) -> f64 {
    match continent {
        "Europe" => 0.7,
        "NorthAmerica" => 0.5,
        "Asia" => 0.5,
        "Oceania" => 0.6,
        "SouthAmerica" => 0.4,
        "Africa" => 0.3,
        _ => 0.5,
    }
}

fn simple_hash(seed: u64, index: u64) -> f64 {
    let mut h = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(index.wrapping_mul(1442695040888963407));
    h ^= h >> 33;
    h = h.wrapping_mul(0xff51afd7ed558ccd);
    h ^= h >> 33;
    (h % 10000) as f64 / 10000.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use gt_common::types::MapSize;

    #[test]
    fn test_earth_data_loads() {
        let data: EarthData = serde_json::from_str(EARTH_DATA).unwrap();
        assert!(!data.countries.is_empty());
        assert!(!data.cities.is_empty());
        assert!(data.countries.len() >= 40);
        assert!(data.cities.len() >= 60);
    }

    #[test]
    fn test_real_earth_generation() {
        let grid = GeodesicGrid::new(MapSize::Small.grid_subdivisions());
        let world = generate_real_earth(&grid, 42);

        assert!(!world.regions.is_empty(), "should have regions");
        assert!(!world.cities.is_empty(), "should have cities");
        assert!(!world.parcels.is_empty(), "should have land parcels");
        assert!(
            world.economics.global_demand > 0.0,
            "should have demand data"
        );

        // Check that some cities have Urban terrain
        let urban_count = world
            .cities
            .iter()
            .filter(|c| c.cell_index < world.terrains.len())
            .filter(|c| world.terrains[c.cell_index] == TerrainType::Urban)
            .count();
        assert!(urban_count > 0, "cities should have Urban terrain");
    }

    #[test]
    fn test_real_earth_has_known_countries() {
        let grid = GeodesicGrid::new(MapSize::Small.grid_subdivisions());
        let world = generate_real_earth(&grid, 42);

        let region_names: Vec<&str> = world.regions.iter().map(|r| r.name.as_str()).collect();
        assert!(
            region_names.contains(&"United States"),
            "should have USA: {:?}",
            region_names
        );
        assert!(
            region_names.contains(&"China"),
            "should have China: {:?}",
            region_names
        );
        assert!(
            region_names.contains(&"Japan"),
            "should have Japan: {:?}",
            region_names
        );
    }

    #[test]
    fn test_real_earth_deterministic() {
        let grid = GeodesicGrid::new(MapSize::Small.grid_subdivisions());
        let w1 = generate_real_earth(&grid, 42);
        let w2 = generate_real_earth(&grid, 42);

        assert_eq!(w1.regions.len(), w2.regions.len());
        assert_eq!(w1.cities.len(), w2.cities.len());
        assert_eq!(w1.elevations, w2.elevations);
    }
}
