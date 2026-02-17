use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub x: f64,
    pub y: f64,
    pub region_id: Option<gt_common::types::EntityId>,
}

impl Position {
    pub fn new(x: f64, y: f64) -> Self {
        Self {
            x,
            y,
            region_id: None,
        }
    }
}
