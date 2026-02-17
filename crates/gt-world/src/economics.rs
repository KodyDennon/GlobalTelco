use gt_common::types::{Era, TerrainType};
use serde::{Deserialize, Serialize};

use crate::cities::City;
use crate::regions::Region;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionEconomics {
    pub region_index: usize,
    pub gdp: f64,
    pub avg_income: f64,
    pub demand_modifier: f64,
    pub market_maturity: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CityEconomics {
    pub city_index: usize,
    pub telecom_demand: f64,
    pub business_density: f64,
    pub infrastructure_need: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EconomicData {
    pub regions: Vec<RegionEconomics>,
    pub cities: Vec<CityEconomics>,
    pub global_demand: f64,
}

pub fn seed_economics(
    regions: &[Region],
    cities: &[City],
    terrains: &[TerrainType],
    era: Era,
) -> EconomicData {
    let era_modifier = era_demand_modifier(era);

    let region_economics: Vec<RegionEconomics> = regions
        .iter()
        .enumerate()
        .map(|(i, region)| {
            // Coastal access bonus
            let coastal_cells = region
                .cells
                .iter()
                .filter(|&&c| terrains.get(c).copied() == Some(TerrainType::Coastal))
                .count();
            let coastal_bonus =
                1.0 + (coastal_cells as f64 / region.cells.len().max(1) as f64) * 0.5;

            // City count bonus
            let region_cities: Vec<&City> =
                cities.iter().filter(|c| c.region_id == region.id).collect();
            let city_bonus = 1.0 + region_cities.len() as f64 * 0.1;

            let gdp = region.population as f64
                * region.development
                * 30_000.0
                * coastal_bonus
                * city_bonus;
            let avg_income = gdp / region.population.max(1) as f64;
            let demand_modifier = region.development * era_modifier * coastal_bonus;
            let market_maturity = region.development * 0.5
                + (coastal_cells as f64 / region.cells.len().max(1) as f64) * 0.3;

            RegionEconomics {
                region_index: i,
                gdp,
                avg_income,
                demand_modifier,
                market_maturity: market_maturity.clamp(0.0, 1.0),
            }
        })
        .collect();

    let city_economics: Vec<CityEconomics> = cities
        .iter()
        .enumerate()
        .map(|(i, city)| {
            let telecom_demand = city.population as f64 * city.development * era_modifier;
            let business_density = city.development * 0.8;
            let infrastructure_need = telecom_demand * (1.0 - business_density * 0.3);

            CityEconomics {
                city_index: i,
                telecom_demand,
                business_density,
                infrastructure_need,
            }
        })
        .collect();

    let global_demand = city_economics.iter().map(|c| c.telecom_demand).sum();

    EconomicData {
        regions: region_economics,
        cities: city_economics,
        global_demand,
    }
}

fn era_demand_modifier(era: Era) -> f64 {
    match era {
        Era::Telegraph => 0.01,
        Era::Telephone => 0.05,
        Era::EarlyDigital => 0.15,
        Era::Internet => 0.5,
        Era::Modern => 1.0,
        Era::NearFuture => 2.0,
    }
}
