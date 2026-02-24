/**
 * Unified game command dispatch.
 *
 * In multiplayer (worldId is set): sends command to server via WebSocket.
 * The server is authoritative — client does NOT execute locally.
 *
 * In single-player (worldId is null): executes command on local WASM bridge.
 *
 * All UI components should use `gameCommand()` instead of `bridge.processCommand()`.
 */
import { get } from 'svelte/store';
import { worldId } from '$lib/stores/multiplayerState';
import { sendCommand } from '$lib/multiplayer/WebSocketClient';
import * as bridge from '$lib/wasm/bridge';

export function gameCommand(command: Record<string, unknown>): void {
	const wId = get(worldId);
	if (wId) {
		// Multiplayer: send to server, don't execute locally
		sendCommand(wId, command);
	} else {
		// Single-player: execute locally
		bridge.processCommand(command);
	}
}
