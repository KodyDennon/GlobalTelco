use std::sync::Arc;

use tokio::time::{interval, Duration};
use tracing::debug;
#[cfg(feature = "postgres")]
use tracing::warn;

use gt_common::protocol::{CorpDelta, ServerMessage};

use crate::state::{AppState, WorldInstance};
#[cfg(feature = "postgres")]
use crate::state::SharedDb;

/// Snapshot interval: save world state every N ticks
#[cfg(feature = "postgres")]
const SNAPSHOT_INTERVAL_TICKS: u64 = 100;

/// Start the tick loop for a specific world
#[cfg(feature = "postgres")]
pub fn spawn_world_tick_loop(world: Arc<WorldInstance>, db: SharedDb) {
    tokio::spawn(async move {
        let mut tick_interval = interval(Duration::from_millis(world.tick_rate_ms));

        loop {
            tick_interval.tick().await;

            // Check if world has any connected players
            let player_count = world.player_count().await;
            if player_count == 0 {
                // Still tick but at a slower rate when empty (AI-only worlds)
                tokio::time::sleep(Duration::from_secs(5)).await;
                continue;
            }

            // Advance the simulation
            let (tick, events, corp_deltas, snapshot_data) = {
                let mut w = world.world.lock().await;

                // Check if paused
                if w.speed() == gt_common::types::GameSpeed::Paused {
                    continue;
                }

                w.tick();

                let tick = w.current_tick();
                let events: Vec<gt_common::events::GameEvent> =
                    w.event_queue.drain().into_iter().map(|(_, e)| e).collect();

                // Collect corp deltas for connected players
                let players = world.players.read().await;
                let mut deltas = Vec::new();
                for (_, &corp_id) in players.iter() {
                    if let Some(fin) = w.financials.get(&corp_id) {
                        deltas.push(CorpDelta {
                            corp_id,
                            cash: Some(fin.cash),
                            revenue: Some(fin.revenue_per_tick),
                            cost: Some(fin.cost_per_tick),
                            debt: Some(fin.debt),
                            node_count: w.corp_infra_nodes.get(&corp_id).map(|n| n.len() as u32),
                        });
                    }
                }

                // Periodic snapshot for database persistence
                let snap = if tick % SNAPSHOT_INTERVAL_TICKS == 0 && db.is_some() {
                    match w.save_game_binary() {
                        Ok(data) => Some(data),
                        Err(e) => {
                            warn!("Failed to serialize snapshot: {e}");
                            None
                        }
                    }
                } else {
                    None
                };

                (tick, events, deltas, snap)
            };

            // Save snapshot to database
            if let (Some(db_ref), Some(data)) = (db.as_ref(), snapshot_data) {
                let world_id = world.id;
                let db = Arc::clone(db_ref);
                // Save in background to not block the tick loop
                tokio::spawn(async move {
                    if let Err(e) = db.save_snapshot(world_id, tick as i64, &data).await {
                        warn!("Failed to save snapshot to database: {e}");
                    } else {
                        debug!("Saved snapshot for world {} at tick {}", world_id, tick);
                    }
                });
            }

            // Broadcast tick update to all connected clients
            if !corp_deltas.is_empty() || !events.is_empty() {
                let _ = world.broadcast_tx.send(ServerMessage::TickUpdate {
                    tick,
                    corp_updates: corp_deltas,
                    events,
                });
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

        loop {
            tick_interval.tick().await;

            let player_count = world.player_count().await;
            if player_count == 0 {
                tokio::time::sleep(Duration::from_secs(5)).await;
                continue;
            }

            let (tick, events, corp_deltas) = {
                let mut w = world.world.lock().await;

                if w.speed() == gt_common::types::GameSpeed::Paused {
                    continue;
                }

                w.tick();

                let tick = w.current_tick();
                let events: Vec<gt_common::events::GameEvent> =
                    w.event_queue.drain().into_iter().map(|(_, e)| e).collect();

                let players = world.players.read().await;
                let mut deltas = Vec::new();
                for (_, &corp_id) in players.iter() {
                    if let Some(fin) = w.financials.get(&corp_id) {
                        deltas.push(CorpDelta {
                            corp_id,
                            cash: Some(fin.cash),
                            revenue: Some(fin.revenue_per_tick),
                            cost: Some(fin.cost_per_tick),
                            debt: Some(fin.debt),
                            node_count: w.corp_infra_nodes.get(&corp_id).map(|n| n.len() as u32),
                        });
                    }
                }

                (tick, events, deltas)
            };

            if !corp_deltas.is_empty() || !events.is_empty() {
                let _ = world.broadcast_tx.send(ServerMessage::TickUpdate {
                    tick,
                    corp_updates: corp_deltas,
                    events,
                });
            }

            debug!("World {} tick {}", world.id, tick);
        }
    });
}

/// Start tick loops for all existing worlds in the server state
#[allow(dead_code)]
pub async fn start_all_tick_loops(state: &Arc<AppState>) {
    let worlds = state.worlds.read().await;
    for (_, world) in worlds.iter() {
        #[cfg(feature = "postgres")]
        spawn_world_tick_loop(Arc::clone(world), state.db.clone());
        #[cfg(not(feature = "postgres"))]
        spawn_world_tick_loop(Arc::clone(world));
    }
}
