mod bridge_impl;
mod queries;
mod typed_arrays;

use gt_common::types::WorldConfig;
use gt_simulation::world::GameWorld;
use wasm_bindgen::prelude::*;

#[cfg(target_family = "wasm")]
#[global_allocator]
static ALLOCATOR: talc::TalckWasm = unsafe { talc::TalckWasm::new_global() };

#[wasm_bindgen(start)]
pub fn init_panic_hook() {
    console_error_panic_hook::set_once();
}

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

    pub fn new_game(config_json: &str) -> Result<WasmBridge, JsValue> {
        let config: WorldConfig = serde_json::from_str(config_json)
            .map_err(|e| JsValue::from_str(&format!("Invalid config: {}", e)))?;
        Ok(Self {
            world: GameWorld::new(config),
        })
    }

    pub fn tick(&mut self) {
        self.world.tick();
    }

    pub fn current_tick(&self) -> u64 {
        self.world.current_tick()
    }

    pub fn process_command(&mut self, command_json: &str) -> Result<String, JsValue> {
        let cmd: gt_common::commands::Command = serde_json::from_str(command_json)
            .map_err(|e| JsValue::from_str(&format!("Invalid command: {}", e)))?;
        self.world.process_command(cmd);
        // Immediately drain any notifications so the frontend gets instant feedback
        // (e.g., "Insufficient funds") even when the game is paused.
        let events = self.world.event_queue.drain();
        if events.is_empty() {
            Ok(String::new())
        } else {
            let notifications: Vec<serde_json::Value> = events
                .iter()
                .map(|(tick, event)| {
                    serde_json::json!({
                        "tick": tick,
                        "event": serde_json::to_value(event).unwrap_or(serde_json::Value::Null),
                    })
                })
                .collect();
            Ok(serde_json::to_string(&notifications).unwrap_or_default())
        }
    }

    /// Apply a batch of delta operations to the local world state.
    /// Used in multiplayer to incrementally update WASM state from
    /// CommandBroadcast messages without requiring a full snapshot reload.
    pub fn apply_batch(&mut self, ops_json: &str) -> Result<(), JsValue> {
        let ops: Vec<gt_common::protocol::DeltaOp> = serde_json::from_str(ops_json)
            .map_err(|e| JsValue::from_str(&format!("Invalid delta ops: {}", e)))?;
        self.world.apply_delta(&ops);
        Ok(())
    }
}
