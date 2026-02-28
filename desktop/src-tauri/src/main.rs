#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod sim_state;

use sim_state::SimState;

fn main() {
    tauri::Builder::default()
        .manage(SimState::new())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_process::init())
        .invoke_handler(tauri::generate_handler![
            // ── Filesystem ────────────────────────────────────────────
            commands::filesystem::get_saves_dir,
            commands::filesystem::save_game_native,
            commands::filesystem::load_game_native,
            commands::filesystem::list_saves,
            // ── Lifecycle / control ───────────────────────────────────
            commands::lifecycle::sim_new_game,
            commands::lifecycle::sim_load_game,
            commands::lifecycle::sim_tick,
            commands::lifecycle::sim_current_tick,
            commands::lifecycle::sim_process_command,
            commands::lifecycle::sim_apply_batch,
            commands::lifecycle::sim_save_game,
            // ── JSON queries (no params) ─────────────────────────────
            commands::queries::sim_get_world_info,
            commands::queries::sim_get_regions,
            commands::queries::sim_get_cities,
            commands::queries::sim_get_all_corporations,
            commands::queries::sim_get_research_state,
            commands::queries::sim_get_notifications,
            commands::queries::sim_get_auctions,
            commands::queries::sim_get_victory_state,
            commands::queries::sim_get_traffic_flows,
            commands::queries::sim_get_weather_forecasts,
            commands::queries::sim_get_orbital_view,
            commands::queries::sim_get_debris_status,
            commands::queries::sim_get_cell_coverage,
            commands::queries::sim_get_all_infrastructure,
            commands::queries::sim_get_grid_cells,
            commands::queries::sim_get_world_geojson,
            commands::queries::sim_get_spectrum_licenses,
            commands::queries::sim_get_spectrum_auctions,
            commands::queries::sim_get_disaster_forecasts,
            commands::queries::sim_get_acquisition_proposals,
            commands::queries::sim_get_road_segments,
            commands::queries::sim_get_player_corp_id,
            commands::queries::sim_get_is_real_earth,
            // ── JSON queries (entity ID) ─────────────────────────────
            commands::queries::sim_get_corporation_data,
            commands::queries::sim_get_contracts,
            commands::queries::sim_get_debt_instruments,
            commands::queries::sim_get_buildable_edges,
            commands::queries::sim_get_damaged_nodes,
            commands::queries::sim_get_covert_ops,
            commands::queries::sim_get_lobbying_campaigns,
            commands::queries::sim_get_achievements,
            commands::queries::sim_get_constellation_data,
            commands::queries::sim_get_launch_schedule,
            commands::queries::sim_get_terminal_inventory,
            commands::queries::sim_get_infrastructure_list,
            commands::queries::sim_get_available_spectrum,
            // ── JSON queries (coords / viewport) ─────────────────────
            commands::queries::sim_get_buildable_nodes,
            commands::queries::sim_get_visible_entities,
            commands::queries::sim_get_parcels_in_view,
            commands::queries::sim_road_pathfind,
            commands::queries::sim_road_fiber_route_cost,
            // ── Binary typed arrays ──────────────────────────────────
            commands::binary::sim_get_infra_binary,
            commands::binary::sim_get_edges_binary,
            commands::binary::sim_get_satellites_binary,
            commands::binary::sim_get_corporations_binary,
            // ── World generation ─────────────────────────────────────
            commands::world_gen::sim_create_world_preview,
        ])
        .run(tauri::generate_context!())
        .expect("error while running GlobalTelco desktop app");
}
