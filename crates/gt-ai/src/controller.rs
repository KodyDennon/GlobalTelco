use gt_common::types::{AIArchetype, AIStrategy, EntityId};

pub struct AiController {
    pub corporation_id: EntityId,
    pub archetype: AIArchetype,
    pub strategy: AIStrategy,
}

impl AiController {
    pub fn new(corporation_id: EntityId, archetype: AIArchetype) -> Self {
        Self {
            corporation_id,
            archetype,
            strategy: AIStrategy::Expand,
        }
    }
}
