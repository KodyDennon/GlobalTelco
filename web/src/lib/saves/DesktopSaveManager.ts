/**
 * Desktop-specific save manager using Tauri IPC commands.
 * Falls back to IndexedDB-based SaveManager when not running in Tauri.
 */

declare global {
	interface Window {
		__TAURI__?: unknown;
		__TAURI_INTERNALS__?: {
			invoke: (cmd: string, args?: Record<string, unknown>) => Promise<unknown>;
		};
	}
}

export function isTauri(): boolean {
	return typeof window !== 'undefined' && !!window.__TAURI_INTERNALS__;
}

async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
	if (!window.__TAURI_INTERNALS__) {
		throw new Error('Tauri API not available');
	}
	return window.__TAURI_INTERNALS__.invoke(cmd, args) as Promise<T>;
}

export async function saveGameDesktop(slot: number, data: string): Promise<string> {
	return invoke<string>('save_game_native', { slot, data });
}

export async function loadGameDesktop(slot: number): Promise<string | null> {
	return invoke<string | null>('load_game_native', { slot });
}

export async function getSavesDir(): Promise<string> {
	return invoke<string>('get_saves_dir');
}

interface NativeSaveEntry {
	name: string;
	path: string;
	size: number;
	modified: number;
}

export async function listSavesDesktop(): Promise<NativeSaveEntry[]> {
	return invoke<NativeSaveEntry[]>('list_saves');
}
