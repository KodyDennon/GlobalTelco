import { encode, decode } from '@msgpack/msgpack';
import { get } from 'svelte/store';
import {
	connectionState,
	worldId,
	playerId,
	playerUsername,
	corpId,
	accessToken,
	refreshToken,
	authError,
	isAuthenticated,
	serverInfo,
	addChatMessage,
	updatePlayerStatus,
	proxySummary,
	type MultiplayerWorldInfo,
	type ServerInfo
} from '$lib/stores/multiplayerState';
import { worldInfo, notifications } from '$lib/stores/gameState';
import type { GameEvent } from '$lib/wasm/types';
import { API_URL, WS_URL } from '$lib/config';

let ws: WebSocket | null = null;
let reconnectTimer: ReturnType<typeof setTimeout> | null = null;
let reconnectAttempts = 0;
const MAX_RECONNECT_ATTEMPTS = 10;

type ServerMessage = Record<string, unknown>;

export function connect() {
	connectionState.set('connecting');
	reconnectAttempts = 0;
	createSocket(WS_URL);
}

export function disconnect() {
	if (reconnectTimer) {
		clearTimeout(reconnectTimer);
		reconnectTimer = null;
	}
	if (ws) {
		ws.close(1000, 'User disconnect');
		ws = null;
	}
	connectionState.set('disconnected');
}

function createSocket(url: string) {
	try {
		ws = new WebSocket(url);
		ws.binaryType = 'arraybuffer';

		ws.onopen = () => {
			connectionState.set('connected');
			reconnectAttempts = 0;
		};

		ws.onmessage = (event) => {
			try {
				let msg: ServerMessage;
				if (event.data instanceof ArrayBuffer) {
					msg = decode(new Uint8Array(event.data)) as ServerMessage;
				} else {
					msg = JSON.parse(event.data);
				}
				handleServerMessage(msg);
			} catch (e) {
				console.error('Failed to parse server message:', e);
			}
		};

		ws.onclose = (event) => {
			if (event.code !== 1000 && reconnectAttempts < MAX_RECONNECT_ATTEMPTS) {
				connectionState.set('reconnecting');
				const delay = Math.min(1000 * Math.pow(2, reconnectAttempts), 30000);
				reconnectAttempts++;
				reconnectTimer = setTimeout(() => createSocket(url), delay);
			} else {
				connectionState.set('disconnected');
			}
		};

		ws.onerror = () => {
			// Error will trigger onclose
		};
	} catch {
		connectionState.set('disconnected');
	}
}

function handleServerMessage(msg: ServerMessage) {
	// ServerMessage is an enum — could be { AuthResult: {...} } or { TickUpdate: {...} } etc.
	if ('AuthResult' in msg) {
		const auth = msg.AuthResult as Record<string, unknown>;
		if ('Success' in auth) {
			const success = auth.Success as Record<string, unknown>;
			playerId.set(success.player_id as string);
			playerUsername.set(success.username as string);
			accessToken.set(success.access_token as string);
			refreshToken.set(success.refresh_token as string);
			isAuthenticated.set(true);
			authError.set('');
		} else if ('GuestSuccess' in auth) {
			const guest = auth.GuestSuccess as Record<string, unknown>;
			playerId.set(guest.player_id as string);
			playerUsername.set(guest.username as string);
			isAuthenticated.set(true);
			authError.set('');
		} else if ('Failed' in auth) {
			const failed = auth.Failed as Record<string, unknown>;
			const reason = (failed.reason as string) || 'Authentication failed';
			console.error('Authentication failed:', reason);
			authError.set(reason);
			isAuthenticated.set(false);
			playerId.set(null);
			playerUsername.set(null);
			accessToken.set(null);
			refreshToken.set(null);
		}
	} else if ('WorldJoined' in msg) {
		const joined = msg.WorldJoined as Record<string, unknown>;
		console.log('[WS] WorldJoined received:', joined);
		worldId.set(joined.world_id as string);
		corpId.set(joined.corp_id as number);
	} else if ('TickUpdate' in msg) {
		const update = msg.TickUpdate as Record<string, unknown>;
		const tick = update.tick as number;
		const events = (update.events as Array<Record<string, unknown>>) || [];
		worldInfo.update((info) => {
			if (info) {
				return { ...info, tick };
			}
			return info;
		});
		if (events.length > 0) {
			const notifs = events.map((e) => ({
				tick,
				event: e as unknown as GameEvent
			}));
			notifications.update((n) => [...notifs, ...n].slice(0, 50));
		}
	} else if ('ChatBroadcast' in msg) {
		const chat = msg.ChatBroadcast as Record<string, unknown>;
		addChatMessage({
			sender: chat.sender as string,
			message: chat.message as string,
			timestamp: chat.timestamp as number
		});
	} else if ('PlayerStatus' in msg) {
		const status = msg.PlayerStatus as Record<string, unknown>;
		updatePlayerStatus(
			status.player_id as string,
			status.username as string,
			status.status as 'Connected' | 'Disconnected' | 'AiProxy'
		);
	} else if ('ProxySummary' in msg) {
		const summary = msg.ProxySummary as Record<string, unknown>;
		proxySummary.set({
			ticks_elapsed: summary.ticks_elapsed as number,
			actions: (summary.actions as Array<{ tick: number; description: string }>) || []
		});
	} else if ('Error' in msg) {
		const error = msg.Error as Record<string, unknown>;
		console.error(`Server error [${error.code}]: ${error.message}`);
	} else if ('SaveList' in msg) {
		// Handle save list response — emit event for UI to handle
		const saves = msg.SaveList as Record<string, unknown>;
		window.dispatchEvent(new CustomEvent('cloud-saves', { detail: saves }));
	} else if ('SaveData' in msg) {
		const data = msg.SaveData as Record<string, unknown>;
		window.dispatchEvent(new CustomEvent('cloud-save-data', { detail: data }));
	}
}

function sendMessage(msg: unknown) {
	if (!ws || ws.readyState !== WebSocket.OPEN) return;
	const bytes = encode(msg);
	ws.send(bytes);
}

// ── Public API ──────────────────────────────────────────────────────────

export function login(username: string, password: string) {
	sendMessage({ Auth: { Login: { username, password } } });
}

export function register(username: string, password: string, email: string) {
	sendMessage({ Auth: { Register: { username, password, email } } });
}

export function loginWithToken(token: string) {
	sendMessage({ Auth: { Token: { access_token: token } } });
}

export function loginAsGuest() {
	sendMessage({ Auth: 'Guest' });
}

export function joinWorld(id: string) {
	sendMessage({ JoinWorld: { world_id: id } });
}

export function leaveWorld() {
	sendMessage('LeaveWorld');
	worldId.set(null);
	corpId.set(null);
}

export function sendCommand(worldIdStr: string, command: Record<string, unknown>) {
	sendMessage({ GameCommand: { world_id: worldIdStr, command } });
}

export function sendChat(message: string) {
	sendMessage({ Chat: { message } });
}

export function requestSaves() {
	sendMessage('RequestSaves');
}

export function ping() {
	const timestamp = Date.now();
	sendMessage({ Ping: { timestamp } });
}

export async function fetchServerInfo(): Promise<ServerInfo | null> {
	try {
		const res = await fetch(`${API_URL}/api/info`);
		if (res.ok) {
			const info: ServerInfo = await res.json();
			serverInfo.set(info);
			return info;
		}
	} catch {
		// Server offline
	}
	serverInfo.set(null);
	return null;
}

export async function fetchWorlds(): Promise<MultiplayerWorldInfo[]> {
	try {
		const token = get(accessToken);
		const headers: Record<string, string> = {};
		if (token) {
			headers['Authorization'] = `Bearer ${token}`;
		}
		const res = await fetch(`${API_URL}/api/worlds`, { headers });
		if (res.ok) {
			return await res.json();
		}
	} catch {
		// Server offline
	}
	return [];
}
