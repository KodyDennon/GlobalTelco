/**
 * Unified game command dispatch.
 *
 * In multiplayer (worldId is set): sends command to server via WebSocket.
 * The server is authoritative — client does NOT execute locally.
 * Visual commands (BuildNode, BuildEdge) create ghost entities for instant feedback.
 *
 * In single-player (worldId is null): executes command on local WASM bridge.
 *
 * All UI components should use `gameCommand()` instead of `bridge.processCommand()`.
 */
import { get } from 'svelte/store';
import { worldId, addGhostNode, addGhostEdge } from '$lib/stores/multiplayerState';
import { sendCommand } from '$lib/multiplayer/WebSocketClient';
import { exitPlacementMode } from '$lib/stores/uiState';
import * as bridge from '$lib/wasm/bridge';
import { audioManager } from '$lib/audio/AudioManager';

/**
 * Returns the sequence number if multiplayer (for correlating acks),
 * or null if single-player.
 */
export function gameCommand(command: Record<string, unknown>): number | null {
	const wId = get(worldId);
	if (wId) {
		// Multiplayer: send to server, don't execute locally
		const seq = sendCommand(wId, command);

		// Play build placement sound for optimistic feedback
		if ('BuildNode' in command || 'BuildEdge' in command) {
			audioManager.playSfx('build');
		}

		// Create ghost entity for visual commands (instant feedback)
		if ('BuildNode' in command) {
			const c = command.BuildNode as Record<string, unknown>;
			addGhostNode(
				seq,
				c.lon as number,
				c.lat as number,
				(c.node_type as string) || 'CellTower',
				(c.network_level as string) || 'Local'
			);
		} else if ('BuildEdge' in command) {
			const c = command.BuildEdge as Record<string, unknown>;
			addGhostEdge(
				seq,
				c.from as number,
				c.to as number,
				(c.edge_type as string) || 'FiberOptic'
			);
		}

		return seq;
	} else {
		// Single-player: execute locally
		const failed = bridge.processCommand(command);

		// Immediately signal map to re-render so new nodes/edges appear
		// without waiting for the 2-second fallback interval
		window.dispatchEvent(new CustomEvent('map-dirty'));

		// Play build placement sound on success
		if (!failed && ('BuildNode' in command || 'BuildEdge' in command)) {
			audioManager.playSfx('build');
		}

		// Auto-exit build mode if a build command failed (e.g. insufficient funds)
		if (failed && ('BuildNode' in command || 'BuildEdge' in command)) {
			audioManager.playSfx('error');
			exitPlacementMode();
		}

		return null;
	}
}
