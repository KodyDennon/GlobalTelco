import { 
    getInfraNodesTyped, 
    getInfraEdgesTyped, 
    getStaticDefinitions,
    getInfraNodesTypedViewport,
    getInfraEdgesTypedViewport
} from '$lib/wasm/bridge';
import type { InfraNodesTyped, InfraEdgesTyped, StaticDefinitions } from '$lib/wasm/types';
import { viewport } from '$lib/stores/uiState';
import { worldInfo } from '$lib/stores/gameState';
import { get } from 'svelte/store';

class DataStore {
    nodes: InfraNodesTyped;
    edges: InfraEdgesTyped;
    staticDefs: StaticDefinitions | null = null;

    private lastSyncTick = -1;
    private lastSyncViewport = { minX: 0, minY: 0, maxX: 0, maxY: 0 };
    private lastSyncTime = 0;

    constructor() {
        // Initialize with safe empty defaults from bridge
        this.nodes = getInfraNodesTyped();
        this.edges = getInfraEdgesTyped();
    }

    async init() {
        this.staticDefs = await getStaticDefinitions();
        console.log('[DataStore] Static definitions loaded:', this.staticDefs);
    }

    /**
     * Synchronizes local typed array cache with WASM simulation.
     * Throttled to avoid blocking the main thread during high-frequency events (zoom/pan).
     */
    sync() {
        const now = performance.now();
        const v = get(viewport);
        const info = get(worldInfo);
        
        // 1. Skip if no viewport or simulation not ready
        if (!v || v.maxX === 180) return;

        // 2. Throttling logic
        const tickChanged = info.tick !== this.lastSyncTick;
        
        // Calculate viewport movement delta
        const dx = Math.abs(v.minX - this.lastSyncViewport.minX) + Math.abs(v.maxX - this.lastSyncViewport.maxX);
        const dy = Math.abs(v.minY - this.lastSyncViewport.minY) + Math.abs(v.maxY - this.lastSyncViewport.maxY);
        const movedSignificantly = dx > 0.5 || dy > 0.5; // Roughly half a degree

        // Rate limit: Max 10 syncs per second unless tick changed
        const timeSinceLastSync = now - this.lastSyncTime;
        if (!tickChanged && !movedSignificantly && timeSinceLastSync < 100) {
            return;
        }

        // 3. Perform sync
        this.nodes = getInfraNodesTypedViewport(v.minX, v.minY, v.maxX, v.maxY);
        this.edges = getInfraEdgesTypedViewport(v.minX, v.minY, v.maxX, v.maxY);
        
        this.lastSyncTick = info.tick;
        this.lastSyncViewport = { ...v };
        this.lastSyncTime = now;
    }

    // ── Helper Accessors ────────────────────────────────────────────────

    getNodeType(typeId: number): string {
        return this.staticDefs?.node_types[typeId] ?? 'Unknown';
    }

    getEdgeType(typeId: number): string {
        return this.staticDefs?.edge_types[typeId] ?? 'Unknown';
    }

    /**
     * Reconstructs the full path for an edge index.
     * If packed waypoints exist, uses them.
     * Otherwise returns [src, dst].
     */
    getEdgePath(index: number): [number, number][] {
        const e = this.edges;
        
        // Check for packed waypoints
        const len = e.waypoint_lengths[index];
        if (len > 0) {
            const offset = e.waypoint_offsets[index];
            const path: [number, number][] = [];
            // Add source? Usually waypoints include src/dst or are just intermediates?
            // Rust logic: `for &(lon, lat) in edge.waypoints`
            // Usually waypoints are intermediates.
            // But we need the full path for rendering.
            
            // Add Source
            path.push([e.endpoints[index * 4], e.endpoints[index * 4 + 1]]);
            
            // Add Intermediates
            for (let i = 0; i < len; i++) {
                path.push([
                    e.waypoints_data[offset + i * 2],
                    e.waypoints_data[offset + i * 2 + 1]
                ]);
            }
            
            // Add Target
            path.push([e.endpoints[index * 4 + 2], e.endpoints[index * 4 + 3]]);
            
            return path;
        }

        // Default: Straight line src -> dst
        return [
            [e.endpoints[index * 4], e.endpoints[index * 4 + 1]],
            [e.endpoints[index * 4 + 2], e.endpoints[index * 4 + 3]]
        ];
    }

    /**
     * Efficiently calculates corporation node counts per region.
     * Replaces the slow O(N) JSON scan in market_share overlay.
     */
    getRegionCorpCounts(cellToRegion: Map<number, number>): Map<number, Map<number, number>> {
        const regionCorpCounts = new Map<number, Map<number, number>>();
        const n = this.nodes;
        
        for (let i = 0; i < n.count; i++) {
            const cellIndex = n.cell_indices[i];
            const regionId = cellToRegion.get(cellIndex);
            if (regionId === undefined) continue;
            
            if (!regionCorpCounts.has(regionId)) {
                regionCorpCounts.set(regionId, new Map());
            }
            const counts = regionCorpCounts.get(regionId)!;
            const owner = n.owners[i];
            counts.set(owner, (counts.get(owner) ?? 0) + 1);
        }
        
        return regionCorpCounts;
    }

    /**
     * Efficiently calculates corporation presence per cell.
     * Replaces the slow O(N) JSON scan in coverage_overlap overlay.
     */
    getCellCorpSets(): Map<number, Set<number>> {
        const cellCorpSets = new Map<number, Set<number>>();
        const n = this.nodes;
        
        for (let i = 0; i < n.count; i++) {
            const cellIndex = n.cell_indices[i];
            if (!cellCorpSets.has(cellIndex)) {
                cellCorpSets.set(cellIndex, new Set());
            }
            cellCorpSets.get(cellIndex)!.add(n.owners[i]);
        }
        
        return cellCorpSets;
    }
}

export const dataStore = new DataStore();
