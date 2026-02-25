use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::Instant;
use serde::Serialize;
use tokio::sync::{broadcast, Mutex, RwLock};
use uuid::Uuid;

use gt_common::protocol::ServerMessage;
use gt_common::types::{EntityId, WorldConfig};
use gt_simulation::world::GameWorld;

use crate::auth::AuthConfig;
use crate::db::Database;

/// Shared database handle (wrapped in Arc for cheap cloning across tasks)
pub type SharedDb = Option<Arc<Database>>;

/// A player connected to the server
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ConnectedPlayer {
    pub id: Uuid,
    pub username: String,
    pub is_guest: bool,
    pub is_admin: bool,
    pub is_spectator: bool,
    pub world_id: Option<Uuid>,
    pub corp_id: Option<EntityId>,
}

/// A single entry in the server audit log
#[derive(Debug, Clone, Serialize)]
pub struct AuditEntry {
    pub tick: u64,
    pub player_id: Uuid,
    pub command_type: String,
    pub timestamp: u64,
}

/// A running game world instance
#[allow(dead_code)]
pub struct WorldInstance {
    pub id: Uuid,
    pub name: String,
    pub world: Mutex<GameWorld>,
    pub config: WorldConfig,
    pub broadcast_tx: broadcast::Sender<ServerMessage>,
    pub players: RwLock<HashMap<Uuid, EntityId>>, // player_id → corp_id
    pub max_players: u32,
    pub tick_rate_ms: u64,
    /// Player who created the world (has override power for speed)
    pub creator_id: RwLock<Option<Uuid>>,
    /// Speed votes: player_id → requested speed string
    pub speed_votes: RwLock<HashMap<Uuid, String>>,
    /// Banned players (by account ID)
    pub banned_players: RwLock<std::collections::HashSet<Uuid>>,
}

impl WorldInstance {
    pub fn new(id: Uuid, name: String, config: WorldConfig, max_players: u32) -> Self {
        let world = GameWorld::new(config.clone());
        let (broadcast_tx, _) = broadcast::channel(256);
        Self {
            id,
            name,
            world: Mutex::new(world),
            config,
            broadcast_tx,
            players: RwLock::new(HashMap::new()),
            max_players,
            tick_rate_ms: 1000, // Default 1 tick/sec
            creator_id: RwLock::new(None),
            speed_votes: RwLock::new(HashMap::new()),
            banned_players: RwLock::new(std::collections::HashSet::new()),
        }
    }

    pub async fn player_count(&self) -> usize {
        self.players.read().await.len()
    }

    pub async fn is_full(&self) -> bool {
        self.player_count().await >= self.max_players as usize
    }

    pub async fn add_player(&self, player_id: Uuid, corp_id: EntityId) {
        self.players.write().await.insert(player_id, corp_id);
    }

    pub async fn remove_player(&self, player_id: &Uuid) -> Option<EntityId> {
        self.players.write().await.remove(player_id)
    }
}

/// In-memory account store (replace with PostgreSQL in production)
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct AccountRecord {
    pub id: Uuid,
    pub username: String,
    pub email: Option<String>,
    pub password_hash: String,
    pub is_guest: bool,
}

/// Global server state shared across all handlers
#[allow(dead_code)]
pub struct AppState {
    pub auth_config: AuthConfig,
    pub worlds: RwLock<HashMap<Uuid, Arc<WorldInstance>>>,
    pub players: RwLock<HashMap<Uuid, ConnectedPlayer>>,
    pub accounts: RwLock<HashMap<String, AccountRecord>>, // username → account
    pub audit_log: RwLock<Vec<AuditEntry>>,
    pub ip_connections: RwLock<HashMap<IpAddr, usize>>,
    pub db: SharedDb,
    pub started_at: Instant,
}

impl AppState {
    pub fn new(auth_config: AuthConfig, db: Option<Database>) -> Self {
        Self {
            auth_config,
            worlds: RwLock::new(HashMap::new()),
            players: RwLock::new(HashMap::new()),
            accounts: RwLock::new(HashMap::new()),
            audit_log: RwLock::new(Vec::new()),
            ip_connections: RwLock::new(HashMap::new()),
            db: db.map(Arc::new),
            started_at: Instant::now(),
        }
    }

    /// Increment the connection count for an IP. Returns the new count.
    pub async fn ip_connect(&self, ip: IpAddr) -> usize {
        let mut conns = self.ip_connections.write().await;
        let count = conns.entry(ip).or_insert(0);
        *count += 1;
        *count
    }

    /// Decrement the connection count for an IP.
    pub async fn ip_disconnect(&self, ip: IpAddr) {
        let mut conns = self.ip_connections.write().await;
        if let Some(count) = conns.get_mut(&ip) {
            *count = count.saturating_sub(1);
            if *count == 0 {
                conns.remove(&ip);
            }
        }
    }

    /// Create a new game world and start its tick loop
    pub async fn create_world(&self, name: String, config: WorldConfig, max_players: u32) -> Uuid {
        let id = Uuid::new_v4();
        let instance = Arc::new(WorldInstance::new(id, name, config, max_players));
        self.worlds.write().await.insert(id, instance);
        id
    }

    /// Get a world instance by ID
    pub async fn get_world(&self, world_id: &Uuid) -> Option<Arc<WorldInstance>> {
        self.worlds.read().await.get(world_id).cloned()
    }

    /// List all active worlds
    pub async fn list_worlds(&self) -> Vec<gt_common::protocol::WorldInfo> {
        let worlds = self.worlds.read().await;
        let mut result = Vec::new();
        for (_, instance) in worlds.iter() {
            let world = instance.world.lock().await;
            result.push(gt_common::protocol::WorldInfo {
                id: instance.id,
                name: instance.name.clone(),
                player_count: instance.player_count().await as u32,
                max_players: instance.max_players,
                tick: world.current_tick(),
                speed: world.speed(),
                era: world.config().starting_era,
                map_size: world.config().map_size,
            });
        }
        result
    }

    /// Register a new account
    pub async fn register_account(
        &self,
        username: String,
        email: Option<String>,
        password_hash: String,
    ) -> Result<AccountRecord, String> {
        // Try database first
        #[cfg(feature = "postgres")]
        if let Some(db) = self.db.as_ref() {
            let id = db
                .create_account(
                    &username,
                    email.as_deref(),
                    &password_hash,
                    false,
                )
                .await
                .map_err(|e| format!("Database error: {e}"))?;

            let record = AccountRecord {
                id,
                username: username.clone(),
                email,
                password_hash,
                is_guest: false,
            };
            // Cache in memory too
            self.accounts.write().await.insert(username, record.clone());
            return Ok(record);
        }

        // Fallback to in-memory
        let mut accounts = self.accounts.write().await;
        if accounts.contains_key(&username) {
            return Err("Username already taken".to_string());
        }
        let record = AccountRecord {
            id: Uuid::new_v4(),
            username: username.clone(),
            email,
            password_hash,
            is_guest: false,
        };
        accounts.insert(username, record.clone());
        Ok(record)
    }

    /// Look up an account by username
    pub async fn get_account(&self, username: &str) -> Option<AccountRecord> {
        // Check in-memory cache first
        if let Some(record) = self.accounts.read().await.get(username).cloned() {
            return Some(record);
        }

        // Try database
        #[cfg(feature = "postgres")]
        if let Some(db) = self.db.as_ref() {
            if let Ok(Some(row)) = db.get_account_by_username(username).await {
                let record = AccountRecord {
                    id: row.id,
                    username: row.username.clone(),
                    email: row.email,
                    password_hash: row.password_hash,
                    is_guest: row.is_guest,
                };
                // Cache it
                self.accounts.write().await.insert(row.username, record.clone());
                return Some(record);
            }
        }

        None
    }

    /// Register a guest account
    pub async fn register_guest(&self) -> AccountRecord {
        let id = Uuid::new_v4();
        let username = format!("Guest_{}", &id.to_string()[..8]);

        // Try database first
        #[cfg(feature = "postgres")]
        if let Some(db) = self.db.as_ref() {
            if let Ok(db_id) = db.create_account(&username, None, "", true).await {
                let record = AccountRecord {
                    id: db_id,
                    username: username.clone(),
                    email: None,
                    password_hash: String::new(),
                    is_guest: true,
                };
                self.accounts.write().await.insert(username, record.clone());
                return record;
            }
        }

        let record = AccountRecord {
            id,
            username: username.clone(),
            email: None,
            password_hash: String::new(),
            is_guest: true,
        };
        self.accounts.write().await.insert(username, record.clone());
        record
    }

    /// Log a player command to the audit log
    pub async fn log_command(&self, player_id: Uuid, command_type: String, tick: u64) {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let entry = AuditEntry {
            tick,
            player_id,
            command_type,
            timestamp,
        };
        self.audit_log.write().await.push(entry);
    }

    /// Return a clone of the entire audit log
    pub async fn get_audit_log(&self) -> Vec<AuditEntry> {
        self.audit_log.read().await.clone()
    }

    /// Kick a player by removing them from the connected players map.
    /// Returns true if the player was found and removed.
    pub async fn kick_player(&self, player_id: &Uuid) -> bool {
        self.players.write().await.remove(player_id).is_some()
    }

    /// Remove a world instance. Returns true if the world existed.
    pub async fn remove_world(&self, world_id: &Uuid) -> bool {
        self.worlds.write().await.remove(world_id).is_some()
    }

    /// Return the server uptime in seconds.
    pub fn uptime_secs(&self) -> u64 {
        self.started_at.elapsed().as_secs()
    }

    /// Send a broadcast message to all players in a specific world.
    pub async fn broadcast_to_world(&self, world_id: &Uuid, msg: ServerMessage) -> bool {
        if let Some(world) = self.get_world(world_id).await {
            let _ = world.broadcast_tx.send(msg);
            true
        } else {
            false
        }
    }
}
