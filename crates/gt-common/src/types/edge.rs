use serde::{Deserialize, Serialize};

use super::{Era, NetworkTier, NodeType};

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

    // ── Satellite System (dynamic edges) ──
    /// Dynamic downlink from satellite to ground station
    SatelliteDownlink,
    /// In-plane inter-satellite link (auto-formed between constellation neighbors)
    IntraplaneISL,
    /// Cross-plane inter-satellite link (requires research)
    CrossplaneISL,
    /// Semi-permanent GEO satellite to ground station link
    GEO_GroundLink,
    /// Semi-permanent MEO satellite to ground station link
    MEO_GroundLink,
}

impl std::fmt::Display for EdgeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl EdgeType {
    /// All edge type variants for iteration.
    pub const ALL: &'static [EdgeType] = &[
        // Original 7
        EdgeType::FiberLocal,
        EdgeType::FiberRegional,
        EdgeType::FiberNational,
        EdgeType::Copper,
        EdgeType::Microwave,
        EdgeType::Satellite,
        EdgeType::Submarine,
        // Era 1: Telegraph
        EdgeType::TelegraphWire,
        EdgeType::SubseaTelegraphCable,
        // Era 2: Telephone
        EdgeType::CopperTrunkLine,
        EdgeType::LongDistanceCopper,
        // Era 3: Early Digital
        EdgeType::CoaxialCable,
        EdgeType::MicrowaveLink,
        EdgeType::EarlySatelliteLink,
        // Era 4: Internet
        EdgeType::SubseaFiberCable,
        // Era 5: Modern
        EdgeType::FiberMetro,
        EdgeType::FiberLongHaul,
        EdgeType::DWDM_Backbone,
        EdgeType::SatelliteLEOLink,
        EdgeType::FeederFiber,
        EdgeType::DistributionFiber,
        EdgeType::DropCable,
        // Era 6: Near Future
        EdgeType::QuantumFiberLink,
        EdgeType::TerahertzBeam,
        EdgeType::LaserInterSatelliteLink,
        // Satellite System
        EdgeType::SatelliteDownlink,
        EdgeType::IntraplaneISL,
        EdgeType::CrossplaneISL,
        EdgeType::GEO_GroundLink,
        EdgeType::MEO_GroundLink,
    ];

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

            // Satellite System
            EdgeType::SatelliteDownlink => &[
                (NetworkTier::Backbone, NetworkTier::Global),
                (NetworkTier::Global, NetworkTier::Global),
            ],
            EdgeType::IntraplaneISL | EdgeType::CrossplaneISL => &[
                (NetworkTier::Global, NetworkTier::Global),
            ],
            EdgeType::GEO_GroundLink | EdgeType::MEO_GroundLink => &[
                (NetworkTier::Backbone, NetworkTier::Global),
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

            // Satellite System (dynamic, no distance limit)
            EdgeType::SatelliteDownlink => f64::INFINITY,
            EdgeType::IntraplaneISL => f64::INFINITY,
            EdgeType::CrossplaneISL => f64::INFINITY,
            EdgeType::GEO_GroundLink => f64::INFINITY,
            EdgeType::MEO_GroundLink => f64::INFINITY,
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

            // Satellite System
            EdgeType::SatelliteDownlink => "Satellite Downlink",
            EdgeType::IntraplaneISL => "In-Plane ISL",
            EdgeType::CrossplaneISL => "Cross-Plane ISL",
            EdgeType::GEO_GroundLink => "GEO Ground Link",
            EdgeType::MEO_GroundLink => "MEO Ground Link",
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

            // Satellite System
            EdgeType::SatelliteDownlink => 2.0,
            EdgeType::IntraplaneISL => 1.5,
            EdgeType::CrossplaneISL => 1.5,
            EdgeType::GEO_GroundLink => 2.5,
            EdgeType::MEO_GroundLink => 2.0,
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

            // Satellite System
            EdgeType::GEO_GroundLink => Era::Internet,
            EdgeType::SatelliteDownlink
            | EdgeType::MEO_GroundLink => Era::Modern,
            EdgeType::IntraplaneISL
            | EdgeType::CrossplaneISL => Era::Modern,
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

            // Satellite System (dynamic, no per-km cost)
            EdgeType::SatelliteDownlink => 0,
            EdgeType::IntraplaneISL => 0,
            EdgeType::CrossplaneISL => 0,
            EdgeType::GEO_GroundLink => 0,
            EdgeType::MEO_GroundLink => 0,
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

            // Satellite System
            EdgeType::SatelliteDownlink => 50_000,
            EdgeType::IntraplaneISL => 100_000,
            EdgeType::CrossplaneISL => 80_000,
            EdgeType::GEO_GroundLink => 10_000,
            EdgeType::MEO_GroundLink => 30_000,
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
            | EdgeType::LaserInterSatelliteLink
            | EdgeType::SatelliteDownlink
            | EdgeType::IntraplaneISL
            | EdgeType::CrossplaneISL
            | EdgeType::GEO_GroundLink
            | EdgeType::MEO_GroundLink => 0,

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
