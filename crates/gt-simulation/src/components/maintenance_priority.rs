use serde::{Deserialize, Serialize};

/// Maintenance priority tier for infrastructure nodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MaintenanceTier {
    /// Immediate repair, highest cost priority.
    Critical,
    /// Standard repair queue.
    Standard,
    /// Repaired when resources are available.
    Low,
    /// No maintenance — node degrades freely.
    Deferred,
}

impl Default for MaintenanceTier {
    fn default() -> Self {
        MaintenanceTier::Standard
    }
}

impl MaintenanceTier {
    /// Cost multiplier for maintenance at this tier.
    pub fn cost_multiplier(&self) -> f64 {
        match self {
            MaintenanceTier::Critical => 1.5,
            MaintenanceTier::Standard => 1.0,
            MaintenanceTier::Low => 0.7,
            MaintenanceTier::Deferred => 0.0,
        }
    }

    /// Repair speed multiplier (how quickly repairs complete).
    pub fn speed_multiplier(&self) -> f64 {
        match self {
            MaintenanceTier::Critical => 2.0,
            MaintenanceTier::Standard => 1.0,
            MaintenanceTier::Low => 0.5,
            MaintenanceTier::Deferred => 0.0,
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "Critical" => MaintenanceTier::Critical,
            "Low" => MaintenanceTier::Low,
            "Deferred" => MaintenanceTier::Deferred,
            _ => MaintenanceTier::Standard,
        }
    }
}

/// Per-node maintenance configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaintenancePriority {
    pub tier: MaintenanceTier,
    pub auto_repair: bool,
}

impl Default for MaintenancePriority {
    fn default() -> Self {
        Self {
            tier: MaintenanceTier::Standard,
            auto_repair: true,
        }
    }
}
