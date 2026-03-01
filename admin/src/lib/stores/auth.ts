import { writable, derived } from 'svelte/store';

const STORAGE_KEY = 'gt_admin_key';

function createAdminKeyStore() {
	const stored = typeof sessionStorage !== 'undefined' ? sessionStorage.getItem(STORAGE_KEY) : null;
	const store = writable<string>(stored || '');

	store.subscribe((value) => {
		if (typeof sessionStorage !== 'undefined') {
			if (value) {
				sessionStorage.setItem(STORAGE_KEY, value);
			} else {
				sessionStorage.removeItem(STORAGE_KEY);
			}
		}
	});

	return store;
}

export const adminKey = createAdminKeyStore();
export const adminAuthed = derived(adminKey, ($key) => $key.length > 0);

export function clearAdmin() {
	adminKey.set('');
	if (typeof sessionStorage !== 'undefined') {
		sessionStorage.removeItem(STORAGE_KEY);
	}
}
