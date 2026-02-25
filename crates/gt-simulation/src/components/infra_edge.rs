use gt_common::types::{EdgeType, EntityId, Money, NetworkLevel};
use serde::{Deserialize, Serialize};

/// Whether this edge segment is deployed aerially (on poles) or underground (buried/conduit).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum DeploymentMethod {
    /// Mounted on utility poles — cheaper, faster to build, but vulnerable to weather/storms.
    #[default]
    Aerial,
    /// Buried in conduit or direct-buried — more expensive but resilient to weather.
    Underground,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InfraEdge {
    pub edge_type: EdgeType,
    pub source: EntityId,
    pub target: EntityId,
    pub bandwidth: f64,
    pub current_load: f64,
    pub latency_ms: f64,
    pub length_km: f64,
    pub construction_cost: Money,
    pub maintenance_cost: Money,
    pub owner: EntityId,
    pub health: f64,
    /// Ordered waypoints (lon, lat) defining the cable route geometry.
    /// Minimum 2 entries: source position and target position.
    /// Additional intermediate waypoints define the route path (rendered as Catmull-Rom spline).
    #[serde(default)]
    pub waypoints: Vec<(f64, f64)>,
    /// How this edge is physically deployed.
    #[serde(default)]
    pub deployment: DeploymentMethod,
    /// Whether this edge is currently being repaired (standard or emergency).
    #[serde(default)]
    pub repairing: bool,
    /// Ticks remaining until repair completes.
    #[serde(default)]
    pub repair_ticks_left: u32,
    /// Health to restore per tick during active repair.
    #[serde(default)]
    pub repair_health_per_tick: f64,
    /// Last tick this edge was damaged by a disaster (for insurance premium calculation).
    #[serde(default)]
    pub last_damage_tick: Option<u64>,
}

impl InfraEdge {
    pub fn new(
        edge_type: EdgeType,
        source: EntityId,
        target: EntityId,
        length_km: f64,
        owner: EntityId,
    ) -> Self {
        let (bandwidth, latency_per_km, cost_per_km, maint_per_km) = match edge_type {
            // Original 7 (unchanged values)
            EdgeType::Copper => (1_000.0, 0.02, 10_000, 100),
            EdgeType::FiberLocal => (10_000.0, 0.005, 20_000, 120),
            EdgeType::Microwave => (5_000.0, 0.003, 20_000, 150),
            EdgeType::FiberRegional => (50_000.0, 0.005, 40_000, 240),
            EdgeType::FiberNational => (200_000.0, 0.005, 80_000, 480),
            EdgeType::Satellite => (2_000.0, 0.5, 0, 600),
            EdgeType::Submarine => (500_000.0, 0.005, 200_000, 1_200),

            // Era 1: Telegraph
            EdgeType::TelegraphWire => (5.0, 0.1, 500, 10),
            EdgeType::SubseaTelegraphCable => (10.0, 0.2, 5_000, 50),

            // Era 2: Telephone
            EdgeType::CopperTrunkLine => (50.0, 0.03, 3_000, 40),
            EdgeType::LongDistanceCopper => (100.0, 0.025, 8_000, 80),

            // Era 3: Early Digital
            EdgeType::CoaxialCable => (500.0, 0.015, 8_000, 60),
            EdgeType::MicrowaveLink => (2_000.0, 0.003, 15_000, 120),
            EdgeType::EarlySatelliteLink => (500.0, 0.6, 0, 400),

            // Era 4: Internet
            EdgeType::SubseaFiberCable => (1_000_000.0, 0.005, 250_000, 1_500),

            // Era 5: Modern
            EdgeType::FiberMetro => (100_000.0, 0.004, 50_000, 300),
            EdgeType::FiberLongHaul => (400_000.0, 0.004, 100_000, 600),
            EdgeType::DWDM_Backbone => (1_000_000.0, 0.003, 150_000, 900),
            EdgeType::SatelliteLEOLink => (20_000.0, 0.01, 0, 800),
            EdgeType::FeederFiber => (40_000.0, 0.005, 30_000, 180),
            EdgeType::DistributionFiber => (10_000.0, 0.005, 15_000, 90),
            EdgeType::DropCable => (1_000.0, 0.005, 2_000, 20),

            // Era 6: Near Future
            EdgeType::QuantumFiberLink => (500_000.0, 0.001, 500_000, 3_000),
            EdgeType::TerahertzBeam => (100_000.0, 0.001, 100_000, 800),
            EdgeType::LaserInterSatelliteLink => (2_000_000.0, 0.0005, 0, 1_000),
        };

        let is_flat_cost = matches!(
            edge_type,
            EdgeType::Satellite
                | EdgeType::EarlySatelliteLink
                | EdgeType::SatelliteLEOLink
                | EdgeType::LaserInterSatelliteLink
        );

        let construction_cost = if is_flat_cost {
            match edge_type {
                EdgeType::Satellite => 5_000_000,
                EdgeType::EarlySatelliteLink => 2_000_000,
                EdgeType::SatelliteLEOLink => 10_000_000,
                EdgeType::LaserInterSatelliteLink => 50_000_000,
                _ => 5_000_000,
            }
        } else {
            (cost_per_km as f64 * length_km) as Money
        };

        let maintenance_cost = if is_flat_cost {
            match edge_type {
                EdgeType::Satellite => 30_000,
                EdgeType::EarlySatelliteLink => 20_000,
                EdgeType::SatelliteLEOLink => 50_000,
                EdgeType::LaserInterSatelliteLink => 80_000,
                _ => 30_000,
            }
        } else {
            (maint_per_km as f64 * length_km) as Money
        };

        Self {
            edge_type,
            source,
            target,
            bandwidth,
            current_load: 0.0,
            latency_ms: latency_per_km * length_km,
            length_km,
            construction_cost,
            maintenance_cost,
            owner,
            health: 1.0,
            waypoints: Vec::new(),
            deployment: DeploymentMethod::default(),
            repairing: false,
            repair_ticks_left: 0,
            repair_health_per_tick: 0.0,
            last_damage_tick: None,
        }
    }

    pub fn utilization(&self) -> f64 {
        if self.bandwidth == 0.0 {
            0.0
        } else {
            self.current_load / self.bandwidth
        }
    }

    /// Effective bandwidth accounting for health degradation.
    /// Below 0.5 health, bandwidth is proportionally reduced.
    pub fn effective_bandwidth(&self) -> f64 {
        if self.health < 0.3 {
            0.0 // Effectively destroyed
        } else if self.health < 0.5 {
            self.bandwidth * self.health
        } else {
            self.bandwidth
        }
    }

    /// Returns the network level this edge type typically operates at.
    pub fn network_level(&self) -> NetworkLevel {
        match self.edge_type {
            // Local
            EdgeType::FiberLocal
            | EdgeType::Copper
            | EdgeType::TelegraphWire
            | EdgeType::CopperTrunkLine
            | EdgeType::CoaxialCable
            | EdgeType::DropCable
            | EdgeType::DistributionFiber
            | EdgeType::TerahertzBeam => NetworkLevel::Local,

            // Regional
            EdgeType::FiberRegional
            | EdgeType::Microwave
            | EdgeType::LongDistanceCopper
            | EdgeType::MicrowaveLink
            | EdgeType::FiberMetro
            | EdgeType::FeederFiber => NetworkLevel::Regional,

            // National
            EdgeType::FiberNational
            | EdgeType::FiberLongHaul
            | EdgeType::DWDM_Backbone
            | EdgeType::QuantumFiberLink => NetworkLevel::National,

            // Continental
            EdgeType::Satellite
            | EdgeType::EarlySatelliteLink
            | EdgeType::SatelliteLEOLink => NetworkLevel::Continental,

            // Global
            EdgeType::Submarine
            | EdgeType::SubseaTelegraphCable
            | EdgeType::SubseaFiberCable
            | EdgeType::LaserInterSatelliteLink => NetworkLevel::GlobalBackbone,
        }
    }
}
