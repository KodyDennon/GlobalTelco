import { 
    getInfraNodesTyped, 
    getInfraEdgesTyped, 
    getStaticDefinitions,
    getInfraNodesTypedViewport,
    getInfraEdgesTypedViewport
} from '$lib/wasm/bridge';
import type { InfraNodesTyped, InfraEdgesTyped, StaticDefinitions } from '$lib/wasm/types';
import { viewport } from '$lib/stores/uiState';
import { get } from 'svelte/store';

class DataStore {
    nodes: InfraNodesTyped;
    edges: InfraEdgesTyped;
    staticDefs: StaticDefinitions | null = null;

    constructor() {
        // Initialize with safe empty defaults from bridge
        this.nodes = getInfraNodesTyped();
        this.edges = getInfraEdgesTyped();
    }

    async init() {
        this.staticDefs = await getStaticDefinitions();
        console.log('[DataStore] Static definitions loaded:', this.staticDefs);
    }

    sync() {
        // This is called every frame or tick by the render loop
        const v = get(viewport);
        if (v && v.maxX !== 180) { // Check if viewport is actualy set/relevant
            this.nodes = getInfraNodesTypedViewport(v.minX, v.minY, v.maxX, v.maxY);
            this.edges = getInfraEdgesTypedViewport(v.minX, v.minY, v.maxX, v.maxY);
        } else {
            this.nodes = getInfraNodesTyped();
            this.edges = getInfraEdgesTyped();
        }
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
}

export const dataStore = new DataStore();
