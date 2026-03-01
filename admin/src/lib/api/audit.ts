import { adminFetch } from './client.js';
import type { AuditEntry } from './types.js';

export function fetchAuditLog(
	limit = 100,
	offset = 0,
	actor?: string
): Promise<{ audit_log: AuditEntry[]; total: number }> {
	const params = new URLSearchParams({
		limit: String(limit),
		offset: String(offset)
	});
	if (actor) params.set('actor', actor);
	return adminFetch(`/api/admin/audit?${params}`);
}
