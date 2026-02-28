// ── Satellite overlay layers ─────────────────────────────────────────────────
// Renders orbital satellite positions, inter-satellite links, coverage
// footprints, and ground station downlinks when the 'satellite' overlay is
// active. Uses typed array data from the WASM bridge for performance.

import { ScatterplotLayer, LineLayer, PathLayer } from '@deck.gl/layers';
import type { Layer } from '@deck.gl/core';
import * as bridge from '$lib/wasm/bridge';
import type { SatelliteArrays } from '$lib/wasm/bridge';
import { CORP_COLORS } from '../constants';

// ── Orbit type colors ────────────────────────────────────────────────────────

/** Color per orbit type index (matches OrbitType enum order: LEO=0, MEO=1, GEO=2, HEO=3). */
const ORBIT_COLORS: [number, number, number][] = [
    [56, 189, 248],   // LEO — sky blue
    [96, 165, 250],   // MEO — blue
    [251, 191, 36],   // GEO — amber
    [192, 132, 252],  // HEO — purple
];

/** Status filter: only show operational (2) and decaying (3) satellites on the map. */
const VISIBLE_STATUSES = new Set([2, 3]);

// ── Types ────────────────────────────────────────────────────────────────────

interface SatellitePoint {
    position: [number, number];
    altitude: number;
    orbitType: number;
    status: number;
    fuel: number;
    owner: number;
    id: number;
}

interface ISLLink {
    source: [number, number];
    target: [number, number];
    orbitType: number;
}

interface CoverageFootprint {
    position: [number, number];
    radius: number;
    owner: number;
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/** Approximate coverage footprint radius in meters from altitude (simplified cone). */
function footprintRadiusM(altitudeKm: number): number {
    const R = 6371; // Earth radius km
    const h = altitudeKm;
    const minElev = 25 * (Math.PI / 180); // 25 degree minimum elevation
    const sinRho = R / (R + h);
    const eta = Math.asin(sinRho * Math.cos(minElev));
    const lambda = Math.PI / 2 - minElev - eta;
    return lambda * R * 1000; // convert km to meters
}

// ── Layer creation ───────────────────────────────────────────────────────────

/**
 * Creates satellite overlay layers from typed array data.
 * Returns an array of deck.gl layers rendered above the base map.
 */
export function createSatelliteLayers(
    visible: boolean,
    currentZoom: number,
): Layer[] {
    if (!visible || !bridge.isInitialized()) return [];

    // Fetch typed arrays from WASM bridge
    const arrays = bridge.getSatelliteArrays();
    if (!arrays || arrays.ids.length === 0) return [];

    // Build satellite point data for operational/decaying sats
    const satPoints: SatellitePoint[] = [];
    for (let i = 0; i < arrays.ids.length; i++) {
        if (!VISIBLE_STATUSES.has(arrays.statuses[i])) continue;
        satPoints.push({
            position: [arrays.positions[i * 2], arrays.positions[i * 2 + 1]],
            altitude: arrays.altitudes[i],
            orbitType: arrays.orbitTypes[i],
            status: arrays.statuses[i],
            fuel: arrays.fuelLevels[i],
            owner: arrays.owners[i],
            id: arrays.ids[i],
        });
    }

    if (satPoints.length === 0) return [];

    const layers: Layer[] = [];

    // 1. Coverage footprints (semi-transparent circles under satellites)
    const footprints: CoverageFootprint[] = satPoints
        .filter(s => s.status === 2) // operational only
        .map(s => ({
            position: s.position,
            radius: footprintRadiusM(s.altitude),
            owner: s.owner,
        }));

    if (footprints.length > 0) {
        layers.push(new ScatterplotLayer({
            id: 'satellite-coverage-footprint',
            data: footprints,
            getPosition: (d: CoverageFootprint) => d.position,
            getRadius: (d: CoverageFootprint) => d.radius,
            getFillColor: (d: CoverageFootprint) => {
                const c = CORP_COLORS[d.owner % CORP_COLORS.length];
                return [c[0], c[1], c[2], 20] as [number, number, number, number];
            },
            getLineColor: (d: CoverageFootprint) => {
                const c = CORP_COLORS[d.owner % CORP_COLORS.length];
                return [c[0], c[1], c[2], 50] as [number, number, number, number];
            },
            stroked: true,
            filled: true,
            lineWidthMinPixels: 1,
            pickable: false,
            parameters: {
                depthTest: false,
                blend: true,
                blendFunc: [WebGLRenderingContext.SRC_ALPHA, WebGLRenderingContext.ONE_MINUS_SRC_ALPHA],
            },
        }));
    }

    // 2. Inter-satellite links (connect nearby sats in same orbit type)
    // Build ISL links from orbital data (approximate: connect sequential sats by owner+orbitType)
    const islLinks: ISLLink[] = [];
    const byOwnerOrbit = new Map<string, SatellitePoint[]>();
    for (const s of satPoints) {
        if (s.status !== 2) continue; // operational only
        const key = `${s.owner}-${s.orbitType}`;
        if (!byOwnerOrbit.has(key)) byOwnerOrbit.set(key, []);
        byOwnerOrbit.get(key)!.push(s);
    }

    for (const group of byOwnerOrbit.values()) {
        if (group.length < 2) continue;
        // Sort by longitude for in-plane ISL approximation
        group.sort((a, b) => a.position[0] - b.position[0]);
        for (let i = 0; i < group.length - 1; i++) {
            const a = group[i];
            const b = group[i + 1];
            // Only link if they're reasonably close (< 30 degrees apart)
            const dLon = Math.abs(a.position[0] - b.position[0]);
            if (dLon < 30) {
                islLinks.push({
                    source: a.position,
                    target: b.position,
                    orbitType: a.orbitType,
                });
            }
        }
    }

    if (islLinks.length > 0 && currentZoom >= 2) {
        layers.push(new LineLayer({
            id: 'satellite-isl-links',
            data: islLinks,
            getSourcePosition: (d: ISLLink) => d.source,
            getTargetPosition: (d: ISLLink) => d.target,
            getColor: (d: ISLLink) => {
                const c = ORBIT_COLORS[d.orbitType] ?? ORBIT_COLORS[0];
                return [c[0], c[1], c[2], 80] as [number, number, number, number];
            },
            getWidth: 1,
            widthMinPixels: 1,
            pickable: false,
            parameters: { depthTest: false },
        }));
    }

    // 3. Satellite position dots (main visualization)
    layers.push(new ScatterplotLayer({
        id: 'satellite-positions',
        data: satPoints,
        getPosition: (d: SatellitePoint) => d.position,
        getFillColor: (d: SatellitePoint) => {
            if (d.status === 3) return [239, 68, 68, 200] as [number, number, number, number]; // decaying = red
            const c = ORBIT_COLORS[d.orbitType] ?? ORBIT_COLORS[0];
            return [c[0], c[1], c[2], 220] as [number, number, number, number];
        },
        getLineColor: [255, 255, 255, 120],
        getRadius: (d: SatellitePoint) => {
            // GEO sats are bigger on the map
            if (d.orbitType === 2) return 80000;
            if (d.orbitType === 1) return 50000;
            return 30000;
        },
        radiusMinPixels: 3,
        radiusMaxPixels: 12,
        stroked: true,
        lineWidthMinPixels: 1,
        pickable: true,
        autoHighlight: true,
        highlightColor: [255, 255, 255, 80],
        onClick: ({ object }: any) => {
            if (object) {
                window.dispatchEvent(new CustomEvent('entity-selected', {
                    detail: { id: object.id, type: 'satellite' },
                }));
            }
        },
        parameters: { depthTest: false },
    }));

    // 4. Satellite glow (additive blending for a space-like look)
    layers.push(new ScatterplotLayer({
        id: 'satellite-glow',
        data: satPoints.filter(s => s.status === 2),
        getPosition: (d: SatellitePoint) => d.position,
        getFillColor: (d: SatellitePoint) => {
            const c = ORBIT_COLORS[d.orbitType] ?? ORBIT_COLORS[0];
            return [c[0], c[1], c[2], 60] as [number, number, number, number];
        },
        getRadius: (d: SatellitePoint) => {
            if (d.orbitType === 2) return 160000;
            if (d.orbitType === 1) return 100000;
            return 60000;
        },
        radiusMinPixels: 6,
        radiusMaxPixels: 20,
        pickable: false,
        parameters: {
            depthTest: false,
            blend: true,
            blendFunc: [WebGLRenderingContext.SRC_ALPHA, WebGLRenderingContext.ONE],
        },
    }));

    return layers;
}
