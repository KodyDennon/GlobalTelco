import * as bridge from '$lib/wasm/bridge';
import { gameCommand } from '$lib/game/commandRouter';
import { get } from 'svelte/store';
import {
	buildMode,
	buildEdgeSource,
	buildMenuLocation,
	activeOverlay,
	activePanelGroup,
	activeGroupTab,
	selectedEntityId,
	selectedEntityType,
	openPanelGroup,
	closePanelGroup,
	showConfirm,
	PANEL_GROUP_TABS,
	hotbarSlots,
	enterPlacementMode,
	exitPlacementMode,
	radialMenuOpen,
} from '$lib/stores/uiState';
import type { PanelGroupType, OverlayType } from '$lib/stores/uiState';
import { setSpeed, togglePause, quickSave } from './GameLoop';

type KeyAction = () => void;

/** Normalize a KeyboardEvent into a canonical binding string like "ctrl+s", "shift+tab". */
function normalizeKey(e: KeyboardEvent): string {
	const parts: string[] = [];
	if (e.ctrlKey || e.metaKey) parts.push('ctrl');
	if (e.altKey) parts.push('alt');
	if (e.shiftKey) parts.push('shift');

	let key = e.key;
	// Normalize common key names
	if (key === ' ') key = 'space';
	else if (key === '?') key = '?';
	else key = key.toLowerCase();

	parts.push(key);
	return parts.join('+');
}

/** Whether the user is typing into an interactive input element. */
function isInputFocused(e: KeyboardEvent): boolean {
	const target = e.target;
	if (target instanceof HTMLInputElement) return true;
	if (target instanceof HTMLTextAreaElement) return true;
	if (target instanceof HTMLSelectElement) return true;
	if (target instanceof HTMLElement && target.isContentEditable) return true;
	return false;
}

// Store for hotkey overlay visibility
import { writable } from 'svelte/store';
export const hotkeyOverlayVisible = writable<boolean>(false);

// Store for search overlay visibility (toggled by / key)
export const searchOverlayVisible = writable<boolean>(false);

export class KeyboardManager {
	private bindings: Map<string, KeyAction> = new Map();
	private enabled: boolean = true;
	private boundHandler: (e: KeyboardEvent) => void;

	constructor() {
		this.boundHandler = (e: KeyboardEvent) => this.handleKeyDown(e);
	}

	/** Start listening for keyboard events on the window. */
	attach(): void {
		window.addEventListener('keydown', this.boundHandler);
	}

	/** Stop listening for keyboard events. */
	detach(): void {
		window.removeEventListener('keydown', this.boundHandler);
	}

	/** Temporarily disable all hotkeys (e.g., modal is open). */
	enable(): void {
		this.enabled = true;
	}

	/** Temporarily enable all hotkeys. */
	disable(): void {
		this.enabled = false;
	}

	/** Register a key binding. */
	bind(key: string, action: KeyAction): void {
		this.bindings.set(key.toLowerCase(), action);
	}

	/** Remove a key binding. */
	unbind(key: string): void {
		this.bindings.delete(key.toLowerCase());
	}

	/** Core handler dispatched on every keydown. */
	handleKeyDown(e: KeyboardEvent): void {
		if (!this.enabled) return;
		if (isInputFocused(e)) return;

		const normalized = normalizeKey(e);
		const action = this.bindings.get(normalized);
		if (action) {
			e.preventDefault();
			action();
		}
	}

	/** Clean up everything. */
	dispose(): void {
		this.detach();
		this.bindings.clear();
	}
}

// ── Helpers for overlay toggling ───────────────────────────────────────────

function toggleOverlay(overlay: OverlayType): void {
	activeOverlay.update((current) => (current === overlay ? 'none' : overlay));
}

// ── Helpers for panel group toggling ───────────────────────────────────────

function togglePanelGroup(group: PanelGroupType): void {
	if (get(activePanelGroup) === group) {
		closePanelGroup();
	} else {
		openPanelGroup(group);
	}
}

// ── Tab cycling within the active panel group ─────────────────────────────

function cycleTab(): void {
	const group = get(activePanelGroup);
	if (group === 'none') return;
	const tabs = PANEL_GROUP_TABS[group];
	if (!tabs || tabs.length === 0) return;
	const currentTab = get(activeGroupTab);
	const currentIndex = tabs.findIndex((t) => t.key === currentTab);
	const nextIndex = (currentIndex + 1) % tabs.length;
	openPanelGroup(group, tabs[nextIndex].key);
}

// ── Build mode toggles ────────────────────────────────────────────────────

function toggleNodeBuild(): void {
	buildMode.update((m) => {
		if (m === 'node') {
			buildMenuLocation.set(null);
			return null;
		}
		buildEdgeSource.set(null);
		return 'node';
	});
}

function toggleEdgeBuild(): void {
	buildMode.update((m) => {
		if (m === 'edge') {
			buildEdgeSource.set(null);
			return null;
		}
		buildMenuLocation.set(null);
		return 'edge';
	});
}

function cancelOrClose(): void {
	// Priority: close radial menu > cancel build mode > deselect entity > close panel
	if (get(radialMenuOpen)) {
		radialMenuOpen.set(false);
	} else if (get(buildMode)) {
		exitPlacementMode();
	} else if (get(selectedEntityId) !== null) {
		selectedEntityId.set(null);
		selectedEntityType.set(null);
	} else if (get(activePanelGroup) !== 'none') {
		closePanelGroup();
	}
}

function decommissionSelected(): void {
	const entityId = get(selectedEntityId);
	const entityType = get(selectedEntityType);
	if (entityId === null || entityType !== 'node') return;

	showConfirm(
		`Decommission infrastructure #${entityId}? This cannot be undone.`,
		() => {
			gameCommand({ DecommissionNode: { node_id: entityId } });
			selectedEntityId.set(null);
			selectedEntityType.set(null);
		}
	);
}

// ── Hotbar activation ─────────────────────────────────────────────────────

function activateHotbarSlot(index: number): void {
	const slots = get(hotbarSlots);
	const slot = slots[index];
	if (!slot || !slot.itemType || !slot.category) return;
	enterPlacementMode(slot.itemType, slot.category);
}

// ── Context-sensitive number keys (1-6: panel groups when not building, hotbar otherwise) ──

const PANEL_GROUP_ORDER: PanelGroupType[] = ['finance', 'operations', 'diplomacy', 'research', 'market', 'info'];

function numberKeyAction(index: number): void {
	// If in build mode or a build item is selected, use hotbar
	if (get(buildMode) !== null) {
		activateHotbarSlot(index);
		return;
	}
	// If index 0-5, toggle panel group; if 6-8, only hotbar
	if (index < PANEL_GROUP_ORDER.length) {
		togglePanelGroup(PANEL_GROUP_ORDER[index]);
	} else {
		activateHotbarSlot(index);
	}
}

// ── Map dispatch helpers ──────────────────────────────────────────────────

function panMap(direction: 'up' | 'down' | 'left' | 'right'): void {
	window.dispatchEvent(new CustomEvent('map-pan', { detail: { direction } }));
}

function zoomMap(direction: 'in' | 'out'): void {
	window.dispatchEvent(new CustomEvent('map-zoom', { detail: { direction } }));
}

function resetView(): void {
	window.dispatchEvent(new CustomEvent('map-reset-view'));
}

function togglePitch(): void {
	window.dispatchEvent(new CustomEvent('map-toggle-pitch'));
}

// ── Default bindings ──────────────────────────────────────────────────────

export function createDefaultBindings(manager: KeyboardManager): void {
	// Navigation — WASD (d omitted: used for demand overlay per spec)
	manager.bind('w', () => panMap('up'));
	manager.bind('a', () => panMap('left'));
	manager.bind('s', () => panMap('down'));
	// Navigation — Arrow keys
	manager.bind('arrowup', () => panMap('up'));
	manager.bind('arrowdown', () => panMap('down'));
	manager.bind('arrowleft', () => panMap('left'));
	manager.bind('arrowright', () => panMap('right'));
	// Zoom
	manager.bind('=', () => zoomMap('in'));
	manager.bind('shift++', () => zoomMap('in')); // Shift+= produces '+'
	manager.bind('-', () => zoomMap('out'));
	// Reset view
	manager.bind('home', () => resetView());

	// Speed controls (Shift+number for speed, plain number for hotbar)
	manager.bind('space', () => togglePause());
	manager.bind('shift+1', () => setSpeed(1));
	manager.bind('shift+2', () => setSpeed(2));
	manager.bind('shift+3', () => setSpeed(4));
	manager.bind('shift+4', () => setSpeed(8));

	// Number keys 1-6: context-sensitive (panel groups when idle, hotbar when building)
	// Number keys 7-9: always hotbar (no panel group for those indices)
	manager.bind('1', () => numberKeyAction(0));
	manager.bind('2', () => numberKeyAction(1));
	manager.bind('3', () => numberKeyAction(2));
	manager.bind('4', () => numberKeyAction(3));
	manager.bind('5', () => numberKeyAction(4));
	manager.bind('6', () => numberKeyAction(5));
	manager.bind('7', () => activateHotbarSlot(6));
	manager.bind('8', () => activateHotbarSlot(7));
	manager.bind('9', () => activateHotbarSlot(8));

	// Build modes
	manager.bind('b', () => toggleNodeBuild());
	manager.bind('e', () => toggleEdgeBuild());
	manager.bind('escape', () => cancelOrClose());

	// Overlays
	manager.bind('t', () => toggleOverlay('terrain'));
	manager.bind('o', () => toggleOverlay('ownership'));
	manager.bind('d', () => toggleOverlay('demand'));
	manager.bind('c', () => toggleOverlay('coverage'));
	manager.bind('r', () => toggleOverlay('disaster'));
	manager.bind('g', () => toggleOverlay('congestion'));
	manager.bind('f', () => toggleOverlay('traffic'));
	manager.bind('n', () => activeOverlay.set('none'));

	// Panels — F-keys
	manager.bind('f1', () => togglePanelGroup('finance'));
	manager.bind('f2', () => togglePanelGroup('operations'));
	manager.bind('f3', () => togglePanelGroup('diplomacy'));
	manager.bind('f4', () => togglePanelGroup('research'));
	manager.bind('f5', () => togglePanelGroup('market'));
	manager.bind('f6', () => togglePanelGroup('info'));
	// Tab cycling within open panel
	manager.bind('tab', () => cycleTab());
	// Close panel
	manager.bind('q', () => closePanelGroup());

	// Quick actions
	manager.bind('ctrl+s', () => {
		quickSave();
	});
	manager.bind('delete', () => decommissionSelected());

	// Map modes
	manager.bind('p', () => togglePitch());

	// Help / Hotkey overlay
	manager.bind('h', () => hotkeyOverlayVisible.update((v) => !v));
	manager.bind('shift+?', () => hotkeyOverlayVisible.update((v) => !v));

	// Search overlay (/ key)
	manager.bind('/', () => searchOverlayVisible.set(true));

	// Minimap toggle
	manager.bind('m', () => window.dispatchEvent(new CustomEvent('minimap-toggle')));
}
