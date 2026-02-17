use crate::types::{DifficultyPreset, Era};

pub struct DifficultyConfig {
    pub starting_capital_multiplier: f64,
    pub ai_aggressiveness: f64,
    pub disaster_frequency: f64,
    pub market_volatility: f64,
    pub construction_time_multiplier: f64,
}

impl DifficultyConfig {
    pub fn from_preset(preset: DifficultyPreset) -> Self {
        match preset {
            DifficultyPreset::Easy => Self {
                starting_capital_multiplier: 1.5,
                ai_aggressiveness: 0.5,
                disaster_frequency: 0.5,
                market_volatility: 0.5,
                construction_time_multiplier: 0.75,
            },
            DifficultyPreset::Normal => Self {
                starting_capital_multiplier: 1.0,
                ai_aggressiveness: 1.0,
                disaster_frequency: 1.0,
                market_volatility: 1.0,
                construction_time_multiplier: 1.0,
            },
            DifficultyPreset::Hard => Self {
                starting_capital_multiplier: 0.75,
                ai_aggressiveness: 1.5,
                disaster_frequency: 1.5,
                market_volatility: 1.5,
                construction_time_multiplier: 1.25,
            },
            DifficultyPreset::Expert => Self {
                starting_capital_multiplier: 0.5,
                ai_aggressiveness: 2.0,
                disaster_frequency: 2.0,
                market_volatility: 2.0,
                construction_time_multiplier: 1.5,
            },
        }
    }
}

pub struct EraConfig {
    pub name: &'static str,
    pub year_start: u32,
    pub year_end: u32,
    pub starting_capital: i64,
    pub available_node_types: Vec<&'static str>,
    pub available_edge_types: Vec<&'static str>,
}

impl EraConfig {
    pub fn from_era(era: Era) -> Self {
        match era {
            Era::Telegraph => Self {
                name: "Telegraph",
                year_start: 1850,
                year_end: 1899,
                starting_capital: 10_000,
                available_node_types: vec!["CentralOffice"],
                available_edge_types: vec!["Copper"],
            },
            Era::Telephone => Self {
                name: "Telephone",
                year_start: 1900,
                year_end: 1969,
                starting_capital: 50_000,
                available_node_types: vec!["CentralOffice", "ExchangePoint"],
                available_edge_types: vec!["Copper"],
            },
            Era::EarlyDigital => Self {
                name: "Early Digital",
                year_start: 1970,
                year_end: 1989,
                starting_capital: 500_000,
                available_node_types: vec!["CentralOffice", "ExchangePoint", "SatelliteGround"],
                available_edge_types: vec!["Copper", "Microwave", "Satellite"],
            },
            Era::Internet => Self {
                name: "Internet",
                year_start: 1990,
                year_end: 2009,
                starting_capital: 5_000_000,
                available_node_types: vec![
                    "CentralOffice",
                    "ExchangePoint",
                    "DataCenter",
                    "SatelliteGround",
                ],
                available_edge_types: vec![
                    "Copper",
                    "FiberOptic",
                    "Microwave",
                    "Satellite",
                    "Submarine",
                ],
            },
            Era::Modern => Self {
                name: "Modern",
                year_start: 2010,
                year_end: 2029,
                starting_capital: 50_000_000,
                available_node_types: vec![
                    "CentralOffice",
                    "ExchangePoint",
                    "CellTower",
                    "DataCenter",
                    "SatelliteGround",
                    "SubmarineLanding",
                ],
                available_edge_types: vec![
                    "Copper",
                    "FiberOptic",
                    "Microwave",
                    "Satellite",
                    "Submarine",
                ],
            },
            Era::NearFuture => Self {
                name: "Near Future",
                year_start: 2030,
                year_end: 2060,
                starting_capital: 100_000_000,
                available_node_types: vec![
                    "CentralOffice",
                    "ExchangePoint",
                    "CellTower",
                    "DataCenter",
                    "SatelliteGround",
                    "SubmarineLanding",
                    "WirelessRelay",
                ],
                available_edge_types: vec!["FiberOptic", "Microwave", "Satellite", "Submarine"],
            },
        }
    }
}
