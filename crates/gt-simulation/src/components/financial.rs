use gt_common::types::Money;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Financial {
    pub cash: Money,
    pub revenue_per_tick: Money,
    pub cost_per_tick: Money,
    pub debt: Money,
}

impl Financial {
    pub fn net_income(&self) -> Money {
        self.revenue_per_tick - self.cost_per_tick
    }
}
