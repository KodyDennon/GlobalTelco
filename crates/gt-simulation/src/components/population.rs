use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Population {
    pub count: u64,
    pub growth_rate: f64,
    pub income_level: f64,
}

impl Population {
    pub fn new(count: u64) -> Self {
        Self { count, growth_rate: 0.01, income_level: 1.0 }
    }
}
