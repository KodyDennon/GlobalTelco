// ── Server Health ────────────────────────────────────────────────────────

export interface ServerHealth {
	version: string;
	uptime_secs: number;
	active_worlds: number;
	connected_players: number;
	registered_accounts: number;
	audit_log_entries: number;
	worlds: WorldSummary[];
	has_database: boolean;
}

export interface WorldSummary {
	id: string;
	name: string;
	tick: number;
	speed: string;
	player_count: number;
	max_players: number;
	tick_rate_ms: number;
	era: string;
	map_size: string;
}

// ── Players ─────────────────────────────────────────────────────────────

export interface PlayerInfo {
	id: string;
	username: string;
	is_guest: boolean;
	is_admin: boolean;
	is_spectator?: boolean;
	world_id: string | null;
	corp_id: number | null;
}

export interface ConnectionInfo {
	id: string;
	username: string;
	world_id: string | null;
	world_name: string | null;
	corp_id: number | null;
	is_guest: boolean;
	is_spectator: boolean;
	connected_at: number;
}

export interface AccountInfo {
	id: string;
	username: string;
	email: string | null;
	display_name: string | null;
	avatar_id: string | null;
	auth_provider: string | null;
	is_guest: boolean;
	is_banned?: boolean;
	created_at: string;
	last_login: string | null;
	deleted_at: string | null;
}

export interface AccountListResponse {
	accounts: AccountInfo[];
	total: number;
	page: number;
	per_page: number;
}

// ── Worlds ──────────────────────────────────────────────────────────────

export interface WorldConfig {
	seed?: number;
	starting_era?: string;
	difficulty?: string;
	map_size?: string;
	ai_corporations?: number;
	max_ai_corporations?: number;
	use_real_earth?: boolean;
	continent_count?: number;
	ocean_percentage?: number;
	terrain_roughness?: number;
	climate_variation?: number;
	city_density?: number;
	disaster_frequency?: number;
	sandbox?: boolean;
}

export interface WorldDebug {
	world_id: string;
	world_name: string;
	tick: number;
	speed: string;
	tick_rate_ms: number;
	broadcast_subscribers: number;
	corporations: CorpSummary[];
	connected_players: PlayerInfo[];
	entity_counts: Record<string, number>;
}

export interface CorpSummary {
	corp_id: number;
	name: string;
	cash: number | null;
	revenue: number | null;
	cost: number | null;
	debt: number | null;
	nodes: number;
}

// ── Templates ───────────────────────────────────────────────────────────

export interface WorldTemplate {
	id: string;
	name: string;
	description: string;
	icon: string;
	config_defaults: Record<string, unknown>;
	config_bounds: Record<string, unknown>;
	max_instances: number;
	enabled: boolean;
	sort_order: number;
}

// ── Bans ────────────────────────────────────────────────────────────────

export interface Ban {
	id: string;
	account_id: string;
	username: string;
	world_id: string | null;
	reason: string;
	banned_at: string;
	expires_at: string | null;
}

// ── Audit ───────────────────────────────────────────────────────────────

export interface AuditEntry {
	id: number | string;
	actor: string;
	action: string;
	target: string | null;
	details: Record<string, unknown> | null;
	ip_address: string | null;
	created_at: string;
}

// ── Metrics ─────────────────────────────────────────────────────────────

export interface ServerMetrics {
	server: {
		uptime_secs: number;
		connected_players: number;
		world_count: number;
		memory_estimate_bytes: number;
		memory_mb: number;
		ws_messages_per_sec: number;
	};
	worlds: WorldMetrics[];
}

export interface WorldMetrics {
	id: string;
	name: string;
	tick: number;
	speed: string;
	player_count: number;
	max_players: number;
	config: {
		starting_era: string;
		map_size: string;
		ai_corporations: number;
		sandbox: boolean;
	};
	last_tick_us: number;
	avg_tick_us: number;
	max_tick_us: number;
	p99_tick_us: number;
	entity_count: number;
	broadcast_subscribers: number;
	tick_history: number[];
	system_times: Record<string, number>;
}

// ── Chat ────────────────────────────────────────────────────────────────

export interface ChatMessage {
	id: number;
	username: string;
	message: string;
	created_at: string;
}

// ── Speed Votes ─────────────────────────────────────────────────────────

export interface SpeedVotes {
	votes: Record<string, string>;
	current_speed: string;
	creator_id: string | null;
}

// ── Reset Queue ─────────────────────────────────────────────────────────

export interface ResetRequest {
	id: string;
	account_id: string;
	username: string;
	status: string;
	created_at: string;
}

// ── Server Config ───────────────────────────────────────────────────────

export interface ServerConfig {
	env_vars: Record<string, boolean>;
	database: {
		connected: boolean;
		pool_size: number;
	};
	features: {
		postgres: boolean;
		r2: boolean;
	};
}
