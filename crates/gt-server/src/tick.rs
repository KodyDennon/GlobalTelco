use std::sync::Arc;

use tokio::time::{interval, Duration};
use tracing::debug;
#[cfg(feature = "postgres")]
use tracing::warn;

use gt_common::protocol::{CorpDelta, ServerMessage};
use gt_common::types::EntityId;
use gt_simulation::world::GameWorld;

use crate::state::{AppState, WorldInstance};
#[cfg(feature = "postgres")]
use crate::state::SharedDb;

/// Build a full CorpDelta for a given corporation, including operational data.
/// The per-player filtering in ws.rs will scrub fields based on intel level.
fn build_corp_delta(w: &GameWorld, corp_id: EntityId) -> Option<CorpDelta> {
    let fin = w.financials.get(&corp_id)?;
    let node_ids = w.corp_infra_nodes.get(&corp_id);
    let node_count = node_ids.map(|n| n.len() as u32);

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

                // Collect corp deltas for ALL corporations (filtering happens per-player in ws.rs)
                let mut deltas = Vec::new();
                let corp_ids: Vec<EntityId> = w.corporations.keys().copied().collect();
                for corp_id in corp_ids {
                    if let Some(delta) = build_corp_delta(&w, corp_id) {
                        deltas.push(delta);
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

                // Collect corp deltas for ALL corporations (filtering happens per-player in ws.rs)
                let mut deltas = Vec::new();
                let corp_ids: Vec<EntityId> = w.corporations.keys().copied().collect();
                for corp_id in corp_ids {
                    if let Some(delta) = build_corp_delta(&w, corp_id) {
                        deltas.push(delta);
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
