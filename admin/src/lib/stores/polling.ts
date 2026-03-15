import { writable, get } from 'svelte/store';
import { preferences } from './preferences.js';

export const pollingEnabled = writable(true);
export const pollingInterval = writable(10000);
export const lastRefresh = writable<Date | null>(null);

type PollingCallback = () => Promise<void>;

interface PollingEntry {
	callback: PollingCallback;
	timer: ReturnType<typeof setInterval> | null;
	intervalMs: number;
}

const activePollers = new Map<string, PollingEntry>();

function restartTimer(entry: PollingEntry) {
	if (entry.timer) clearInterval(entry.timer);
	entry.timer = setInterval(async () => {
		if (!get(pollingEnabled)) return;
		if (typeof document !== 'undefined' && document.hidden) return;
		try {
			await entry.callback();
			lastRefresh.set(new Date());
		} catch {
			// Handled by callers via their own error state
		}
	}, entry.intervalMs);
}

export function startPolling(key: string, callback: PollingCallback, intervalMs?: number) {
	stopPolling(key);
	const ms = intervalMs ?? get(pollingInterval);
	const entry: PollingEntry = { callback, timer: null, intervalMs: ms };
	activePollers.set(key, entry);

	// Run immediately (fire-and-forget)
	callback()
		.then(() => lastRefresh.set(new Date()))
		.catch(() => {});

	restartTimer(entry);
}

export function stopPolling(key: string) {
	const entry = activePollers.get(key);
	if (entry) {
		if (entry.timer) clearInterval(entry.timer);
		activePollers.delete(key);
	}
}

export function stopAllPolling() {
	for (const key of activePollers.keys()) {
		stopPolling(key);
	}
}

/** Trigger an immediate refresh of all active pollers. Called by "R" key shortcut. */
export async function refreshAll() {
	const promises = [...activePollers.values()].map(e => e.callback().catch(() => {}));
	await Promise.allSettled(promises);
	lastRefresh.set(new Date());
}

// Sync polling enabled state and interval from preferences store
preferences.subscribe((prefs) => {
	pollingEnabled.set(prefs.autoRefresh);
	const currentMs = get(pollingInterval);
	if (prefs.refreshInterval !== currentMs) {
		pollingInterval.set(prefs.refreshInterval);
		for (const entry of activePollers.values()) {
			entry.intervalMs = prefs.refreshInterval;
			restartTimer(entry);
		}
	}
});

// When tab becomes visible, immediately refresh all pollers
if (typeof document !== 'undefined') {
	document.addEventListener('visibilitychange', () => {
		if (!document.hidden && get(pollingEnabled)) {
			for (const entry of activePollers.values()) {
				entry.callback()
					.then(() => lastRefresh.set(new Date()))
					.catch(() => {});
			}
		}
	});
}
