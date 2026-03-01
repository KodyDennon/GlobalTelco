import { writable } from 'svelte/store';

export interface Friend {
	id: string;
	username: string;
	display_name: string | null;
	avatar_id: string;
	online: boolean;
	world_id: string | null;
	world_name: string | null;
}

export interface FriendRequest {
	id: string;
	from_id: string;
	from_username: string;
	to_id: string;
	to_username: string;
	status: string;
	created_at: string;
}

export interface WorldInvite {
	from_username: string;
	world_id: string;
	world_name: string;
	invite_code: string;
	received_at: number;
}

export const friends = writable<Friend[]>([]);
export const friendRequests = writable<{ incoming: FriendRequest[]; outgoing: FriendRequest[] }>({
	incoming: [],
	outgoing: []
});
export const pendingInvites = writable<WorldInvite[]>([]);

export function resetSocialState() {
	friends.set([]);
	friendRequests.set({ incoming: [], outgoing: [] });
	pendingInvites.set([]);
}
