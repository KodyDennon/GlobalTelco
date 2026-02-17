use gt_common::types::{EntityId, Money};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ResearchCategory {
    OpticalNetworks,
    Wireless5G,
    Satellite,
    DataCenter,
    NetworkResilience,
    OperationalEfficiency,
}

impl ResearchCategory {
    pub fn base_cost(&self) -> Money {
        match self {
            ResearchCategory::OpticalNetworks => 5_000_000,
            ResearchCategory::Wireless5G => 8_000_000,
            ResearchCategory::Satellite => 10_000_000,
            ResearchCategory::DataCenter => 6_000_000,
            ResearchCategory::NetworkResilience => 3_000_000,
            ResearchCategory::OperationalEfficiency => 2_000_000,
        }
    }

    pub fn throughput_bonus(&self) -> f64 {
        match self {
            ResearchCategory::OpticalNetworks => 0.25,
            ResearchCategory::Wireless5G => 0.20,
            ResearchCategory::Satellite => 0.15,
            ResearchCategory::DataCenter => 0.30,
            ResearchCategory::NetworkResilience => 0.05,
            ResearchCategory::OperationalEfficiency => 0.0,
        }
    }

    pub fn cost_reduction(&self) -> f64 {
        match self {
            ResearchCategory::OperationalEfficiency => 0.15,
            ResearchCategory::NetworkResilience => 0.05,
            _ => 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TechResearch {
    pub category: ResearchCategory,
    pub name: String,
    pub progress: f64,
    pub total_cost: Money,
    pub researcher: Option<EntityId>,
    pub completed: bool,
}

impl TechResearch {
    pub fn new(category: ResearchCategory, name: impl Into<String>) -> Self {
        Self {
            total_cost: category.base_cost(),
            category,
            name: name.into(),
            progress: 0.0,
            researcher: None,
            completed: false,
        }
    }

    pub fn advance(&mut self, investment: Money) -> bool {
        if self.completed {
            return true;
        }
        self.progress += investment as f64;
        if self.progress >= self.total_cost as f64 {
            self.completed = true;
        }
        self.completed
    }

    pub fn progress_pct(&self) -> f64 {
        if self.total_cost == 0 {
            1.0
        } else {
            (self.progress / self.total_cost as f64).clamp(0.0, 1.0)
        }
    }
}
