use tauri::State;

use crate::sim_state::SimState;

#[tauri::command]
pub async fn sim_new_game(state: State<'_, SimState>, config_json: String) -> Result<(), String> {
    state.sim.new_game(config_json).await
}

#[tauri::command]
pub async fn sim_load_game(state: State<'_, SimState>, data: String) -> Result<(), String> {
    state.sim.load_game(data).await
}

#[tauri::command]
pub async fn sim_tick(state: State<'_, SimState>) -> Result<(), String> {
    state.sim.tick().await
}

#[tauri::command]
pub async fn sim_current_tick(state: State<'_, SimState>) -> Result<u64, String> {
    let snap = state
        .sim
        .render_snapshot()
        .read()
        .map_err(|_| "Lock poisoned".to_string())?;
    Ok(snap.tick)
}

#[tauri::command]
pub async fn sim_process_command(
    state: State<'_, SimState>,
    command_json: String,
) -> Result<String, String> {
    state.sim.process_command(command_json).await
}

#[tauri::command]
pub async fn sim_apply_batch(
    state: State<'_, SimState>,
    ops_json: String,
) -> Result<(), String> {
    state.sim.apply_batch(ops_json).await
}

#[tauri::command]
pub async fn sim_save_game(state: State<'_, SimState>) -> Result<String, String> {
    state.sim.save_game().await
}
