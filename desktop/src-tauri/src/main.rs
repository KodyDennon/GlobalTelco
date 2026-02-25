#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

use tauri::Manager;

use gt_bridge::BridgeQuery;
use gt_tauri::TauriBridge;

/// Managed simulation state — None until a game is started or loaded.
struct SimState(Mutex<Option<TauriBridge>>);

/// Get the saves directory for the platform
fn saves_dir(app: &tauri::AppHandle) -> PathBuf {
    let data_dir = app
        .path()
        .app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."));
    let saves = data_dir.join("saves");
    let _ = fs::create_dir_all(&saves);
    saves
}

// ── File system commands ────────────────────────────────────────────────

#[tauri::command]
fn get_saves_dir(app: tauri::AppHandle) -> String {
    saves_dir(&app).to_string_lossy().to_string()
}

#[tauri::command]
fn save_game_native(app: tauri::AppHandle, slot: u32, data: String) -> Result<String, String> {
    let dir = saves_dir(&app);
    let path = dir.join(format!("save_{}.gtco", slot));
    fs::write(&path, &data).map_err(|e| format!("Failed to save: {}", e))?;
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
fn load_game_native(app: tauri::AppHandle, slot: u32) -> Result<Option<String>, String> {
    let dir = saves_dir(&app);
    let path = dir.join(format!("save_{}.gtco", slot));
    if !path.exists() {
        return Ok(None);
    }
    let data = fs::read_to_string(&path).map_err(|e| format!("Failed to load: {}", e))?;
    Ok(Some(data))
}

#[tauri::command]
fn list_saves(app: tauri::AppHandle) -> Result<Vec<SaveEntry>, String> {
    let dir = saves_dir(&app);
    let mut entries = Vec::new();
    if let Ok(read_dir) = fs::read_dir(&dir) {
        for entry in read_dir.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("gtco") {
                let name = path
                    .file_stem()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
                    .to_string();
                let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                let modified = entry
                    .metadata()
                    .ok()
                    .and_then(|m| m.modified().ok())
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                entries.push(SaveEntry {
                    name,
                    path: path.to_string_lossy().to_string(),
                    size,
                    modified,
                });
            }
        }
    }
    entries.sort_by(|a, b| b.modified.cmp(&a.modified));
    Ok(entries)
}

#[derive(serde::Serialize)]
struct SaveEntry {
    name: String,
    path: String,
    size: u64,
    modified: u64,
}

// ── Native simulation commands ──────────────────────────────────────────

#[tauri::command]
fn sim_new_game(state: tauri::State<SimState>, config_json: String) -> Result<(), String> {
    let bridge = gt_tauri::cmd_new_game(&config_json)?;
    *state.0.lock().unwrap() = Some(bridge);
    Ok(())
}

#[tauri::command]
fn sim_load_game(state: tauri::State<SimState>, data: String) -> Result<(), String> {
    let bridge = TauriBridge::from_save(&data)?;
    *state.0.lock().unwrap() = Some(bridge);
    Ok(())
}

#[tauri::command]
fn sim_tick(state: tauri::State<SimState>) -> Result<(), String> {
    let mut guard = state.0.lock().unwrap();
    let bridge = guard.as_mut().ok_or("No game loaded")?;
    bridge.tick();
    Ok(())
}

#[tauri::command]
fn sim_current_tick(state: tauri::State<SimState>) -> Result<u64, String> {
    let guard = state.0.lock().unwrap();
    let bridge = guard.as_ref().ok_or("No game loaded")?;
    Ok(bridge.current_tick())
}

#[tauri::command]
fn sim_process_command(state: tauri::State<SimState>, command_json: String) -> Result<String, String> {
    let mut guard = state.0.lock().unwrap();
    let bridge = guard.as_mut().ok_or("No game loaded")?;
    bridge.process_command(&command_json)
}

#[tauri::command]
fn sim_apply_batch(state: tauri::State<SimState>, ops_json: String) -> Result<(), String> {
    let mut guard = state.0.lock().unwrap();
    let bridge = guard.as_mut().ok_or("No game loaded")?;
    bridge.apply_batch(&ops_json)
}

#[tauri::command]
fn sim_get_world_info(state: tauri::State<SimState>) -> Result<String, String> {
    let guard = state.0.lock().unwrap();
    let bridge = guard.as_ref().ok_or("No game loaded")?;
    Ok(bridge.get_world_info())
}

#[tauri::command]
fn sim_get_corporation_data(state: tauri::State<SimState>, corp_id: u64) -> Result<String, String> {
    let guard = state.0.lock().unwrap();
    let bridge = guard.as_ref().ok_or("No game loaded")?;
    Ok(bridge.get_corporation_data(corp_id))
}

#[tauri::command]
fn sim_get_regions(state: tauri::State<SimState>) -> Result<String, String> {
    let guard = state.0.lock().unwrap();
    let bridge = guard.as_ref().ok_or("No game loaded")?;
    Ok(bridge.get_regions())
}

#[tauri::command]
fn sim_get_cities(state: tauri::State<SimState>) -> Result<String, String> {
    let guard = state.0.lock().unwrap();
    let bridge = guard.as_ref().ok_or("No game loaded")?;
    Ok(bridge.get_cities())
}

#[tauri::command]
fn sim_get_all_corporations(state: tauri::State<SimState>) -> Result<String, String> {
    let guard = state.0.lock().unwrap();
    let bridge = guard.as_ref().ok_or("No game loaded")?;
    Ok(bridge.get_all_corporations())
}

#[tauri::command]
fn sim_get_all_infrastructure(state: tauri::State<SimState>) -> Result<String, String> {
    let guard = state.0.lock().unwrap();
    let bridge = guard.as_ref().ok_or("No game loaded")?;
    // Use typed arrays internally, serialize to JSON for now
    // A future optimization can use binary IPC channels
    let infra = bridge.get_infra_arrays();
    let edges = bridge.get_edge_arrays();
    Ok(serde_json::json!({
        "node_count": infra.ids.len(),
        "edge_count": edges.ids.len(),
    }).to_string())
}

#[tauri::command]
fn sim_get_notifications(state: tauri::State<SimState>) -> Result<String, String> {
    let mut guard = state.0.lock().unwrap();
    let bridge = guard.as_mut().ok_or("No game loaded")?;
    Ok(bridge.get_notifications())
}

#[tauri::command]
fn sim_save_game(state: tauri::State<SimState>) -> Result<String, String> {
    let guard = state.0.lock().unwrap();
    let bridge = guard.as_ref().ok_or("No game loaded")?;
    bridge.save_game()
}

fn main() {
    tauri::Builder::default()
        .manage(SimState(Mutex::new(None)))
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_process::init())
        .invoke_handler(tauri::generate_handler![
            // File system
            get_saves_dir,
            save_game_native,
            load_game_native,
            list_saves,
            // Native simulation
            sim_new_game,
            sim_load_game,
            sim_tick,
            sim_current_tick,
            sim_process_command,
            sim_apply_batch,
            sim_get_world_info,
            sim_get_corporation_data,
            sim_get_regions,
            sim_get_cities,
            sim_get_all_corporations,
            sim_get_all_infrastructure,
            sim_get_notifications,
            sim_save_game,
        ])
        .run(tauri::generate_context!())
        .expect("error while running GlobalTelco desktop app");
}
