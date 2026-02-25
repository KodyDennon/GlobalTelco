use gt_common::types::{EntityId, Money, NetworkLevel, NodeType};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InfraNode {
    pub node_type: NodeType,
    pub network_level: NetworkLevel,
    pub max_throughput: f64,
    pub current_load: f64,
    pub latency_ms: f64,
    pub reliability: f64,
    pub construction_cost: Money,
    pub maintenance_cost: Money,
    pub cell_index: usize,
    pub owner: EntityId,
    pub insured: bool,
    pub insurance_premium: Money,
    /// Spectrum band assigned to this wireless node (e.g. "Band700MHz", "Band3500MHz").
    /// Wireless nodes without an assigned band operate at 50% throughput.
    #[serde(default)]
    pub assigned_band: Option<String>,
    /// Whether this NAP has a validated FTTH chain (CO -> FeederFiber -> FDH -> DistributionFiber -> NAP).
    /// Set by the FTTH system each tick. Only meaningful for NetworkAccessPoint nodes.
    #[serde(default)]
    pub active_ftth: bool,
    /// Whether this node is currently being repaired (standard or emergency).
    #[serde(default)]
    pub repairing: bool,
    /// Ticks remaining until repair completes.
    #[serde(default)]
    pub repair_ticks_left: u32,
    /// Health to restore per tick during active repair.
    #[serde(default)]
    pub repair_health_per_tick: f64,
}

impl InfraNode {
    pub fn new(node_type: NodeType, cell_index: usize, owner: EntityId) -> Self {
        let (throughput, latency, cost, maintenance, level) = match node_type {
            // Original 8 (unchanged values)
            NodeType::CellTower => (500.0, 10.0, 200_000, 1_500, NetworkLevel::Local),
            NodeType::WirelessRelay => (300.0, 15.0, 100_000, 1_000, NetworkLevel::Local),
            NodeType::CentralOffice => (1000.0, 5.0, 500_000, 3_000, NetworkLevel::Local),
            NodeType::ExchangePoint => (5000.0, 2.0, 2_000_000, 12_000, NetworkLevel::Regional),
            NodeType::DataCenter => (50000.0, 1.0, 10_000_000, 60_000, NetworkLevel::National),
            NodeType::BackboneRouter => (20000.0, 2.0, 3_000_000, 18_000, NetworkLevel::National),
            NodeType::SatelliteGround => {
                (2000.0, 50.0, 5_000_000, 30_000, NetworkLevel::Continental)
            }
            NodeType::SubmarineLanding => (
                100000.0,
                3.0,
                20_000_000,
                120_000,
                NetworkLevel::GlobalBackbone,
            ),

            // Era 1: Telegraph — very low throughput, high latency
            NodeType::TelegraphOffice => (5.0, 500.0, 5_000, 50, NetworkLevel::Local),
            NodeType::TelegraphRelay => (3.0, 200.0, 2_000, 30, NetworkLevel::Local),
            NodeType::CableHut => (10.0, 800.0, 15_000, 100, NetworkLevel::GlobalBackbone),

            // Era 2: Telephone — low throughput
            NodeType::ManualExchange => (20.0, 50.0, 20_000, 200, NetworkLevel::Local),
            NodeType::AutomaticExchange => (50.0, 30.0, 50_000, 400, NetworkLevel::Regional),
            NodeType::TelephonePole => (10.0, 20.0, 1_000, 15, NetworkLevel::Local),
            NodeType::LongDistanceRelay => (30.0, 40.0, 30_000, 250, NetworkLevel::Regional),

            // Era 3: Early Digital
            NodeType::DigitalSwitch => (500.0, 5.0, 200_000, 2_000, NetworkLevel::Regional),
            NodeType::MicrowaveTower => (200.0, 8.0, 150_000, 1_200, NetworkLevel::Regional),
            NodeType::CoaxHub => (300.0, 10.0, 80_000, 600, NetworkLevel::Local),
            NodeType::EarlyDataCenter => (5000.0, 3.0, 2_000_000, 15_000, NetworkLevel::National),
            NodeType::SatelliteGroundStation => {
                (1000.0, 80.0, 3_000_000, 20_000, NetworkLevel::Continental)
            }

            // Era 4: Internet
            NodeType::FiberPOP => (10000.0, 2.0, 800_000, 5_000, NetworkLevel::Regional),
            NodeType::InternetExchangePoint => {
                (50000.0, 1.0, 3_000_000, 20_000, NetworkLevel::National)
            }
            NodeType::SubseaLandingStation => (
                200000.0,
                3.0,
                25_000_000,
                150_000,
                NetworkLevel::GlobalBackbone,
            ),
            NodeType::ColocationFacility => {
                (30000.0, 2.0, 8_000_000, 50_000, NetworkLevel::National)
            }
            NodeType::ISPGateway => (2000.0, 5.0, 400_000, 3_000, NetworkLevel::Local),

            // Era 5: Modern
            NodeType::MacroCell => (1000.0, 8.0, 300_000, 2_000, NetworkLevel::Local),
            NodeType::SmallCell => (200.0, 5.0, 50_000, 400, NetworkLevel::Local),
            NodeType::EdgeDataCenter => {
                (20000.0, 1.0, 5_000_000, 30_000, NetworkLevel::Regional)
            }
            NodeType::HyperscaleDataCenter => (
                1_000_000.0,
                0.5,
                500_000_000,
                3_000_000,
                NetworkLevel::National,
            ),
            NodeType::CloudOnRamp => (50000.0, 1.0, 2_000_000, 12_000, NetworkLevel::National),
            NodeType::ContentDeliveryNode => {
                (30000.0, 1.0, 1_000_000, 6_000, NetworkLevel::Regional)
            }
            NodeType::FiberSplicePoint => (0.0, 0.0, 10_000, 50, NetworkLevel::Local),
            NodeType::DWDM_Terminal => {
                (100000.0, 0.5, 4_000_000, 25_000, NetworkLevel::National)
            }
            NodeType::FiberDistributionHub => (1000.0, 2.0, 25_000, 200, NetworkLevel::Local),
            NodeType::NetworkAccessPoint => (100.0, 3.0, 5_000, 50, NetworkLevel::Local),

            // Era 6: Near Future
            NodeType::LEO_SatelliteGateway => (
                500000.0,
                5.0,
                50_000_000,
                300_000,
                NetworkLevel::GlobalBackbone,
            ),
            NodeType::QuantumRepeater => {
                (10000.0, 0.1, 20_000_000, 120_000, NetworkLevel::Regional)
            }
            NodeType::MeshDroneRelay => (500.0, 12.0, 500_000, 5_000, NetworkLevel::Local),
            NodeType::UnderwaterDataCenter => (
                2_000_000.0,
                1.0,
                200_000_000,
                500_000,
                NetworkLevel::GlobalBackbone,
            ),
            NodeType::NeuromorphicEdgeNode => {
                (100000.0, 0.5, 15_000_000, 80_000, NetworkLevel::National)
            }
            NodeType::TerahertzRelay => (50000.0, 2.0, 3_000_000, 20_000, NetworkLevel::Local),
        };

        Self {
            node_type,
            network_level: level,
            max_throughput: throughput,
            current_load: 0.0,
            latency_ms: latency,
            reliability: 1.0,
            construction_cost: cost,
            maintenance_cost: maintenance,
            cell_index,
            owner,
            insured: false,
            insurance_premium: cost / 50, // 2% of construction cost per tick cycle
            assigned_band: None,
            active_ftth: false,
            repairing: false,
            repair_ticks_left: 0,
            repair_health_per_tick: 0.0,
        }
    }

    pub fn utilization(&self) -> f64 {
        if self.max_throughput == 0.0 {
            0.0
        } else {
            self.current_load / self.max_throughput
        }
    }

    /// Create a node with terrain multipliers applied to cost, maintenance, and reliability.
    pub fn new_on_terrain(
        node_type: NodeType,
        cell_index: usize,
        owner: EntityId,
        terrain: gt_common::types::TerrainType,
    ) -> Self {
        let mut node = Self::new(node_type, cell_index, owner);
        node.construction_cost =
            (node.construction_cost as f64 * terrain.construction_cost_multiplier()) as Money;
        node.maintenance_cost =
            (node.maintenance_cost as f64 * terrain.maintenance_cost_multiplier()) as Money;
        node.reliability *= terrain.reliability_modifier();
        node
    }

    pub fn jobs_created(&self) -> u32 {
        match self.node_type {
            // Original 8 (unchanged)
            NodeType::CentralOffice => 20,
            NodeType::ExchangePoint => 15,
            NodeType::CellTower => 5,
            NodeType::DataCenter => 50,
            NodeType::SatelliteGround => 30,
            NodeType::SubmarineLanding => 25,
            NodeType::WirelessRelay => 3,
            NodeType::BackboneRouter => 10,

            // Era 1: Telegraph
            NodeType::TelegraphOffice => 8,
            NodeType::TelegraphRelay => 2,
            NodeType::CableHut => 5,

            // Era 2: Telephone
            NodeType::ManualExchange => 15,
            NodeType::AutomaticExchange => 8,
            NodeType::TelephonePole => 1,
            NodeType::LongDistanceRelay => 4,

            // Era 3: Early Digital
            NodeType::DigitalSwitch => 12,
            NodeType::MicrowaveTower => 4,
            NodeType::CoaxHub => 6,
            NodeType::EarlyDataCenter => 40,
            NodeType::SatelliteGroundStation => 25,

            // Era 4: Internet
            NodeType::FiberPOP => 10,
            NodeType::InternetExchangePoint => 20,
            NodeType::SubseaLandingStation => 30,
            NodeType::ColocationFacility => 35,
            NodeType::ISPGateway => 5,

            // Era 5: Modern
            NodeType::MacroCell => 3,
            NodeType::SmallCell => 1,
            NodeType::EdgeDataCenter => 15,
            NodeType::HyperscaleDataCenter => 500,
            NodeType::CloudOnRamp => 8,
            NodeType::ContentDeliveryNode => 5,
            NodeType::FiberSplicePoint => 0,
            NodeType::DWDM_Terminal => 10,
            NodeType::FiberDistributionHub => 1,
            NodeType::NetworkAccessPoint => 0,

            // Era 6: Near Future
            NodeType::LEO_SatelliteGateway => 50,
            NodeType::QuantumRepeater => 20,
            NodeType::MeshDroneRelay => 1,
            NodeType::UnderwaterDataCenter => 100,
            NodeType::NeuromorphicEdgeNode => 10,
            NodeType::TerahertzRelay => 2,
        }
    }
}
