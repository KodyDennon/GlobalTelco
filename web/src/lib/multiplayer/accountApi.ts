import { API_URL } from '$lib/config';
import { accessToken } from '$lib/stores/multiplayerState';
import { userProfile, avatarList } from '$lib/stores/accountState';
import type { UserProfile } from '$lib/stores/accountState';
import { get } from 'svelte/store';

function authHeaders(): Record<string, string> {
	const token = get(accessToken);
	return {
		'Content-Type': 'application/json',
		...(token ? { Authorization: `Bearer ${token}` } : {})
	};
}

export async function fetchProfile(): Promise<UserProfile> {
	const res = await fetch(`${API_URL}/api/profile`, { headers: authHeaders() });
	if (!res.ok) throw new Error(`Failed to fetch profile: ${res.status}`);
	const data = await res.json();
	userProfile.set(data);
	return data;
}

export async function updateProfile(displayName: string, avatarId: string): Promise<void> {
	const res = await fetch(`${API_URL}/api/profile`, {
		method: 'PUT',
		headers: authHeaders(),
		body: JSON.stringify({ display_name: displayName, avatar_id: avatarId })
	});
	if (!res.ok) throw new Error(`Failed to update profile: ${res.status}`);
	const data = await res.json();
	userProfile.set(data);
}

export async function fetchAvatars(): Promise<string[]> {
	const res = await fetch(`${API_URL}/api/avatars`);
	if (!res.ok) throw new Error(`Failed to fetch avatars: ${res.status}`);
	const data = await res.json();
	avatarList.set(data.avatars);
	return data.avatars;
}

export async function deleteAccount(): Promise<void> {
	const res = await fetch(`${API_URL}/api/account/delete`, {
		method: 'POST',
		headers: authHeaders()
	});
	if (!res.ok) {
		const err = await res.json().catch(() => ({ error: 'Unknown error' }));
		throw new Error(err.error || 'Failed to delete account');
	}
}

export async function requestPasswordReset(username: string): Promise<void> {
	const res = await fetch(`${API_URL}/api/auth/reset-request`, {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({ username })
	});
	if (!res.ok) {
		const err = await res.json().catch(() => ({ error: 'Unknown error' }));
		throw new Error(err.error || 'Failed to request reset');
	}
}

export async function confirmPasswordReset(token: string, newPassword: string): Promise<void> {
	const res = await fetch(`${API_URL}/api/auth/reset-confirm`, {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({ token, new_password: newPassword })
	});
	if (!res.ok) {
		const err = await res.json().catch(() => ({ error: 'Unknown error' }));
		throw new Error(err.error || 'Failed to reset password');
	}
}

export async function getGitHubAuthUrl(): Promise<string> {
	const res = await fetch(`${API_URL}/api/auth/github`);
	if (!res.ok) throw new Error('GitHub OAuth not configured');
	const data = await res.json();
	return data.url;
}

export interface AuthResult {
	player_id: string;
	username: string;
	access_token: string;
	refresh_token: string;
}

export async function githubCallback(code: string): Promise<AuthResult> {
	const res = await fetch(`${API_URL}/api/auth/github/callback?code=${encodeURIComponent(code)}`);
	if (!res.ok) {
		const err = await res.json().catch(() => ({ error: 'GitHub auth failed' }));
		throw new Error(err.error || 'GitHub auth failed');
	}
	return await res.json();
}

export async function refreshAccessToken(refreshTokenValue: string): Promise<AuthResult> {
	const res = await fetch(`${API_URL}/api/auth/refresh`, {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({ refresh_token: refreshTokenValue })
	});
	if (!res.ok) throw new Error('Token refresh failed');
	return await res.json();
}
