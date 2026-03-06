// Cable glow and pole dot layers for infrastructure edges.
// - At zoom < 5: wider semi-transparent glow behind cables
// - At zoom > 7 for aerial edges: small dots along spline representing poles

import { PathLayer, ScatterplotLayer } from '@deck.gl/layers';
import type { Layer } from '@deck.gl/core';
import { dataStore } from '../DataStore';

// Edge type colors (matches infraLayer style)
const EDGE_TYPE_COLORS: Record<string, [number, number, number]> = {
    FiberOptic: [0, 200, 255],
    FeederFiber: [0, 180, 220],
    DistributionFiber: [0, 160, 200],
    DropCable: [0, 140, 180],
    FiberLocal: [0, 120, 200],
    Copper: [200, 150, 50],
    Coaxial: [180, 120, 40],
    Submarine: [80, 180, 255],
    SubseaTelegraphCable: [100, 160, 200],
    SubseaFiberCable: [60, 200, 255],
    Microwave: [255, 200, 50],
    Satellite: [200, 100, 255],
    FiveG: [255, 100, 200],
    TelegraphWire: [160, 140, 100],
};

const DEFAULT_EDGE_COLOR: [number, number, number] = [100, 150, 200];

function getEdgeColor(edgeType: string): [number, number, number] {
    return EDGE_TYPE_COLORS[edgeType] ?? DEFAULT_EDGE_COLOR;
}

/**
 * Create cable glow and pole dot layers.
 *
 * @param _unused - Legacy edges argument (kept for signature compatibility if needed, but ignored)
 * @param currentZoom - Current map zoom level.
 * @returns Array of deck.gl layers.
 */
export function createCableGlowLayers(
    _unused: any,
    currentZoom: number,
): Layer[] {
    const { edges } = dataStore;
    if (edges.count === 0) return [];

    const layers: Layer[] = [];

    // 1. Cable glow at low zoom (zoom < 5)
    if (currentZoom < 5 && currentZoom >= 2) {
        // Filter edges for glow
        const glowIndices: number[] = [];
        for (let i = 0; i < edges.count; i++) {
            // Only glow if it has a path (length > 0)
            if (edges.endpoints.length > i * 4) glowIndices.push(i);
        }

        if (glowIndices.length > 0) {
            layers.push(new PathLayer({
                id: 'cable-glow',
                data: glowIndices,
                getPath: (i: number) => dataStore.getEdgePath(i),
                getColor: (i: number) => {
                    const typeStr = dataStore.getEdgeType(edges.edge_types[i]);
                    const baseColor = getEdgeColor(typeStr);
                    const alpha = currentZoom < 3 ? 40 : 25;
                    return [baseColor[0], baseColor[1], baseColor[2], alpha];
                },
                getWidth: currentZoom < 3 ? 8 : 6,
                widthUnits: 'pixels',
                widthMinPixels: 4,
                widthMaxPixels: 10,
                capRounded: true,
                jointRounded: true,
                pickable: false,
                parameters: {
                    depthTest: false,
                    blend: true,
                    blendFunc: [WebGLRenderingContext.SRC_ALPHA, WebGLRenderingContext.ONE],
                },
            }));
        }
    }

    // 2. Pole dots on aerial edges at high zoom (zoom > 7)
    if (currentZoom > 7) {
        // Generate dots for aerial edges
        // Note: ScatterplotLayer works best with flat point data. 
        // We can't easily use "index" accessor here if one edge generates multiple points.
        // So we must generate the points array. 
        // But we iterate indices to access DataStore.
        
        const poleDots: Float64Array[] = []; // using array of positions [x,y]

        for (let i = 0; i < edges.count; i++) {
            // Deployment type: 0=Underground, 1=Aerial
            if (edges.deployment_types[i] !== 1) continue;

            const path = dataStore.getEdgePath(i);
            if (path.length < 2) continue;

            // Place poles every ~8 points along the tessellated path
            // Note: getEdgePath returns the full waypoints including CatmullRom if implemented in DataStore?
            // Wait, DataStore.getEdgePath currently returns linear segments + packed waypoints.
            // It does NOT do CatmullRom spline tessellation.
            // infraLayer.ts does `catmullRomSpline(rawWaypoints)`.
            
            // If DataStore returns raw waypoints, we might need to spline them here?
            // `infraLayer.ts` does spline in JS.
            // For now, let's assume raw path is enough for poles or we spline if needed.
            // Simulating poles on straight segments is fine.
            
            const step = Math.max(1, Math.floor(path.length / 12));
            for (let j = 0; j < path.length; j += step) {
                poleDots.push(new Float64Array([path[j][0], path[j][1]]));
            }
        }

        if (poleDots.length > 0) {
            layers.push(new ScatterplotLayer({
                id: 'cable-poles',
                data: poleDots,
                getPosition: (d: Float64Array) => [d[0], d[1]],
                getFillColor: [140, 120, 80, 200],
                getRadius: 30,
                radiusMinPixels: 2,
                radiusMaxPixels: 4,
                pickable: false,
                parameters: { depthTest: false },
            }));
        }
    }

    return layers;
}
