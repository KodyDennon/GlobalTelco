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

    pub fn maintenance_cost_multiplier(&self) -> f64 {
        match self {
            TerrainType::Urban => 1.5,
            TerrainType::Suburban => 1.1,
            TerrainType::Rural => 1.0,
            TerrainType::Mountainous => 2.0,
            TerrainType::Desert => 1.5,
            TerrainType::Coastal => 1.3,
            TerrainType::OceanShallow => 3.0,
            TerrainType::OceanDeep => 5.0,
            TerrainType::Tundra => 1.8,
            TerrainType::Frozen => 2.5,
        }
    }

    pub fn reliability_modifier(&self) -> f64 {
        match self {
            TerrainType::Urban => 1.0,
            TerrainType::Suburban => 1.0,
            TerrainType::Rural => 0.95,
            TerrainType::Mountainous => 0.85,
            TerrainType::Desert => 0.90,
            TerrainType::Coastal => 0.88,
            TerrainType::OceanShallow => 0.80,
            TerrainType::OceanDeep => 0.75,
            TerrainType::Tundra => 0.85,
            TerrainType::Frozen => 0.80,
        }
    }

    pub fn is_land(&self) -> bool {
        !matches!(self, TerrainType::OceanShallow | TerrainType::OceanDeep)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum NetworkTier {
    Access = 1,
    Aggregation = 2,
    Core = 3,
    Backbone = 4,
    Global = 5,
}

impl NetworkTier {
    /// Human-readable display name.
    pub fn display_name(&self) -> &'static str {
        match self {
            NetworkTier::Access => "Access",
            NetworkTier::Aggregation => "Aggregation",
            NetworkTier::Core => "Core",
            NetworkTier::Backbone => "Backbone",
            NetworkTier::Global => "Global",
        }
    }

    pub fn can_connect_to(&self, other: &NetworkTier) -> bool {
        let diff = (*self as i32 - *other as i32).abs();
        diff <= 1
    }

    pub fn value(&self) -> u8 {
        *self as u8
    }
}

/// Traffic demand between an origin-destination pair.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrafficDemand {
    pub source_city: EntityId,
    pub dest_city: EntityId,
    pub demand: f64,
}

/// Aggregated traffic flow data stored on GameWorld.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TrafficMatrix {
    /// OD pairs and their demand values
    pub od_pairs: Vec<TrafficDemand>,
    /// External (internet-bound) traffic per city
    pub external_traffic: Vec<(EntityId, f64)>,
    /// Total demand across all cities
    pub total_demand: f64,
    /// Tick when this matrix was last recomputed
    pub last_computed_tick: Tick,
    /// Per-node accumulated traffic load from OD routing
    pub node_traffic: std::collections::HashMap<EntityId, f64>,
    /// Per-edge accumulated traffic load from OD routing
    pub edge_traffic: std::collections::HashMap<EntityId, f64>,
    /// Total traffic successfully served
    pub total_served: f64,
    /// Total traffic dropped due to congestion or no path
    pub total_dropped: f64,
    /// Per-corporation traffic served
    pub corp_traffic_served: std::collections::HashMap<EntityId, f64>,
    /// Per-corporation traffic dropped
    pub corp_traffic_dropped: std::collections::HashMap<EntityId, f64>,
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
    BackboneRouter,
}

impl NodeType {
    /// Human-readable display name for UI.
    pub fn display_name(&self) -> &'static str {
        match self {
            NodeType::CellTower => "Cell Tower",
            NodeType::WirelessRelay => "Wireless Relay",
            NodeType::CentralOffice => "Central Office",
            NodeType::ExchangePoint => "Exchange Point",
            NodeType::DataCenter => "Data Center",
            NodeType::BackboneRouter => "Backbone Router",
            NodeType::SatelliteGround => "Satellite Ground Station",
            NodeType::SubmarineLanding => "Submarine Landing Point",
        }
    }

    /// Returns the network tier for this node type.
    pub fn tier(&self) -> NetworkTier {
        match self {
            NodeType::CellTower | NodeType::WirelessRelay => NetworkTier::Access,
            NodeType::CentralOffice | NodeType::ExchangePoint => NetworkTier::Aggregation,
            NodeType::DataCenter => NetworkTier::Core,
            NodeType::BackboneRouter => NetworkTier::Backbone,
            NodeType::SatelliteGround | NodeType::SubmarineLanding => NetworkTier::Global,
        }
    }

    /// Coverage radius in km. Wireless nodes cover a wide area; wired nodes only their immediate cell.
    pub fn coverage_radius_km(&self) -> f64 {
        match self {
            NodeType::CellTower => 15.0,
            NodeType::WirelessRelay => 8.0,
            NodeType::CentralOffice => 5.0,
            NodeType::ExchangePoint => 2.0,
            NodeType::DataCenter => 1.0,
            NodeType::BackboneRouter => 1.0,
            NodeType::SatelliteGround => 200.0,
            NodeType::SubmarineLanding => 0.5,
        }
    }

    /// Whether this node provides wireless coverage to nearby cells without requiring edges.
    pub fn is_wireless(&self) -> bool {
        matches!(self, NodeType::CellTower | NodeType::WirelessRelay | NodeType::SatelliteGround)
    }

    /// Fraction of throughput that serves coverage area (vs backbone transit).
    pub fn coverage_capacity_fraction(&self) -> f64 {
        match self {
            NodeType::CellTower => 0.8,
            NodeType::WirelessRelay => 0.9,
            NodeType::SatelliteGround => 0.6,
            NodeType::CentralOffice => 0.5,
            NodeType::ExchangePoint => 0.1,
            NodeType::DataCenter => 0.05,
            NodeType::BackboneRouter => 0.02,
            NodeType::SubmarineLanding => 0.02,
        }
    }

    /// Base construction cost for this node type (before terrain multipliers).
    pub fn construction_cost(&self) -> i64 {
        match self {
            NodeType::CellTower => 200_000,
            NodeType::WirelessRelay => 100_000,
            NodeType::CentralOffice => 500_000,
            NodeType::ExchangePoint => 2_000_000,
            NodeType::DataCenter => 10_000_000,
            NodeType::BackboneRouter => 3_000_000,
            NodeType::SatelliteGround => 5_000_000,
            NodeType::SubmarineLanding => 20_000_000,
        }
    }

    /// Revenue rate per unit of traffic for this node tier.
    pub fn traffic_revenue_rate(&self) -> f64 {
        match self {
            NodeType::CellTower | NodeType::WirelessRelay => 0.05,
            NodeType::CentralOffice | NodeType::ExchangePoint => 0.02,
            NodeType::DataCenter => 0.08,
            NodeType::BackboneRouter => 0.01,
            NodeType::SatelliteGround | NodeType::SubmarineLanding => 0.15,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EdgeType {
    FiberLocal,
    FiberRegional,
    FiberNational,
    Copper,
    Microwave,
    Satellite,
    Submarine,
}

impl EdgeType {
    /// Returns the set of tier pairs this edge type can connect.
    /// Each tuple is (min_tier, max_tier) — the edge can connect nodes
    /// whose tiers fall within this range (order: lower tier first).
    pub fn allowed_tier_connections(&self) -> &[(NetworkTier, NetworkTier)] {
        match self {
            EdgeType::Copper => &[
                (NetworkTier::Access, NetworkTier::Access),
                (NetworkTier::Access, NetworkTier::Aggregation),
            ],
            EdgeType::FiberLocal => &[
                (NetworkTier::Access, NetworkTier::Access),
                (NetworkTier::Access, NetworkTier::Aggregation),
                (NetworkTier::Aggregation, NetworkTier::Aggregation),
            ],
            EdgeType::FiberRegional => &[
                (NetworkTier::Aggregation, NetworkTier::Aggregation),
                (NetworkTier::Aggregation, NetworkTier::Core),
                (NetworkTier::Core, NetworkTier::Core),
            ],
            EdgeType::FiberNational => &[
                (NetworkTier::Core, NetworkTier::Core),
                (NetworkTier::Core, NetworkTier::Backbone),
                (NetworkTier::Backbone, NetworkTier::Backbone),
            ],
            EdgeType::Microwave => &[
                (NetworkTier::Access, NetworkTier::Access),
                (NetworkTier::Access, NetworkTier::Aggregation),
                (NetworkTier::Aggregation, NetworkTier::Aggregation),
                (NetworkTier::Aggregation, NetworkTier::Core),
            ],
            EdgeType::Satellite => &[
                (NetworkTier::Core, NetworkTier::Global),
                (NetworkTier::Backbone, NetworkTier::Global),
                (NetworkTier::Global, NetworkTier::Global),
            ],
            EdgeType::Submarine => &[
                (NetworkTier::Global, NetworkTier::Global),
            ],
        }
    }

    /// Check if this edge type can connect two node types.
    pub fn can_connect(&self, a: NodeType, b: NodeType) -> bool {
        let tier_a = a.tier();
        let tier_b = b.tier();
        let (lo, hi) = if tier_a <= tier_b {
            (tier_a, tier_b)
        } else {
            (tier_b, tier_a)
        };
        self.allowed_tier_connections()
            .iter()
            .any(|(t_lo, t_hi)| *t_lo == lo && *t_hi == hi)
    }

    /// Distance multiplier relative to cell_spacing_km.
    pub fn distance_multiplier(&self) -> f64 {
        match self {
            EdgeType::Copper => 2.0,
            EdgeType::FiberLocal => 5.0,
            EdgeType::Microwave => 8.0,
            EdgeType::FiberRegional => 15.0,
            EdgeType::FiberNational => 40.0,
            EdgeType::Satellite => f64::INFINITY,
            EdgeType::Submarine => 60.0,
        }
    }

    /// Human-readable display name for UI.
    pub fn display_name(&self) -> &'static str {
        match self {
            EdgeType::Copper => "Copper",
            EdgeType::FiberLocal => "Fiber Local",
            EdgeType::FiberRegional => "Fiber Regional",
            EdgeType::FiberNational => "Fiber National",
            EdgeType::Microwave => "Microwave",
            EdgeType::Satellite => "Satellite",
            EdgeType::Submarine => "Submarine Cable",
        }
    }

    /// Revenue rate per unit of traffic on this edge type.
    pub fn traffic_revenue_rate(&self) -> f64 {
        match self {
            EdgeType::FiberLocal => 0.005,
            EdgeType::FiberRegional => 0.01,
            EdgeType::FiberNational => 0.02,
            EdgeType::Microwave => 0.008,
            EdgeType::Satellite => 0.03,
            EdgeType::Submarine => 0.025,
            EdgeType::Copper => 0.003,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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
}

fn default_continent_count() -> u8 { 4 }
fn default_ocean_percentage() -> f64 { 0.7 }
fn default_terrain_roughness() -> f64 { 0.5 }
fn default_climate_variation() -> f64 { 0.5 }
fn default_city_density() -> f64 { 0.5 }

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
