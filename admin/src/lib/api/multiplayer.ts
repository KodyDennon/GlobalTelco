import { adminFetch } from './client.js';
import type { ConnectionInfo } from './types.js';

export function fetchConnections(): Promise<{ connections: ConnectionInfo[] }> {
	return adminFetch('/api/admin/connections');
}
