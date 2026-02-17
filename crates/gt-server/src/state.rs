use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex, RwLock};
use uuid::Uuid;

use gt_common::protocol::ServerMessage;
use gt_common::types::{EntityId, WorldConfig};
use gt_simulation::world::GameWorld;

use crate::auth::AuthConfig;

/// A player connected to the server
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ConnectedPlayer {
    pub id: Uuid,
    pub username: String,
    pub is_guest: bool,
    pub world_id: Option<Uuid>,
    pub corp_id: Option<EntityId>,
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
pub struct AppState {
    pub auth_config: AuthConfig,
    pub worlds: RwLock<HashMap<Uuid, Arc<WorldInstance>>>,
    pub players: RwLock<HashMap<Uuid, ConnectedPlayer>>,
    pub accounts: RwLock<HashMap<String, AccountRecord>>, // username → account
}

impl AppState {
    pub fn new(auth_config: AuthConfig) -> Self {
        Self {
            auth_config,
            worlds: RwLock::new(HashMap::new()),
            players: RwLock::new(HashMap::new()),
            accounts: RwLock::new(HashMap::new()),
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
        self.accounts.read().await.get(username).cloned()
    }

    /// Register a guest account
    pub async fn register_guest(&self) -> AccountRecord {
        let id = Uuid::new_v4();
        let username = format!("Guest_{}", &id.to_string()[..8]);
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
}
