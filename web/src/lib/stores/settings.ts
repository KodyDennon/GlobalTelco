import { writable } from 'svelte/store';
import { browser } from '$app/environment';

function persistentStore<T>(key: string, initial: T) {
	const stored = browser ? localStorage.getItem(key) : null;
	const value = stored ? JSON.parse(stored) : initial;
	const store = writable<T>(value);

	if (browser) {
		store.subscribe((v) => localStorage.setItem(key, JSON.stringify(v)));
	}

	return store;
}

export const musicVolume = persistentStore('gt_music_volume', 0.5);
export const sfxVolume = persistentStore('gt_sfx_volume', 0.7);
export const autoSaveInterval = persistentStore('gt_autosave', 50);
export const mapQuality = persistentStore<'low' | 'medium' | 'high'>('gt_map_quality', 'medium');
export const showNotifications = persistentStore('gt_show_notifs', true);
export const notificationCategories = persistentStore<Record<string, boolean>>('gt_notif_cats', {
	disaster: true,
	infrastructure: true,
	finance: true,
	contract: true,
	research: true,
	market: true
});
