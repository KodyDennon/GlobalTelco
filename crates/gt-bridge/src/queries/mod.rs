//! Shared pure-logic query functions for gt-wasm and gt-tauri.
//!
//! Each function takes a `&GameWorld` (or `&mut GameWorld` for drain operations)
//! and returns a `String` (JSON). Both the WASM and Tauri bridges delegate to
//! these functions, eliminating duplicated serialization logic.

mod world;
mod infrastructure;
mod corporation;
mod satellite;
mod social;
mod typed_arrays;

// ── World / Region / City Queries ───────────────────────────────────────
pub use world::{
    query_static_definitions,
    query_world_info,
    query_regions,
    query_cities,
};

// ── Infrastructure Queries ──────────────────────────────────────────────
pub use infrastructure::{
    query_terrain_at,
    query_node_metadata,
    query_edge_metadata,
    query_nodes_metadata,
    query_infrastructure_list,
    query_visible_entities,
    query_parcels_in_view,
    query_cell_coverage,
    query_all_infrastructure,
    query_grid_cells,
    query_world_geojson,
    query_buildable_nodes,
    query_buildable_edges,
    query_damaged_nodes,
    query_traffic_flows,
    query_road_pathfind,
    query_road_segments,
};

// ── Corporation / Finance / Research Queries ─────────────────────────────
pub use corporation::{
    query_corporation_data,
    query_all_corporations,
    query_research_state,
    query_contracts,
    query_debt_instruments,
    query_notifications,
    query_covert_ops,
    query_lobbying_campaigns,
    query_achievements,
    query_victory_state,
    query_auctions,
    query_acquisition_proposals,
};

// ── Satellite Queries ───────────────────────────────────────────────────
pub use satellite::{
    query_satellite_inventory,
    query_constellation_data,
    query_orbital_view,
    query_launch_schedule,
    query_terminal_inventory,
    query_debris_status,
};

// ── Social / Alliance / Legal / Spectrum Queries ────────────────────────
pub use social::{
    query_grants,
    query_spectrum_licenses,
    query_spectrum_auctions,
    query_available_spectrum,
    query_co_ownership_proposals,
    query_pending_upgrade_votes,
    query_alliances,
    query_lawsuits,
    query_stock_market,
    query_region_pricing,
    query_maintenance_priorities,
};

// ── Typed Array Helpers ─────────────────────────────────────────────────
pub use typed_arrays::{
    build_infra_arrays,
    build_infra_arrays_viewport,
    build_edge_arrays,
    build_edge_arrays_viewport,
    build_satellite_arrays,
};
