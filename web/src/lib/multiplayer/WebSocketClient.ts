import { encode, decode } from '@msgpack/msgpack';
import { decompress } from 'fzstd';
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
	latestSnapshot,
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

// Command sequence tracking for correlating acks
let nextSeq = 1;

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
		corpId.set(Number(joined.corp_id));
		latestSnapshot.set(null); // Clear any previous snapshot
		// Auto-request full world snapshot so client can populate WASM
		requestSnapshot(joined.world_id as string);
	} else if ('TickUpdate' in msg) {
		const update = msg.TickUpdate as Record<string, unknown>;
		const tick = update.tick as number;
		const events = (update.events as Array<Record<string, unknown>>) || [];
		const corpUpdates = (update.corp_updates as Array<Record<string, unknown>>) || [];

		// Monotonic tick guard: only advance forward, never backward.
		// This prevents stale/reordered messages from rolling the tick back.
		worldInfo.update((info) => {
			if (!info) return info;
			if (tick <= info.tick) return info;
			return { ...info, tick };
		});

		// Apply corporation deltas from server to keep stores in sync
		window.dispatchEvent(new CustomEvent('mp-corp-deltas', {
			detail: { tick, deltas: corpUpdates }
		}));
		if (events.length > 0) {
			const notifs = events.map((e) => ({
				tick,
				event: e as unknown as GameEvent
			}));
			notifications.update((n) => [...notifs, ...n].slice(0, 50));
			// Dispatch events for UI effects
			for (const notif of notifs) {
				window.dispatchEvent(new CustomEvent('game-event', { detail: notif }));
			}
		}
		// Server now auto-pushes snapshots every 5 ticks — no client polling needed
	} else if ('ChatBroadcast' in msg) {
		const chat = msg.ChatBroadcast as Record<string, unknown>;
		addChatMessage({
			sender: chat.sender as string,
			message: chat.message as string,
			timestamp: chat.timestamp as number
		});
	} else if ('PlayerStatus' in msg) {
		const status = msg.PlayerStatus as Record<string, unknown>;
		const username = status.username as string;
		const playerStatus = status.status as 'Connected' | 'Disconnected' | 'AiProxy';
		updatePlayerStatus(
			status.player_id as string,
			username,
			playerStatus
		);
		// Add system message to chat for lobby awareness
		const statusMsg =
			playerStatus === 'Connected' ? `${username} joined the world` :
			playerStatus === 'Disconnected' ? `${username} left the world` :
			`${username}'s corporation is now AI-managed`;
		addChatMessage({
			sender: '[System]',
			message: statusMsg,
			timestamp: Date.now()
		});
	} else if ('ProxySummary' in msg) {
		const summary = msg.ProxySummary as Record<string, unknown>;
		proxySummary.set({
			ticks_elapsed: summary.ticks_elapsed as number,
			actions: (summary.actions as Array<{ tick: number; description: string }>) || []
		});
	} else if ('Snapshot' in msg) {
		const snapshot = msg.Snapshot as Record<string, unknown>;
		console.log('[WS] Snapshot received, tick:', snapshot.tick);
		const snapData = {
			tick: snapshot.tick as number,
			state_json: snapshot.state_json as string,
		};
		latestSnapshot.set(snapData);
		window.dispatchEvent(new CustomEvent('mp-snapshot', { detail: snapData }));
	} else if ('CompressedSnapshot' in msg) {
		const snap = msg.CompressedSnapshot as Record<string, unknown>;
		try {
			const compressed = new Uint8Array(snap.compressed_data as ArrayBuffer | number[]);
			const decompressed = decompress(compressed);
			const stateJson = new TextDecoder().decode(decompressed);
			console.log(
				'[WS] CompressedSnapshot received, tick:', snap.tick,
				'compressed:', compressed.length, '-> decompressed:', decompressed.length
			);
			const snapData = {
				tick: snap.tick as number,
				state_json: stateJson,
			};
			latestSnapshot.set(snapData);
			window.dispatchEvent(new CustomEvent('mp-snapshot', { detail: snapData }));
		} catch (e) {
			console.error('[WS] Failed to decompress snapshot:', e);
		}
	} else if ('CommandAck' in msg) {
		const ack = msg.CommandAck as Record<string, unknown>;
		window.dispatchEvent(new CustomEvent('mp-command-ack', {
			detail: {
				success: ack.success as boolean,
				error: ack.error as string | null,
				seq: ack.seq as number | null,
				entity_id: ack.entity_id as number | null,
				effective_tick: ack.effective_tick as number | null,
			}
		}));
	} else if ('CommandBroadcast' in msg) {
		const broadcast = msg.CommandBroadcast as Record<string, unknown>;
		window.dispatchEvent(new CustomEvent('mp-command-broadcast', {
			detail: {
				tick: broadcast.tick as number,
				corp_id: broadcast.corp_id as number,
				ops: broadcast.ops as Array<Record<string, unknown>>,
			}
		}));
	} else if ('SpeedVoteUpdate' in msg) {
		const vote = msg.SpeedVoteUpdate as Record<string, unknown>;
		window.dispatchEvent(new CustomEvent('mp-speed-vote', {
			detail: {
				votes: vote.votes as Array<{ username: string; speed: string }>,
				resolved_speed: vote.resolved_speed as string,
			}
		}));
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
	} else if ('FriendRequestReceived' in msg) {
		const req = msg.FriendRequestReceived as Record<string, unknown>;
		window.dispatchEvent(new CustomEvent('mp-friend-request', {
			detail: {
				from_id: req.from_id as string,
				from_username: req.from_username as string,
			}
		}));
	} else if ('FriendPresenceUpdate' in msg) {
		const presence = msg.FriendPresenceUpdate as Record<string, unknown>;
		window.dispatchEvent(new CustomEvent('mp-friend-presence', {
			detail: {
				friend_id: presence.friend_id as string,
				username: presence.username as string,
				online: presence.online as boolean,
				world_id: presence.world_id as string | null,
				world_name: presence.world_name as string | null,
			}
		}));
	} else if ('WorldInvite' in msg) {
		const invite = msg.WorldInvite as Record<string, unknown>;
		window.dispatchEvent(new CustomEvent('mp-world-invite', {
			detail: {
				from_username: invite.from_username as string,
				world_id: invite.world_id as string,
				world_name: invite.world_name as string,
				invite_code: invite.invite_code as string,
			}
		}));
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

export function sendCommand(worldIdStr: string, command: Record<string, unknown>): number {
	const seq = nextSeq++;
	sendMessage({ GameCommand: { world_id: worldIdStr, command, seq } });
	return seq;
}

export function requestSnapshot(id: string) {
	sendMessage({ RequestSnapshot: { world_id: id } });
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
