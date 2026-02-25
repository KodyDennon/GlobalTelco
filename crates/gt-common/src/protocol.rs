use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::commands::Command;
use crate::events::GameEvent;
use crate::types::{EdgeType, EntityId, GameSpeed, Money, NetworkLevel, NodeType, Tick};

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
        /// Client-assigned sequence number for correlation
        #[serde(default)]
        seq: Option<u64>,
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
        #[serde(default)]
        spectator: bool,
    },
    Register {
        username: String,
        password: String,
        email: String,
        #[serde(default)]
        spectator: bool,
    },
    Token {
        access_token: String,
        #[serde(default)]
        spectator: bool,
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
    /// Command acknowledged (enriched with entity ID and tick)
    CommandAck {
        success: bool,
        error: Option<String>,
        /// Echo of client-assigned sequence number
        #[serde(skip_serializing_if = "Option::is_none")]
        seq: Option<u64>,
        /// Entity created/affected by the command (if applicable)
        #[serde(skip_serializing_if = "Option::is_none")]
        entity_id: Option<EntityId>,
        /// Tick at which the command took effect
        #[serde(skip_serializing_if = "Option::is_none")]
        effective_tick: Option<Tick>,
    },
    /// Broadcast of a command's visible effects to all players
    CommandBroadcast {
        tick: Tick,
        /// Corp that executed the command
        corp_id: EntityId,
        /// Delta operations describing what changed
        ops: Vec<DeltaOp>,
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
    /// Speed vote update broadcast
    SpeedVoteUpdate {
        /// Current tally of speed votes
        votes: Vec<SpeedVoteEntry>,
        /// The resolved speed (majority or creator override)
        resolved_speed: GameSpeed,
    },
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
    // Operational data (intel level 3 only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avg_utilization: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avg_health: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_throughput: Option<f64>,
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

/// Entry in a speed vote tally
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeedVoteEntry {
    pub username: String,
    pub speed: GameSpeed,
}

// ── Command Result (returned by simulation) ──────────────────────────────

/// Result of processing a game command in the simulation.
/// Contains success/failure plus any entities created and delta ops for broadcast.
#[derive(Debug, Clone)]
pub struct CommandResult {
    pub success: bool,
    pub error: Option<String>,
    /// Primary entity created or affected
    pub entity_id: Option<EntityId>,
    /// Delta operations for broadcasting to other players
    pub ops: Vec<DeltaOp>,
}

impl CommandResult {
    pub fn ok() -> Self {
        Self {
            success: true,
            error: None,
            entity_id: None,
            ops: Vec::new(),
        }
    }

    pub fn ok_with_entity(entity_id: EntityId) -> Self {
        Self {
            success: true,
            error: None,
            entity_id: Some(entity_id),
            ops: Vec::new(),
        }
    }

    pub fn fail(error: impl Into<String>) -> Self {
        Self {
            success: false,
            error: Some(error.into()),
            entity_id: None,
            ops: Vec::new(),
        }
    }

    pub fn with_op(mut self, op: DeltaOp) -> Self {
        self.ops.push(op);
        self
    }
}

// ── Delta Operations (for CommandBroadcast) ──────────────────────────────

/// A single delta operation describing a visible change to the world.
/// These are broadcast to all players so they can update their local state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeltaOp {
    /// A new infrastructure node was created
    NodeCreated {
        entity_id: EntityId,
        owner: EntityId,
        node_type: NodeType,
        network_level: NetworkLevel,
        lon: f64,
        lat: f64,
        /// Whether the node is still under construction
        under_construction: bool,
    },
    /// A new infrastructure edge was created
    EdgeCreated {
        entity_id: EntityId,
        owner: EntityId,
        edge_type: EdgeType,
        from_node: EntityId,
        to_node: EntityId,
    },
    /// A node was upgraded
    NodeUpgraded {
        entity_id: EntityId,
        node_type: NodeType,
    },
    /// A node was decommissioned / removed
    NodeRemoved {
        entity_id: EntityId,
    },
    /// An edge was removed
    EdgeRemoved {
        entity_id: EntityId,
    },
    /// A node's construction completed
    ConstructionCompleted {
        entity_id: EntityId,
    },
}

// ── Serialization Helpers ──────────────────────────────────────────────────

/// Serialize a message to MessagePack bytes.
/// Uses `to_vec_named` so struct fields are serialized as maps with field names,
/// not positional arrays. This is required because JS clients access fields by
/// name (e.g., `msg.WorldJoined.world_id`), not by index.
pub fn serialize_msgpack<T: Serialize>(msg: &T) -> Result<Vec<u8>, rmp_serde::encode::Error> {
    rmp_serde::to_vec_named(msg)
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
            spectator: false,
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
