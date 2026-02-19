use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::commands::Command;
use crate::events::GameEvent;
use crate::types::{EntityId, GameSpeed, Money, Tick};

/// Serde helper: always serialize/deserialize Uuid as a string.
/// MessagePack (non-human-readable) would otherwise use 16-byte binary,
/// which JS clients can't produce — they send UUID strings like "2391ef0a-...".
mod uuid_string {
    use serde::{self, Deserialize, Deserializer, Serializer};
    use uuid::Uuid;

    pub fn serialize<S>(uuid: &Uuid, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&uuid.to_string())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Uuid, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Uuid::parse_str(&s).map_err(serde::de::Error::custom)
    }
}

// ── Client → Server Messages ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientMessage {
    /// Authenticate with the server
    Auth(AuthRequest),
    /// Send a game command
    GameCommand {
        #[serde(with = "uuid_string")]
        world_id: Uuid,
        command: Command,
    },
    /// Request a state snapshot
    RequestSnapshot {
        #[serde(with = "uuid_string")]
        world_id: Uuid,
    },
    /// Join a game world
    JoinWorld {
        #[serde(with = "uuid_string")]
        world_id: Uuid,
    },
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
        #[serde(with = "uuid_string")]
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
        #[serde(with = "uuid_string")]
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
        #[serde(with = "uuid_string")]
        player_id: Uuid,
        username: String,
        access_token: String,
        refresh_token: String,
    },
    GuestSuccess {
        #[serde(with = "uuid_string")]
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
    #[serde(with = "uuid_string")]
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
    fn test_uuid_roundtrip_msgpack() {
        // Verify UUIDs survive MessagePack roundtrip as strings (not 16-byte binary)
        let id = Uuid::parse_str("2391ef0a-8c1f-4f2e-a784-6a55a0e5b8a5").unwrap();
        let msg = ClientMessage::JoinWorld { world_id: id };
        let bytes = serialize_msgpack(&msg).unwrap();
        let decoded: ClientMessage = deserialize_msgpack(&bytes).unwrap();
        match decoded {
            ClientMessage::JoinWorld { world_id } => assert_eq!(world_id, id),
            _ => panic!("Wrong variant"),
        }

        // Verify the bytes contain the UUID string (not binary)
        let bytes_str = String::from_utf8_lossy(&bytes);
        assert!(
            bytes_str.contains("2391ef0a"),
            "UUID should be serialized as string in msgpack"
        );
    }

    #[test]
    fn test_uuid_from_js_string_msgpack() {
        // Simulate what JS client sends: UUID as a plain string in MessagePack
        // JS: encode({ JoinWorld: { world_id: "2391ef0a-8c1f-4f2e-a784-6a55a0e5b8a5" } })
        let js_style = serde_json::json!({
            "JoinWorld": {
                "world_id": "2391ef0a-8c1f-4f2e-a784-6a55a0e5b8a5"
            }
        });
        // Serialize to msgpack as JS would
        let bytes = rmp_serde::to_vec(&js_style).unwrap();
        // Deserialize as Rust expects
        let decoded: ClientMessage = deserialize_msgpack(&bytes).unwrap();
        match decoded {
            ClientMessage::JoinWorld { world_id } => {
                assert_eq!(
                    world_id,
                    Uuid::parse_str("2391ef0a-8c1f-4f2e-a784-6a55a0e5b8a5").unwrap()
                );
            }
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
