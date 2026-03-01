import { adminFetch } from './client.js';
import type { PlayerInfo, AccountListResponse, Ban } from './types.js';

export function fetchPlayers(): Promise<{ players: PlayerInfo[] }> {
	return adminFetch('/api/admin/players');
}

export function kickPlayer(playerId: string): Promise<{ kicked: boolean }> {
	return adminFetch('/api/admin/kick', {
		method: 'POST',
		body: JSON.stringify({ player_id: playerId })
	});
}

// ── Accounts ────────────────────────────────────────────────────────────

export function fetchAccounts(
	search = '',
	page = 0,
	perPage = 50,
	sort = 'created_at',
	order = 'desc'
): Promise<AccountListResponse> {
	const params = new URLSearchParams({
		search,
		page: String(page),
		per_page: String(perPage),
		sort,
		order
	});
	return adminFetch(`/api/admin/accounts?${params}`);
}

// ── Bans ────────────────────────────────────────────────────────────────

export function fetchBans(): Promise<Ban[]> {
	return adminFetch('/api/admin/bans');
}

export function banPlayer(
	accountId: string,
	reason: string,
	worldId?: string,
	expiresAt?: string
): Promise<{ banned: boolean }> {
	return adminFetch('/api/admin/ban', {
		method: 'POST',
		body: JSON.stringify({
			account_id: accountId,
			reason,
			world_id: worldId || undefined,
			expires_at: expiresAt || undefined
		})
	});
}

export function unbanPlayer(accountId: string, worldId?: string): Promise<{ unbanned: boolean }> {
	return adminFetch('/api/admin/unban', {
		method: 'POST',
		body: JSON.stringify({
			account_id: accountId,
			reason: '',
			world_id: worldId || undefined
		})
	});
}
