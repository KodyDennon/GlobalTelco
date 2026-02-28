use std::fs;
use std::path::PathBuf;

use tauri::Manager;

/// Get the saves directory for the platform.
fn saves_dir(app: &tauri::AppHandle) -> PathBuf {
    let data_dir = app
        .path()
        .app_data_dir()
        .unwrap_or_else(|_| PathBuf::from("."));
    let saves = data_dir.join("saves");
    let _ = fs::create_dir_all(&saves);
    saves
}

#[derive(serde::Serialize)]
pub struct SaveEntry {
    name: String,
    path: String,
    size: u64,
    modified: u64,
}

#[tauri::command]
pub fn get_saves_dir(app: tauri::AppHandle) -> String {
    saves_dir(&app).to_string_lossy().to_string()
}

#[tauri::command]
pub fn save_game_native(app: tauri::AppHandle, slot: u32, data: String) -> Result<String, String> {
    let dir = saves_dir(&app);
    let path = dir.join(format!("save_{}.gtco", slot));
    fs::write(&path, &data).map_err(|e| format!("Failed to save: {}", e))?;
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn load_game_native(app: tauri::AppHandle, slot: u32) -> Result<Option<String>, String> {
    let dir = saves_dir(&app);
    let path = dir.join(format!("save_{}.gtco", slot));
    if !path.exists() {
        return Ok(None);
    }
    let data = fs::read_to_string(&path).map_err(|e| format!("Failed to load: {}", e))?;
    Ok(Some(data))
}

#[tauri::command]
pub fn list_saves(app: tauri::AppHandle) -> Result<Vec<SaveEntry>, String> {
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
