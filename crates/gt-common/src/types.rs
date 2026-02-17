use serde::{Deserialize, Serialize};

pub type EntityId = u64;
pub type Tick = u64;
pub type Money = i64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TerrainType {
    Urban,
    Suburban,
    Rural,
    Mountainous,
    Desert,
    Coastal,
    OceanShallow,
    OceanDeep,
    Tundra,
    Frozen,
}

impl TerrainType {
    pub fn construction_cost_multiplier(&self) -> f64 {
        match self {
            TerrainType::Urban => 2.0,
            TerrainType::Suburban => 1.2,
            TerrainType::Rural => 1.0,
            TerrainType::Mountainous => 3.0,
            TerrainType::Desert => 1.8,
            TerrainType::Coastal => 1.5,
            TerrainType::OceanShallow => 5.0,
            TerrainType::OceanDeep => 10.0,
            TerrainType::Tundra => 2.5,
            TerrainType::Frozen => 4.0,
        }
    }

    pub fn is_land(&self) -> bool {
        !matches!(self, TerrainType::OceanShallow | TerrainType::OceanDeep)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NodeType {
    CentralOffice,
    ExchangePoint,
    CellTower,
    DataCenter,
    SatelliteGround,
    SubmarineLanding,
    WirelessRelay,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EdgeType {
    FiberOptic,
    Copper,
    Microwave,
    Satellite,
    Submarine,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NetworkLevel {
    Local,
    Regional,
    National,
    Continental,
    GlobalBackbone,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CreditRating {
    AAA,
    AA,
    A,
    BBB,
    BB,
    B,
    CCC,
    D,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AIArchetype {
    AggressiveExpander,
    DefensiveConsolidator,
    TechInnovator,
    BudgetOperator,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AIStrategy {
    Expand,
    Consolidate,
    Compete,
    Survive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Era {
    Telegraph,
    Telephone,
    EarlyDigital,
    Internet,
    Modern,
    NearFuture,
}

impl Era {
    pub fn display_name(&self) -> &'static str {
        match self {
            Era::Telegraph => "Telegraph (~1850s)",
            Era::Telephone => "Telephone (~1900s)",
            Era::EarlyDigital => "Early Digital (~1970s)",
            Era::Internet => "Internet (~1990s)",
            Era::Modern => "Modern (~2010s)",
            Era::NearFuture => "Near Future (~2030s)",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GameSpeed {
    Paused,
    Normal,
    Fast,
    VeryFast,
    Ultra,
}

impl GameSpeed {
    pub fn ticks_per_second(&self) -> u32 {
        match self {
            GameSpeed::Paused => 0,
            GameSpeed::Normal => 1,
            GameSpeed::Fast => 2,
            GameSpeed::VeryFast => 4,
            GameSpeed::Ultra => 8,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldConfig {
    pub seed: u64,
    pub starting_era: Era,
    pub difficulty: DifficultyPreset,
    pub map_size: MapSize,
    pub ai_corporations: u32,
    pub use_real_earth: bool,
}

impl Default for WorldConfig {
    fn default() -> Self {
        Self {
            seed: 42,
            starting_era: Era::Internet,
            difficulty: DifficultyPreset::Normal,
            map_size: MapSize::Medium,
            ai_corporations: 4,
            use_real_earth: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DifficultyPreset {
    Easy,
    Normal,
    Hard,
    Expert,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MapSize {
    Small,
    Medium,
    Large,
    Huge,
}

impl MapSize {
    /// Returns the number of icosahedral subdivision iterations.
    /// Each iteration multiplies face count by 4.
    /// Vertices ≈ 10 * 4^n + 2:
    ///   4 → ~2,562 cells, 5 → ~10,242 cells, 6 → ~40,962 cells, 7 → ~163,842 cells
    pub fn grid_subdivisions(&self) -> u32 {
        match self {
            MapSize::Small => 4,
            MapSize::Medium => 5,
            MapSize::Large => 6,
            MapSize::Huge => 7,
        }
    }
}
