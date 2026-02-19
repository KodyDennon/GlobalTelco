use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::commands::Command;
use crate::events::GameEvent;
use crate::types::{EntityId, GameSpeed, Money, Tick};

// ── Client → Server Messages ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientMessage {
    /// Authenticate with the server
    Auth(AuthRequest),
    /// Send a game command
    GameCommand { world_id: Uuid, command: Command },
    /// Request a state snapshot
    RequestSnapshot { world_id: Uuid },
    /// Join a game world
    JoinWorld { world_id: Uuid },
    /// Leave the current game world
    LeaveWorld,
    /// Keepalive
    Ping { timestamp: u64 },
    /// Chat message
    Chat { message: String },
    /// Upload a cloud save
    UploadSave {
        slot: i32,
        name: String,
        save_data: Vec<u8>,
        tick: Tick,
        config_json: String,
    },
    /// Request list of cloud saves
    RequestSaves,
    /// Download a cloud save by slot
    DownloadSave { slot: i32 },
    /// Delete a cloud save
    DeleteSave { slot: i32 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthRequest {
    Login {
        username: String,
        password: String,
    },
    Register {
        username: String,
        password: String,
        email: String,
    },
    Token {
        access_token: String,
    },
    TokenRefresh {
        refresh_token: String,
    },
    Guest,
}

// ── Server → Client Messages ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerMessage {
    /// Authentication result
    AuthResult(AuthResponse),
    /// World join result
    WorldJoined {
        world_id: Uuid,
        corp_id: EntityId,
        tick: Tick,
    },
    /// State delta after a tick
    TickUpdate {
        tick: Tick,
        corp_updates: Vec<CorpDelta>,
        events: Vec<GameEvent>,
    },
    /// Full state snapshot (on join or request)
    Snapshot { tick: Tick, state_json: String },
    /// Command acknowledged
    CommandAck {
        success: bool,
        error: Option<String>,
    },
    /// Server error
    Error { code: ErrorCode, message: String },
    /// Keepalive response
    Pong { timestamp: u64, server_time: u64 },
    /// Player connected/disconnected notification
    PlayerStatus {
        player_id: Uuid,
        username: String,
        status: PlayerConnectionStatus,
    },
    /// Chat broadcast
    ChatBroadcast {
        sender: String,
        message: String,
        timestamp: u64,
    },
    /// World list response
    WorldList { worlds: Vec<WorldInfo> },
    /// Cloud save uploaded acknowledgement
    SaveUploaded { slot: i32, success: bool },
    /// List of cloud saves
    SaveList { saves: Vec<CloudSaveInfo> },
    /// Cloud save data download
    SaveData { slot: i32, save_data: Vec<u8> },
    /// AI proxy summary after reconnection
    ProxySummary {
        ticks_elapsed: u64,
        actions: Vec<ProxyAction>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthResponse {
    Success {
        player_id: Uuid,
        username: String,
        access_token: String,
        refresh_token: String,
    },
    GuestSuccess {
        player_id: Uuid,
        username: String,
    },
    Failed {
        reason: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorpDelta {
    pub corp_id: EntityId,
    pub cash: Option<Money>,
    pub revenue: Option<Money>,
    pub cost: Option<Money>,
    pub debt: Option<Money>,
    pub node_count: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorCode {
    NotAuthenticated,
    NotInWorld,
    WorldNotFound,
    WorldFull,
    InvalidCommand,
    InsufficientFunds,
    PermissionDenied,
    RateLimited,
    InternalError,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlayerConnectionStatus {
    Connected,
    Disconnected,
    AiProxy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudSaveInfo {
    pub slot: i32,
    pub name: String,
    pub tick: Tick,
    pub size_bytes: i64,
    pub created_at: u64, // unix timestamp
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyAction {
    pub tick: Tick,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldInfo {
    pub id: Uuid,
    pub name: String,
    pub player_count: u32,
    pub max_players: u32,
    pub tick: Tick,
    pub speed: GameSpeed,
    pub era: crate::types::Era,
    pub map_size: crate::types::MapSize,
}

// ── Serialization Helpers ──────────────────────────────────────────────────

/// Serialize a message to MessagePack bytes
pub fn serialize_msgpack<T: Serialize>(msg: &T) -> Result<Vec<u8>, rmp_serde::encode::Error> {
    rmp_serde::to_vec(msg)
}

/// Deserialize from MessagePack bytes
pub fn deserialize_msgpack<'a, T: Deserialize<'a>>(
    bytes: &'a [u8],
) -> Result<T, rmp_serde::decode::Error> {
    rmp_serde::from_slice(bytes)
}

/// Serialize to JSON (for debug/development)
pub fn serialize_json<T: Serialize>(msg: &T) -> Result<String, serde_json::Error> {
    serde_json::to_string(msg)
}

/// Deserialize from JSON
pub fn deserialize_json<'a, T: Deserialize<'a>>(json: &'a str) -> Result<T, serde_json::Error> {
    serde_json::from_str(json)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_message_roundtrip_msgpack() {
        let msg = ClientMessage::Ping { timestamp: 12345 };
        let bytes = serialize_msgpack(&msg).unwrap();
        let decoded: ClientMessage = deserialize_msgpack(&bytes).unwrap();
        match decoded {
            ClientMessage::Ping { timestamp } => assert_eq!(timestamp, 12345),
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_server_message_roundtrip_json() {
        let msg = ServerMessage::Pong {
            timestamp: 100,
            server_time: 200,
        };
        let json = serialize_json(&msg).unwrap();
        let decoded: ServerMessage = deserialize_json(&json).unwrap();
        match decoded {
            ServerMessage::Pong {
                timestamp,
                server_time,
            } => {
                assert_eq!(timestamp, 100);
                assert_eq!(server_time, 200);
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_auth_request_serialize() {
        let auth = ClientMessage::Auth(AuthRequest::Login {
            username: "player1".to_string(),
            password: "secret".to_string(),
        });
        let bytes = serialize_msgpack(&auth).unwrap();
        assert!(!bytes.is_empty());
        let decoded: ClientMessage = deserialize_msgpack(&bytes).unwrap();
        match decoded {
            ClientMessage::Auth(AuthRequest::Login { username, .. }) => {
                assert_eq!(username, "player1");
            }
            _ => panic!("Wrong variant"),
        }
    }
}
