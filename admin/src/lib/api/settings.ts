import { adminFetch } from './client.js';
import type { ServerConfig, ResetRequest } from './types.js';

export function fetchServerConfig(): Promise<ServerConfig> {
	return adminFetch('/api/admin/config');
}

export function broadcast(message: string, worldId?: string): Promise<{ broadcast: boolean }> {
	return adminFetch('/api/admin/broadcast', {
		method: 'POST',
		body: JSON.stringify({ message, world_id: worldId || undefined })
	});
}

export function fetchResetQueue(): Promise<ResetRequest[]> {
	return adminFetch('/api/admin/reset-queue');
}

export function resolveReset(requestId: string): Promise<{ status: string; temp_password: string }> {
	return adminFetch('/api/admin/reset-resolve', {
		method: 'POST',
		body: JSON.stringify({ request_id: requestId })
	});
}
