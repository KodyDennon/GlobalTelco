import { browser } from '$app/environment';
import { writable, derived, get } from 'svelte/store';
import { resetAccountState } from '$lib/stores/accountState';

export type ConnectionState = 'disconnected' | 'connecting' | 'connected' | 'reconnecting';

export interface ChatMessage {
	sender: string;
	message: string;
	timestamp: number;
}

export interface PlayerInfo {
	id: string;
	username: string;
	status: 'Connected' | 'Disconnected' | 'AiProxy';
}

export interface MultiplayerWorldInfo {
	id: string;
	name: string;
	player_count: number;
	max_players: number;
	tick: number;
	speed: string;
	era: string;
	map_size: string;
}

const initialAccessToken = browser ? localStorage.getItem('gt_access_token') : null;
const initialRefreshToken = browser ? localStorage.getItem('gt_refresh_token') : null;
const initialPlayerUsername = browser ? localStorage.getItem('gt_player_username') : null;
const initialPlayerId = browser ? localStorage.getItem('gt_player_id') : null;

export const connectionState = writable<ConnectionState>('disconnected');
export const worldId = writable<string | null>(null);
export const playerId = writable<string | null>(initialPlayerId);
export const playerUsername = writable<string | null>(initialPlayerUsername);
export const corpId = writable<number | null>(null);
export const chatMessages = writable<ChatMessage[]>([]);
export const playerList = writable<PlayerInfo[]>([]);
export const accessToken = writable<string | null>(initialAccessToken);
export const refreshToken = writable<string | null>(initialRefreshToken);

// Persist tokens and user info to localStorage
if (browser) {
	accessToken.subscribe((value) => {
		if (value) localStorage.setItem('gt_access_token', value);
		else localStorage.removeItem('gt_access_token');
	});
	refreshToken.subscribe((value) => {
		if (value) localStorage.setItem('gt_refresh_token', value);
		else localStorage.removeItem('gt_refresh_token');
	});
	playerUsername.subscribe((value) => {
		if (value) localStorage.setItem('gt_player_username', value);
		else localStorage.removeItem('gt_player_username');
	});
	playerId.subscribe((value) => {
		if (value) localStorage.setItem('gt_player_id', value);
		else localStorage.removeItem('gt_player_id');
	});
}
export const authError = writable<string>('');
export const isAuthenticated = writable<boolean>(false);

export interface ServerInfo {
	version: string;
	active_worlds: number;
	connected_players: number;
}

export const serverInfo = writable<ServerInfo | null>(null);
export const proxySummary = writable<{ ticks_elapsed: number; actions: { tick: number; description: string }[] } | null>(null);

export const isMultiplayer = derived(connectionState, ($state) => $state !== 'disconnected');
export const isConnected = derived(connectionState, ($state) => $state === 'connected');

export function addChatMessage(msg: ChatMessage) {
	chatMessages.update((msgs) => [...msgs, msg].slice(-100));
}

export function updatePlayerStatus(id: string, username: string, status: 'Connected' | 'Disconnected' | 'AiProxy') {
	playerList.update((players) => {
		const existing = players.findIndex((p) => p.id === id);
		if (status === 'Disconnected') {
			// Keep in list but mark as disconnected
			if (existing >= 0) {
				players[existing].status = status;
			}
		} else if (existing >= 0) {
			players[existing].status = status;
		} else {
			players.push({ id, username, status });
		}
		return [...players];
	});
}

// ── Ghost Entity System (optimistic UI) ─────────────────────────────────
export interface GhostNode {
	seq: number;
	lon: number;
	lat: number;
	node_type: string;
	network_level: string;
	created_at: number;
}

export interface GhostEdge {
	seq: number;
	from_node: number;
	to_node: number;
	edge_type: string;
	created_at: number;
}

export const ghostNodes = writable<GhostNode[]>([]);
export const ghostEdges = writable<GhostEdge[]>([]);

const GHOST_TIMEOUT_MS = 5000; // Remove unresolved ghosts after 5s

export function addGhostNode(seq: number, lon: number, lat: number, node_type: string, network_level: string) {
	ghostNodes.update((nodes) => [...nodes, { seq, lon, lat, node_type, network_level, created_at: Date.now() }]);
	// Auto-remove after timeout
	setTimeout(() => removeGhost(seq), GHOST_TIMEOUT_MS);
}

export function addGhostEdge(seq: number, from_node: number, to_node: number, edge_type: string) {
	ghostEdges.update((edges) => [...edges, { seq, from_node, to_node, edge_type, created_at: Date.now() }]);
	setTimeout(() => removeGhost(seq), GHOST_TIMEOUT_MS);
}

export function removeGhost(seq: number) {
	ghostNodes.update((nodes) => nodes.filter((n) => n.seq !== seq));
	ghostEdges.update((edges) => edges.filter((e) => e.seq !== seq));
}

// ── Speed Vote System ───────────────────────────────────────────────────
export interface SpeedVote {
	player_id: string;
	username: string;
	speed: string;
	timestamp: number;
}

export const speedVotes = writable<SpeedVote[]>([]);
export const speedVoteDeadline = writable<number | null>(null);

export const latestSnapshot = writable<{ tick: number; state_json: string } | null>(null);

export function resetMultiplayerState() {
	connectionState.set('disconnected');
	worldId.set(null);
	playerId.set(null);
	playerUsername.set(null);
	corpId.set(null);
	chatMessages.set([]);
	playerList.set([]);
	accessToken.set(null);
	refreshToken.set(null);
	authError.set('');
	isAuthenticated.set(false);
	serverInfo.set(null);
	proxySummary.set(null);
	ghostNodes.set([]);
	ghostEdges.set([]);
	speedVotes.set([]);
	speedVoteDeadline.set(null);
	latestSnapshot.set(null);
	resetAccountState();
}
