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
    /// Extreme ocean depths (trenches, subduction zones). Very high construction cost
    /// for submarine cables and lowest reliability.
    OceanTrench,
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
            TerrainType::OceanTrench => 15.0,
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
            TerrainType::OceanTrench => 8.0,
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
            TerrainType::OceanTrench => 0.65,
            TerrainType::Tundra => 0.85,
            TerrainType::Frozen => 0.80,
        }
    }

    pub fn is_land(&self) -> bool {
        !matches!(
            self,
            TerrainType::OceanShallow | TerrainType::OceanDeep | TerrainType::OceanTrench
        )
    }

    /// Returns the bathymetry cost multiplier for submarine cable construction.
    ///
    /// This is separate from the general `construction_cost_multiplier` because
    /// submarine cables have specialized depth-based cost scaling:
    /// - Continental shelf (shallow): 1.0x base cost
    /// - Deep ocean: 2.0x base cost (pressure, specialized equipment)
    /// - Ocean trenches: 3.0x base cost (extreme depth, seismic risk)
    /// - Non-ocean terrain: 1.0x (landing station endpoints)
    pub fn submarine_bathymetry_multiplier(&self) -> f64 {
        match self {
            TerrainType::Coastal => 1.0,
            TerrainType::OceanShallow => 1.0,
            TerrainType::OceanDeep => 2.0,
            TerrainType::OceanTrench => 3.0,
            _ => 1.0, // land endpoints (landing stations)
        }
    }

    /// Returns true if this terrain is an ocean type (shallow, deep, or trench).
    pub fn is_ocean(&self) -> bool {
        matches!(
            self,
            TerrainType::OceanShallow | TerrainType::OceanDeep | TerrainType::OceanTrench
        )
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
#[allow(non_camel_case_types)]
pub enum NodeType {
    // ── Original 8 (kept for backward compat) ──
    CentralOffice,
    ExchangePoint,
    CellTower,
    DataCenter,
    SatelliteGround,
    SubmarineLanding,
    WirelessRelay,
    BackboneRouter,

    // ── Era 1: Telegraph (~1850s) ──
    TelegraphOffice,
    TelegraphRelay,
    CableHut,

    // ── Era 2: Telephone (~1900s) ──
    ManualExchange,
    AutomaticExchange,
    TelephonePole,
    LongDistanceRelay,

    // ── Era 3: Early Digital (~1970s) ──
    DigitalSwitch,
    MicrowaveTower,
    CoaxHub,
    EarlyDataCenter,
    SatelliteGroundStation,

    // ── Era 4: Internet (~1990s) ──
    FiberPOP,
    InternetExchangePoint,
    SubseaLandingStation,
    ColocationFacility,
    ISPGateway,

    // ── Era 5: Modern (~2010s) ──
    MacroCell,
    SmallCell,
    EdgeDataCenter,
    HyperscaleDataCenter,
    CloudOnRamp,
    ContentDeliveryNode,
    FiberSplicePoint,
    DWDM_Terminal,
    FiberDistributionHub,
    NetworkAccessPoint,

    // ── Era 6: Near Future (~2030s) ──
    LEO_SatelliteGateway,
    QuantumRepeater,
    MeshDroneRelay,
    UnderwaterDataCenter,
    NeuromorphicEdgeNode,
    TerahertzRelay,
}

impl NodeType {
    /// Human-readable display name for UI.
    pub fn display_name(&self) -> &'static str {
        match self {
            // Original 8
            NodeType::CellTower => "Cell Tower",
            NodeType::WirelessRelay => "Wireless Relay",
            NodeType::CentralOffice => "Central Office",
            NodeType::ExchangePoint => "Exchange Point",
            NodeType::DataCenter => "Data Center",
            NodeType::BackboneRouter => "Backbone Router",
            NodeType::SatelliteGround => "Satellite Ground Station",
            NodeType::SubmarineLanding => "Submarine Landing Point",

            // Era 1: Telegraph
            NodeType::TelegraphOffice => "Telegraph Office",
            NodeType::TelegraphRelay => "Telegraph Relay",
            NodeType::CableHut => "Cable Hut",

            // Era 2: Telephone
            NodeType::ManualExchange => "Manual Exchange",
            NodeType::AutomaticExchange => "Automatic Exchange",
            NodeType::TelephonePole => "Telephone Pole",
            NodeType::LongDistanceRelay => "Long Distance Relay",

            // Era 3: Early Digital
            NodeType::DigitalSwitch => "Digital Switch",
            NodeType::MicrowaveTower => "Microwave Tower",
            NodeType::CoaxHub => "Coax Hub",
            NodeType::EarlyDataCenter => "Early Data Center",
            NodeType::SatelliteGroundStation => "Satellite Ground Station (GEO)",

            // Era 4: Internet
            NodeType::FiberPOP => "Fiber Point of Presence",
            NodeType::InternetExchangePoint => "Internet Exchange Point",
            NodeType::SubseaLandingStation => "Subsea Landing Station",
            NodeType::ColocationFacility => "Colocation Facility",
            NodeType::ISPGateway => "ISP Gateway",

            // Era 5: Modern
            NodeType::MacroCell => "Macro Cell (4G/5G)",
            NodeType::SmallCell => "Small Cell (5G)",
            NodeType::EdgeDataCenter => "Edge Data Center",
            NodeType::HyperscaleDataCenter => "Hyperscale Data Center",
            NodeType::CloudOnRamp => "Cloud On-Ramp",
            NodeType::ContentDeliveryNode => "Content Delivery Node",
            NodeType::FiberSplicePoint => "Fiber Splice Point",
            NodeType::DWDM_Terminal => "DWDM Terminal",
            NodeType::FiberDistributionHub => "Fiber Distribution Hub",
            NodeType::NetworkAccessPoint => "Network Access Point",

            // Era 6: Near Future
            NodeType::LEO_SatelliteGateway => "LEO Satellite Gateway",
            NodeType::QuantumRepeater => "Quantum Repeater",
            NodeType::MeshDroneRelay => "Mesh Drone Relay",
            NodeType::UnderwaterDataCenter => "Underwater Data Center",
            NodeType::NeuromorphicEdgeNode => "Neuromorphic Edge Node",
            NodeType::TerahertzRelay => "Terahertz Relay",
        }
    }

    /// Returns the network tier for this node type.
    pub fn tier(&self) -> NetworkTier {
        match self {
            // Access tier — last-mile / end-user facing
            NodeType::CellTower
            | NodeType::WirelessRelay
            | NodeType::TelegraphOffice
            | NodeType::TelegraphRelay
            | NodeType::TelephonePole
            | NodeType::SmallCell
            | NodeType::NetworkAccessPoint
            | NodeType::MeshDroneRelay
            | NodeType::TerahertzRelay => NetworkTier::Access,

            // Aggregation tier — city/metro level
            NodeType::CentralOffice
            | NodeType::ExchangePoint
            | NodeType::ManualExchange
            | NodeType::AutomaticExchange
            | NodeType::LongDistanceRelay
            | NodeType::DigitalSwitch
            | NodeType::CoaxHub
            | NodeType::FiberPOP
            | NodeType::ISPGateway
            | NodeType::MacroCell
            | NodeType::FiberSplicePoint
            | NodeType::FiberDistributionHub
            | NodeType::QuantumRepeater => NetworkTier::Aggregation,

            // Core tier — regional / national switching
            NodeType::DataCenter
            | NodeType::MicrowaveTower
            | NodeType::EarlyDataCenter
            | NodeType::InternetExchangePoint
            | NodeType::ColocationFacility
            | NodeType::EdgeDataCenter
            | NodeType::ContentDeliveryNode
            | NodeType::DWDM_Terminal
            | NodeType::CloudOnRamp
            | NodeType::NeuromorphicEdgeNode => NetworkTier::Core,

            // Backbone tier — national / continental
            NodeType::BackboneRouter
            | NodeType::HyperscaleDataCenter
            | NodeType::SatelliteGroundStation => NetworkTier::Backbone,

            // Global tier — intercontinental
            NodeType::SatelliteGround
            | NodeType::SubmarineLanding
            | NodeType::CableHut
            | NodeType::SubseaLandingStation
            | NodeType::LEO_SatelliteGateway
            | NodeType::UnderwaterDataCenter => NetworkTier::Global,
        }
    }

    /// Coverage radius in km. Wireless nodes cover a wide area; wired nodes only their immediate cell.
    pub fn coverage_radius_km(&self) -> f64 {
        match self {
            // Original 8 (unchanged)
            NodeType::CellTower => 15.0,
            NodeType::WirelessRelay => 8.0,
            NodeType::CentralOffice => 5.0,
            NodeType::ExchangePoint => 2.0,
            NodeType::DataCenter => 1.0,
            NodeType::BackboneRouter => 1.0,
            NodeType::SatelliteGround => 200.0,
            NodeType::SubmarineLanding => 0.5,

            // Era 1: Telegraph
            NodeType::TelegraphOffice => 3.0,
            NodeType::TelegraphRelay => 1.0,
            NodeType::CableHut => 0.5,

            // Era 2: Telephone
            NodeType::ManualExchange => 5.0,
            NodeType::AutomaticExchange => 8.0,
            NodeType::TelephonePole => 0.5,
            NodeType::LongDistanceRelay => 2.0,

            // Era 3: Early Digital
            NodeType::DigitalSwitch => 8.0,
            NodeType::MicrowaveTower => 50.0,
            NodeType::CoaxHub => 5.0,
            NodeType::EarlyDataCenter => 1.0,
            NodeType::SatelliteGroundStation => 300.0,

            // Era 4: Internet
            NodeType::FiberPOP => 3.0,
            NodeType::InternetExchangePoint => 2.0,
            NodeType::SubseaLandingStation => 0.5,
            NodeType::ColocationFacility => 1.5,
            NodeType::ISPGateway => 5.0,

            // Era 5: Modern
            NodeType::MacroCell => 20.0,
            NodeType::SmallCell => 1.0,
            NodeType::EdgeDataCenter => 2.0,
            NodeType::HyperscaleDataCenter => 1.0,
            NodeType::CloudOnRamp => 1.0,
            NodeType::ContentDeliveryNode => 3.0,
            NodeType::FiberSplicePoint => 0.2,
            NodeType::DWDM_Terminal => 0.5,
            NodeType::FiberDistributionHub => 2.0,
            NodeType::NetworkAccessPoint => 0.3,

            // Era 6: Near Future
            NodeType::LEO_SatelliteGateway => 500.0,
            NodeType::QuantumRepeater => 1.0,
            NodeType::MeshDroneRelay => 30.0,
            NodeType::UnderwaterDataCenter => 0.5,
            NodeType::NeuromorphicEdgeNode => 2.0,
            NodeType::TerahertzRelay => 0.5,
        }
    }

    /// Whether this node provides wireless coverage to nearby cells without requiring edges.
    pub fn is_wireless(&self) -> bool {
        matches!(
            self,
            NodeType::CellTower
                | NodeType::WirelessRelay
                | NodeType::SatelliteGround
                | NodeType::MicrowaveTower
                | NodeType::SatelliteGroundStation
                | NodeType::MacroCell
                | NodeType::SmallCell
                | NodeType::LEO_SatelliteGateway
                | NodeType::MeshDroneRelay
                | NodeType::TerahertzRelay
        )
    }

    /// Fraction of throughput that serves coverage area (vs backbone transit).
    pub fn coverage_capacity_fraction(&self) -> f64 {
        match self {
            // Original 8 (unchanged)
            NodeType::CellTower => 0.8,
            NodeType::WirelessRelay => 0.9,
            NodeType::SatelliteGround => 0.6,
            NodeType::CentralOffice => 0.5,
            NodeType::ExchangePoint => 0.1,
            NodeType::DataCenter => 0.05,
            NodeType::BackboneRouter => 0.02,
            NodeType::SubmarineLanding => 0.02,

            // Era 1: Telegraph — mostly local coverage
            NodeType::TelegraphOffice => 0.9,
            NodeType::TelegraphRelay => 0.3,
            NodeType::CableHut => 0.05,

            // Era 2: Telephone
            NodeType::ManualExchange => 0.7,
            NodeType::AutomaticExchange => 0.6,
            NodeType::TelephonePole => 0.95,
            NodeType::LongDistanceRelay => 0.2,

            // Era 3: Early Digital
            NodeType::DigitalSwitch => 0.5,
            NodeType::MicrowaveTower => 0.7,
            NodeType::CoaxHub => 0.8,
            NodeType::EarlyDataCenter => 0.05,
            NodeType::SatelliteGroundStation => 0.5,

            // Era 4: Internet
            NodeType::FiberPOP => 0.3,
            NodeType::InternetExchangePoint => 0.1,
            NodeType::SubseaLandingStation => 0.02,
            NodeType::ColocationFacility => 0.1,
            NodeType::ISPGateway => 0.6,

            // Era 5: Modern
            NodeType::MacroCell => 0.8,
            NodeType::SmallCell => 0.9,
            NodeType::EdgeDataCenter => 0.15,
            NodeType::HyperscaleDataCenter => 0.03,
            NodeType::CloudOnRamp => 0.05,
            NodeType::ContentDeliveryNode => 0.7,
            NodeType::FiberSplicePoint => 0.0,     // passive junction, no coverage
            NodeType::DWDM_Terminal => 0.02,
            NodeType::FiberDistributionHub => 0.1,
            NodeType::NetworkAccessPoint => 0.9,

            // Era 6: Near Future
            NodeType::LEO_SatelliteGateway => 0.5,
            NodeType::QuantumRepeater => 0.05,
            NodeType::MeshDroneRelay => 0.85,
            NodeType::UnderwaterDataCenter => 0.02,
            NodeType::NeuromorphicEdgeNode => 0.3,
            NodeType::TerahertzRelay => 0.8,
        }
    }

    /// Base construction cost for this node type (before terrain multipliers).
    pub fn construction_cost(&self) -> i64 {
        match self {
            // Original 8 (unchanged)
            NodeType::CellTower => 200_000,
            NodeType::WirelessRelay => 100_000,
            NodeType::CentralOffice => 500_000,
            NodeType::ExchangePoint => 2_000_000,
            NodeType::DataCenter => 10_000_000,
            NodeType::BackboneRouter => 3_000_000,
            NodeType::SatelliteGround => 5_000_000,
            NodeType::SubmarineLanding => 20_000_000,

            // Era 1: Telegraph — very cheap
            NodeType::TelegraphOffice => 5_000,
            NodeType::TelegraphRelay => 2_000,
            NodeType::CableHut => 15_000,

            // Era 2: Telephone — cheap
            NodeType::ManualExchange => 20_000,
            NodeType::AutomaticExchange => 50_000,
            NodeType::TelephonePole => 1_000,
            NodeType::LongDistanceRelay => 30_000,

            // Era 3: Early Digital — moderate
            NodeType::DigitalSwitch => 200_000,
            NodeType::MicrowaveTower => 150_000,
            NodeType::CoaxHub => 80_000,
            NodeType::EarlyDataCenter => 2_000_000,
            NodeType::SatelliteGroundStation => 3_000_000,

            // Era 4: Internet — moderate to expensive
            NodeType::FiberPOP => 800_000,
            NodeType::InternetExchangePoint => 3_000_000,
            NodeType::SubseaLandingStation => 25_000_000,
            NodeType::ColocationFacility => 8_000_000,
            NodeType::ISPGateway => 400_000,

            // Era 5: Modern — expensive
            NodeType::MacroCell => 300_000,
            NodeType::SmallCell => 50_000,
            NodeType::EdgeDataCenter => 5_000_000,
            NodeType::HyperscaleDataCenter => 500_000_000,
            NodeType::CloudOnRamp => 2_000_000,
            NodeType::ContentDeliveryNode => 1_000_000,
            NodeType::FiberSplicePoint => 10_000,
            NodeType::DWDM_Terminal => 4_000_000,
            NodeType::FiberDistributionHub => 25_000,
            NodeType::NetworkAccessPoint => 5_000,

            // Era 6: Near Future — very expensive
            NodeType::LEO_SatelliteGateway => 50_000_000,
            NodeType::QuantumRepeater => 20_000_000,
            NodeType::MeshDroneRelay => 500_000,
            NodeType::UnderwaterDataCenter => 200_000_000,
            NodeType::NeuromorphicEdgeNode => 15_000_000,
            NodeType::TerahertzRelay => 3_000_000,
        }
    }

    /// Revenue rate per unit of traffic for this node tier.
    /// Calibrated so a node at ~50% utilization covers its maintenance.
    pub fn traffic_revenue_rate(&self) -> f64 {
        match self {
            // Original 8 (unchanged)
            NodeType::CellTower => 5.0,            // max 500 throughput -> $2,500 at full util
            NodeType::WirelessRelay => 6.0,         // max 300 -> $1,800
            NodeType::CentralOffice => 4.0,         // max 1,000 -> $4,000
            NodeType::ExchangePoint => 3.0,         // max 5,000 -> $15,000
            NodeType::DataCenter => 2.0,            // max 50,000 -> $100,000
            NodeType::BackboneRouter => 1.5,        // max 20,000 -> $30,000
            NodeType::SatelliteGround => 20.0,      // max 2,000 -> $40,000
            NodeType::SubmarineLanding => 2.0,      // max 100,000 -> $200,000

            // Era 1: Telegraph — high rate per unit (scarce commodity)
            NodeType::TelegraphOffice => 50.0,
            NodeType::TelegraphRelay => 40.0,
            NodeType::CableHut => 30.0,

            // Era 2: Telephone
            NodeType::ManualExchange => 25.0,
            NodeType::AutomaticExchange => 15.0,
            NodeType::TelephonePole => 20.0,
            NodeType::LongDistanceRelay => 18.0,

            // Era 3: Early Digital
            NodeType::DigitalSwitch => 8.0,
            NodeType::MicrowaveTower => 10.0,
            NodeType::CoaxHub => 7.0,
            NodeType::EarlyDataCenter => 4.0,
            NodeType::SatelliteGroundStation => 15.0,

            // Era 4: Internet
            NodeType::FiberPOP => 3.5,
            NodeType::InternetExchangePoint => 2.5,
            NodeType::SubseaLandingStation => 1.8,
            NodeType::ColocationFacility => 2.0,
            NodeType::ISPGateway => 5.0,

            // Era 5: Modern
            NodeType::MacroCell => 4.0,
            NodeType::SmallCell => 8.0,
            NodeType::EdgeDataCenter => 3.0,
            NodeType::HyperscaleDataCenter => 0.5,
            NodeType::CloudOnRamp => 2.0,
            NodeType::ContentDeliveryNode => 3.5,
            NodeType::FiberSplicePoint => 0.1,
            NodeType::DWDM_Terminal => 1.0,
            NodeType::FiberDistributionHub => 4.0,
            NodeType::NetworkAccessPoint => 6.0,

            // Era 6: Near Future
            NodeType::LEO_SatelliteGateway => 1.0,
            NodeType::QuantumRepeater => 25.0,
            NodeType::MeshDroneRelay => 5.0,
            NodeType::UnderwaterDataCenter => 0.4,
            NodeType::NeuromorphicEdgeNode => 2.0,
            NodeType::TerahertzRelay => 6.0,
        }
    }

    /// Which era this node type belongs to.
    pub fn era(&self) -> Era {
        match self {
            // Era 1: Telegraph
            NodeType::TelegraphOffice
            | NodeType::TelegraphRelay
            | NodeType::CableHut => Era::Telegraph,

            // Era 2: Telephone
            NodeType::ManualExchange
            | NodeType::AutomaticExchange
            | NodeType::TelephonePole
            | NodeType::LongDistanceRelay => Era::Telephone,

            // Era 3: Early Digital
            NodeType::DigitalSwitch
            | NodeType::MicrowaveTower
            | NodeType::CoaxHub
            | NodeType::EarlyDataCenter
            | NodeType::SatelliteGroundStation => Era::EarlyDigital,

            // Era 4: Internet (original 8 fall here as defaults)
            NodeType::CentralOffice
            | NodeType::ExchangePoint
            | NodeType::CellTower
            | NodeType::DataCenter
            | NodeType::SatelliteGround
            | NodeType::SubmarineLanding
            | NodeType::WirelessRelay
            | NodeType::BackboneRouter
            | NodeType::FiberPOP
            | NodeType::InternetExchangePoint
            | NodeType::SubseaLandingStation
            | NodeType::ColocationFacility
            | NodeType::ISPGateway => Era::Internet,

            // Era 5: Modern
            NodeType::MacroCell
            | NodeType::SmallCell
            | NodeType::EdgeDataCenter
            | NodeType::HyperscaleDataCenter
            | NodeType::CloudOnRamp
            | NodeType::ContentDeliveryNode
            | NodeType::FiberSplicePoint
            | NodeType::DWDM_Terminal
            | NodeType::FiberDistributionHub
            | NodeType::NetworkAccessPoint => Era::Modern,

            // Era 6: Near Future
            NodeType::LEO_SatelliteGateway
            | NodeType::QuantumRepeater
            | NodeType::MeshDroneRelay
            | NodeType::UnderwaterDataCenter
            | NodeType::NeuromorphicEdgeNode
            | NodeType::TerahertzRelay => Era::NearFuture,
        }
    }

    /// Maximum throughput capacity for this node type.
    pub fn max_throughput(&self) -> u64 {
        match self {
            // Original 8 (match existing InfraNode::new values)
            NodeType::CellTower => 500,
            NodeType::WirelessRelay => 300,
            NodeType::CentralOffice => 1_000,
            NodeType::ExchangePoint => 5_000,
            NodeType::DataCenter => 50_000,
            NodeType::BackboneRouter => 20_000,
            NodeType::SatelliteGround => 2_000,
            NodeType::SubmarineLanding => 100_000,

            // Era 1: Telegraph — very low throughput
            NodeType::TelegraphOffice => 5,
            NodeType::TelegraphRelay => 3,
            NodeType::CableHut => 10,

            // Era 2: Telephone — low
            NodeType::ManualExchange => 20,
            NodeType::AutomaticExchange => 50,
            NodeType::TelephonePole => 10,
            NodeType::LongDistanceRelay => 30,

            // Era 3: Early Digital — moderate
            NodeType::DigitalSwitch => 500,
            NodeType::MicrowaveTower => 200,
            NodeType::CoaxHub => 300,
            NodeType::EarlyDataCenter => 5_000,
            NodeType::SatelliteGroundStation => 1_000,

            // Era 4: Internet
            NodeType::FiberPOP => 10_000,
            NodeType::InternetExchangePoint => 50_000,
            NodeType::SubseaLandingStation => 200_000,
            NodeType::ColocationFacility => 30_000,
            NodeType::ISPGateway => 2_000,

            // Era 5: Modern
            NodeType::MacroCell => 1_000,
            NodeType::SmallCell => 200,
            NodeType::EdgeDataCenter => 20_000,
            NodeType::HyperscaleDataCenter => 1_000_000,
            NodeType::CloudOnRamp => 50_000,
            NodeType::ContentDeliveryNode => 30_000,
            NodeType::FiberSplicePoint => 0,           // passive, no throughput
            NodeType::DWDM_Terminal => 100_000,
            NodeType::FiberDistributionHub => 1_000,
            NodeType::NetworkAccessPoint => 100,

            // Era 6: Near Future
            NodeType::LEO_SatelliteGateway => 500_000,
            NodeType::QuantumRepeater => 10_000,
            NodeType::MeshDroneRelay => 500,
            NodeType::UnderwaterDataCenter => 2_000_000,
            NodeType::NeuromorphicEdgeNode => 100_000,
            NodeType::TerahertzRelay => 50_000,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[allow(non_camel_case_types)]
pub enum EdgeType {
    // ── Original 7 (kept for backward compat) ──
    FiberLocal,
    FiberRegional,
    FiberNational,
    Copper,
    Microwave,
    Satellite,
    Submarine,

    // ── Era 1: Telegraph (~1850s) ──
    TelegraphWire,
    SubseaTelegraphCable,

    // ── Era 2: Telephone (~1900s) ──
    CopperTrunkLine,
    LongDistanceCopper,

    // ── Era 3: Early Digital (~1970s) ──
    CoaxialCable,
    MicrowaveLink,
    EarlySatelliteLink,

    // ── Era 4: Internet (~1990s) ──
    SubseaFiberCable,

    // ── Era 5: Modern (~2010s) ──
    FiberMetro,
    FiberLongHaul,
    DWDM_Backbone,
    SatelliteLEOLink,
    FeederFiber,
    DistributionFiber,
    DropCable,

    // ── Era 6: Near Future (~2030s) ──
    QuantumFiberLink,
    TerahertzBeam,
    LaserInterSatelliteLink,
}

impl EdgeType {
    /// Returns the set of tier pairs this edge type can connect.
    /// Each tuple is (min_tier, max_tier) — the edge can connect nodes
    /// whose tiers fall within this range (order: lower tier first).
    pub fn allowed_tier_connections(&self) -> &[(NetworkTier, NetworkTier)] {
        match self {
            // Original 7 (unchanged)
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

            // Era 1: Telegraph
            EdgeType::TelegraphWire => &[
                (NetworkTier::Access, NetworkTier::Access),
                (NetworkTier::Access, NetworkTier::Aggregation),
                (NetworkTier::Aggregation, NetworkTier::Aggregation),
            ],
            EdgeType::SubseaTelegraphCable => &[
                (NetworkTier::Global, NetworkTier::Global),
                (NetworkTier::Aggregation, NetworkTier::Global),
            ],

            // Era 2: Telephone
            EdgeType::CopperTrunkLine => &[
                (NetworkTier::Access, NetworkTier::Aggregation),
                (NetworkTier::Aggregation, NetworkTier::Aggregation),
            ],
            EdgeType::LongDistanceCopper => &[
                (NetworkTier::Aggregation, NetworkTier::Aggregation),
                (NetworkTier::Aggregation, NetworkTier::Core),
                (NetworkTier::Core, NetworkTier::Core),
            ],

            // Era 3: Early Digital
            EdgeType::CoaxialCable => &[
                (NetworkTier::Access, NetworkTier::Access),
                (NetworkTier::Access, NetworkTier::Aggregation),
                (NetworkTier::Aggregation, NetworkTier::Aggregation),
            ],
            EdgeType::MicrowaveLink => &[
                (NetworkTier::Aggregation, NetworkTier::Aggregation),
                (NetworkTier::Aggregation, NetworkTier::Core),
                (NetworkTier::Core, NetworkTier::Core),
                (NetworkTier::Core, NetworkTier::Backbone),
            ],
            EdgeType::EarlySatelliteLink => &[
                (NetworkTier::Core, NetworkTier::Global),
                (NetworkTier::Backbone, NetworkTier::Global),
                (NetworkTier::Global, NetworkTier::Global),
            ],

            // Era 4: Internet
            EdgeType::SubseaFiberCable => &[
                (NetworkTier::Backbone, NetworkTier::Global),
                (NetworkTier::Global, NetworkTier::Global),
            ],

            // Era 5: Modern
            EdgeType::FiberMetro => &[
                (NetworkTier::Aggregation, NetworkTier::Aggregation),
                (NetworkTier::Aggregation, NetworkTier::Core),
                (NetworkTier::Core, NetworkTier::Core),
            ],
            EdgeType::FiberLongHaul => &[
                (NetworkTier::Core, NetworkTier::Core),
                (NetworkTier::Core, NetworkTier::Backbone),
                (NetworkTier::Backbone, NetworkTier::Backbone),
            ],
            EdgeType::DWDM_Backbone => &[
                (NetworkTier::Core, NetworkTier::Backbone),
                (NetworkTier::Backbone, NetworkTier::Backbone),
                (NetworkTier::Backbone, NetworkTier::Global),
            ],
            EdgeType::SatelliteLEOLink => &[
                (NetworkTier::Access, NetworkTier::Global),
                (NetworkTier::Core, NetworkTier::Global),
                (NetworkTier::Backbone, NetworkTier::Global),
                (NetworkTier::Global, NetworkTier::Global),
            ],
            EdgeType::FeederFiber => &[
                (NetworkTier::Aggregation, NetworkTier::Aggregation),
                (NetworkTier::Aggregation, NetworkTier::Core),
            ],
            EdgeType::DistributionFiber => &[
                (NetworkTier::Access, NetworkTier::Aggregation),
                (NetworkTier::Aggregation, NetworkTier::Aggregation),
            ],
            EdgeType::DropCable => &[
                (NetworkTier::Access, NetworkTier::Access),
                (NetworkTier::Access, NetworkTier::Aggregation),
            ],

            // Era 6: Near Future
            EdgeType::QuantumFiberLink => &[
                (NetworkTier::Core, NetworkTier::Core),
                (NetworkTier::Core, NetworkTier::Backbone),
                (NetworkTier::Backbone, NetworkTier::Backbone),
                (NetworkTier::Backbone, NetworkTier::Global),
            ],
            EdgeType::TerahertzBeam => &[
                (NetworkTier::Access, NetworkTier::Access),
                (NetworkTier::Access, NetworkTier::Aggregation),
                (NetworkTier::Aggregation, NetworkTier::Aggregation),
            ],
            EdgeType::LaserInterSatelliteLink => &[
                (NetworkTier::Global, NetworkTier::Global),
                (NetworkTier::Backbone, NetworkTier::Global),
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
            // Original 7 (unchanged)
            EdgeType::Copper => 2.0,
            EdgeType::FiberLocal => 5.0,
            EdgeType::Microwave => 8.0,
            EdgeType::FiberRegional => 15.0,
            EdgeType::FiberNational => 40.0,
            EdgeType::Satellite => f64::INFINITY,
            EdgeType::Submarine => 60.0,

            // Era 1: Telegraph
            EdgeType::TelegraphWire => 3.0,
            EdgeType::SubseaTelegraphCable => 80.0,

            // Era 2: Telephone
            EdgeType::CopperTrunkLine => 4.0,
            EdgeType::LongDistanceCopper => 10.0,

            // Era 3: Early Digital
            EdgeType::CoaxialCable => 3.0,
            EdgeType::MicrowaveLink => 12.0,
            EdgeType::EarlySatelliteLink => f64::INFINITY,

            // Era 4: Internet
            EdgeType::SubseaFiberCable => 80.0,

            // Era 5: Modern
            EdgeType::FiberMetro => 10.0,
            EdgeType::FiberLongHaul => 50.0,
            EdgeType::DWDM_Backbone => 60.0,
            EdgeType::SatelliteLEOLink => f64::INFINITY,
            EdgeType::FeederFiber => 8.0,
            EdgeType::DistributionFiber => 3.0,
            EdgeType::DropCable => 1.0,

            // Era 6: Near Future
            EdgeType::QuantumFiberLink => 30.0,
            EdgeType::TerahertzBeam => 2.0,
            EdgeType::LaserInterSatelliteLink => f64::INFINITY,
        }
    }

    /// Human-readable display name for UI.
    pub fn display_name(&self) -> &'static str {
        match self {
            // Original 7 (unchanged)
            EdgeType::Copper => "Copper",
            EdgeType::FiberLocal => "Fiber Local",
            EdgeType::FiberRegional => "Fiber Regional",
            EdgeType::FiberNational => "Fiber National",
            EdgeType::Microwave => "Microwave",
            EdgeType::Satellite => "Satellite",
            EdgeType::Submarine => "Submarine Cable",

            // Era 1: Telegraph
            EdgeType::TelegraphWire => "Telegraph Wire",
            EdgeType::SubseaTelegraphCable => "Subsea Telegraph Cable",

            // Era 2: Telephone
            EdgeType::CopperTrunkLine => "Copper Trunk Line",
            EdgeType::LongDistanceCopper => "Long Distance Copper",

            // Era 3: Early Digital
            EdgeType::CoaxialCable => "Coaxial Cable",
            EdgeType::MicrowaveLink => "Microwave Link",
            EdgeType::EarlySatelliteLink => "Early Satellite Link",

            // Era 4: Internet
            EdgeType::SubseaFiberCable => "Subsea Fiber Cable",

            // Era 5: Modern
            EdgeType::FiberMetro => "Fiber Metro",
            EdgeType::FiberLongHaul => "Fiber Long Haul",
            EdgeType::DWDM_Backbone => "DWDM Backbone",
            EdgeType::SatelliteLEOLink => "Satellite LEO Link",
            EdgeType::FeederFiber => "Feeder Fiber",
            EdgeType::DistributionFiber => "Distribution Fiber",
            EdgeType::DropCable => "Drop Cable",

            // Era 6: Near Future
            EdgeType::QuantumFiberLink => "Quantum Fiber Link",
            EdgeType::TerahertzBeam => "Terahertz Beam",
            EdgeType::LaserInterSatelliteLink => "Laser Inter-Satellite Link",
        }
    }

    /// Revenue rate per unit of traffic on this edge type.
    /// Transit fees — edges earn revenue proportional to traffic they carry.
    pub fn traffic_revenue_rate(&self) -> f64 {
        match self {
            // Original 7 (unchanged)
            EdgeType::Copper => 0.3,
            EdgeType::FiberLocal => 0.5,
            EdgeType::Microwave => 0.8,
            EdgeType::FiberRegional => 1.5,
            EdgeType::FiberNational => 3.0,
            EdgeType::Satellite => 5.0,
            EdgeType::Submarine => 4.0,

            // Era 1: Telegraph — high rate per unit (premium service)
            EdgeType::TelegraphWire => 10.0,
            EdgeType::SubseaTelegraphCable => 20.0,

            // Era 2: Telephone
            EdgeType::CopperTrunkLine => 2.0,
            EdgeType::LongDistanceCopper => 4.0,

            // Era 3: Early Digital
            EdgeType::CoaxialCable => 0.5,
            EdgeType::MicrowaveLink => 1.5,
            EdgeType::EarlySatelliteLink => 8.0,

            // Era 4: Internet
            EdgeType::SubseaFiberCable => 3.5,

            // Era 5: Modern
            EdgeType::FiberMetro => 1.0,
            EdgeType::FiberLongHaul => 2.5,
            EdgeType::DWDM_Backbone => 2.0,
            EdgeType::SatelliteLEOLink => 3.0,
            EdgeType::FeederFiber => 0.8,
            EdgeType::DistributionFiber => 0.6,
            EdgeType::DropCable => 0.4,

            // Era 6: Near Future
            EdgeType::QuantumFiberLink => 5.0,
            EdgeType::TerahertzBeam => 4.0,
            EdgeType::LaserInterSatelliteLink => 3.0,
        }
    }

    /// Which era this edge type belongs to.
    pub fn era(&self) -> Era {
        match self {
            EdgeType::TelegraphWire | EdgeType::SubseaTelegraphCable => Era::Telegraph,
            EdgeType::CopperTrunkLine | EdgeType::LongDistanceCopper => Era::Telephone,
            EdgeType::CoaxialCable | EdgeType::MicrowaveLink | EdgeType::EarlySatelliteLink => {
                Era::EarlyDigital
            }
            EdgeType::Copper
            | EdgeType::FiberLocal
            | EdgeType::FiberRegional
            | EdgeType::FiberNational
            | EdgeType::Microwave
            | EdgeType::Satellite
            | EdgeType::Submarine
            | EdgeType::SubseaFiberCable => Era::Internet,
            EdgeType::FiberMetro
            | EdgeType::FiberLongHaul
            | EdgeType::DWDM_Backbone
            | EdgeType::SatelliteLEOLink
            | EdgeType::FeederFiber
            | EdgeType::DistributionFiber
            | EdgeType::DropCable => Era::Modern,
            EdgeType::QuantumFiberLink
            | EdgeType::TerahertzBeam
            | EdgeType::LaserInterSatelliteLink => Era::NearFuture,
        }
    }

    /// Returns true if this edge type is a submarine/subsea cable.
    pub fn is_submarine(&self) -> bool {
        matches!(
            self,
            EdgeType::Submarine | EdgeType::SubseaTelegraphCable | EdgeType::SubseaFiberCable
        )
    }

    /// Cost per kilometer for this edge type.
    pub fn cost_per_km(&self) -> i64 {
        match self {
            // Original 7 (match existing InfraEdge::new values)
            EdgeType::Copper => 10_000,
            EdgeType::FiberLocal => 20_000,
            EdgeType::Microwave => 20_000,
            EdgeType::FiberRegional => 40_000,
            EdgeType::FiberNational => 80_000,
            EdgeType::Satellite => 0,             // flat cost satellite
            EdgeType::Submarine => 200_000,

            // Era 1: Telegraph
            EdgeType::TelegraphWire => 500,
            EdgeType::SubseaTelegraphCable => 5_000,

            // Era 2: Telephone
            EdgeType::CopperTrunkLine => 3_000,
            EdgeType::LongDistanceCopper => 8_000,

            // Era 3: Early Digital
            EdgeType::CoaxialCable => 8_000,
            EdgeType::MicrowaveLink => 15_000,
            EdgeType::EarlySatelliteLink => 0,     // flat cost

            // Era 4: Internet
            EdgeType::SubseaFiberCable => 250_000,

            // Era 5: Modern
            EdgeType::FiberMetro => 50_000,
            EdgeType::FiberLongHaul => 100_000,
            EdgeType::DWDM_Backbone => 150_000,
            EdgeType::SatelliteLEOLink => 0,       // flat cost
            EdgeType::FeederFiber => 30_000,
            EdgeType::DistributionFiber => 15_000,
            EdgeType::DropCable => 2_000,

            // Era 6: Near Future
            EdgeType::QuantumFiberLink => 500_000,
            EdgeType::TerahertzBeam => 100_000,
            EdgeType::LaserInterSatelliteLink => 0, // flat cost
        }
    }

    /// Base bandwidth capacity for this edge type.
    pub fn bandwidth(&self) -> u64 {
        match self {
            // Original 7 (match existing InfraEdge::new values)
            EdgeType::Copper => 1_000,
            EdgeType::FiberLocal => 10_000,
            EdgeType::Microwave => 5_000,
            EdgeType::FiberRegional => 50_000,
            EdgeType::FiberNational => 200_000,
            EdgeType::Satellite => 2_000,
            EdgeType::Submarine => 500_000,

            // Era 1: Telegraph
            EdgeType::TelegraphWire => 5,
            EdgeType::SubseaTelegraphCable => 10,

            // Era 2: Telephone
            EdgeType::CopperTrunkLine => 50,
            EdgeType::LongDistanceCopper => 100,

            // Era 3: Early Digital
            EdgeType::CoaxialCable => 500,
            EdgeType::MicrowaveLink => 2_000,
            EdgeType::EarlySatelliteLink => 500,

            // Era 4: Internet
            EdgeType::SubseaFiberCable => 1_000_000,

            // Era 5: Modern
            EdgeType::FiberMetro => 100_000,
            EdgeType::FiberLongHaul => 400_000,
            EdgeType::DWDM_Backbone => 1_000_000,
            EdgeType::SatelliteLEOLink => 20_000,
            EdgeType::FeederFiber => 40_000,
            EdgeType::DistributionFiber => 10_000,
            EdgeType::DropCable => 1_000,

            // Era 6: Near Future
            EdgeType::QuantumFiberLink => 500_000,
            EdgeType::TerahertzBeam => 100_000,
            EdgeType::LaserInterSatelliteLink => 2_000_000,
        }
    }

    /// Fiber strand count (0 for non-fiber edge types).
    pub fn strand_count(&self) -> u32 {
        match self {
            // Non-fiber types
            EdgeType::Copper
            | EdgeType::Microwave
            | EdgeType::Satellite
            | EdgeType::TelegraphWire
            | EdgeType::SubseaTelegraphCable
            | EdgeType::CopperTrunkLine
            | EdgeType::LongDistanceCopper
            | EdgeType::CoaxialCable
            | EdgeType::MicrowaveLink
            | EdgeType::EarlySatelliteLink
            | EdgeType::SatelliteLEOLink
            | EdgeType::TerahertzBeam
            | EdgeType::LaserInterSatelliteLink => 0,

            // Fiber types with strand counts
            EdgeType::DropCable => 2,
            EdgeType::FiberLocal => 12,
            EdgeType::DistributionFiber => 24,
            EdgeType::FiberRegional => 48,
            EdgeType::FeederFiber => 96,
            EdgeType::FiberMetro => 96,
            EdgeType::FiberNational => 144,
            EdgeType::FiberLongHaul => 288,
            EdgeType::DWDM_Backbone => 432,
            EdgeType::Submarine => 8,              // subsea pairs
            EdgeType::SubseaFiberCable => 16,
            EdgeType::QuantumFiberLink => 48,
        }
    }

    /// Whether this edge type can be deployed aerially (on poles).
    pub fn is_aerial_capable(&self) -> bool {
        matches!(
            self,
            EdgeType::TelegraphWire
                | EdgeType::Copper
                | EdgeType::CopperTrunkLine
                | EdgeType::LongDistanceCopper
                | EdgeType::CoaxialCable
                | EdgeType::FiberLocal
                | EdgeType::DistributionFiber
                | EdgeType::DropCable
                | EdgeType::FiberMetro
                | EdgeType::Microwave
                | EdgeType::MicrowaveLink
                | EdgeType::TerahertzBeam
        )
    }

    /// Whether this edge type can be deployed underground (buried/conduit).
    pub fn is_underground_capable(&self) -> bool {
        matches!(
            self,
            EdgeType::Copper
                | EdgeType::CopperTrunkLine
                | EdgeType::LongDistanceCopper
                | EdgeType::CoaxialCable
                | EdgeType::FiberLocal
                | EdgeType::FiberRegional
                | EdgeType::FiberNational
                | EdgeType::FiberMetro
                | EdgeType::FiberLongHaul
                | EdgeType::DWDM_Backbone
                | EdgeType::FeederFiber
                | EdgeType::DistributionFiber
                | EdgeType::DropCable
                | EdgeType::SubseaFiberCable
                | EdgeType::SubseaTelegraphCable
                | EdgeType::Submarine
                | EdgeType::QuantumFiberLink
        )
    }
}

// ── Phase 8: Spectrum & Frequency Management ──

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FrequencyBand {
    // Low band (wide coverage, lower speed)
    Band700MHz,
    Band800MHz,
    Band900MHz,
    // Mid band (balanced)
    Band1800MHz,
    Band2100MHz,
    Band2600MHz,
    // High band (short range, highest speed)
    Band3500MHz,
    Band28GHz,  // mmWave
    Band39GHz,  // mmWave
}

impl FrequencyBand {
    /// Coverage radius in km for this frequency band.
    pub fn coverage_radius_km(&self) -> f64 {
        match self {
            FrequencyBand::Band700MHz => 30.0,
            FrequencyBand::Band800MHz => 25.0,
            FrequencyBand::Band900MHz => 20.0,
            FrequencyBand::Band1800MHz => 10.0,
            FrequencyBand::Band2100MHz => 8.0,
            FrequencyBand::Band2600MHz => 5.0,
            FrequencyBand::Band3500MHz => 2.0,
            FrequencyBand::Band28GHz => 0.5,
            FrequencyBand::Band39GHz => 0.3,
        }
    }

    /// Maximum bandwidth available in this band (MHz).
    pub fn max_bandwidth_mhz(&self) -> f64 {
        match self {
            FrequencyBand::Band700MHz => 45.0,
            FrequencyBand::Band800MHz => 30.0,
            FrequencyBand::Band900MHz => 35.0,
            FrequencyBand::Band1800MHz => 75.0,
            FrequencyBand::Band2100MHz => 60.0,
            FrequencyBand::Band2600MHz => 70.0,
            FrequencyBand::Band3500MHz => 100.0,
            FrequencyBand::Band28GHz => 800.0,
            FrequencyBand::Band39GHz => 1000.0,
        }
    }

    /// Base auction price per MHz of spectrum in this band.
    pub fn cost_per_mhz(&self) -> Money {
        match self {
            FrequencyBand::Band700MHz => 500_000,
            FrequencyBand::Band800MHz => 400_000,
            FrequencyBand::Band900MHz => 350_000,
            FrequencyBand::Band1800MHz => 250_000,
            FrequencyBand::Band2100MHz => 200_000,
            FrequencyBand::Band2600MHz => 150_000,
            FrequencyBand::Band3500MHz => 100_000,
            FrequencyBand::Band28GHz => 50_000,
            FrequencyBand::Band39GHz => 30_000,
        }
    }

    /// Human-readable display name.
    pub fn display_name(&self) -> &'static str {
        match self {
            FrequencyBand::Band700MHz => "700 MHz",
            FrequencyBand::Band800MHz => "800 MHz",
            FrequencyBand::Band900MHz => "900 MHz",
            FrequencyBand::Band1800MHz => "1800 MHz",
            FrequencyBand::Band2100MHz => "2100 MHz",
            FrequencyBand::Band2600MHz => "2600 MHz",
            FrequencyBand::Band3500MHz => "3.5 GHz",
            FrequencyBand::Band28GHz => "28 GHz (mmWave)",
            FrequencyBand::Band39GHz => "39 GHz (mmWave)",
        }
    }

    /// Band category for UI color coding: "low", "mid", or "high".
    pub fn category(&self) -> &'static str {
        match self {
            FrequencyBand::Band700MHz
            | FrequencyBand::Band800MHz
            | FrequencyBand::Band900MHz => "low",
            FrequencyBand::Band1800MHz
            | FrequencyBand::Band2100MHz
            | FrequencyBand::Band2600MHz => "mid",
            FrequencyBand::Band3500MHz
            | FrequencyBand::Band28GHz
            | FrequencyBand::Band39GHz => "high",
        }
    }

    /// All frequency band variants for iteration.
    pub fn all() -> &'static [FrequencyBand] {
        &[
            FrequencyBand::Band700MHz,
            FrequencyBand::Band800MHz,
            FrequencyBand::Band900MHz,
            FrequencyBand::Band1800MHz,
            FrequencyBand::Band2100MHz,
            FrequencyBand::Band2600MHz,
            FrequencyBand::Band3500MHz,
            FrequencyBand::Band28GHz,
            FrequencyBand::Band39GHz,
        ]
    }

    /// Parse from string name (e.g. "Band700MHz").
    pub fn from_name(name: &str) -> Option<FrequencyBand> {
        match name {
            "Band700MHz" => Some(FrequencyBand::Band700MHz),
            "Band800MHz" => Some(FrequencyBand::Band800MHz),
            "Band900MHz" => Some(FrequencyBand::Band900MHz),
            "Band1800MHz" => Some(FrequencyBand::Band1800MHz),
            "Band2100MHz" => Some(FrequencyBand::Band2100MHz),
            "Band2600MHz" => Some(FrequencyBand::Band2600MHz),
            "Band3500MHz" => Some(FrequencyBand::Band3500MHz),
            "Band28GHz" => Some(FrequencyBand::Band28GHz),
            "Band39GHz" => Some(FrequencyBand::Band39GHz),
            _ => None,
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
