import { adminFetch } from './client.js';
import type { WorldConfig, WorldDebug, WorldTemplate } from './types.js';

export function fetchWorlds(): Promise<Array<{ id: string; name: string; player_count: number; max_players: number; tick: number; speed: string; era: string; map_size: string }>> {
	return adminFetch('/api/worlds');
}

export function createWorld(name: string, config?: WorldConfig, max_players?: number): Promise<{ world_id: string; name: string }> {
	return adminFetch('/api/admin/worlds', {
		method: 'POST',
		body: JSON.stringify({ name, config, max_players: max_players ?? 8 })
	});
}

export function deleteWorld(worldId: string): Promise<{ deleted: boolean }> {
	return adminFetch(`/api/admin/worlds/${worldId}`, { method: 'DELETE' });
}

export function purgeWorlds(): Promise<{ purged: boolean; count: number }> {
	return adminFetch('/api/admin/worlds/purge', { method: 'POST' });
}

export function setWorldSpeed(worldId: string, speed: string): Promise<{ speed: string }> {
	return adminFetch(`/api/admin/worlds/${worldId}/speed`, {
		method: 'POST',
		body: JSON.stringify({ speed })
	});
}

export function pauseWorld(worldId: string): Promise<{ paused: boolean; speed: string }> {
	return adminFetch('/api/admin/pause', {
		method: 'POST',
		body: JSON.stringify({ world_id: worldId })
	});
}

export function debugWorld(worldId: string): Promise<WorldDebug> {
	return adminFetch(`/api/admin/debug/${worldId}`);
}

export function assignPlayer(worldId: string, playerId: string, corpId: number): Promise<{ assigned: boolean }> {
	return adminFetch(`/api/admin/worlds/${worldId}/assign`, {
		method: 'POST',
		body: JSON.stringify({ player_id: playerId, corp_id: corpId })
	});
}

export function toggleSpectator(worldId: string, playerId: string, spectator: boolean): Promise<{ updated: boolean }> {
	return adminFetch(`/api/admin/worlds/${worldId}/spectator`, {
		method: 'POST',
		body: JSON.stringify({ player_id: playerId, spectator })
	});
}

export function transferWorld(worldId: string, newOwnerId: string): Promise<{ transferred: boolean }> {
	return adminFetch(`/api/admin/worlds/${worldId}/transfer`, {
		method: 'POST',
		body: JSON.stringify({ new_owner_id: newOwnerId })
	});
}

export function fetchWorldVotes(worldId: string): Promise<{ votes: Record<string, string>; current_speed: string; creator_id: string | null }> {
	return adminFetch(`/api/admin/worlds/${worldId}/votes`);
}

export function fetchWorldChat(worldId: string, limit = 100, before?: string): Promise<{ messages: Array<{ id: number; username: string; message: string; created_at: string }> }> {
	const params = new URLSearchParams({ limit: String(limit) });
	if (before) params.set('before', before);
	return adminFetch(`/api/admin/worlds/${worldId}/chat?${params}`);
}

// ── Templates ───────────────────────────────────────────────────────────

export function fetchTemplates(): Promise<WorldTemplate[]> {
	return adminFetch('/api/admin/templates');
}

export function createTemplate(template: Omit<WorldTemplate, 'id'>): Promise<{ id: string; status: string }> {
	return adminFetch('/api/admin/templates', {
		method: 'POST',
		body: JSON.stringify(template)
	});
}

export function updateTemplate(id: string, template: Omit<WorldTemplate, 'id'>): Promise<{ status: string }> {
	return adminFetch(`/api/admin/templates/${id}`, {
		method: 'PUT',
		body: JSON.stringify(template)
	});
}

export function deleteTemplate(id: string): Promise<{ status: string }> {
	return adminFetch(`/api/admin/templates/${id}`, { method: 'DELETE' });
}

// ── Server Limits ────────────────────────────────────────────────────

export interface ServerLimits {
	max_active_worlds: number;
	max_worlds_per_player: number;
	active_world_count: number;
}

export function fetchServerLimits(): Promise<ServerLimits> {
	return adminFetch('/api/admin/limits');
}

export function setServerLimits(limits: { max_active_worlds?: number; max_worlds_per_player?: number }): Promise<{ max_active_worlds: number; max_worlds_per_player: number }> {
	return adminFetch('/api/admin/limits', {
		method: 'POST',
		body: JSON.stringify(limits)
	});
}
