use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capacity {
    pub max_throughput: f64,
    pub current_load: f64,
}

impl Capacity {
    pub fn new(max_throughput: f64) -> Self {
        Self { max_throughput, current_load: 0.0 }
    }

    pub fn utilization(&self) -> f64 {
        if self.max_throughput == 0.0 { 0.0 } else { self.current_load / self.max_throughput }
    }
}
