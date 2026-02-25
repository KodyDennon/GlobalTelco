// Cable glow and pole dot layers for infrastructure edges.
// - At zoom < 5: wider semi-transparent glow behind cables
// - At zoom > 7 for aerial edges: small dots along spline representing poles

import { PathLayer, ScatterplotLayer } from '@deck.gl/layers';
import type { Layer } from '@deck.gl/core';
import type { AllInfraEdge } from '$lib/wasm/types';
import { catmullRomSpline } from '../spline';

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

interface GlowEdge {
    path: [number, number][];
    color: [number, number, number, number];
}

interface PoleDot {
    position: [number, number];
}

function getEdgePath(edge: AllInfraEdge): [number, number][] {
    if (edge.waypoints && edge.waypoints.length >= 2) {
        return catmullRomSpline(edge.waypoints, 8);
    }
    return [[edge.src_x, edge.src_y], [edge.dst_x, edge.dst_y]];
}

function getEdgeColor(edgeType: string): [number, number, number] {
    return EDGE_TYPE_COLORS[edgeType] ?? DEFAULT_EDGE_COLOR;
}

/**
 * Create cable glow and pole dot layers.
 *
 * @param edges - All infrastructure edges to render effects for.
 * @param currentZoom - Current map zoom level.
 * @param isRealEarth - Whether in Real Earth mode (skip for procgen effects only? No - glow applies always).
 * @returns Array of deck.gl layers.
 */
export function createCableGlowLayers(
    edges: AllInfraEdge[],
    currentZoom: number,
): Layer[] {
    if (!edges || edges.length === 0) return [];

    const layers: Layer[] = [];

    // 1. Cable glow at low zoom (zoom < 5)
    if (currentZoom < 5 && currentZoom >= 2) {
        const glowData: GlowEdge[] = [];
        for (const edge of edges) {
            const path = getEdgePath(edge);
            if (path.length < 2) continue;
            const baseColor = getEdgeColor(edge.edge_type);
            // Glow alpha: brighter at very low zoom, subtle otherwise
            const alpha = currentZoom < 3 ? 40 : 25;
            glowData.push({
                path,
                color: [baseColor[0], baseColor[1], baseColor[2], alpha],
            });
        }

        if (glowData.length > 0) {
            layers.push(new PathLayer({
                id: 'cable-glow',
                data: glowData,
                getPath: (d: GlowEdge) => d.path,
                getColor: (d: GlowEdge) => d.color,
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
        const poleDots: PoleDot[] = [];

        for (const edge of edges) {
            if (edge.deployment !== 'Aerial') continue;

            const path = getEdgePath(edge);
            if (path.length < 2) continue;

            // Place poles every ~8 points along the tessellated path
            const step = Math.max(1, Math.floor(path.length / 12));
            for (let i = 0; i < path.length; i += step) {
                poleDots.push({ position: path[i] });
            }
        }

        if (poleDots.length > 0) {
            layers.push(new ScatterplotLayer({
                id: 'cable-poles',
                data: poleDots,
                getPosition: (d: PoleDot) => d.position,
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
