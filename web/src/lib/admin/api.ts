import { API_URL } from '$lib/config';

// ── Types ─────────────────────────────────────────────────────────────────

export interface ServerInfo {
	version: string;
	active_worlds: number;
	connected_players: number;
}

export interface ServerHealth {
	version: string;
	uptime_secs: number;
	active_worlds: number;
	connected_players: number;
	registered_accounts: number;
	audit_log_entries: number;
	has_database: boolean;
	worlds: WorldDetail[];
}

export interface WorldDetail {
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

export interface PlayerInfo {
	id: string;
	username: string;
	is_guest: boolean;
	is_admin: boolean;
	world_id: string | null;
	corp_id: number | null;
}

export interface WorldInfo {
	id: string;
	name: string;
	player_count: number;
	max_players: number;
	tick: number;
	speed: string;
	era: string;
	map_size: string;
}

export interface AuditEntry {
	tick: number;
	player_id: string;
	command_type: string;
	timestamp: number;
}

export interface PauseResult {
	world_id: string;
	paused: boolean;
	speed: string;
}

export interface SpeedResult {
	world_id: string;
	speed: string;
	paused: boolean;
}

// ── Helpers ───────────────────────────────────────────────────────────────

function adminHeaders(key: string): Record<string, string> {
	return {
		'Content-Type': 'application/json',
		'X-Admin-Key': key
	};
}

// ── API Functions ─────────────────────────────────────────────────────────

/** Validate an admin key by attempting to list players. */
export async function validateAdminKey(key: string): Promise<boolean> {
	try {
		const res = await fetch(`${API_URL}/api/admin/players`, {
			headers: adminHeaders(key)
		});
		return res.ok;
	} catch {
		return false;
	}
}

/** Fetch server info (no auth required). */
export async function fetchServerInfo(): Promise<ServerInfo> {
	const res = await fetch(`${API_URL}/api/info`);
	if (!res.ok) throw new Error(`Server info failed: ${res.status}`);
	return res.json();
}

/** Fetch detailed server health (admin). */
export async function fetchHealth(key: string): Promise<ServerHealth> {
	const res = await fetch(`${API_URL}/api/admin/health`, {
		headers: adminHeaders(key)
	});
	if (!res.ok) throw new Error(`Health check failed: ${res.status}`);
	return res.json();
}

/** Fetch connected players (admin). */
export async function fetchPlayers(key: string): Promise<PlayerInfo[]> {
	const res = await fetch(`${API_URL}/api/admin/players`, {
		headers: adminHeaders(key)
	});
	if (!res.ok) throw new Error(`Fetch players failed: ${res.status}`);
	const data = await res.json();
	return data.players;
}

/** Kick a player by ID (admin). */
export async function kickPlayer(key: string, playerId: string): Promise<boolean> {
	const res = await fetch(`${API_URL}/api/admin/kick`, {
		method: 'POST',
		headers: adminHeaders(key),
		body: JSON.stringify({ player_id: playerId })
	});
	if (!res.ok) throw new Error(`Kick failed: ${res.status}`);
	const data = await res.json();
	return data.kicked;
}

/** Fetch all worlds (no auth required). */
export async function fetchWorlds(): Promise<WorldInfo[]> {
	const res = await fetch(`${API_URL}/api/worlds`);
	if (!res.ok) throw new Error(`Fetch worlds failed: ${res.status}`);
	return res.json();
}

/** Toggle pause on a world (admin). */
export async function pauseWorld(key: string, worldId: string): Promise<PauseResult> {
	const res = await fetch(`${API_URL}/api/admin/pause`, {
		method: 'POST',
		headers: adminHeaders(key),
		body: JSON.stringify({ world_id: worldId })
	});
	if (!res.ok) throw new Error(`Pause failed: ${res.status}`);
	return res.json();
}

/** Set world speed (admin). */
export async function setWorldSpeed(key: string, worldId: string, speed: string): Promise<SpeedResult> {
	const res = await fetch(`${API_URL}/api/admin/worlds/${worldId}/speed`, {
		method: 'POST',
		headers: adminHeaders(key),
		body: JSON.stringify({ speed })
	});
	if (!res.ok) throw new Error(`Set speed failed: ${res.status}`);
	return res.json();
}

/** World configuration for creating new worlds. */
export interface CreateWorldConfig {
	seed?: number;
	starting_era?: string;
	difficulty?: string;
	map_size?: string;
	ai_corporations?: number;
	use_real_earth?: boolean;
}

/** Create a new world (admin). */
export async function createWorld(
	key: string,
	name: string,
	maxPlayers: number,
	config?: CreateWorldConfig
): Promise<{ world_id: string; name: string }> {
	const body: Record<string, unknown> = { name, max_players: maxPlayers };
	if (config) {
		body.config = config;
	}
	const res = await fetch(`${API_URL}/api/admin/worlds`, {
		method: 'POST',
		headers: adminHeaders(key),
		body: JSON.stringify(body)
	});
	if (!res.ok) throw new Error(`Create world failed: ${res.status}`);
	return res.json();
}

/** Delete a world (admin). */
export async function deleteWorld(
	key: string,
	worldId: string
): Promise<{ deleted: boolean; kicked_players: number }> {
	const res = await fetch(`${API_URL}/api/admin/worlds/${worldId}`, {
		method: 'DELETE',
		headers: adminHeaders(key)
	});
	if (!res.ok) throw new Error(`Delete world failed: ${res.status}`);
	return res.json();
}

/** Send a broadcast message (admin). */
export async function broadcastMessage(
	key: string,
	message: string,
	worldId?: string
): Promise<{ broadcast: boolean; scope: string }> {
	const body: Record<string, unknown> = { message };
	if (worldId) body.world_id = worldId;
	const res = await fetch(`${API_URL}/api/admin/broadcast`, {
		method: 'POST',
		headers: adminHeaders(key),
		body: JSON.stringify(body)
	});
	if (!res.ok) throw new Error(`Broadcast failed: ${res.status}`);
	return res.json();
}

/** Fetch the audit log (admin). */
export async function fetchAuditLog(key: string): Promise<AuditEntry[]> {
	const res = await fetch(`${API_URL}/api/admin/audit`, {
		headers: adminHeaders(key)
	});
	if (!res.ok) throw new Error(`Fetch audit log failed: ${res.status}`);
	const data = await res.json();
	return data.audit_log;
}

// ── Template Management (Phase 2+) ──────────────────────────────────────

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

export interface CreateTemplateParams {
	name: string;
	description: string;
	icon: string;
	config_defaults: Record<string, unknown>;
	config_bounds: Record<string, unknown>;
	max_instances: number;
	enabled: boolean;
}

export async function fetchTemplates(key: string): Promise<WorldTemplate[]> {
	const res = await fetch(`${API_URL}/api/admin/templates`, {
		headers: adminHeaders(key)
	});
	if (!res.ok) throw new Error(`Fetch templates failed: ${res.status}`);
	return await res.json();
}

export async function createTemplate(key: string, template: CreateTemplateParams): Promise<void> {
	const res = await fetch(`${API_URL}/api/admin/templates`, {
		method: 'POST',
		headers: adminHeaders(key),
		body: JSON.stringify(template)
	});
	if (!res.ok) throw new Error(`Create template failed: ${res.status}`);
}

export async function updateTemplate(
	key: string,
	id: string,
	updates: Partial<CreateTemplateParams>
): Promise<void> {
	const res = await fetch(`${API_URL}/api/admin/templates/${id}`, {
		method: 'PUT',
		headers: adminHeaders(key),
		body: JSON.stringify(updates)
	});
	if (!res.ok) throw new Error(`Update template failed: ${res.status}`);
}

export async function deleteTemplate(key: string, id: string): Promise<void> {
	const res = await fetch(`${API_URL}/api/admin/templates/${id}`, {
		method: 'DELETE',
		headers: adminHeaders(key)
	});
	if (!res.ok) throw new Error(`Delete template failed: ${res.status}`);
}

// ── Ban Management (Phase 2+) ───────────────────────────────────────────

export interface Ban {
	id: string;
	account_id: string;
	username: string;
	world_id: string | null;
	reason: string;
	banned_at: string;
	expires_at: string | null;
}

export async function fetchBans(key: string): Promise<Ban[]> {
	const res = await fetch(`${API_URL}/api/admin/bans`, {
		headers: adminHeaders(key)
	});
	if (!res.ok) throw new Error(`Fetch bans failed: ${res.status}`);
	return await res.json();
}

export async function createBan(
	key: string,
	accountId: string,
	reason: string,
	worldId?: string,
	expiresAt?: string
): Promise<void> {
	const body: Record<string, unknown> = { account_id: accountId, reason };
	if (worldId) body.world_id = worldId;
	if (expiresAt) body.expires_at = expiresAt;
	const res = await fetch(`${API_URL}/api/admin/ban`, {
		method: 'POST',
		headers: adminHeaders(key),
		body: JSON.stringify(body)
	});
	if (!res.ok) throw new Error(`Create ban failed: ${res.status}`);
}

export async function removeBan(key: string, accountId: string, worldId?: string): Promise<void> {
	const body: Record<string, unknown> = { account_id: accountId };
	if (worldId) body.world_id = worldId;
	const res = await fetch(`${API_URL}/api/admin/unban`, {
		method: 'POST',
		headers: adminHeaders(key),
		body: JSON.stringify(body)
	});
	if (!res.ok) throw new Error(`Remove ban failed: ${res.status}`);
}

// ── Password Reset Queue (Phase 2+) ────────────────────────────────────

export interface ResetRequest {
	id: string;
	account_id: string;
	username: string;
	status: string;
	created_at: string;
}

export async function fetchResetQueue(key: string): Promise<ResetRequest[]> {
	const res = await fetch(`${API_URL}/api/admin/reset-queue`, {
		headers: adminHeaders(key)
	});
	if (!res.ok) throw new Error(`Fetch reset queue failed: ${res.status}`);
	return await res.json();
}

export async function resolveReset(
	key: string,
	requestId: string
): Promise<{ temp_password: string }> {
	const res = await fetch(`${API_URL}/api/admin/reset-resolve`, {
		method: 'POST',
		headers: adminHeaders(key),
		body: JSON.stringify({ request_id: requestId })
	});
	if (!res.ok) throw new Error(`Resolve reset failed: ${res.status}`);
	return await res.json();
}

// ── Server Metrics (Phase 2+) ──────────────────────────────────────────

export interface ServerMetrics {
	worlds: WorldMetrics[];
	server: {
		memory_mb: number;
		connected_players: number;
		uptime_secs: number;
		ws_messages_per_sec: number;
	};
}

export interface WorldMetrics {
	id: string;
	name: string;
	avg_tick_us: number;
	max_tick_us: number;
	p99_tick_us: number;
	entity_count: number;
	last_system_times: Record<string, number>;
}

export async function fetchMetrics(key: string): Promise<ServerMetrics> {
	const res = await fetch(`${API_URL}/api/admin/metrics`, {
		headers: adminHeaders(key)
	});
	if (!res.ok) throw new Error(`Fetch metrics failed: ${res.status}`);
	return await res.json();
}
