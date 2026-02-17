use gt_common::types::Tick;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Construction {
    pub started_at: Tick,
    pub completes_at: Tick,
    pub progress: f64,
}

impl Construction {
    pub fn new(started_at: Tick, duration: Tick) -> Self {
        Self {
            started_at,
            completes_at: started_at + duration,
            progress: 0.0,
        }
    }

    pub fn is_complete(&self, current_tick: Tick) -> bool {
        current_tick >= self.completes_at
    }
}
