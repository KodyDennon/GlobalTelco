import { adminFetch } from './client.js';
import type { AuditEntry } from './types.js';

export function fetchAuditLog(
	limit = 100,
	offset = 0,
	actor?: string,
	action?: string,
	from?: string,
	to?: string
): Promise<{ audit_log: AuditEntry[]; total: number }> {
	const params = new URLSearchParams({
		limit: String(limit),
		offset: String(offset)
	});
	if (actor) params.set('actor', actor);
	if (action) params.set('action', action);
	if (from) params.set('from', from);
	if (to) params.set('to', to);
	return adminFetch(`/api/admin/audit?${params}`);
}
