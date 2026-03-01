import { browser } from '$app/environment';
import { writable } from 'svelte/store';

export interface UserProfile {
	id: string;
	username: string;
	display_name: string | null;
	avatar_id: string;
	auth_provider: string;
	created_at: string;
}

export const userProfile = writable<UserProfile | null>(null);
export const avatarList = writable<string[]>([]);

// Persist profile to localStorage
if (browser) {
	userProfile.subscribe((v) => {
		if (v) localStorage.setItem('gt_profile', JSON.stringify(v));
		else localStorage.removeItem('gt_profile');
	});

	// Initialize from localStorage
	const savedProfile = localStorage.getItem('gt_profile');
	if (savedProfile) {
		try {
			userProfile.set(JSON.parse(savedProfile));
		} catch {
			// ignore
		}
	}
}

export function resetAccountState() {
	userProfile.set(null);
	avatarList.set([]);
}
