use std::sync::Arc;

use gt_common::commands::Command;
use gt_common::protocol::{ErrorCode, ServerMessage, SpeedVoteEntry};

use crate::state::{AppState, ConnectedPlayer, WorldInstance};

/// Process a GameCommand from a player, including speed vote handling.
///
/// Returns a `ServerMessage` response (typically `CommandAck`).
pub(crate) async fn handle_game_command(
    world_id: uuid::Uuid,
    command: Command,
    seq: Option<u64>,
    state: &Arc<AppState>,
    player: &ConnectedPlayer,
) -> ServerMessage {
    // Spectators cannot send game commands (double-check in case they
    // bypassed the outer check somehow)
    if player.is_spectator {
        return ServerMessage::Error {
            code: ErrorCode::PermissionDenied,
            message: "Spectators cannot send game commands".to_string(),
        };
    }

    if player.world_id.as_ref() != Some(&world_id) {
        return ServerMessage::Error {
            code: ErrorCode::NotInWorld,
            message: "Not in this world".to_string(),
        };
    }

    // Validate command parameters
    if let Err(reason) = super::validation::validate_command(&command) {
        return ServerMessage::Error {
            code: ErrorCode::InvalidCommand,
            message: reason.to_string(),
        };
    }

    // Anti-cheat: verify the player owns the corporation targeted by the command
    if let Some(target_corp) = super::validation::command_target_corp(&command) {
        if player.corp_id != Some(target_corp) {
            return ServerMessage::Error {
                code: ErrorCode::PermissionDenied,
                message: "Command targets a corporation you do not own".to_string(),
            };
        }
    }

    let world = match state.get_world(&world_id).await {
        Some(w) => w,
        None => {
            return ServerMessage::Error {
                code: ErrorCode::WorldNotFound,
                message: "World not found".to_string(),
            };
        }
    };

    // Speed vote system: SetSpeed/TogglePause go through voting
    if matches!(command, Command::SetSpeed(_) | Command::TogglePause) {
        return handle_speed_vote(&world, &command, seq, player).await;
    }

    // Process the command using the player's corp and collect result
    let corp_id = player.corp_id.unwrap_or(0);
    let mut w = world.world.lock().await;
    let tick = w.current_tick();
    let command_debug = format!("{:?}", command);
    let result = w.process_command_for_corp(command, corp_id);
    drop(w);

    state.log_command(player.id, command_debug, tick).await;

    // Broadcast delta ops to all players if command produced visible changes
    if result.success && !result.ops.is_empty() {
        let _ = world.broadcast_tx.send(ServerMessage::CommandBroadcast {
            tick,
            corp_id,
            ops: result.ops,
        });
    }

    ServerMessage::CommandAck {
        success: result.success,
        error: result.error,
        seq,
        entity_id: result.entity_id,
        effective_tick: Some(tick),
    }
}

/// Handle speed vote logic for SetSpeed/TogglePause commands.
async fn handle_speed_vote(
    world: &WorldInstance,
    command: &Command,
    seq: Option<u64>,
    player: &ConnectedPlayer,
) -> ServerMessage {
    let requested_speed = match command {
        Command::SetSpeed(speed) => *speed,
        Command::TogglePause => {
            let w = world.world.lock().await;
            if w.speed() == gt_common::types::GameSpeed::Paused {
                gt_common::types::GameSpeed::Normal
            } else {
                gt_common::types::GameSpeed::Paused
            }
        }
        _ => unreachable!("handle_speed_vote only called with SetSpeed/TogglePause"),
    };

    let is_creator = {
        let creator = world.creator_id.read().await;
        *creator == Some(player.id)
    };

    // Creator has override power -- set speed directly
    if is_creator {
        let mut w = world.world.lock().await;
        w.process_command(Command::SetSpeed(requested_speed));
        let tick = w.current_tick();
        drop(w);

        // Clear any pending votes and broadcast
        world.speed_votes.write().await.clear();
        let _ = world.broadcast_tx.send(ServerMessage::SpeedVoteUpdate {
            votes: vec![],
            resolved_speed: requested_speed,
        });

        return ServerMessage::CommandAck {
            success: true,
            error: None,
            seq,
            entity_id: None,
            effective_tick: Some(tick),
        };
    }

    // Non-creator: register vote
    let speed_str = format!("{:?}", requested_speed);
    world
        .speed_votes
        .write()
        .await
        .insert(player.id, speed_str);

    // Tally votes and resolve by majority
    let players = world.players.read().await;
    let total_players = players.len();
    let votes = world.speed_votes.read().await;
    let mut vote_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    for speed in votes.values() {
        *vote_counts.entry(speed.clone()).or_insert(0) += 1;
    }
    drop(players);

    // Find the speed with the most votes
    let majority_threshold = (total_players / 2) + 1;
    let resolved = vote_counts
        .iter()
        .filter(|(_, count)| **count >= majority_threshold)
        .max_by_key(|(_, count)| *count)
        .map(|(speed, _)| speed.clone());

    if let Some(speed_str) = resolved {
        let resolved_speed = parse_speed_str(&speed_str);

        let mut w = world.world.lock().await;
        w.process_command(Command::SetSpeed(resolved_speed));
        let tick = w.current_tick();
        drop(w);

        // Clear votes after resolution
        world.speed_votes.write().await.clear();
        let _ = world.broadcast_tx.send(ServerMessage::SpeedVoteUpdate {
            votes: vec![],
            resolved_speed,
        });

        return ServerMessage::CommandAck {
            success: true,
            error: None,
            seq,
            entity_id: None,
            effective_tick: Some(tick),
        };
    }

    // No majority yet -- broadcast vote tally
    let vote_entries: Vec<SpeedVoteEntry> = votes
        .iter()
        .map(|(pid, speed)| {
            // Look up username from state
            SpeedVoteEntry {
                username: pid.to_string(), // Will be resolved below
                speed: parse_speed_str(speed),
            }
        })
        .collect();
    drop(votes);

    let current_speed = {
        let w = world.world.lock().await;
        w.speed()
    };
    let _ = world.broadcast_tx.send(ServerMessage::SpeedVoteUpdate {
        votes: vote_entries,
        resolved_speed: current_speed,
    });

    ServerMessage::CommandAck {
        success: true,
        error: Some("Speed vote registered, waiting for majority".to_string()),
        seq,
        entity_id: None,
        effective_tick: None,
    }
}

/// Parse a speed debug string back into a GameSpeed enum.
fn parse_speed_str(s: &str) -> gt_common::types::GameSpeed {
    match s {
        "Paused" => gt_common::types::GameSpeed::Paused,
        "Normal" => gt_common::types::GameSpeed::Normal,
        "Fast" => gt_common::types::GameSpeed::Fast,
        "VeryFast" => gt_common::types::GameSpeed::VeryFast,
        "Ultra" => gt_common::types::GameSpeed::Ultra,
        _ => gt_common::types::GameSpeed::Normal,
    }
}
