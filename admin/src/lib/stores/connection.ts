import { writable, derived, get } from 'svelte/store';
import { API_URL } from '../config.js';
import { adminKey } from './auth.js';

export type ConnectionState = 'connected' | 'disconnected' | 'checking';

export const connectionState = writable<ConnectionState>('checking');
export const lastError = writable<string | null>(null);
export const consecutiveFailures = writable(0);

export const isConnected = derived(connectionState, ($s) => $s === 'connected');

let checkTimer: ReturnType<typeof setInterval> | null = null;

async function checkConnection(): Promise<boolean> {
	const key = get(adminKey);
	if (!key) return false;

	try {
		const res = await fetch(`${API_URL}/api/admin/health`, {
			headers: { 'X-Admin-Key': key },
			signal: AbortSignal.timeout(5000)
		});
		if (res.ok) {
			connectionState.set('connected');
			lastError.set(null);
			consecutiveFailures.set(0);
			return true;
		}
		connectionState.set('disconnected');
		lastError.set(`Server returned ${res.status}`);
		consecutiveFailures.update(n => n + 1);
		return false;
	} catch (e) {
		connectionState.set('disconnected');
		lastError.set(e instanceof Error ? e.message : 'Connection failed');
		consecutiveFailures.update(n => n + 1);
		return false;
	}
}

export function startConnectionMonitor() {
	stopConnectionMonitor();
	checkConnection();
	// Check every 15 seconds
	checkTimer = setInterval(checkConnection, 15000);
}

export function stopConnectionMonitor() {
	if (checkTimer) {
		clearInterval(checkTimer);
		checkTimer = null;
	}
}

/** Force an immediate connection check */
export function forceCheck() {
	connectionState.set('checking');
	checkConnection();
}

// Start/stop monitor based on auth state
adminKey.subscribe((key) => {
	if (key) {
		startConnectionMonitor();
	} else {
		stopConnectionMonitor();
		connectionState.set('checking');
		lastError.set(null);
		consecutiveFailures.set(0);
	}
});
