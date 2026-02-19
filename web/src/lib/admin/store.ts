import { writable, derived } from 'svelte/store';
import { browser } from '$app/environment';

const STORAGE_KEY = 'gt_admin_key';

function createAdminKeyStore() {
	const initial = browser ? (sessionStorage.getItem(STORAGE_KEY) ?? '') : '';
	const store = writable(initial);

	if (browser) {
		store.subscribe((v) => {
			if (v) {
				sessionStorage.setItem(STORAGE_KEY, v);
			} else {
				sessionStorage.removeItem(STORAGE_KEY);
			}
		});
	}

	return store;
}

export const adminKey = createAdminKeyStore();
export const adminAuthed = derived(adminKey, ($key) => $key.length > 0);

export function clearAdmin() {
	adminKey.set('');
}
