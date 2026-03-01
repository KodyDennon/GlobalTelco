//! Tauri-native simulation bridge for desktop builds.
//!
//! Wraps gt-simulation's `GameWorld` and implements `gt_bridge::BridgeQuery`
//! for native Rust execution -- no WASM overhead. The `SimThread` module
//! provides a dedicated background thread that owns the `GameWorld`.
//!
//! Frontend calls Tauri `invoke()` instead of calling WASM when running
//! in the desktop environment.

pub mod binary;
mod bridge_impl;
mod queries;
pub mod sim_thread;

use std::sync::Mutex;

use gt_common::types::WorldConfig;
use gt_simulation::world::GameWorld;

/// Native simulation bridge for desktop.
/// Holds a `GameWorld` behind a `Mutex` so it can be shared via Tauri state.
pub struct TauriBridge {
    pub world: Mutex<GameWorld>,
}

impl TauriBridge {
    pub fn new(config: WorldConfig) -> Self {
        Self {
            world: Mutex::new(GameWorld::new(config)),
        }
    }

    pub fn from_save(data: &str) -> Result<Self, String> {
        let world = GameWorld::load_game(data).map_err(|e| format!("Load failed: {e}"))?;
        Ok(Self {
            world: Mutex::new(world),
        })
    }
}

/// Create a new game world with the given config JSON.
pub fn cmd_new_game(config_json: &str) -> Result<TauriBridge, String> {
    let config: WorldConfig =
        serde_json::from_str(config_json).map_err(|e| format!("Invalid config: {e}"))?;
    Ok(TauriBridge::new(config))
}
