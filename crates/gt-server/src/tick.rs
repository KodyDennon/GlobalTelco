use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use tokio::sync::Mutex;
use tokio::time::{interval, Duration};
use tracing::{debug, warn};

use gt_common::protocol::{CorpDelta, ServerMessage};
use gt_common::types::EntityId;
use gt_simulation::world::GameWorld;

use crate::state::{AppState, WorldInstance};
#[cfg(feature = "postgres")]
use crate::state::SharedDb;
#[cfg(feature = "r2")]
use crate::r2::R2Storage;

/// Build a full CorpDelta for a given corporation, including operational data.
/// The per-player filtering in ws.rs will scrub fields based on intel level.
fn build_corp_delta(w: &GameWorld, corp_id: EntityId) -> Option<CorpDelta> {
    let fin = w.financials.get(&corp_id)?;
    let node_ids = w.corp_infra_nodes.get(&corp_id);
    let node_count = Some(node_ids.map(|n| n.len()).unwrap_or(0) as u32);

    // Compute operational stats across all nodes
    let (avg_util, avg_hp, total_tp) = if let Some(ids) = node_ids {
        if ids.is_empty() {
            (0.0, 1.0, 0.0)
        } else {
            let mut sum_util = 0.0;
            let mut sum_health = 0.0;
            let mut sum_throughput = 0.0;
            let mut count = 0usize;
            for &nid in ids {
                if let Some(cap) = w.capacities.get(&nid) {
                    sum_util += cap.utilization();
                }
                if let Some(h) = w.healths.get(&nid) {
                    sum_health += h.condition;
                }
                if let Some(node) = w.infra_nodes.get(&nid) {
                    sum_throughput += node.max_throughput;
                }
                count += 1;
            }
            let n = count.max(1) as f64;
            (sum_util / n, sum_health / n, sum_throughput)
        }
    } else {
        (0.0, 1.0, 0.0)
    };

    Some(CorpDelta {
        corp_id,
        cash: Some(fin.cash),
        revenue: Some(fin.revenue_per_tick),
        cost: Some(fin.cost_per_tick),
        debt: Some(fin.debt),
        node_count,
        avg_utilization: Some(avg_util),
        avg_health: Some(avg_hp),
        total_throughput: Some(total_tp),
    })
}

/// Check if a CorpDelta has changed meaningfully from the previous one.
/// Uses epsilon comparison for floating-point fields.
fn delta_changed(prev: &CorpDelta, curr: &CorpDelta) -> bool {
    const EPS: f64 = 0.01;

    if prev.cash != curr.cash || prev.revenue != curr.revenue
        || prev.cost != curr.cost || prev.debt != curr.debt
        || prev.node_count != curr.node_count
    {
        return true;
    }

    fn f64_changed(a: Option<f64>, b: Option<f64>) -> bool {
        match (a, b) {
            (Some(x), Some(y)) => (x - y).abs() > EPS,
            (None, None) => false,
            _ => true,
        }
    }

    f64_changed(prev.avg_utilization, curr.avg_utilization)
        || f64_changed(prev.avg_health, curr.avg_health)
        || f64_changed(prev.total_throughput, curr.total_throughput)
}

/// Compress a JSON string with zstd at compression level 3.
/// Falls back to uncompressed on error.
fn compress_snapshot(json: &str) -> Option<(Vec<u8>, u32)> {
    let uncompressed_size = json.len() as u32;
    match zstd::encode_all(json.as_bytes(), 3) {
        Ok(compressed) => {
            debug!(
                "Snapshot compressed: {} -> {} bytes ({:.0}% reduction)",
                uncompressed_size,
                compressed.len(),
                (1.0 - compressed.len() as f64 / uncompressed_size as f64) * 100.0
            );
            Some((compressed, uncompressed_size))
        }
        Err(e) => {
            warn!("Snapshot compression failed: {e}");
            None
        }
    }
}

/// Snapshot interval: persist world state to database every N ticks
#[cfg(feature = "postgres")]
const DB_SNAPSHOT_INTERVAL_TICKS: u64 = 100;

/// Maximum snapshots to keep per world (older ones are pruned from R2 + DB)
#[cfg(feature = "postgres")]
const MAX_SNAPSHOTS_PER_WORLD: i64 = 5;

/// Number of ticks to keep in the event log (pruned periodically)
#[cfg(feature = "postgres")]
const EVENT_RETENTION_TICKS: i64 = 10000;

/// Client snapshot interval: push full state to clients every N ticks
/// as a safety net. CommandBroadcast handles instant sync for commands.
const CLIENT_SNAPSHOT_INTERVAL_TICKS: u64 = 30;

/// Higher snapshot interval for worlds with many players (reduces bandwidth)
const CLIENT_SNAPSHOT_INTERVAL_BUSY: u64 = 60;

/// Player count threshold for using the busy snapshot interval
const BUSY_WORLD_PLAYER_THRESHOLD: usize = 3;

/// Start the tick loop for a specific world
#[cfg(feature = "postgres")]
pub fn spawn_world_tick_loop(
    world: Arc<WorldInstance>,
    db: SharedDb,
    #[cfg(feature = "r2")] r2: Option<Arc<R2Storage>>,
) {
    tokio::spawn(async move {
        let mut tick_interval = interval(Duration::from_millis(world.tick_rate_ms));
        let prev_deltas: Mutex<HashMap<EntityId, CorpDelta>> = Mutex::new(HashMap::new());

        // Buffer for events to reduce DB pressure
        let mut event_buffer: Vec<(i64, String, serde_json::Value)> = Vec::new();
        let mut last_event_flush = Instant::now();
        const EVENT_FLUSH_INTERVAL: Duration = Duration::from_secs(2);
        const MAX_EVENT_BUFFER: usize = 200;

        loop {
            tick_interval.tick().await;

            // Check if world has any connected players
            let player_count = world.player_count().await;
            if player_count == 0 {
                // Still tick but at a slower rate when empty (AI-only worlds)
                tokio::time::sleep(Duration::from_secs(5)).await;
                continue;
            }

            // Dynamic snapshot interval based on player count
            let snapshot_interval = if player_count >= BUSY_WORLD_PLAYER_THRESHOLD {
                CLIENT_SNAPSHOT_INTERVAL_BUSY
            } else {
                CLIENT_SNAPSHOT_INTERVAL_TICKS
            };

            // Advance the simulation
            let (tick, events, corp_deltas, db_snapshot, client_snapshot, speed_str, config_json) = {
                let mut w = world.world.lock().await;

                // Check if paused
                if w.speed() == gt_common::types::GameSpeed::Paused {
                    continue;
                }

                let tick_start = Instant::now();
                w.tick();
                let tick_elapsed_us = tick_start.elapsed().as_micros() as u64;

                // Update tick_profiling metrics via the WorldInstance method
                world.record_tick_duration(tick_elapsed_us).await;

                let tick = w.current_tick();
                let events: Vec<gt_common::events::GameEvent> =
                    w.event_queue.drain().into_iter().map(|(_, e)| e).collect();

                // Collect corp deltas, filtering out unchanged ones
                let mut deltas = Vec::new();
                let mut prev = prev_deltas.lock().await;
                let corp_ids: Vec<EntityId> = w.corporations.keys().copied().collect();
                for corp_id in corp_ids {
                    if let Some(delta) = build_corp_delta(&w, corp_id) {
                        let changed = match prev.get(&corp_id) {
                            Some(prev_delta) => delta_changed(prev_delta, &delta),
                            None => true, // New corp, always send
                        };
                        if changed {
                            prev.insert(corp_id, delta.clone());
                            deltas.push(delta);
                        }
                    }
                }

                // Periodic binary snapshot for database persistence
                let db_snap = if tick % DB_SNAPSHOT_INTERVAL_TICKS == 0 && db.is_some() {
                    match w.save_game_binary() {
                        Ok(data) => Some(data),
                        Err(e) => {
                            warn!("Failed to serialize binary snapshot: {e}");
                            None
                        }
                    }
                } else {
                    None
                };

                // Periodic JSON snapshot pushed to clients for WASM state sync
                let client_snap = if tick % snapshot_interval == 0 {
                    match w.save_game() {
                        Ok(json) => Some(json),
                        Err(e) => {
                            warn!("Failed to serialize client snapshot: {e}");
                            None
                        }
                    }
                } else {
                    None
                };

                // Collect world metadata for periodic save
                let speed_str = format!("{:?}", w.speed());
                let config_json = serde_json::to_value(w.config()).unwrap_or_default();

                (tick, events, deltas, db_snap, client_snap, speed_str, config_json)
            };

            // Save snapshot to database (or R2 + metadata)
            if let (Some(db_ref), Some(data)) = (db.as_ref(), db_snapshot) {
                let world_id = world.id;
                let world_name = world.name.clone();
                let max_players = world.max_players as i32;
                let db = Arc::clone(db_ref);
                let speed = speed_str.clone();
                let config = config_json.clone();

                // Clone R2 handle if available
                #[cfg(feature = "r2")]
                let r2_clone = r2.clone();

                // Save in background to not block the tick loop
                tokio::spawn(async move {
                    let mut saved_to_r2 = false;

                    // Try R2 first
                    #[cfg(feature = "r2")]
                    if let Some(ref r2_ref) = r2_clone {
                        let r2_key = R2Storage::snapshot_key(world_id, tick);
                        let size_bytes = data.len() as i64;
                        match r2_ref.put(&r2_key, &data).await {
                            Ok(()) => {
                                if let Err(e) = db.save_snapshot_metadata(
                                    world_id, tick as i64, &r2_key, size_bytes,
                                ).await {
                                    warn!("Failed to save snapshot metadata: {e}");
                                } else {
                                    debug!("Saved R2 snapshot for world {} at tick {} ({} bytes)", world_id, tick, size_bytes);
                                    saved_to_r2 = true;
                                }

                                // Prune old snapshots
                                match db.prune_old_snapshots(world_id, MAX_SNAPSHOTS_PER_WORLD).await {
                                    Ok(old_keys) => {
                                        for key in old_keys {
                                            if let Err(e) = r2_ref.delete(&key).await {
                                                warn!("Failed to delete old R2 snapshot {key}: {e}");
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        warn!("Failed to prune old snapshots: {e}");
                                    }
                                }
                            }
                            Err(e) => {
                                warn!("R2 PUT failed, falling back to DB blob: {e}");
                            }
                        }
                    }

                    // Fall back to DB blob if R2 not used
                    if !saved_to_r2 {
                        if let Err(e) = db.save_snapshot(world_id, tick as i64, &data).await {
                            warn!("Failed to save snapshot to database: {e}");
                        }
                    }

                    // Persist world metadata regardless
                    if let Err(e) = db.save_world(world_id, &world_name, &config, tick as i64, &speed, max_players).await {
                        warn!("Failed to save world metadata: {e}");
                    } else {
                        debug!("Saved snapshot for world {} at tick {}", world_id, tick);
                    }
                });
            }

            // Buffering events for database persistence
            if let Some(db_ref) = db.as_ref() {
                if !events.is_empty() {
                    for e in &events {
                        // Extract a readable variant name for the event type
                        let event_type = match e {
                            gt_common::events::GameEvent::ConstructionStarted { .. } => "ConstructionStarted",
                            gt_common::events::GameEvent::ConstructionCompleted { .. } => "ConstructionCompleted",
                            gt_common::events::GameEvent::NodeBuilt { .. } => "NodeBuilt",
                            gt_common::events::GameEvent::NodeDestroyed { .. } => "NodeDestroyed",
                            gt_common::events::GameEvent::RevenueEarned { .. } => "RevenueEarned",
                            gt_common::events::GameEvent::CostIncurred { .. } => "CostIncurred",
                            gt_common::events::GameEvent::BankruptcyDeclared { .. } => "BankruptcyDeclared",
                            gt_common::events::GameEvent::ResearchStarted { .. } => "ResearchStarted",
                            gt_common::events::GameEvent::ResearchCompleted { .. } => "ResearchCompleted",
                            gt_common::events::GameEvent::DisasterStruck { .. } => "DisasterStruck",
                            gt_common::events::GameEvent::RepairCompleted { .. } => "RepairCompleted",
                            gt_common::events::GameEvent::CorporationFounded { .. } => "CorporationFounded",
                            _ => "Other",
                        }.to_string();
                        
                        if let Ok(v) = serde_json::to_value(e) {
                            event_buffer.push((tick as i64, event_type, v));
                        }
                    }
                }

                // Flush buffer if interval elapsed or buffer is full
                if !event_buffer.is_empty() && (last_event_flush.elapsed() >= EVENT_FLUSH_INTERVAL || event_buffer.len() >= MAX_EVENT_BUFFER) {
                    let db = Arc::clone(db_ref);
                    let world_id = world.id;
                    let to_flush = std::mem::take(&mut event_buffer);
                    last_event_flush = Instant::now();

                    tokio::spawn(async move {
                        let refs: Vec<(i64, &str, &serde_json::Value)> = to_flush
                            .iter()
                            .map(|(t, et, v)| (*t, et.as_str(), v))
                            .collect();
                        if let Err(e) = db.batch_insert_events(world_id, &refs).await {
                            warn!("Failed to persist events: {e}");
                        }

                        // Periodic database maintenance: prune old events (only on flush)
                        if tick % DB_SNAPSHOT_INTERVAL_TICKS == 0 {
                            match db.prune_old_events(world_id, EVENT_RETENTION_TICKS).await {
                                Ok(count) => if count > 0 { debug!("Pruned {} old events for world {}", count, world_id); },
                                Err(e) => warn!("Failed to prune old events: {e}"),
                            }
                        }
                    });
                }

                // Update leaderboard periodically (every DB_SNAPSHOT_INTERVAL_TICKS)
                if tick % DB_SNAPSHOT_INTERVAL_TICKS == 0 {
                    let db = Arc::clone(db_ref);
                    let world_id = world.id;
                    let world_ref = Arc::clone(&world);
                    tokio::spawn(async move {
                        let players = world_ref.players.read().await;
                        let w = world_ref.world.lock().await;
                        for (&player_id, &corp_id) in players.iter() {
                            if let Some(fin) = w.financials.get(&corp_id) {
                                let corp_name = w
                                    .corporations
                                    .get(&corp_id)
                                    .map(|c| c.name.clone())
                                    .unwrap_or_else(|| format!("Corp_{}", corp_id));
                                let net_worth = fin.cash + fin.revenue_per_tick * 100 - fin.debt;
                                let score = net_worth as i64;
                                let _ = db
                                    .update_leaderboard(
                                        player_id,
                                        world_id,
                                        &corp_name,
                                        score,
                                        net_worth as i64,
                                        tick as i64,
                                    )
                                    .await;
                            }
                        }
                    });
                }
            }

            // Broadcast tick update to all connected clients (unconditionally)
            let _ = world.broadcast_tx.send(ServerMessage::TickUpdate {
                tick,
                corp_updates: corp_deltas,
                events,
            });

            // Push compressed world snapshot to clients periodically
            if let Some(state_json) = client_snapshot {
                if let Some((compressed_data, uncompressed_size)) = compress_snapshot(&state_json) {
                    let _ = world.broadcast_tx.send(ServerMessage::CompressedSnapshot {
                        tick,
                        compressed_data,
                        uncompressed_size,
                    });
                } else {
                    // Fallback to uncompressed if compression fails
                    let _ = world.broadcast_tx.send(ServerMessage::Snapshot {
                        tick,
                        state_json,
                    });
                }
            }

            debug!("World {} tick {}", world.id, tick);
        }
    });
}

/// Start the tick loop for a specific world (no-database variant)
#[cfg(not(feature = "postgres"))]
pub fn spawn_world_tick_loop(world: Arc<WorldInstance>) {
    tokio::spawn(async move {
        let mut tick_interval = interval(Duration::from_millis(world.tick_rate_ms));
        let prev_deltas: Mutex<HashMap<EntityId, CorpDelta>> = Mutex::new(HashMap::new());

        loop {
            tick_interval.tick().await;

            let player_count = world.player_count().await;
            if player_count == 0 {
                tokio::time::sleep(Duration::from_secs(5)).await;
                continue;
            }

            // Dynamic snapshot interval based on player count
            let snapshot_interval = if player_count >= BUSY_WORLD_PLAYER_THRESHOLD {
                CLIENT_SNAPSHOT_INTERVAL_BUSY
            } else {
                CLIENT_SNAPSHOT_INTERVAL_TICKS
            };

            let (tick, events, corp_deltas, client_snapshot) = {
                let mut w = world.world.lock().await;

                if w.speed() == gt_common::types::GameSpeed::Paused {
                    continue;
                }

                let tick_start = Instant::now();
                w.tick();
                let tick_elapsed_us = tick_start.elapsed().as_micros() as u64;

                // Update tick_profiling metrics via the WorldInstance method
                world.record_tick_duration(tick_elapsed_us).await;

                let tick = w.current_tick();
                let events: Vec<gt_common::events::GameEvent> =
                    w.event_queue.drain().into_iter().map(|(_, e)| e).collect();

                // Collect corp deltas, filtering out unchanged ones
                let mut deltas = Vec::new();
                let mut prev = prev_deltas.lock().await;
                let corp_ids: Vec<EntityId> = w.corporations.keys().copied().collect();
                for corp_id in corp_ids {
                    if let Some(delta) = build_corp_delta(&w, corp_id) {
                        let changed = match prev.get(&corp_id) {
                            Some(prev_delta) => delta_changed(prev_delta, &delta),
                            None => true,
                        };
                        if changed {
                            prev.insert(corp_id, delta.clone());
                            deltas.push(delta);
                        }
                    }
                }

                // Periodic JSON snapshot pushed to clients for WASM state sync
                let client_snap = if tick % snapshot_interval == 0 {
                    match w.save_game() {
                        Ok(json) => Some(json),
                        Err(e) => {
                            warn!("Failed to serialize client snapshot: {e}");
                            None
                        }
                    }
                } else {
                    None
                };

                (tick, events, deltas, client_snap)
            };

            // Broadcast tick update unconditionally
            let _ = world.broadcast_tx.send(ServerMessage::TickUpdate {
                tick,
                corp_updates: corp_deltas,
                events,
            });

            // Push compressed world snapshot to clients periodically
            if let Some(state_json) = client_snapshot {
                if let Some((compressed_data, uncompressed_size)) = compress_snapshot(&state_json) {
                    let _ = world.broadcast_tx.send(ServerMessage::CompressedSnapshot {
                        tick,
                        compressed_data,
                        uncompressed_size,
                    });
                } else {
                    let _ = world.broadcast_tx.send(ServerMessage::Snapshot {
                        tick,
                        state_json,
                    });
                }
            }

            debug!("World {} tick {}", world.id, tick);
        }
    });
}

/// Start tick loops for all existing worlds in the server state
pub async fn start_all_tick_loops(state: &Arc<AppState>) {
    let worlds = state.worlds.read().await;
    for (_, world) in worlds.iter() {
        #[cfg(feature = "postgres")]
        spawn_world_tick_loop(
            Arc::clone(world),
            state.db.clone(),
            #[cfg(feature = "r2")]
            state.r2.clone(),
        );
        #[cfg(not(feature = "postgres"))]
        spawn_world_tick_loop(Arc::clone(world));
    }
}
