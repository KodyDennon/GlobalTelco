use tauri::State;

use crate::sim_state::SimState;

/// Return pre-packed infrastructure node binary data.
/// Tauri delivers `Vec<u8>` as `ArrayBuffer` in JavaScript.
#[tauri::command]
pub async fn sim_get_infra_binary(state: State<'_, SimState>) -> Result<Vec<u8>, String> {
    let snap = state
        .sim
        .render_snapshot()
        .read()
        .map_err(|_| "Lock poisoned".to_string())?;
    Ok(snap.infra_bytes.clone())
}

/// Return pre-packed edge binary data.
#[tauri::command]
pub async fn sim_get_edges_binary(state: State<'_, SimState>) -> Result<Vec<u8>, String> {
    let snap = state
        .sim
        .render_snapshot()
        .read()
        .map_err(|_| "Lock poisoned".to_string())?;
    Ok(snap.edge_bytes.clone())
}

/// Return pre-packed satellite binary data.
#[tauri::command]
pub async fn sim_get_satellites_binary(state: State<'_, SimState>) -> Result<Vec<u8>, String> {
    let snap = state
        .sim
        .render_snapshot()
        .read()
        .map_err(|_| "Lock poisoned".to_string())?;
    Ok(snap.satellite_bytes.clone())
}

/// Return pre-packed corporation binary data.
#[tauri::command]
pub async fn sim_get_corporations_binary(
    state: State<'_, SimState>,
) -> Result<Vec<u8>, String> {
    let snap = state
        .sim
        .render_snapshot()
        .read()
        .map_err(|_| "Lock poisoned".to_string())?;
    Ok(snap.corp_bytes.clone())
}
