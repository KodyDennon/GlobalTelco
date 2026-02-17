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
}

impl InfraNode {
    pub fn new(node_type: NodeType, cell_index: usize, owner: EntityId) -> Self {
        let (throughput, latency, cost, maintenance, level) = match node_type {
            NodeType::CentralOffice => (1000.0, 5.0, 500_000, 5_000, NetworkLevel::Local),
            NodeType::ExchangePoint => (5000.0, 2.0, 2_000_000, 20_000, NetworkLevel::Regional),
            NodeType::CellTower => (500.0, 10.0, 200_000, 3_000, NetworkLevel::Local),
            NodeType::DataCenter => (50000.0, 1.0, 10_000_000, 100_000, NetworkLevel::National),
            NodeType::SatelliteGround => {
                (2000.0, 50.0, 5_000_000, 50_000, NetworkLevel::Continental)
            }
            NodeType::SubmarineLanding => (
                100000.0,
                3.0,
                20_000_000,
                200_000,
                NetworkLevel::GlobalBackbone,
            ),
            NodeType::WirelessRelay => (300.0, 15.0, 100_000, 2_000, NetworkLevel::Local),
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
            NodeType::CentralOffice => 20,
            NodeType::ExchangePoint => 15,
            NodeType::CellTower => 5,
            NodeType::DataCenter => 50,
            NodeType::SatelliteGround => 30,
            NodeType::SubmarineLanding => 25,
            NodeType::WirelessRelay => 3,
        }
    }
}
