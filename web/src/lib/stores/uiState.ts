import { writable } from 'svelte/store';

export type PanelType = 'none' | 'info' | 'dashboard' | 'infrastructure' | 'research' | 'contracts';

export const activePanel = writable<PanelType>('none');
export const selectedEntityId = writable<number | null>(null);
export const selectedEntityType = writable<string | null>(null);
export const hoveredEntityId = writable<number | null>(null);
export const buildMode = writable<string | null>(null);
export const zoomLevel = writable<number>(1);
export const viewport = writable({ minX: -180, minY: -90, maxX: 180, maxY: 90 });
