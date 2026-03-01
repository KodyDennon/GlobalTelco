use gt_common::protocol::{CorpDelta, ServerMessage};
use gt_common::types::EntityId;

use super::chat::approximate_money;

/// Filter a TickUpdate for per-player data visibility using graduated intel levels.
///
/// Intel levels (per spy_corp -> target_corp pair):
///   0 = Infrastructure positions only (node_count visible, financials/ops hidden)
///   1 = Basic financials (revenue, cost, cash, debt as approximate ranges)
///   2 = Detailed financials (exact revenue, cost, cash, debt numbers)
///   3 = Full operational data (utilization, health, throughput) + exact financials
///
/// Rules:
/// - Spectators see ALL data (no filtering).
/// - A player's OWN corp data is always fully visible.
/// - Competitor data is filtered based on the player's intel level against that competitor.
/// - Intel levels are populated by the espionage system (covert_ops).
pub(crate) fn filter_tick_update_for_player(
    update: &ServerMessage,
    player_corp_id: Option<EntityId>,
    is_spectator: bool,
    intel_levels: &std::collections::HashMap<EntityId, u8>,
) -> ServerMessage {
    // Spectators see everything
    if is_spectator {
        return update.clone();
    }

    match update {
        ServerMessage::TickUpdate {
            tick,
            corp_updates,
            events,
        } => {
            let filtered_updates: Vec<CorpDelta> = corp_updates
                .iter()
                .map(|delta| {
                    // Player's own corp data is always fully visible
                    if Some(delta.corp_id) == player_corp_id {
                        return delta.clone();
                    }

                    // Look up intel level for this competitor
                    let intel = intel_levels.get(&delta.corp_id).copied().unwrap_or(0);

                    match intel {
                        0 => {
                            // Level 0: infrastructure positions only (node_count visible)
                            CorpDelta {
                                corp_id: delta.corp_id,
                                node_count: delta.node_count,
                                cash: None,
                                revenue: None,
                                cost: None,
                                debt: None,
                                avg_utilization: None,
                                avg_health: None,
                                total_throughput: None,
                            }
                        }
                        1 => {
                            // Level 1: basic financials (approximate ranges)
                            CorpDelta {
                                corp_id: delta.corp_id,
                                node_count: delta.node_count,
                                cash: delta.cash.map(approximate_money),
                                revenue: delta.revenue.map(approximate_money),
                                cost: delta.cost.map(approximate_money),
                                debt: delta.debt.map(approximate_money),
                                avg_utilization: None,
                                avg_health: None,
                                total_throughput: None,
                            }
                        }
                        2 => {
                            // Level 2: exact financials, no operational data
                            CorpDelta {
                                corp_id: delta.corp_id,
                                node_count: delta.node_count,
                                cash: delta.cash,
                                revenue: delta.revenue,
                                cost: delta.cost,
                                debt: delta.debt,
                                avg_utilization: None,
                                avg_health: None,
                                total_throughput: None,
                            }
                        }
                        _ => {
                            // Level 3+: full data (exact financials + operational)
                            delta.clone()
                        }
                    }
                })
                .collect();

            // Filter events: global events (empty related_corps) go to everyone;
            // private events only go to the relevant corporations.
            let filtered_events: Vec<gt_common::events::GameEvent> = events
                .iter()
                .filter(|event| {
                    let corps = event.related_corps();
                    // Empty = global event, send to all
                    if corps.is_empty() {
                        return true;
                    }
                    // Send to player if their corp is in the related list
                    if let Some(pc) = player_corp_id {
                        corps.contains(&pc)
                    } else {
                        false
                    }
                })
                .cloned()
                .collect();

            ServerMessage::TickUpdate {
                tick: *tick,
                corp_updates: filtered_updates,
                events: filtered_events,
            }
        }
        // Non-TickUpdate messages pass through unmodified
        other => other.clone(),
    }
}
