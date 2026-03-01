use serde::{Deserialize, Serialize};

use super::{Era, NetworkTier};

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

    // ── Satellite System ──
    /// LEO satellite in orbit (~160-2000km)
    LEO_Satellite,
    /// MEO satellite in orbit (~2000-35786km)
    MEO_Satellite,
    /// GEO satellite in geostationary orbit (~35786km)
    GEO_Satellite,
    /// HEO satellite in highly elliptical orbit
    HEO_Satellite,
    /// Ground station for LEO satellite constellations (Era 5)
    LEO_GroundStation,
    /// Ground station for MEO satellite constellations (Era 4)
    MEO_GroundStation,
    /// Factory that produces satellites
    SatelliteFactory,
    /// Factory that produces customer terminals
    TerminalFactory,
    /// Regional warehouse for terminal distribution
    SatelliteWarehouse,
    /// Launch pad for rocket launches
    LaunchPad,
}

impl NodeType {
    /// All node type variants for iteration.
    pub const ALL: &'static [NodeType] = &[
        // Original 8
        NodeType::CentralOffice,
        NodeType::ExchangePoint,
        NodeType::CellTower,
        NodeType::DataCenter,
        NodeType::SatelliteGround,
        NodeType::SubmarineLanding,
        NodeType::WirelessRelay,
        NodeType::BackboneRouter,
        // Era 1: Telegraph
        NodeType::TelegraphOffice,
        NodeType::TelegraphRelay,
        NodeType::CableHut,
        // Era 2: Telephone
        NodeType::ManualExchange,
        NodeType::AutomaticExchange,
        NodeType::TelephonePole,
        NodeType::LongDistanceRelay,
        // Era 3: Early Digital
        NodeType::DigitalSwitch,
        NodeType::MicrowaveTower,
        NodeType::CoaxHub,
        NodeType::EarlyDataCenter,
        NodeType::SatelliteGroundStation,
        // Era 4: Internet
        NodeType::FiberPOP,
        NodeType::InternetExchangePoint,
        NodeType::SubseaLandingStation,
        NodeType::ColocationFacility,
        NodeType::ISPGateway,
        // Era 5: Modern
        NodeType::MacroCell,
        NodeType::SmallCell,
        NodeType::EdgeDataCenter,
        NodeType::HyperscaleDataCenter,
        NodeType::CloudOnRamp,
        NodeType::ContentDeliveryNode,
        NodeType::FiberSplicePoint,
        NodeType::DWDM_Terminal,
        NodeType::FiberDistributionHub,
        NodeType::NetworkAccessPoint,
        // Era 6: Near Future
        NodeType::LEO_SatelliteGateway,
        NodeType::QuantumRepeater,
        NodeType::MeshDroneRelay,
        NodeType::UnderwaterDataCenter,
        NodeType::NeuromorphicEdgeNode,
        NodeType::TerahertzRelay,
        // Satellite System
        NodeType::LEO_Satellite,
        NodeType::MEO_Satellite,
        NodeType::GEO_Satellite,
        NodeType::HEO_Satellite,
        NodeType::LEO_GroundStation,
        NodeType::MEO_GroundStation,
        NodeType::SatelliteFactory,
        NodeType::TerminalFactory,
        NodeType::SatelliteWarehouse,
        NodeType::LaunchPad,
    ];

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

            // Satellite System
            NodeType::LEO_Satellite => "LEO Satellite",
            NodeType::MEO_Satellite => "MEO Satellite",
            NodeType::GEO_Satellite => "GEO Satellite",
            NodeType::HEO_Satellite => "HEO Satellite",
            NodeType::LEO_GroundStation => "LEO Ground Station",
            NodeType::MEO_GroundStation => "MEO Ground Station",
            NodeType::SatelliteFactory => "Satellite Factory",
            NodeType::TerminalFactory => "Terminal Factory",
            NodeType::SatelliteWarehouse => "Satellite Warehouse",
            NodeType::LaunchPad => "Launch Pad",
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
            | NodeType::UnderwaterDataCenter
            | NodeType::LEO_Satellite
            | NodeType::MEO_Satellite
            | NodeType::GEO_Satellite
            | NodeType::HEO_Satellite => NetworkTier::Global,

            // Satellite ground infrastructure
            NodeType::LEO_GroundStation
            | NodeType::MEO_GroundStation => NetworkTier::Backbone,

            // Satellite manufacturing & logistics
            NodeType::SatelliteFactory => NetworkTier::Core,
            NodeType::TerminalFactory
            | NodeType::SatelliteWarehouse => NetworkTier::Aggregation,
            NodeType::LaunchPad => NetworkTier::Backbone,
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

            // Satellite System — orbital coverage (dynamic, but base values)
            NodeType::LEO_Satellite => 1000.0,     // ~1000km footprint radius at 550km alt
            NodeType::MEO_Satellite => 3000.0,     // ~3000km footprint at 8000km alt
            NodeType::GEO_Satellite => 5000.0,     // ~5000km footprint at 35786km
            NodeType::HEO_Satellite => 2000.0,     // variable, average coverage
            NodeType::LEO_GroundStation => 50.0,   // ground station facility coverage
            NodeType::MEO_GroundStation => 50.0,
            NodeType::SatelliteFactory => 0.0,     // no coverage
            NodeType::TerminalFactory => 0.0,
            NodeType::SatelliteWarehouse => 0.0,
            NodeType::LaunchPad => 0.0,
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
                | NodeType::LEO_Satellite
                | NodeType::MEO_Satellite
                | NodeType::GEO_Satellite
                | NodeType::HEO_Satellite
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

            // Satellite System
            NodeType::LEO_Satellite => 0.7,
            NodeType::MEO_Satellite => 0.6,
            NodeType::GEO_Satellite => 0.5,
            NodeType::HEO_Satellite => 0.6,
            NodeType::LEO_GroundStation => 0.1,   // mostly backhaul
            NodeType::MEO_GroundStation => 0.1,
            NodeType::SatelliteFactory => 0.0,
            NodeType::TerminalFactory => 0.0,
            NodeType::SatelliteWarehouse => 0.0,
            NodeType::LaunchPad => 0.0,
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

            // Satellite System
            NodeType::LEO_Satellite => 5_000_000,         // per satellite (manufactured, then launched)
            NodeType::MEO_Satellite => 15_000_000,
            NodeType::GEO_Satellite => 50_000_000,
            NodeType::HEO_Satellite => 25_000_000,
            NodeType::LEO_GroundStation => 10_000_000,
            NodeType::MEO_GroundStation => 15_000_000,
            NodeType::SatelliteFactory => 100_000_000,
            NodeType::TerminalFactory => 20_000_000,
            NodeType::SatelliteWarehouse => 5_000_000,
            NodeType::LaunchPad => 200_000_000,
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

            // Satellite System
            NodeType::LEO_Satellite => 2.0,
            NodeType::MEO_Satellite => 1.5,
            NodeType::GEO_Satellite => 1.0,
            NodeType::HEO_Satellite => 1.5,
            NodeType::LEO_GroundStation => 0.5,
            NodeType::MEO_GroundStation => 0.5,
            NodeType::SatelliteFactory => 0.0,
            NodeType::TerminalFactory => 0.0,
            NodeType::SatelliteWarehouse => 0.0,
            NodeType::LaunchPad => 0.0,
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

            // Satellite System (Era 4-5, research-gated)
            NodeType::MEO_GroundStation
            | NodeType::SatelliteFactory
            | NodeType::LaunchPad => Era::Internet,

            NodeType::LEO_GroundStation
            | NodeType::TerminalFactory
            | NodeType::SatelliteWarehouse
            | NodeType::LEO_Satellite
            | NodeType::MEO_Satellite
            | NodeType::HEO_Satellite => Era::Modern,

            NodeType::GEO_Satellite => Era::Internet,
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

            // Satellite System
            NodeType::LEO_Satellite => 20_000,
            NodeType::MEO_Satellite => 10_000,
            NodeType::GEO_Satellite => 5_000,
            NodeType::HEO_Satellite => 8_000,
            NodeType::LEO_GroundStation => 200_000,
            NodeType::MEO_GroundStation => 100_000,
            NodeType::SatelliteFactory => 0,
            NodeType::TerminalFactory => 0,
            NodeType::SatelliteWarehouse => 0,
            NodeType::LaunchPad => 0,
        }
    }

    /// Whether this node type is a satellite in orbit.
    pub fn is_satellite(&self) -> bool {
        matches!(
            self,
            NodeType::LEO_Satellite
                | NodeType::MEO_Satellite
                | NodeType::GEO_Satellite
                | NodeType::HEO_Satellite
        )
    }

    /// Whether this node type is a satellite ground station.
    pub fn is_satellite_ground_station(&self) -> bool {
        matches!(
            self,
            NodeType::LEO_GroundStation
                | NodeType::MEO_GroundStation
                | NodeType::SatelliteGround
                | NodeType::SatelliteGroundStation
                | NodeType::LEO_SatelliteGateway
        )
    }
}
