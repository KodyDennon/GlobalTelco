import { PathLayer, ScatterplotLayer, TextLayer, IconLayer } from '@deck.gl/layers';
import { CollisionFilterExtension } from '@deck.gl/extensions';
import type { Layer } from '@deck.gl/core';

import * as bridge from '$lib/wasm/bridge';
import { dataStore } from '../DataStore';
import { CORP_COLORS, EDGE_STYLES, NODE_TIER_SIZE, NETWORK_TIER_LABEL, toIconKey } from '../constants';
import type { ActiveDisaster } from '../../WeatherLayer';

// ── Types ────────────────────────────────────────────────────────────────────

export interface IconMapping {
    x: number;
    y: number;
    width: number;
    height: number;
    mask: boolean;
}

// ── Constants ────────────────────────────────────────────────────────────────

const TIER_RANK: Record<string, number> = {
    Local: 1,
    Regional: 2,
    National: 3,
    Continental: 4,
    GlobalBackbone: 5,
};

const COLUMN_HEIGHT: Record<string, number> = {
    Local: 200,
    Regional: 500,
    National: 1000,
    Continental: 2000,
    GlobalBackbone: 3000,
};

const COVERAGE_RADIUS: Record<string, number> = {
    CellTower: 15000,
    WirelessRelay: 8000,
    SatelliteGround: 200000,
};

// ── Helpers ──────────────────────────────────────────────────────────────────

function minTierForZoom(zoom: number): number {
    if (zoom < 3) return 4;
    if (zoom < 5) return 3;
    return 1;
}

function edgeTierRank(edgeType: string): number {
    // Map edge type string to rank. 
    // Optimization: This could be pre-calculated or cached in DataStore/Rust.
    // For now, we use the string lookup.
    switch (edgeType) {
        case 'Copper': case 'FiberLocal': case 'TelegraphWire': case 'DropCable': case 'DistributionFiber': return 1;
        case 'Microwave': case 'MicrowaveLink': case 'FiberRegional': case 'FeederFiber': case 'FiberMetro': case 'CoaxialCable': case 'CopperTrunkLine': return 2;
        case 'FiberNational': case 'FiberLongHaul': case 'LongDistanceCopper': return 3;
        case 'Satellite': case 'SatelliteLEOLink': case 'EarlySatelliteLink': case 'SatelliteDownlink': case 'IntraplaneISL': case 'CrossplaneISL': case 'Submarine': case 'SubseaFiberCable': case 'SubseaTelegraphCable': case 'DWDM_Backbone': return 4;
        case 'QuantumFiberLink': case 'TerahertzBeam': case 'LaserInterSatelliteLink': return 5;
        default: return 1;
    }
}

function edgeWidthByType(edgeType: string): number {
    // Similar mapping logic...
    if (edgeType.startsWith('Fiber')) return 3;
    if (edgeType.startsWith('Sub')) return 5;
    return 2;
}

/** Get the corp color for an owner ID. Falls back to grey if unknown. */
function getCorpColor(ownerId: number, corpIndex: Map<number, number>): [number, number, number] {
    const idx = corpIndex.get(ownerId);
    if (idx !== undefined) {
        const color = CORP_COLORS[idx % CORP_COLORS.length];
        if (color && color[0] + color[1] + color[2] > 0) return color;
    }
    return [160, 160, 160];
}

// ── Main export ──────────────────────────────────────────────────────────────

export function createInfraLayers(opts: {
    iconAtlas: HTMLCanvasElement | null;
    iconMapping: Record<string, IconMapping>;
    iconAtlasReady: boolean;
    activeOverlay: string;
    currentZoom: number;
    currentTime: number;
    pitch: number;
    hoveredNodeId: number | null;
    playerCorpId?: number;
    activeDisasters?: ActiveDisaster[];
    bounds?: [number, number, number, number];
}): Layer[] {
    const {
        iconAtlas,
        iconMapping,
        iconAtlasReady,
        activeOverlay,
        currentZoom,
        currentTime,
        pitch,
        hoveredNodeId,
        playerCorpId,
        activeDisasters,
        bounds,
    } = opts;

    // Ensure we have access to the corps list for coloring
    // We still fetch corps via JSON for now (it's small), or we could use Typed Corporations
    const corps = bridge.getAllCorporations();
    const corpIndex = new Map<number, number>();
    for (let i = 0; i < corps.length; i++) {
        corpIndex.set(corps[i].id, i);
    }

    const { nodes, edges } = dataStore;
    const minTier = minTierForZoom(currentZoom);

    // ── Filter indices (LOD) ────────────────────────────────────────────────
    
    // We construct a list of indices to render. 
    // This is much faster than constructing objects.
    
    const visibleNodeIndices: number[] = [];
    const visibleEdgeIndices: number[] = [];
    
    // Nodes
    for (let i = 0; i < nodes.count; i++) {
        // Viewport culling (with margin)
        if (bounds) {
            const x = nodes.positions[i*2];
            const y = nodes.positions[i*2+1];
            // Margin of ~1 degree to avoid popping
            if (x < bounds[0] - 1 || x > bounds[2] + 1 || y < bounds[1] - 1 || y > bounds[3] + 1) continue;
        }

        const level = nodes.network_levels[i]; 
        const tier = (level || 0) + 1;
        const owner = nodes.owners[i];
        const isPlayer = playerCorpId !== undefined && owner === playerCorpId;
        
        if (tier < minTier && !isPlayer) continue;
        visibleNodeIndices.push(i);
    }

    // Edges
    for (let i = 0; i < edges.count; i++) {
        // Viewport culling for edges
        if (bounds) {
            const srcX = edges.endpoints[i*4];
            const srcY = edges.endpoints[i*4+1];
            const dstX = edges.endpoints[i*4+2];
            const dstY = edges.endpoints[i*4+3];
            
            const minX = bounds[0] - 1;
            const maxX = bounds[2] + 1;
            const minY = bounds[1] - 1;
            const maxY = bounds[3] + 1;
            
            if (Math.max(srcX, dstX) < minX || Math.min(srcX, dstX) > maxX ||
                Math.max(srcY, dstY) < minY || Math.min(srcY, dstY) > maxY) {
                continue;
            }
        }

        const typeId = edges.edge_types[i];
        const typeStr = dataStore.getEdgeType(typeId);
        const tier = edgeTierRank(typeStr);
        const owner = edges.owners[i];
        const isPlayer = playerCorpId !== undefined && owner === playerCorpId;

        if (tier < minTier && !isPlayer) continue;
        visibleEdgeIndices.push(i);
    }

    // ── Accessors ───────────────────────────────────────────────────────────

    const getNodePosition = (i: number): [number, number] => [nodes.positions[i*2], nodes.positions[i*2+1]];
    const getNodeColor = (i: number): [number, number, number, number] => {
        const owner = nodes.owners[i];
        const c = getCorpColor(owner, corpIndex);
        const health = nodes.stats[i*3]; 
        
        if (health < 0.2) return [239, 68, 68, 220]; // Damaged
        if (health < 0.5) return [245, 158, 11, 220]; // Degraded
        
        if (playerCorpId !== undefined && owner !== playerCorpId) {
            return [c[0], c[1], c[2], 150];
        }
        return [c[0], c[1], c[2], 255];
    };

    const getEdgeColor = (i: number): [number, number, number, number] => {
        const owner = edges.owners[i];
        const c = getCorpColor(owner, corpIndex);
        if (playerCorpId !== undefined && owner !== playerCorpId) {
            return [c[0], c[1], c[2], 100];
        }
        return [c[0], c[1], c[2], 255];
    };
    
    // ── Layers ──────────────────────────────────────────────────────────────
    
    const layers: Layer[] = [];

    // Edges (Binary Layer)
    // For edges, we use the binary format for attributes.
    // Note: PathLayer with waypoints is tricky in binary mode (requires flattened path indices).
    // For now, we keep PathLayer with data mapping but optimize update triggers.
    layers.push(new PathLayer({
        id: 'infra-edges',
        data: visibleEdgeIndices,
        getPath: (i: number): [number, number][] => dataStore.getEdgePath(i),
        getColor: (i: number): [number, number, number, number] => getEdgeColor(i),
        getWidth: (i: number): number => {
             const typeStr = dataStore.getEdgeType(edges.edge_types[i]);
             return edgeWidthByType(typeStr);
        },
        widthUnits: 'pixels',
        widthMinPixels: 1,
        widthMaxPixels: 12,
        jointRounded: true,
        capRounded: true,
        pickable: true,
        autoHighlight: true,
        // Update triggers are important since we use external dataStore
        updateTriggers: {
            getPath: [edges.count, edges.ids], // Only re-eval if data count changes
            getColor: [edges.owners, playerCorpId],
        }
    }));

    // Nodes (Binary Icon Layer)
    if (iconAtlasReady && iconAtlas) {
        // Optimized: Construct binary attributes for the IconLayer
        // This avoids calling getPosition/getIcon/getColor for thousands of nodes in JS.
        const nodeCount = visibleNodeIndices.length;
        const positions = new Float32Array(nodeCount * 3);
        const colors = new Uint8ClampedArray(nodeCount * 4);
        const sizes = new Float32Array(nodeCount);
        const iconIndices = new Uint32Array(nodeCount);

        const nodeTypes = bridge.getStaticDefinitions().node_types;

        for (let j = 0; j < nodeCount; j++) {
            const i = visibleNodeIndices[j];
            
            // Position (X, Y, Z=0)
            positions[j * 3] = nodes.positions[i * 2];
            positions[j * 3 + 1] = nodes.positions[i * 2 + 1];
            positions[j * 3 + 2] = 0;

            // Color
            const color = getNodeColor(i);
            colors[j * 4] = color[0];
            colors[j * 4 + 1] = color[1];
            colors[j * 4 + 2] = color[2];
            colors[j * 4 + 3] = color[3];

            // Size
            const lvl = nodes.network_levels[i];
            const sizeValues = [20, 28, 36, 48, 64];
            sizes[j] = sizeValues[Math.min(lvl, 4)];

            // Icon Index
            // Note: IconLayer in binary mode expects icon indices into the atlas
            // We'll approximate with the node_type index for now or mapping if available
            iconIndices[j] = nodes.node_types[i];
        }

        layers.push(new IconLayer({
            id: 'infra-nodes',
            data: {
                length: nodeCount,
                attributes: {
                    getPosition: { value: positions, size: 3 },
                    getColor: { value: colors, size: 4 },
                    getSize: { value: sizes, size: 1 },
                    // Mapping icon index to the atlas key
                    getIcon: { value: iconIndices, size: 1 }
                }
            },
            iconAtlas: iconAtlas as any,
            iconMapping: iconMapping,
            // Custom accessor for binary icon mapping
            getIcon: (d: any) => {
                const typeId = d.index !== undefined ? iconIndices[d.index] : 0;
                return toIconKey(nodeTypes[typeId]);
            },
            sizeMinPixels: 10,
            sizeMaxPixels: 64,
            pickable: true,
            autoHighlight: true,
            parameters: { depthTest: false }
        } as any));
    } else {
        layers.push(new ScatterplotLayer({
            id: 'infra-nodes-fallback',
            data: visibleNodeIndices,
            getPosition: (i: number): [number, number] => getNodePosition(i),
            getFillColor: (i: number): [number, number, number, number] => getNodeColor(i),
            getRadius: 500, // meters
            radiusMinPixels: 5,
            pickable: true,
        }));
    }

    return layers;
}
