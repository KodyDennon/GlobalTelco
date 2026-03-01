use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AIArchetype {
    AggressiveExpander,
    DefensiveConsolidator,
    TechInnovator,
    BudgetOperator,
    SatellitePioneer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AIStrategy {
    Expand,
    Consolidate,
    Compete,
    Survive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LobbyPolicy {
    ReduceTax,
    RelaxZoning,
    FastTrackPermits,
    IncreasedCompetitorBurden,
    SubsidyRequest,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum GameSpeed {
    Paused,
    #[default]
    Normal,
    Fast,
    VeryFast,
    Ultra,
    /// 32x speed — only available in sandbox mode
    Ludicrous,
}

impl GameSpeed {
    pub fn ticks_per_second(&self) -> u32 {
        match self {
            GameSpeed::Paused => 0,
            GameSpeed::Normal => 1,
            GameSpeed::Fast => 2,
            GameSpeed::VeryFast => 4,
            GameSpeed::Ultra => 8,
            GameSpeed::Ludicrous => 32,
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
    #[serde(default)]
    pub corp_name: Option<String>,
    /// Number of continents to generate (1-8). Default 4.
    #[serde(default = "default_continent_count")]
    pub continent_count: u8,
    /// Fraction of world covered by ocean (0.3-0.9). Default 0.7.
    #[serde(default = "default_ocean_percentage")]
    pub ocean_percentage: f64,
    /// Roughness of terrain elevation (0.0-1.0). Default 0.5.
    #[serde(default = "default_terrain_roughness")]
    pub terrain_roughness: f64,
    /// Variation in climate zones (0.0-1.0). Default 0.5.
    #[serde(default = "default_climate_variation")]
    pub climate_variation: f64,
    /// Density of city placement (0.0-1.0). Default 0.5.
    #[serde(default = "default_city_density")]
    pub city_density: f64,
    /// Disaster frequency multiplier (0.1-3.0). Default determined by difficulty.
    #[serde(default = "default_disaster_frequency")]
    pub disaster_frequency: f64,
    /// Sandbox mode: infinite money, instant build, all tech unlocked.
    #[serde(default)]
    pub sandbox: bool,
    /// Maximum number of AI corporations allowed (including dynamically spawned ones). Default 8.
    #[serde(default = "default_max_ai_corporations")]
    pub max_ai_corporations: u32,
}

fn default_continent_count() -> u8 { 4 }
fn default_ocean_percentage() -> f64 { 0.7 }
fn default_terrain_roughness() -> f64 { 0.5 }
fn default_climate_variation() -> f64 { 0.5 }
fn default_city_density() -> f64 { 0.5 }
fn default_disaster_frequency() -> f64 { 1.0 }
fn default_max_ai_corporations() -> u32 { 8 }

impl Default for WorldConfig {
    fn default() -> Self {
        Self {
            seed: 42,
            starting_era: Era::Internet,
            difficulty: DifficultyPreset::Normal,
            map_size: MapSize::Medium,
            ai_corporations: 4,
            use_real_earth: false,
            corp_name: None,
            continent_count: 4,
            ocean_percentage: 0.7,
            terrain_roughness: 0.5,
            climate_variation: 0.5,
            city_density: 0.5,
            disaster_frequency: 1.0,
            sandbox: false,
            max_ai_corporations: 8,
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
