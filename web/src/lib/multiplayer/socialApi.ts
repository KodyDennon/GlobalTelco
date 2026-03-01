import { API_URL } from '$lib/config';
import { accessToken } from '$lib/stores/multiplayerState';
import { get } from 'svelte/store';
import type { Friend, FriendRequest } from '$lib/stores/socialState';
import type { UserProfile } from '$lib/stores/accountState';

function authHeaders(): Record<string, string> {
	const token = get(accessToken);
	return {
		'Content-Type': 'application/json',
		...(token ? { Authorization: `Bearer ${token}` } : {})
	};
}

export interface RecentPlayer {
	id: string;
	other_id: string;
	other_username: string;
	other_display_name: string | null;
	other_avatar_id: string;
	world_name: string;
	last_seen: string;
}

export async function fetchFriends(): Promise<Friend[]> {
	const res = await fetch(`${API_URL}/api/friends`, { headers: authHeaders() });
	if (!res.ok) throw new Error(`Failed to fetch friends: ${res.status}`);
	return await res.json();
}

export async function sendFriendRequest(username: string): Promise<void> {
	const res = await fetch(`${API_URL}/api/friends/request`, {
		method: 'POST',
		headers: authHeaders(),
		body: JSON.stringify({ username })
	});
	if (!res.ok) {
		const err = await res.json().catch(() => ({ error: 'Failed to send request' }));
		throw new Error(err.error || 'Failed to send friend request');
	}
}

export async function fetchFriendRequests(): Promise<{
	incoming: FriendRequest[];
	outgoing: FriendRequest[];
}> {
	const res = await fetch(`${API_URL}/api/friends/requests`, { headers: authHeaders() });
	if (!res.ok) throw new Error(`Failed to fetch friend requests: ${res.status}`);
	return await res.json();
}

export async function acceptFriendRequest(requestId: string): Promise<void> {
	const res = await fetch(`${API_URL}/api/friends/accept`, {
		method: 'POST',
		headers: authHeaders(),
		body: JSON.stringify({ request_id: requestId })
	});
	if (!res.ok) throw new Error('Failed to accept friend request');
}

export async function rejectFriendRequest(requestId: string): Promise<void> {
	const res = await fetch(`${API_URL}/api/friends/reject`, {
		method: 'POST',
		headers: authHeaders(),
		body: JSON.stringify({ request_id: requestId })
	});
	if (!res.ok) throw new Error('Failed to reject friend request');
}

export async function removeFriend(friendId: string): Promise<void> {
	const res = await fetch(`${API_URL}/api/friends/${friendId}`, {
		method: 'DELETE',
		headers: authHeaders()
	});
	if (!res.ok) throw new Error('Failed to remove friend');
}

export async function searchUsers(query: string): Promise<UserProfile[]> {
	const res = await fetch(`${API_URL}/api/friends/search?q=${encodeURIComponent(query)}`, {
		headers: authHeaders()
	});
	if (!res.ok) throw new Error(`Search failed: ${res.status}`);
	return await res.json();
}

export async function inviteFriendToWorld(friendId: string, worldId: string): Promise<void> {
	const res = await fetch(`${API_URL}/api/friends/invite`, {
		method: 'POST',
		headers: authHeaders(),
		body: JSON.stringify({ friend_id: friendId, world_id: worldId })
	});
	if (!res.ok) throw new Error('Failed to send invite');
}

export async function fetchRecentPlayers(): Promise<RecentPlayer[]> {
	const res = await fetch(`${API_URL}/api/recent-players`, { headers: authHeaders() });
	if (!res.ok) throw new Error(`Failed to fetch recent players: ${res.status}`);
	return await res.json();
}
