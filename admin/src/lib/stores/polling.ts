import { writable, get } from 'svelte/store';

export const pollingEnabled = writable(true);
export const pollingInterval = writable(10000); // ms
export const lastRefresh = writable<Date | null>(null);

type PollingCallback = () => Promise<void>;

const activeIntervals: Map<string, ReturnType<typeof setInterval>> = new Map();

export function startPolling(key: string, callback: PollingCallback, intervalMs?: number) {
	stopPolling(key);
	const ms = intervalMs ?? get(pollingInterval);

	const run = async () => {
		if (!get(pollingEnabled)) return;
		if (typeof document !== 'undefined' && document.hidden) return;
		try {
			await callback();
			lastRefresh.set(new Date());
		} catch {
			// Silently ignore polling errors
		}
	};

	// Run immediately
	run();

	const id = setInterval(run, ms);
	activeIntervals.set(key, id);
}

export function stopPolling(key: string) {
	const id = activeIntervals.get(key);
	if (id) {
		clearInterval(id);
		activeIntervals.delete(key);
	}
}

export function stopAllPolling() {
	for (const [key] of activeIntervals) {
		stopPolling(key);
	}
}

// Pause on tab hidden, resume on visible
if (typeof document !== 'undefined') {
	document.addEventListener('visibilitychange', () => {
		if (document.hidden) {
			// Intervals will skip execution via the check in run()
		}
		// When tab becomes visible, all intervals will resume on next tick
	});
}
