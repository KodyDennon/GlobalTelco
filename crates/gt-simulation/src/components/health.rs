use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Health {
    pub condition: f64,
    pub maintenance_cost_per_tick: gt_common::types::Money,
}

impl Health {
    pub fn new() -> Self {
        Self {
            condition: 1.0,
            maintenance_cost_per_tick: 0,
        }
    }

    pub fn degrade(&mut self, amount: f64) {
        self.condition = (self.condition - amount).max(0.0);
    }
}

impl Default for Health {
    fn default() -> Self {
        Self::new()
    }
}
