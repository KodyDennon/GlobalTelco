//! Shared bridge query trait for gt-wasm and gt-tauri.
//!
//! Both the WASM bridge (browser) and the Tauri bridge (desktop native)
//! implement `BridgeQuery` so that the frontend can use the same API
//! regardless of the runtime environment.
//!
//! The `queries` module contains shared pure-logic query functions that take
//! `&GameWorld` and return JSON strings, eliminating duplicated serialization
//! code between the two bridges.

pub mod queries;

use gt_common::types::EntityId;

/// Results of an infrastructure query as flat typed arrays.
/// Each array is parallel-indexed: positions[2*i], positions[2*i+1] = (lon, lat) for entity i.
pub struct InfraArrays {
    /// Entity IDs (parallel with positions)
    pub ids: Vec<u32>,
    /// Owner corp IDs
    pub owners: Vec<u32>,
    /// [lon0, lat0, lon1, lat1, ...] — 2 floats per node
    pub positions: Vec<f64>,
    /// [health0, utilization0, throughput0, health1, utilization1, throughput1, ...] — 3 floats per node
    pub stats: Vec<f64>,
    /// Node type enum discriminants (u8)
    pub node_types: Vec<u8>,
    /// Network level enum discriminants
    pub network_levels: Vec<u32>,
    /// 1 if under construction, 0 if not
    pub construction_flags: Vec<u8>,
    /// Grid cell index for each node
    pub cell_indices: Vec<u32>,
}

/// Edge data as flat typed arrays.
pub struct EdgeArrays {
    /// Entity IDs
    pub ids: Vec<u32>,
    /// Owner corp IDs
    pub owners: Vec<u32>,
    /// [src_lon0, src_lat0, dst_lon0, dst_lat0, ...] — 4 floats per edge
    pub endpoints: Vec<f64>,
    /// [bandwidth0, utilization0, bandwidth1, utilization1, ...] — 2 floats per edge
    pub stats: Vec<f64>,
    /// Edge type enum discriminants (u8)
    pub edge_types: Vec<u8>,
    /// Deployment method (0=Underground, 1=Aerial)
    pub deployment_types: Vec<u8>,
    /// Packed waypoint data [lon0, lat0, lon1, lat1, ...]
    pub waypoints_data: Vec<f64>,
    /// Start index in waypoints_data for each edge
    pub waypoint_offsets: Vec<u32>,
    /// Number of points (pairs) for each edge
    pub waypoint_lengths: Vec<u8>,
}

/// Satellite data as flat typed arrays for orbital overlay rendering.
pub struct SatelliteArrays {
    /// Entity IDs
    pub ids: Vec<u32>,
    /// Owner corp IDs
    pub owners: Vec<u32>,
    /// [lon0, lat0, lon1, lat1, ...] — 2 floats per satellite (sub-satellite point)
    pub positions: Vec<f64>,
    /// [altitude0, altitude1, ...] — km above Earth
    pub altitudes: Vec<f64>,
    /// Orbit type enum discriminants (0=LEO, 1=MEO, 2=GEO, 3=HEO)
    pub orbit_types: Vec<u32>,
    /// Status enum discriminants
    pub statuses: Vec<u32>,
    /// Fuel level (0.0-1.0 fraction)
    pub fuel_levels: Vec<f64>,
}

/// Trait defining the shared query API between WASM and Tauri bridges.
///
/// JSON methods return `String` for compatibility with both wasm-bindgen
/// and Tauri's serde-based IPC. Typed array methods return flat structs
/// for zero-copy rendering in deck.gl.
pub trait BridgeQuery {
    // ── Lifecycle ────────────────────────────────────────────────────────
    fn tick(&mut self);
    fn current_tick(&self) -> u64;
    fn process_command(&mut self, command_json: &str) -> Result<String, String>;
    fn apply_batch(&mut self, ops_json: &str) -> Result<(), String>;

    // ── JSON queries (non-hot-path) ─────────────────────────────────────
    fn get_world_info(&self) -> String;
    fn get_static_definitions(&self) -> String;
    fn get_corporation_data(&self, corp_id: EntityId) -> String;
    fn get_regions(&self) -> String;
    fn get_cities(&self) -> String;
    fn get_all_corporations(&self) -> String;
    fn get_research_state(&self) -> String;
    fn get_contracts(&self, corp_id: EntityId) -> String;
    fn get_debt_instruments(&self, corp_id: EntityId) -> String;
    fn get_notifications(&mut self) -> String;
    fn get_buildable_nodes(&self, lon: f64, lat: f64) -> String;
    fn get_buildable_edges(&self, source_id: EntityId) -> String;
    fn get_damaged_nodes(&self, corp_id: EntityId) -> String;
    fn get_auctions(&self) -> String;
    fn get_covert_ops(&self, corp_id: EntityId) -> String;
    fn get_lobbying_campaigns(&self, corp_id: EntityId) -> String;
    fn get_achievements(&self, corp_id: EntityId) -> String;
    fn get_victory_state(&self) -> String;
    fn get_traffic_flows(&self) -> String;
    fn get_weather_forecasts(&self) -> String;
    fn save_game(&self) -> Result<String, String>;
    fn load_game(&mut self, data: &str) -> Result<(), String>;

    // ── Additional queries ──────────────────────────────────────────────
    fn get_alliances(&self, corp_id: EntityId) -> String;
    fn get_lawsuits(&self, corp_id: EntityId) -> String;
    fn get_stock_market(&self, corp_id: EntityId) -> String;
    fn get_region_pricing(&self, corp_id: EntityId) -> String;
    fn get_maintenance_priorities(&self, corp_id: EntityId) -> String;

    // ── Targeted Metadata Queries (Optimization) ────────────────────────
    fn get_node_metadata(&self, id: EntityId) -> String;
    fn get_nodes_metadata(&self, ids: &[EntityId]) -> String;
    fn get_edge_metadata(&self, id: EntityId) -> String;

    // ── Satellite queries ───────────────────────────────────────────────
    fn get_constellation_data(&self, corp_id: EntityId) -> String;
    fn get_orbital_view(&self) -> String;
    fn get_launch_schedule(&self, corp_id: EntityId) -> String;
    fn get_terminal_inventory(&self, corp_id: EntityId) -> String;
    fn get_debris_status(&self) -> String;

    // ── Typed array queries (hot-path rendering) ────────────────────────
    fn get_infra_arrays(&self) -> InfraArrays;
    fn get_infra_arrays_viewport(&self, west: f64, south: f64, east: f64, north: f64) -> InfraArrays;
    fn get_edge_arrays(&self) -> EdgeArrays;
    fn get_edge_arrays_viewport(&self, west: f64, south: f64, east: f64, north: f64) -> EdgeArrays;
    fn get_satellite_arrays(&self) -> SatelliteArrays;
}
