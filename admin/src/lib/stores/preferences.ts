import { writable } from 'svelte/store';

const PREFS_KEY = 'gt_admin_prefs';

interface AdminPreferences {
	refreshInterval: number; // ms
	autoRefresh: boolean;
	sidebarCollapsed: boolean;
}

const defaults: AdminPreferences = {
	refreshInterval: 10000,
	autoRefresh: true,
	sidebarCollapsed: false
};

function loadPrefs(): AdminPreferences {
	if (typeof localStorage === 'undefined') return { ...defaults };
	try {
		const raw = localStorage.getItem(PREFS_KEY);
		if (raw) return { ...defaults, ...JSON.parse(raw) };
	} catch {
		// ignore
	}
	return { ...defaults };
}

function createPrefsStore() {
	const store = writable<AdminPreferences>(loadPrefs());

	store.subscribe((value) => {
		if (typeof localStorage !== 'undefined') {
			localStorage.setItem(PREFS_KEY, JSON.stringify(value));
		}
	});

	return store;
}

export const preferences = createPrefsStore();
