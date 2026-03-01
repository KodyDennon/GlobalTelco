import { adminFetch } from './client.js';
import type { ServerMetrics } from './types.js';

export function fetchMetrics(): Promise<ServerMetrics> {
	return adminFetch('/api/admin/metrics');
}
