use std::collections::{HashMap, VecDeque};
use std::net::IpAddr;
use std::sync::atomic::{AtomicU64, Ordering};
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
use crate::oauth::OAuthConfig;
#[cfg(feature = "r2")]
use crate::r2::R2Storage;

/// Shared database handle (wrapped in Arc for cheap cloning across tasks)
pub type SharedDb = Option<Arc<Database>>;

/// A player connected to the server
#[derive(Debug, Clone)]
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
    pub id: u64,
    pub actor: String,
    pub action: String,
    pub target: Option<String>,
    pub details: Option<serde_json::Value>,
    pub ip_address: Option<String>,
    pub created_at: String, // ISO 8601
}

/// Online presence for a connected player
#[derive(Debug, Clone, Serialize)]
pub struct OnlinePresence {
    pub username: String,
    pub display_name: Option<String>,
    pub avatar_id: String,
    pub world_id: Option<Uuid>,
    pub world_name: Option<String>,
    pub connected_at: u64, // unix timestamp (Instant is not Serialize)
}

/// A running game world instance
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
    /// Banned players (by account ID) — in-memory for backward compat
    pub banned_players: RwLock<std::collections::HashSet<Uuid>>,
    /// Template that spawned this world (if any)
    pub template_id: Option<Uuid>,
    /// Invite code for joining
    pub invite_code: Option<String>,
    /// Tick profiling: last tick duration in microseconds
    pub last_tick_duration_us: AtomicU64,
    /// Tick profiling: rolling average tick duration in microseconds
    pub avg_tick_duration_us: AtomicU64,
    /// Last 60 tick durations (microseconds)
    pub tick_history: RwLock<VecDeque<u64>>,
    /// Max tick duration seen in last 60 ticks
    pub max_tick_us: AtomicU64,
    /// P99 tick duration from last 100 ticks
    pub p99_tick_us: AtomicU64,
    /// Raw samples for p99 computation (last 100 ticks)
    pub tick_samples: Mutex<VecDeque<u64>>,
}

impl WorldInstance {
    pub fn new(id: Uuid, name: String, config: WorldConfig, max_players: u32) -> Self {
        let world = GameWorld::new(config.clone());
        let (broadcast_tx, _) = broadcast::channel(128);
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
            template_id: None,
            invite_code: None,
            last_tick_duration_us: AtomicU64::new(0),
            avg_tick_duration_us: AtomicU64::new(0),
            tick_history: RwLock::new(VecDeque::new()),
            max_tick_us: AtomicU64::new(0),
            p99_tick_us: AtomicU64::new(0),
            tick_samples: Mutex::new(VecDeque::new()),
        }
    }

    pub fn new_with_template(
        id: Uuid,
        name: String,
        config: WorldConfig,
        max_players: u32,
        template_id: Option<Uuid>,
        invite_code: Option<String>,
    ) -> Self {
        let mut instance = Self::new(id, name, config, max_players);
        instance.template_id = template_id;
        instance.invite_code = invite_code;
        instance
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

    /// Record tick duration and update rolling average, tick_history, max, and p99
    pub async fn record_tick_duration(&self, duration_us: u64) {
        self.last_tick_duration_us.store(duration_us, Ordering::Relaxed);
        let prev = self.avg_tick_duration_us.load(Ordering::Relaxed);
        // Exponential moving average (alpha = 0.1)
        let new_avg = if prev == 0 {
            duration_us
        } else {
            (prev * 9 + duration_us) / 10
        };
        self.avg_tick_duration_us.store(new_avg, Ordering::Relaxed);

        // Update tick_history (cap 60) and recompute max
        {
            let mut history = self.tick_history.write().await;
            history.push_back(duration_us);
            if history.len() > 60 {
                history.pop_front();
            }
            let max = history.iter().copied().max().unwrap_or(0);
            self.max_tick_us.store(max, Ordering::Relaxed);
        }

        // Update tick_samples (cap 100) and recompute p99
        {
            let mut samples = self.tick_samples.lock().await;
            samples.push_back(duration_us);
            if samples.len() > 100 {
                samples.pop_front();
            }
            // Compute p99: sort a copy and take the 99th percentile
            let mut sorted: Vec<u64> = samples.iter().copied().collect();
            sorted.sort_unstable();
            let idx = ((sorted.len() as f64 * 0.99).ceil() as usize).saturating_sub(1).min(sorted.len().saturating_sub(1));
            let p99 = sorted.get(idx).copied().unwrap_or(0);
            self.p99_tick_us.store(p99, Ordering::Relaxed);
        }
    }
}

/// In-memory account store (replace with PostgreSQL in production)
#[derive(Debug, Clone)]
pub struct AccountRecord {
    pub id: Uuid,
    pub username: String,
    pub email: Option<String>,
    pub password_hash: String,
    pub is_guest: bool,
    pub display_name: Option<String>,
    pub avatar_id: String,
    pub auth_provider: String,
    pub github_id: Option<i64>,
}

/// Global server state shared across all handlers
pub struct AppState {
    pub auth_config: AuthConfig,
    pub worlds: RwLock<HashMap<Uuid, Arc<WorldInstance>>>,
    pub players: RwLock<HashMap<Uuid, ConnectedPlayer>>,
    pub accounts: RwLock<HashMap<String, AccountRecord>>, // username → account
    pub audit_log: RwLock<Vec<AuditEntry>>,
    pub ip_connections: RwLock<HashMap<IpAddr, usize>>,
    pub db: SharedDb,
    pub started_at: Instant,
    /// Online player presence: account_id -> presence info
    pub online_players: RwLock<HashMap<Uuid, OnlinePresence>>,
    /// OAuth configuration (optional)
    pub oauth_config: Option<OAuthConfig>,
    /// Cloudflare Worker URL for password reset emails
    pub cf_reset_worker_url: Option<String>,
    /// Cloudflare R2 object storage (optional)
    #[cfg(feature = "r2")]
    pub r2: Option<Arc<R2Storage>>,
    /// Total WebSocket messages received
    pub ws_message_count: AtomicU64,
    /// Snapshot of ws_message_count for rate calculation
    pub ws_count_snapshot: AtomicU64,
    /// Time of last rate snapshot
    pub ws_snapshot_time: Mutex<Instant>,
    /// Counter for in-memory audit entry IDs
    pub id_counter: AtomicU64,
    /// Maximum number of active worlds allowed on this server
    pub max_active_worlds: AtomicU64,
    /// Maximum number of worlds a single player can create
    pub max_worlds_per_player: AtomicU64,
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
            online_players: RwLock::new(HashMap::new()),
            oauth_config: None,
            cf_reset_worker_url: None,
            #[cfg(feature = "r2")]
            r2: None,
            ws_message_count: AtomicU64::new(0),
            ws_count_snapshot: AtomicU64::new(0),
            ws_snapshot_time: Mutex::new(Instant::now()),
            id_counter: AtomicU64::new(1),
            max_active_worlds: AtomicU64::new(
                std::env::var("MAX_ACTIVE_WORLDS")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(10),
            ),
            max_worlds_per_player: AtomicU64::new(
                std::env::var("MAX_WORLDS_PER_PLAYER")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(2),
            ),
        }
    }

    pub fn with_oauth(mut self, oauth: Option<OAuthConfig>) -> Self {
        self.oauth_config = oauth;
        self
    }

    pub fn with_cf_reset_url(mut self, url: Option<String>) -> Self {
        self.cf_reset_worker_url = url;
        self
    }

    #[cfg(feature = "r2")]
    pub fn with_r2(mut self, r2: Option<R2Storage>) -> Self {
        self.r2 = r2.map(Arc::new);
        self
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
        let instance = Arc::new(WorldInstance::new(id, name.clone(), config.clone(), max_players));

        // Persist to database if available
        #[cfg(feature = "postgres")]
        if let Some(db) = self.db.as_ref() {
            let config_json = serde_json::to_value(&config).unwrap_or_default();
            if let Err(e) = db.save_world(id, &name, &config_json, 0, "Paused", max_players as i32).await {
                tracing::error!("Failed to persist new world '{}' to database: {}", name, e);
            }
        }

        self.worlds.write().await.insert(id, instance);
        id
    }

    /// Create a new game world from a template
    pub async fn create_world_from_template(
        &self,
        name: String,
        config: WorldConfig,
        max_players: u32,
        template_id: Option<Uuid>,
        invite_code: Option<String>,
    ) -> Uuid {
        let id = Uuid::new_v4();
        let instance = Arc::new(WorldInstance::new_with_template(
            id,
            name.clone(),
            config.clone(),
            max_players,
            template_id,
            invite_code,
        ));

        // Persist to database if available
        #[cfg(feature = "postgres")]
        if let Some(db) = self.db.as_ref() {
            let config_json = serde_json::to_value(&config).unwrap_or_default();
            if let Err(e) = db.save_world(id, &name, &config_json, 0, "Paused", max_players as i32).await {
                tracing::error!("Failed to persist new template world '{}' to database: {}", name, e);
            }
        }

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
                display_name: None,
                avatar_id: "tower_01".to_string(),
                auth_provider: "local".to_string(),
                github_id: None,
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
            display_name: None,
            avatar_id: "tower_01".to_string(),
            auth_provider: "local".to_string(),
            github_id: None,
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
                    display_name: row.display_name,
                    avatar_id: row.avatar_id.unwrap_or_else(|| "tower_01".to_string()),
                    auth_provider: row.auth_provider.unwrap_or_else(|| "local".to_string()),
                    github_id: row.github_id,
                };
                // Cache it
                self.accounts.write().await.insert(row.username, record.clone());
                return Some(record);
            }
        }

        None
    }

    /// Look up an account by GitHub ID
    pub async fn get_account_by_github_id(&self, github_id: i64) -> Option<AccountRecord> {
        // Check in-memory cache
        {
            let accounts = self.accounts.read().await;
            for record in accounts.values() {
                if record.github_id == Some(github_id) {
                    return Some(record.clone());
                }
            }
        }

        // Try database
        #[cfg(feature = "postgres")]
        if let Some(db) = self.db.as_ref() {
            if let Ok(Some(row)) = db.get_account_by_github_id(github_id).await {
                let record = AccountRecord {
                    id: row.id,
                    username: row.username.clone(),
                    email: row.email,
                    password_hash: row.password_hash,
                    is_guest: row.is_guest,
                    display_name: row.display_name,
                    avatar_id: row.avatar_id.unwrap_or_else(|| "tower_01".to_string()),
                    auth_provider: row.auth_provider.unwrap_or_else(|| "local".to_string()),
                    github_id: row.github_id,
                };
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
                    display_name: None,
                    avatar_id: "tower_01".to_string(),
                    auth_provider: "guest".to_string(),
                    github_id: None,
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
            display_name: None,
            avatar_id: "tower_01".to_string(),
            auth_provider: "guest".to_string(),
            github_id: None,
        };
        self.accounts.write().await.insert(username, record.clone());
        record
    }

    /// Maximum in-memory audit log entries (overflow to DB if available)
    const MAX_AUDIT_LOG_ENTRIES: usize = 1000;

    /// Log a player command to the audit log
    pub async fn log_command(&self, player_id: Uuid, command_type: String, _tick: u64) {
        let id = self.id_counter.fetch_add(1, Ordering::Relaxed);
        let created_at = chrono::Utc::now().to_rfc3339();
        let entry = AuditEntry {
            id,
            actor: player_id.to_string(),
            action: command_type,
            target: None,
            details: None,
            ip_address: None,
            created_at,
        };
        let mut log = self.audit_log.write().await;
        log.push(entry);
        // Cap in-memory audit log to prevent unbounded growth
        if log.len() > Self::MAX_AUDIT_LOG_ENTRIES {
            let drain_count = log.len() - Self::MAX_AUDIT_LOG_ENTRIES;
            log.drain(..drain_count);
        }
    }

    /// Compute WebSocket messages per second since last snapshot
    pub async fn ws_messages_per_sec(&self) -> f64 {
        let current = self.ws_message_count.load(Ordering::Relaxed);
        let prev = self.ws_count_snapshot.load(Ordering::Relaxed);
        let mut last_time = self.ws_snapshot_time.lock().await;
        let elapsed = last_time.elapsed().as_secs_f64();
        if elapsed < 1.0 {
            // Don't update snapshot too frequently, just compute from current values
            if elapsed > 0.0 {
                return (current - prev) as f64 / elapsed;
            }
            return 0.0;
        }
        let rate = (current - prev) as f64 / elapsed;
        self.ws_count_snapshot.store(current, Ordering::Relaxed);
        *last_time = Instant::now();
        rate
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

    /// Return the number of active worlds.
    pub async fn active_world_count(&self) -> usize {
        self.worlds.read().await.len()
    }

    /// Remove a world instance. Returns true if the world existed.
    pub async fn remove_world(&self, world_id: &Uuid) -> bool {
        self.worlds.write().await.remove(world_id).is_some()
    }

    /// Return the server uptime in seconds.
    pub fn uptime_secs(&self) -> u64 {
        self.started_at.elapsed().as_secs()
    }

    /// Estimate memory usage of server state in bytes.
    pub async fn memory_usage_estimate(&self) -> u64 {
        let mut bytes: u64 = 0;
        // Accounts: ~256 bytes each
        bytes += self.accounts.read().await.len() as u64 * 256;
        // Players: ~128 bytes each
        bytes += self.players.read().await.len() as u64 * 128;
        // Audit log: ~128 bytes each
        bytes += self.audit_log.read().await.len() as u64 * 128;
        // Online presence: ~256 bytes each
        bytes += self.online_players.read().await.len() as u64 * 256;
        // IP connections: ~32 bytes each
        bytes += self.ip_connections.read().await.len() as u64 * 32;
        bytes
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

/// Generate a random 8-character alphanumeric invite code
pub fn generate_invite_code() -> String {
    use rand::Rng;
    let mut rng = rand::rng();
    (0..8)
        .map(|_| {
            let idx: u8 = rng.random_range(0..36);
            if idx < 10 {
                (b'0' + idx) as char
            } else {
                (b'A' + idx - 10) as char
            }
        })
        .collect()
}
