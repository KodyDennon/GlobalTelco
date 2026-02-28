//! Background simulation thread for native desktop execution.
//!
//! `SimThread` owns the `GameWorld` on a dedicated OS thread. Tauri IPC
//! handlers communicate with it via channels:
//!
//! - **Commands in:** `mpsc::Sender<SimRequest>` — IPC handlers send requests
//! - **Responses out:** Each request carries a `tokio::sync::oneshot::Sender`
//! - **Render data:** `Arc<RwLock<RenderSnapshot>>` — sim writes after ticks,
//!   IPC handlers read without blocking the sim

use std::sync::{mpsc, Arc, RwLock};
use std::thread::{self, JoinHandle};

use gt_bridge::BridgeQuery;
use gt_common::types::{EntityId, WorldConfig};
use tokio::sync::oneshot;

use crate::binary;

/// Pre-packed binary data for hot-path rendering queries.
/// Updated by the sim thread after each tick.
pub struct RenderSnapshot {
    pub infra_bytes: Vec<u8>,
    pub edge_bytes: Vec<u8>,
    pub satellite_bytes: Vec<u8>,
    pub corp_bytes: Vec<u8>,
    pub tick: u64,
}

impl Default for RenderSnapshot {
    fn default() -> Self {
        Self {
            infra_bytes: Vec::new(),
            edge_bytes: Vec::new(),
            satellite_bytes: Vec::new(),
            corp_bytes: Vec::new(),
            tick: 0,
        }
    }
}

/// Enumeration of all JSON query kinds the sim thread can process.
#[derive(Debug)]
pub enum QueryKind {
    WorldInfo,
    CorporationData(EntityId),
    Regions,
    Cities,
    AllCorporations,
    ResearchState,
    Contracts(EntityId),
    DebtInstruments(EntityId),
    Notifications,
    BuildableNodes(f64, f64),
    BuildableEdges(EntityId),
    DamagedNodes(EntityId),
    Auctions,
    CovertOps(EntityId),
    LobbyingCampaigns(EntityId),
    Achievements(EntityId),
    VictoryState,
    TrafficFlows,
    WeatherForecasts,
    ConstellationData(EntityId),
    OrbitalView,
    LaunchSchedule(EntityId),
    TerminalInventory(EntityId),
    DebrisStatus,
    InfrastructureList(EntityId),
    VisibleEntities(f64, f64, f64, f64),
    ParcelsInView(f64, f64, f64, f64),
    CellCoverage,
    AllInfrastructure,
    GridCells,
    WorldGeoJson,
    SpectrumLicenses,
    SpectrumAuctions,
    AvailableSpectrum(EntityId),
    DisasterForecasts,
    AcquisitionProposals,
    RoadPathfind(f64, f64, f64, f64),
    RoadFiberRouteCost(f64, f64, f64, f64),
    RoadSegments,
    PlayerCorpId,
    IsRealEarth,
}

/// Binary query kinds for typed array data.
#[derive(Debug)]
pub enum BinaryQueryKind {
    InfraNodes,
    InfraEdges,
    Satellites,
    Corporations,
}

/// Requests sent to the sim thread.
pub enum SimRequest {
    Tick {
        reply: oneshot::Sender<()>,
    },
    ProcessCommand {
        json: String,
        reply: oneshot::Sender<Result<String, String>>,
    },
    ApplyBatch {
        json: String,
        reply: oneshot::Sender<Result<(), String>>,
    },
    JsonQuery {
        kind: QueryKind,
        reply: oneshot::Sender<String>,
    },
    BinaryQuery {
        kind: BinaryQueryKind,
        reply: oneshot::Sender<Vec<u8>>,
    },
    NewGame {
        config_json: String,
        reply: oneshot::Sender<Result<(), String>>,
    },
    LoadGame {
        data: String,
        reply: oneshot::Sender<Result<(), String>>,
    },
    SaveGame {
        reply: oneshot::Sender<Result<String, String>>,
    },
    Shutdown,
}

/// Handle to the background simulation thread.
pub struct SimThread {
    cmd_tx: mpsc::Sender<SimRequest>,
    render_snapshot: Arc<RwLock<RenderSnapshot>>,
    handle: Option<JoinHandle<()>>,
}

impl SimThread {
    /// Spawn a new sim thread. Does not create a game world yet —
    /// call `new_game` or `load_game` first.
    pub fn new() -> Self {
        let (cmd_tx, cmd_rx) = mpsc::channel::<SimRequest>();
        let render_snapshot = Arc::new(RwLock::new(RenderSnapshot::default()));
        let snapshot_clone = Arc::clone(&render_snapshot);

        let handle = thread::Builder::new()
            .name("sim-thread".into())
            .spawn(move || {
                sim_thread_main(cmd_rx, snapshot_clone);
            })
            .expect("failed to spawn sim thread");

        Self {
            cmd_tx,
            render_snapshot,
            handle: Some(handle),
        }
    }

    pub fn render_snapshot(&self) -> &Arc<RwLock<RenderSnapshot>> {
        &self.render_snapshot
    }

    pub fn sender(&self) -> &mpsc::Sender<SimRequest> {
        &self.cmd_tx
    }

    /// Send a request and get the oneshot receiver.
    fn send(&self, req: SimRequest) {
        let _ = self.cmd_tx.send(req);
    }

    // ── Async helpers for Tauri commands ──────────────────────────────

    pub async fn tick(&self) -> Result<(), String> {
        let (tx, rx) = oneshot::channel();
        self.send(SimRequest::Tick { reply: tx });
        rx.await.map_err(|_| "Sim thread dropped".to_string())
    }

    pub async fn process_command(&self, json: String) -> Result<String, String> {
        let (tx, rx) = oneshot::channel();
        self.send(SimRequest::ProcessCommand { json, reply: tx });
        rx.await.map_err(|_| "Sim thread dropped".to_string())?
    }

    pub async fn apply_batch(&self, json: String) -> Result<(), String> {
        let (tx, rx) = oneshot::channel();
        self.send(SimRequest::ApplyBatch { json, reply: tx });
        rx.await.map_err(|_| "Sim thread dropped".to_string())?
    }

    pub async fn json_query(&self, kind: QueryKind) -> Result<String, String> {
        let (tx, rx) = oneshot::channel();
        self.send(SimRequest::JsonQuery { kind, reply: tx });
        rx.await.map_err(|_| "Sim thread dropped".to_string())
    }

    pub async fn binary_query(&self, kind: BinaryQueryKind) -> Result<Vec<u8>, String> {
        let (tx, rx) = oneshot::channel();
        self.send(SimRequest::BinaryQuery { kind, reply: tx });
        rx.await.map_err(|_| "Sim thread dropped".to_string())
    }

    pub async fn new_game(&self, config_json: String) -> Result<(), String> {
        let (tx, rx) = oneshot::channel();
        self.send(SimRequest::NewGame {
            config_json,
            reply: tx,
        });
        rx.await.map_err(|_| "Sim thread dropped".to_string())?
    }

    pub async fn load_game(&self, data: String) -> Result<(), String> {
        let (tx, rx) = oneshot::channel();
        self.send(SimRequest::LoadGame { data, reply: tx });
        rx.await.map_err(|_| "Sim thread dropped".to_string())?
    }

    pub async fn save_game(&self) -> Result<String, String> {
        let (tx, rx) = oneshot::channel();
        self.send(SimRequest::SaveGame { reply: tx });
        rx.await.map_err(|_| "Sim thread dropped".to_string())?
    }

    pub fn shutdown(&self) {
        let _ = self.cmd_tx.send(SimRequest::Shutdown);
    }
}

impl Drop for SimThread {
    fn drop(&mut self) {
        self.shutdown();
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

/// Main loop running on the dedicated sim thread.
fn sim_thread_main(
    cmd_rx: mpsc::Receiver<SimRequest>,
    render_snapshot: Arc<RwLock<RenderSnapshot>>,
) {
    use crate::TauriBridge;

    let mut bridge: Option<TauriBridge> = None;

    loop {
        let req = match cmd_rx.recv() {
            Ok(req) => req,
            Err(_) => break, // channel closed
        };

        match req {
            SimRequest::Tick { reply } => {
                if let Some(ref mut b) = bridge {
                    b.tick();
                    update_render_snapshot(b, &render_snapshot);
                }
                let _ = reply.send(());
            }

            SimRequest::ProcessCommand { json, reply } => {
                let result = match bridge.as_mut() {
                    Some(b) => b.process_command(&json),
                    None => Err("No game loaded".to_string()),
                };
                let _ = reply.send(result);
            }

            SimRequest::ApplyBatch { json, reply } => {
                let result = match bridge.as_mut() {
                    Some(b) => b.apply_batch(&json),
                    None => Err("No game loaded".to_string()),
                };
                if result.is_ok() {
                    if let Some(ref b) = bridge {
                        update_render_snapshot(b, &render_snapshot);
                    }
                }
                let _ = reply.send(result);
            }

            SimRequest::JsonQuery { kind, reply } => {
                let result = match bridge.as_mut() {
                    Some(b) => dispatch_json_query(b, kind),
                    None => "{}".to_string(),
                };
                let _ = reply.send(result);
            }

            SimRequest::BinaryQuery { kind, reply } => {
                let result = match bridge.as_ref() {
                    Some(b) => dispatch_binary_query(b, kind),
                    None => Vec::new(),
                };
                let _ = reply.send(result);
            }

            SimRequest::NewGame { config_json, reply } => {
                let result = (|| {
                    let config: WorldConfig = serde_json::from_str(&config_json)
                        .map_err(|e| format!("Invalid config: {e}"))?;
                    let b = TauriBridge::new(config);
                    update_render_snapshot(&b, &render_snapshot);
                    bridge = Some(b);
                    Ok(())
                })();
                let _ = reply.send(result);
            }

            SimRequest::LoadGame { data, reply } => {
                let result = (|| {
                    let b = TauriBridge::from_save(&data)?;
                    update_render_snapshot(&b, &render_snapshot);
                    bridge = Some(b);
                    Ok(())
                })();
                let _ = reply.send(result);
            }

            SimRequest::SaveGame { reply } => {
                let result = match bridge.as_ref() {
                    Some(b) => b.save_game(),
                    None => Err("No game loaded".to_string()),
                };
                let _ = reply.send(result);
            }

            SimRequest::Shutdown => break,
        }
    }
}

/// Update the shared RenderSnapshot with current typed array data.
fn update_render_snapshot(bridge: &crate::TauriBridge, snapshot: &Arc<RwLock<RenderSnapshot>>) {
    let infra = bridge.get_infra_arrays();
    let edges = bridge.get_edge_arrays();
    let sats = bridge.get_satellite_arrays();

    let infra_bytes = binary::pack_infra_arrays(&infra);
    let edge_bytes = binary::pack_edge_arrays(&edges);
    let satellite_bytes = binary::pack_satellite_arrays(&sats);

    // Build corporation typed data
    let w = bridge.world.lock().unwrap();
    let count = w.corporations.len();
    let mut ids = Vec::with_capacity(count);
    let mut financials = Vec::with_capacity(count * 3);
    let mut name_offsets = Vec::with_capacity(count * 2);
    let mut names_packed = Vec::new();
    for (&cid, corp) in &w.corporations {
        ids.push(cid as u32);
        let fin = w.financials.get(&cid);
        financials.push(fin.map(|f| f.cash as f64).unwrap_or(0.0));
        financials.push(fin.map(|f| f.revenue_per_tick as f64).unwrap_or(0.0));
        financials.push(fin.map(|f| f.cost_per_tick as f64).unwrap_or(0.0));
        let name_bytes = corp.name.as_bytes();
        name_offsets.push(names_packed.len() as u32);
        name_offsets.push(name_bytes.len() as u32);
        names_packed.extend_from_slice(name_bytes);
    }
    let tick = w.current_tick();
    drop(w);

    let corp_bytes = binary::pack_corporations_typed(&ids, &financials, &name_offsets, &names_packed);

    if let Ok(mut snap) = snapshot.write() {
        snap.infra_bytes = infra_bytes;
        snap.edge_bytes = edge_bytes;
        snap.satellite_bytes = satellite_bytes;
        snap.corp_bytes = corp_bytes;
        snap.tick = tick;
    }
}

/// Dispatch a JSON query to the appropriate BridgeQuery method.
fn dispatch_json_query(bridge: &mut crate::TauriBridge, kind: QueryKind) -> String {
    match kind {
        QueryKind::WorldInfo => bridge.get_world_info(),
        QueryKind::CorporationData(id) => bridge.get_corporation_data(id),
        QueryKind::Regions => bridge.get_regions(),
        QueryKind::Cities => bridge.get_cities(),
        QueryKind::AllCorporations => bridge.get_all_corporations(),
        QueryKind::ResearchState => bridge.get_research_state(),
        QueryKind::Contracts(id) => bridge.get_contracts(id),
        QueryKind::DebtInstruments(id) => bridge.get_debt_instruments(id),
        QueryKind::Notifications => bridge.get_notifications(),
        QueryKind::BuildableNodes(lon, lat) => bridge.get_buildable_nodes(lon, lat),
        QueryKind::BuildableEdges(id) => bridge.get_buildable_edges(id),
        QueryKind::DamagedNodes(id) => bridge.get_damaged_nodes(id),
        QueryKind::Auctions => bridge.get_auctions(),
        QueryKind::CovertOps(id) => bridge.get_covert_ops(id),
        QueryKind::LobbyingCampaigns(id) => bridge.get_lobbying_campaigns(id),
        QueryKind::Achievements(id) => bridge.get_achievements(id),
        QueryKind::VictoryState => bridge.get_victory_state(),
        QueryKind::TrafficFlows => bridge.get_traffic_flows(),
        QueryKind::WeatherForecasts => bridge.get_weather_forecasts(),
        QueryKind::ConstellationData(id) => bridge.get_constellation_data(id),
        QueryKind::OrbitalView => bridge.get_orbital_view(),
        QueryKind::LaunchSchedule(id) => bridge.get_launch_schedule(id),
        QueryKind::TerminalInventory(id) => bridge.get_terminal_inventory(id),
        QueryKind::DebrisStatus => bridge.get_debris_status(),
        QueryKind::InfrastructureList(id) => bridge.get_infrastructure_list(id),
        QueryKind::VisibleEntities(a, b, c, d) => bridge.get_visible_entities(a, b, c, d),
        QueryKind::ParcelsInView(a, b, c, d) => bridge.get_parcels_in_view(a, b, c, d),
        QueryKind::CellCoverage => bridge.get_cell_coverage(),
        QueryKind::AllInfrastructure => bridge.get_all_infrastructure(),
        QueryKind::GridCells => bridge.get_grid_cells(),
        QueryKind::WorldGeoJson => bridge.get_world_geojson(),
        QueryKind::SpectrumLicenses => bridge.get_spectrum_licenses(),
        QueryKind::SpectrumAuctions => bridge.get_spectrum_auctions(),
        QueryKind::AvailableSpectrum(id) => bridge.get_available_spectrum(id),
        QueryKind::DisasterForecasts => bridge.get_disaster_forecasts(),
        QueryKind::AcquisitionProposals => bridge.get_acquisition_proposals(),
        QueryKind::RoadPathfind(a, b, c, d) => bridge.road_pathfind(a, b, c, d),
        QueryKind::RoadFiberRouteCost(a, b, c, d) => bridge.road_fiber_route_cost(a, b, c, d),
        QueryKind::RoadSegments => bridge.get_road_segments(),
        QueryKind::PlayerCorpId => {
            let id = bridge.get_player_corp_id();
            serde_json::to_string(&id).unwrap_or_default()
        }
        QueryKind::IsRealEarth => {
            let val = bridge.is_real_earth();
            serde_json::to_string(&val).unwrap_or_default()
        }
    }
}

/// Dispatch a binary query to the appropriate method.
fn dispatch_binary_query(bridge: &crate::TauriBridge, kind: BinaryQueryKind) -> Vec<u8> {
    match kind {
        BinaryQueryKind::InfraNodes => {
            let arrays = bridge.get_infra_arrays();
            binary::pack_infra_arrays(&arrays)
        }
        BinaryQueryKind::InfraEdges => {
            let arrays = bridge.get_edge_arrays();
            binary::pack_edge_arrays(&arrays)
        }
        BinaryQueryKind::Satellites => {
            let arrays = bridge.get_satellite_arrays();
            binary::pack_satellite_arrays(&arrays)
        }
        BinaryQueryKind::Corporations => {
            let w = bridge.world.lock().unwrap();
            let count = w.corporations.len();
            let mut ids = Vec::with_capacity(count);
            let mut financials = Vec::with_capacity(count * 3);
            let mut name_offsets = Vec::with_capacity(count * 2);
            let mut names_packed = Vec::new();
            for (&cid, corp) in &w.corporations {
                ids.push(cid as u32);
                let fin = w.financials.get(&cid);
                financials.push(fin.map(|f| f.cash as f64).unwrap_or(0.0));
                financials.push(fin.map(|f| f.revenue_per_tick as f64).unwrap_or(0.0));
                financials.push(fin.map(|f| f.cost_per_tick as f64).unwrap_or(0.0));
                let name_bytes = corp.name.as_bytes();
                name_offsets.push(names_packed.len() as u32);
                name_offsets.push(name_bytes.len() as u32);
                names_packed.extend_from_slice(name_bytes);
            }
            drop(w);
            binary::pack_corporations_typed(&ids, &financials, &name_offsets, &names_packed)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> String {
        r#"{"seed":42,"starting_era":"Modern","difficulty":"Normal","map_size":"Small","ai_corporations":2,"use_real_earth":false}"#.to_string()
    }

    #[test]
    fn sim_thread_lifecycle() {
        let sim = SimThread::new();

        // Create a game
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            sim.new_game(test_config()).await.unwrap();
            sim.tick().await.unwrap();

            let info = sim
                .json_query(QueryKind::WorldInfo)
                .await
                .unwrap();
            let parsed: serde_json::Value = serde_json::from_str(&info).unwrap();
            assert_eq!(parsed["tick"], 1);
        });

        // Verify render snapshot was updated
        let snap = sim.render_snapshot().read().unwrap();
        assert_eq!(snap.tick, 1);
    }

    #[test]
    fn sim_thread_command() {
        let sim = SimThread::new();
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            sim.new_game(test_config()).await.unwrap();

            // SetSpeed is a valid command that always succeeds
            let result = sim
                .process_command(r#"{"SetSpeed":"Normal"}"#.to_string())
                .await;
            assert!(result.is_ok());
        });
    }

    #[test]
    fn sim_thread_save_load() {
        let sim = SimThread::new();
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            sim.new_game(test_config()).await.unwrap();
            sim.tick().await.unwrap();

            let save = sim.save_game().await.unwrap();
            assert!(!save.is_empty());

            sim.load_game(save).await.unwrap();
            let info = sim.json_query(QueryKind::WorldInfo).await.unwrap();
            let parsed: serde_json::Value = serde_json::from_str(&info).unwrap();
            assert_eq!(parsed["tick"], 1);
        });
    }
}
