import { writable, derived, get } from 'svelte/store';

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

export const connectionState = writable<ConnectionState>('disconnected');
export const worldId = writable<string | null>(null);
export const playerId = writable<string | null>(null);
export const playerUsername = writable<string | null>(null);
export const corpId = writable<number | null>(null);
export const chatMessages = writable<ChatMessage[]>([]);
export const playerList = writable<PlayerInfo[]>([]);
export const accessToken = writable<string | null>(null);
export const refreshToken = writable<string | null>(null);
export const serverUrl = writable<string>('ws://localhost:3001/ws');
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
	proxySummary.set(null);
}
