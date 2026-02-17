use std::sync::Arc;

use tokio::time::{interval, Duration};
use tracing::debug;

use gt_common::protocol::{CorpDelta, ServerMessage};

use crate::state::{AppState, WorldInstance};

/// Start the tick loop for a specific world
pub fn spawn_world_tick_loop(world: Arc<WorldInstance>) {
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
            let (tick, events, corp_deltas) = {
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

                (tick, events, deltas)
            };

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

/// Start tick loops for all existing worlds in the server state
#[allow(dead_code)]
pub async fn start_all_tick_loops(state: &Arc<AppState>) {
    let worlds = state.worlds.read().await;
    for (_, world) in worlds.iter() {
        spawn_world_tick_loop(Arc::clone(world));
    }
}
