use gt_common::types::{EntityId, Money, Tick};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MissionType {
    Espionage,
    Sabotage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mission {
    pub mission_type: MissionType,
    pub target: EntityId,
    pub region: EntityId,
    pub start_tick: Tick,
    pub duration: Tick,
    pub cost: Money,
    pub success_chance: f64,
    pub completed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CovertOps {
    pub security_level: u32,
    pub active_missions: Vec<Mission>,
    pub detection_history: Vec<(Tick, EntityId)>,
}

impl CovertOps {
    pub fn new() -> Self {
        Self {
            security_level: 0,
            active_missions: Vec::new(),
            detection_history: Vec::new(),
        }
    }
}

impl Default for CovertOps {
    fn default() -> Self {
        Self::new()
    }
}
