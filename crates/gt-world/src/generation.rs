use gt_common::types::WorldConfig;

pub struct WorldGenerator {
    config: WorldConfig,
}

impl WorldGenerator {
    pub fn new(config: WorldConfig) -> Self {
        Self { config }
    }

    pub fn config(&self) -> &WorldConfig {
        &self.config
    }
}
