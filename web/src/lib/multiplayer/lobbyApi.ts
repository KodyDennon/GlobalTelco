import { API_URL } from '$lib/config';
import { accessToken } from '$lib/stores/multiplayerState';
import { get } from 'svelte/store';

function authHeaders(): Record<string, string> {
	const token = get(accessToken);
	return {
		'Content-Type': 'application/json',
		...(token ? { Authorization: `Bearer ${token}` } : {})
	};
}

export interface WorldTemplate {
	id: string;
	name: string;
	description: string;
	icon: string;
	config_defaults: Record<string, unknown>;
	config_bounds: Record<string, unknown>;
	max_instances: number;
	current_instances: number;
	enabled: boolean;
}

export interface WorldListEntry {
	id: string;
	name: string;
	player_count: number;
	max_players: number;
	tick: number;
	speed: string;
	era: string;
	map_size: string;
	template_name: string | null;
	invite_code: string | null;
	created_by: string | null;
}

export interface WorldHistoryEntry {
	id: string;
	world_id: string;
	world_name: string;
	last_played: string;
}

export async function fetchCatalog(): Promise<WorldTemplate[]> {
	const res = await fetch(`${API_URL}/api/catalog`, { headers: authHeaders() });
	if (!res.ok) throw new Error(`Failed to fetch catalog: ${res.status}`);
	return await res.json();
}

export async function fetchWorldList(): Promise<WorldListEntry[]> {
	const res = await fetch(`${API_URL}/api/worlds`, { headers: authHeaders() });
	if (!res.ok) throw new Error(`Failed to fetch worlds: ${res.status}`);
	const data = await res.json();
	return data.worlds || data;
}

export async function createWorldFromTemplate(
	templateId: string,
	name: string,
	maxPlayers: number,
	overrides: Record<string, unknown>
): Promise<{ world_id: string }> {
	const res = await fetch(`${API_URL}/api/worlds/from-template`, {
		method: 'POST',
		headers: authHeaders(),
		body: JSON.stringify({
			template_id: templateId,
			name,
			max_players: maxPlayers,
			config_overrides: overrides
		})
	});
	if (!res.ok) {
		const err = await res.json().catch(() => ({ error: 'Failed to create world' }));
		throw new Error(err.error || `Failed to create world: ${res.status}`);
	}
	return await res.json();
}

export async function lookupInviteCode(code: string): Promise<WorldListEntry | null> {
	const res = await fetch(`${API_URL}/api/worlds/by-invite/${encodeURIComponent(code)}`, {
		headers: authHeaders()
	});
	if (res.status === 404) return null;
	if (!res.ok) throw new Error(`Failed to lookup invite: ${res.status}`);
	return await res.json();
}

export async function fetchWorldHistory(): Promise<WorldHistoryEntry[]> {
	const res = await fetch(`${API_URL}/api/world-history`, { headers: authHeaders() });
	if (!res.ok) throw new Error(`Failed to fetch world history: ${res.status}`);
	return await res.json();
}
