use gt_common::types::{EdgeType, EntityId, Money};
use serde::{Deserialize, Serialize};

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
            EdgeType::FiberOptic => (100_000.0, 0.005, 50_000, 500),
            EdgeType::Copper => (1_000.0, 0.02, 10_000, 200),
            EdgeType::Microwave => (5_000.0, 0.003, 20_000, 300),
            EdgeType::Satellite => (2_000.0, 0.5, 0, 1_000), // No per-km cost, flat cost
            EdgeType::Submarine => (500_000.0, 0.005, 200_000, 2_000),
        };

        let construction_cost = if edge_type == EdgeType::Satellite {
            5_000_000 // Flat cost for satellite links
        } else {
            (cost_per_km as f64 * length_km) as Money
        };

        let maintenance_cost = if edge_type == EdgeType::Satellite {
            50_000
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
        }
    }

    pub fn utilization(&self) -> f64 {
        if self.bandwidth == 0.0 {
            0.0
        } else {
            self.current_load / self.bandwidth
        }
    }
}
