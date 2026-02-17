use wasm_bindgen::prelude::*;
use gt_simulation::world::GameWorld;
use gt_common::types::WorldConfig;

#[wasm_bindgen]
pub struct WasmBridge {
    world: GameWorld,
}

impl Default for WasmBridge {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
impl WasmBridge {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            world: GameWorld::new(WorldConfig::default()),
        }
    }

    pub fn tick(&mut self) {
        self.world.tick();
    }

    pub fn current_tick(&self) -> u64 {
        self.world.current_tick()
    }
}
