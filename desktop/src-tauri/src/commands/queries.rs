use tauri::State;

use gt_tauri::sim_thread::QueryKind;

use crate::sim_state::SimState;

macro_rules! json_query {
    ($name:ident, $kind:expr) => {
        #[tauri::command]
        pub async fn $name(state: State<'_, SimState>) -> Result<String, String> {
            state.sim.json_query($kind).await
        }
    };
}

macro_rules! json_query_u64 {
    ($name:ident, $kind:ident) => {
        #[tauri::command]
        pub async fn $name(
            state: State<'_, SimState>,
            id: u64,
        ) -> Result<String, String> {
            state.sim.json_query(QueryKind::$kind(id)).await
        }
    };
}

// ── Simple queries (no parameters) ────────────────────────────────────────

json_query!(sim_get_world_info, QueryKind::WorldInfo);
json_query!(sim_get_regions, QueryKind::Regions);
json_query!(sim_get_cities, QueryKind::Cities);
json_query!(sim_get_all_corporations, QueryKind::AllCorporations);
json_query!(sim_get_research_state, QueryKind::ResearchState);
json_query!(sim_get_notifications, QueryKind::Notifications);
json_query!(sim_get_auctions, QueryKind::Auctions);
json_query!(sim_get_victory_state, QueryKind::VictoryState);
json_query!(sim_get_traffic_flows, QueryKind::TrafficFlows);
json_query!(sim_get_orbital_view, QueryKind::OrbitalView);
json_query!(sim_get_debris_status, QueryKind::DebrisStatus);
json_query!(sim_get_cell_coverage, QueryKind::CellCoverage);
json_query!(sim_get_all_infrastructure, QueryKind::AllInfrastructure);
json_query!(sim_get_grid_cells, QueryKind::GridCells);
json_query!(sim_get_world_geojson, QueryKind::WorldGeoJson);
json_query!(sim_get_spectrum_licenses, QueryKind::SpectrumLicenses);
json_query!(sim_get_spectrum_auctions, QueryKind::SpectrumAuctions);
json_query!(sim_get_acquisition_proposals, QueryKind::AcquisitionProposals);
json_query!(sim_get_road_segments, QueryKind::RoadSegments);
json_query!(sim_get_player_corp_id, QueryKind::PlayerCorpId);
json_query!(sim_get_is_real_earth, QueryKind::IsRealEarth);

// ── Queries with a single entity ID ──────────────────────────────────────

json_query_u64!(sim_get_corporation_data, CorporationData);
json_query_u64!(sim_get_contracts, Contracts);
json_query_u64!(sim_get_debt_instruments, DebtInstruments);
json_query_u64!(sim_get_buildable_edges, BuildableEdges);
json_query_u64!(sim_get_damaged_nodes, DamagedNodes);
json_query_u64!(sim_get_covert_ops, CovertOps);
json_query_u64!(sim_get_lobbying_campaigns, LobbyingCampaigns);
json_query_u64!(sim_get_achievements, Achievements);
json_query_u64!(sim_get_constellation_data, ConstellationData);
json_query_u64!(sim_get_launch_schedule, LaunchSchedule);
json_query_u64!(sim_get_terminal_inventory, TerminalInventory);
json_query_u64!(sim_get_infrastructure_list, InfrastructureList);
json_query_u64!(sim_get_available_spectrum, AvailableSpectrum);
json_query_u64!(sim_get_alliances, Alliances);
json_query_u64!(sim_get_lawsuits, Lawsuits);
json_query_u64!(sim_get_stock_market, StockMarket);
json_query_u64!(sim_get_region_pricing, RegionPricing);
json_query_u64!(sim_get_maintenance_priorities, MaintenancePriorities);

// ── Queries with (lon, lat) ──────────────────────────────────────────────

#[tauri::command]
pub async fn sim_get_buildable_nodes(
    state: State<'_, SimState>,
    lon: f64,
    lat: f64,
) -> Result<String, String> {
    state
        .sim
        .json_query(QueryKind::BuildableNodes(lon, lat))
        .await
}

// ── Queries with viewport bounds (min_x, min_y, max_x, max_y) ───────────

#[tauri::command]
pub async fn sim_get_visible_entities(
    state: State<'_, SimState>,
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
) -> Result<String, String> {
    state
        .sim
        .json_query(QueryKind::VisibleEntities(min_x, min_y, max_x, max_y))
        .await
}

#[tauri::command]
pub async fn sim_get_parcels_in_view(
    state: State<'_, SimState>,
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
) -> Result<String, String> {
    state
        .sim
        .json_query(QueryKind::ParcelsInView(min_x, min_y, max_x, max_y))
        .await
}

#[tauri::command]
pub async fn sim_road_pathfind(
    state: State<'_, SimState>,
    from_lon: f64,
    from_lat: f64,
    to_lon: f64,
    to_lat: f64,
) -> Result<String, String> {
    state
        .sim
        .json_query(QueryKind::RoadPathfind(from_lon, from_lat, to_lon, to_lat))
        .await
}

#[tauri::command]
pub async fn sim_road_fiber_route_cost(
    state: State<'_, SimState>,
    from_lon: f64,
    from_lat: f64,
    to_lon: f64,
    to_lat: f64,
) -> Result<String, String> {
    state
        .sim
        .json_query(QueryKind::RoadFiberRouteCost(from_lon, from_lat, to_lon, to_lat))
        .await
}
