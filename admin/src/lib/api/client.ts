import { get } from 'svelte/store';
import { adminKey, clearAdmin } from '../stores/auth.js';
import { API_URL } from '../config.js';

export class ApiError extends Error {
	constructor(
		public status: number,
		message: string
	) {
		super(message);
	}
}

export async function adminFetch<T>(path: string, options: RequestInit = {}): Promise<T> {
	const key = get(adminKey);
	const url = `${API_URL}${path}`;

	const headers: Record<string, string> = {
		'X-Admin-Key': key,
		...(options.headers as Record<string, string>)
	};

	if (options.body && typeof options.body === 'string') {
		headers['Content-Type'] = 'application/json';
	}

	const res = await fetch(url, {
		...options,
		headers
	});

	if (res.status === 401) {
		clearAdmin();
		throw new ApiError(401, 'Invalid admin key');
	}

	if (res.status === 503) {
		throw new ApiError(503, 'Admin not configured on server');
	}

	if (!res.ok) {
		const body = await res.json().catch(() => ({ error: res.statusText }));
		throw new ApiError(res.status, body.error || res.statusText);
	}

	return res.json();
}

export async function validateAdminKey(key: string): Promise<boolean> {
	try {
		const res = await fetch(`${API_URL}/api/admin/health`, {
			headers: { 'X-Admin-Key': key }
		});
		return res.ok;
	} catch {
		return false;
	}
}
