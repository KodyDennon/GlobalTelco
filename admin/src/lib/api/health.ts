import { adminFetch } from './client.js';
import type { ServerHealth } from './types.js';

export function fetchHealth(): Promise<ServerHealth> {
	return adminFetch<ServerHealth>('/api/admin/health');
}
