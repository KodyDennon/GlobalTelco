use gt_common::types::{CreditRating, EntityId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Corporation {
    pub name: String,
    pub is_player: bool,
    pub credit_rating: CreditRating,
    pub reputation: f64,
    pub subsidiaries: Vec<EntityId>,
}

impl Corporation {
    pub fn new(name: impl Into<String>, is_player: bool) -> Self {
        Self {
            name: name.into(),
            is_player,
            credit_rating: CreditRating::BBB,
            reputation: 50.0,
            subsidiaries: Vec::new(),
        }
    }
}
