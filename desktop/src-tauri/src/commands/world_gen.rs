/// Create a world preview from config — static function, no game state needed.
#[tauri::command]
pub fn sim_create_world_preview(config_json: String) -> Result<String, String> {
    gt_tauri::TauriBridge::create_world_preview(&config_json)
}
