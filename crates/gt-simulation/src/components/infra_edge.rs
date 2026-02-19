use gt_common::types::{EdgeType, EntityId, Money, NetworkLevel};
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
    pub health: f64,
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
            EdgeType::FiberLocal => (10_000.0, 0.005, 20_000, 200),
            EdgeType::FiberRegional => (50_000.0, 0.005, 40_000, 400),
            EdgeType::FiberNational => (200_000.0, 0.005, 80_000, 800),
            EdgeType::Copper => (1_000.0, 0.02, 10_000, 200),
            EdgeType::Microwave => (5_000.0, 0.003, 20_000, 300),
            EdgeType::Satellite => (2_000.0, 0.5, 0, 1_000),
            EdgeType::Submarine => (500_000.0, 0.005, 200_000, 2_000),
        };

        let construction_cost = if edge_type == EdgeType::Satellite {
            5_000_000
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
            health: 1.0,
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
            EdgeType::FiberLocal | EdgeType::Copper => NetworkLevel::Local,
            EdgeType::FiberRegional | EdgeType::Microwave => NetworkLevel::Regional,
            EdgeType::FiberNational => NetworkLevel::National,
            EdgeType::Satellite => NetworkLevel::Continental,
            EdgeType::Submarine => NetworkLevel::GlobalBackbone,
        }
    }
}
