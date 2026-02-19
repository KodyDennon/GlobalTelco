import { writable } from 'svelte/store';

export type PanelType = 'none' | 'info' | 'dashboard' | 'infrastructure' | 'research' | 'contracts' | 'region' | 'workforce' | 'advisor' | 'auctions' | 'mergers' | 'intel' | 'achievements';
export type OverlayType = 'none' | 'terrain' | 'ownership' | 'demand' | 'disaster' | 'coverage' | 'congestion' | 'traffic';

export const activePanel = writable<PanelType>('none');
export const selectedEntityId = writable<number | null>(null);
export const selectedEntityType = writable<string | null>(null);
export const hoveredEntityId = writable<number | null>(null);
export const buildMode = writable<string | null>(null); // null | 'node' | 'edge'
export const buildEdgeSource = writable<number | null>(null); // source node ID for edge building
export const buildMenuParcel = writable<{ id: number; x: number; y: number } | null>(null);
export const zoomLevel = writable<number>(1);
export const viewport = writable({ minX: -180, minY: -90, maxX: 180, maxY: 90 });
export const activeOverlay = writable<OverlayType>('none');
export const tooltipData = writable<{ x: number; y: number; content: string } | null>(null);
export const selectedEdgeType = writable<string>('FiberLocal');
