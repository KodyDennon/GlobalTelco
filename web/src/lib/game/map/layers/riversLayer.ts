// ── River, lake, and coastline glow layer ────────────────────────────────────
// Renders rivers as PathLayers traced through low-elevation inland cells toward
// ocean/coastal cells. Lakes rendered as blue PolygonLayers. Coastline glow
// rendered as a semi-transparent cyan PathLayer along land-ocean boundaries.
//
// Rivers are derived from grid cell data: cells adjacent to Coastal/Ocean
// terrain that form chains of descending cells are traced as river paths.
// This is a frontend approximation since the Rust river system cell indices
// are not yet exposed through the WASM bridge.
//
// Procgen mode only — rivers, lakes, and coastline glow are not rendered
// in Real Earth mode (satellite imagery provides these visually).

import { PathLayer, PolygonLayer } from '@deck.gl/layers';
import type { Layer } from '@deck.gl/core';
import type { GridCell } from '$lib/wasm/types';

// ── Types ──────────────────────────────────────────────────────────────────

interface RiverPath {
    path: [number, number][];
    flow: number; // 0..1 normalized flow for width scaling
}

interface LakePolygon {
    polygon: [number, number][];
    index: number;
}

interface CoastGlowSegment {
    path: [number, number][];
}

// ── Constants ──────────────────────────────────────────────────────────────

const OCEAN_TYPES = new Set(['OceanShallow', 'OceanDeep', 'Ocean']);
const WATER_ADJACENT = new Set(['Coastal', 'OceanShallow']);

/** Terrain types that rivers cannot flow through. */
const IMPASSABLE_FOR_RIVERS = new Set(['OceanShallow', 'OceanDeep', 'Ocean', 'Frozen', 'Tundra']);

/** Terrain types that are good river sources (high ground). */
const RIVER_SOURCE_TERRAIN = new Set(['Mountainous']);

/** Terrain types that attract river flow (low ground). */
const LOW_TERRAIN = new Set(['Rural', 'Suburban', 'Coastal']);

// ── Pseudo-elevation for river tracing ─────────────────────────────────────

const TERRAIN_HEIGHT: Record<string, number> = {
    OceanDeep: 0,
    Ocean: 0,
    OceanShallow: 0.1,
    Coastal: 0.2,
    Rural: 0.35,
    Suburban: 0.4,
    Urban: 0.45,
    Desert: 0.5,
    Tundra: 0.55,
    Frozen: 0.6,
    Mountainous: 0.85,
};

/** Deterministic hash for cell-based randomness. */
function cellHash(index: number): number {
    let h = index * 2654435761;
    h = ((h >>> 16) ^ h) * 0x45d9f3b;
    h = ((h >>> 16) ^ h) * 0x45d9f3b;
    h = (h >>> 16) ^ h;
    return (h & 0xffff) / 0xffff;
}

// ── River tracing ──────────────────────────────────────────────────────────

/**
 * Trace rivers from mountain cells down to coastal/ocean cells using
 * steepest-descent through the Voronoi grid. This approximates the
 * Rust-side hydraulic simulation using only grid cell terrain types.
 */
function traceRivers(cells: GridCell[]): RiverPath[] {
    if (cells.length === 0) return [];

    const n = cells.length;
    const indexToPos = new Map<number, number>();
    for (let i = 0; i < n; i++) {
        indexToPos.set(cells[i].index, i);
    }

    // Assign pseudo-elevation with per-cell noise
    const elevation = new Float64Array(n);
    for (let i = 0; i < n; i++) {
        const base = TERRAIN_HEIGHT[cells[i].terrain] ?? 0.4;
        elevation[i] = base + (cellHash(cells[i].index) - 0.5) * 0.1;
    }

    // Find mountain source cells
    const sources: { pos: number; elev: number }[] = [];
    for (let i = 0; i < n; i++) {
        if (RIVER_SOURCE_TERRAIN.has(cells[i].terrain)) {
            // Only pick mountains that have lower neighbors (not isolated peaks)
            const hasLower = cells[i].neighbors.some(nIdx => {
                const np = indexToPos.get(nIdx);
                return np !== undefined && elevation[np] < elevation[i];
            });
            if (hasLower) {
                sources.push({ pos: i, elev: elevation[i] });
            }
        }
    }

    // Sort sources by elevation descending, take limited set for performance
    sources.sort((a, b) => b.elev - a.elev);
    const maxRivers = Math.min(sources.length, 80);

    const visited = new Uint8Array(n);
    const rivers: RiverPath[] = [];

    for (let si = 0; si < maxRivers; si++) {
        const start = sources[si].pos;
        if (visited[start]) continue;

        const path: [number, number][] = [];
        let current = start;
        const maxSteps = 60;

        for (let step = 0; step < maxSteps; step++) {
            const cell = cells[current];
            if (Math.abs(cell.lat) > 85) break;

            path.push([cell.lon, cell.lat]);
            visited[current] = 1;

            // Reached coast or ocean — river is complete
            if (OCEAN_TYPES.has(cell.terrain) || cell.terrain === 'Coastal') {
                break;
            }

            // Find steepest descent neighbor
            let bestNext = -1;
            let bestDrop = -Infinity;

            for (const nIdx of cell.neighbors) {
                const np = indexToPos.get(nIdx);
                if (np === undefined) continue;
                if (IMPASSABLE_FOR_RIVERS.has(cells[np].terrain) && !WATER_ADJACENT.has(cells[np].terrain)) continue;

                const drop = elevation[current] - elevation[np];
                if (drop > bestDrop) {
                    bestDrop = drop;
                    bestNext = np;
                }
            }

            if (bestNext === -1 || bestDrop <= -0.05) break; // stuck or going uphill too much
            current = bestNext;

            // Allow convergence — don't revisit but allow short overlap at ends
            if (visited[current] && step > 2) break;
        }

        if (path.length >= 4) {
            // Normalize flow based on path length (longer rivers = more flow)
            const flow = Math.min(1.0, path.length / 30);
            rivers.push({ path, flow });
        }
    }

    return rivers;
}

// ── Coastline glow extraction ──────────────────────────────────────────────

/**
 * Extract coastline segments for the glow effect.
 * These are edges between land cells and ocean cells, rendered as
 * a semi-transparent cyan line wider than the base coastline.
 */
function extractCoastGlow(cells: GridCell[]): CoastGlowSegment[] {
    if (cells.length === 0) return [];

    const n = cells.length;
    const indexToPos = new Map<number, number>();
    for (let i = 0; i < n; i++) {
        indexToPos.set(cells[i].index, i);
    }

    const segments: CoastGlowSegment[] = [];

    for (let i = 0; i < n; i++) {
        const cell = cells[i];
        if (Math.abs(cell.lat) > 85) continue;
        if (OCEAN_TYPES.has(cell.terrain)) continue; // only from land side

        const neighbors = cell.neighbors;
        if (!neighbors || neighbors.length < 3) continue;

        // Collect midpoints with ocean neighbor classification
        const mids: { lon: number; lat: number; angle: number; isOcean: boolean }[] = [];
        for (const nIdx of neighbors) {
            const np = indexToPos.get(nIdx);
            if (np === undefined) continue;

            const ncell = cells[np];
            let dlon = ncell.lon - cell.lon;
            if (dlon > 180) dlon -= 360;
            if (dlon < -180) dlon += 360;
            const wrappedLon = cell.lon + dlon;

            const midLon = (cell.lon + wrappedLon) / 2;
            const midLat = (cell.lat + ncell.lat) / 2;
            const angle = Math.atan2(midLat - cell.lat, midLon - cell.lon);

            mids.push({
                lon: midLon,
                lat: midLat,
                angle,
                isOcean: OCEAN_TYPES.has(ncell.terrain),
            });
        }

        if (mids.length < 3) continue;
        mids.sort((a, b) => a.angle - b.angle);

        // Walk around: find boundary transitions land<->ocean
        for (let j = 0; j < mids.length; j++) {
            const curr = mids[j];
            const next = mids[(j + 1) % mids.length];

            if (curr.isOcean !== next.isOcean) {
                segments.push({
                    path: [[curr.lon, curr.lat], [next.lon, next.lat]],
                });
            }
        }
    }

    return segments;
}

// ── Lake extraction ────────────────────────────────────────────────────────

/**
 * Find cells that could be lakes: inland OceanShallow cells surrounded
 * mostly by land. Uses the vectorTerrainLayer's Voronoi polygon approach.
 *
 * Since we don't have explicit "Lake" terrain type from Rust, we look for
 * small clusters of OceanShallow cells that are surrounded by land on
 * most sides (enclosed water bodies).
 */
function extractLakes(cells: GridCell[]): LakePolygon[] {
    if (cells.length === 0) return [];

    const n = cells.length;
    const indexToPos = new Map<number, number>();
    for (let i = 0; i < n; i++) {
        indexToPos.set(cells[i].index, i);
    }

    const lakes: LakePolygon[] = [];

    // Find OceanShallow cells where most neighbors are land
    for (let i = 0; i < n; i++) {
        const cell = cells[i];
        if (cell.terrain !== 'OceanShallow') continue;
        if (Math.abs(cell.lat) > 85) continue;

        const neighbors = cell.neighbors;
        if (!neighbors || neighbors.length < 3) continue;

        let landCount = 0;
        let totalCount = 0;

        for (const nIdx of neighbors) {
            const np = indexToPos.get(nIdx);
            if (np === undefined) continue;
            totalCount++;
            if (!OCEAN_TYPES.has(cells[np].terrain)) {
                landCount++;
            }
        }

        // If more than 60% of neighbors are land, this is a lake cell
        if (totalCount > 0 && landCount / totalCount > 0.6) {
            // Build a Voronoi polygon for this cell (same approach as vectorTerrainLayer)
            const midpoints: { lon: number; lat: number; angle: number }[] = [];
            for (const nIdx of neighbors) {
                const np = indexToPos.get(nIdx);
                if (np === undefined) continue;

                const ncell = cells[np];
                let dlon = ncell.lon - cell.lon;
                if (dlon > 180) dlon -= 360;
                if (dlon < -180) dlon += 360;
                const wrappedLon = cell.lon + dlon;

                const midLon = (cell.lon + wrappedLon) / 2;
                const midLat = (cell.lat + ncell.lat) / 2;
                const angle = Math.atan2(midLat - cell.lat, midLon - cell.lon);
                midpoints.push({ lon: midLon, lat: midLat, angle });
            }

            if (midpoints.length < 3) continue;
            midpoints.sort((a, b) => a.angle - b.angle);

            const ring: [number, number][] = midpoints.map(m => [m.lon, m.lat]);
            ring.push(ring[0]);

            lakes.push({ polygon: ring, index: cell.index });
        }
    }

    // Limit to 200 lake cells for performance
    return lakes.slice(0, 200);
}

// ── Cached data ────────────────────────────────────────────────────────────

let cachedRivers: RiverPath[] | null = null;
let cachedCoastGlow: CoastGlowSegment[] | null = null;
let cachedLakes: LakePolygon[] | null = null;

/**
 * Build river, lake, and coastline glow data from grid cells.
 * Call once during map initialization (after vector terrain data is built).
 */
export function buildRiverData(cells: GridCell[]): void {
    cachedRivers = traceRivers(cells);
    cachedCoastGlow = extractCoastGlow(cells);
    cachedLakes = extractLakes(cells);
}

/** Dispose cached river data to free memory. */
export function disposeRiverData(): void {
    cachedRivers = null;
    cachedCoastGlow = null;
    cachedLakes = null;
}

/**
 * Get the cached river paths (read-only) for bridge marker detection.
 * Returns array of { path: [lon, lat][], flow: number } or null if not built.
 */
export function getCachedRiverPaths(): { path: [number, number][]; flow: number }[] | null {
    return cachedRivers;
}

// ── Layer creation ─────────────────────────────────────────────────────────

/**
 * Create river, lake, and coastline glow layers for Procgen mode.
 *
 * @param currentZoom - Current map zoom level.
 * @param isRealEarth - Whether map is in Real Earth mode (skip rendering).
 * @returns Array of deck.gl layers, or empty array if not applicable.
 */
export function createRiversLayers(currentZoom: number, isRealEarth: boolean): Layer[] {
    if (isRealEarth) return [];
    if (currentZoom < 3) return []; // Only visible at zoom 3+

    const layers: Layer[] = [];

    // 1. Coastline glow — wide, semi-transparent cyan
    if (cachedCoastGlow && cachedCoastGlow.length > 0) {
        layers.push(new PathLayer({
            id: 'coastline-glow',
            data: cachedCoastGlow,
            getPath: (d: CoastGlowSegment) => d.path,
            getColor: [100, 200, 255, 60],
            getWidth: currentZoom < 5 ? 3 : currentZoom < 7 ? 4 : 5,
            widthUnits: 'pixels',
            widthMinPixels: 2,
            widthMaxPixels: 6,
            pickable: false,
            parameters: {
                depthTest: false,
                blend: true,
                blendFunc: [WebGLRenderingContext.SRC_ALPHA, WebGLRenderingContext.ONE],
            },
        }));
    }

    // 2. Lakes — blue fill polygons
    if (cachedLakes && cachedLakes.length > 0) {
        layers.push(new PolygonLayer({
            id: 'lakes-fill',
            data: cachedLakes,
            getPolygon: (d: LakePolygon) => d.polygon,
            getFillColor: [30, 70, 140, 180],
            stroked: true,
            getLineColor: [60, 130, 200, 100],
            getLineWidth: 1,
            lineWidthUnits: 'pixels',
            filled: true,
            pickable: false,
            parameters: { depthTest: false },
        }));
    }

    // 3. Rivers — blue paths with zoom-dependent width
    if (cachedRivers && cachedRivers.length > 0) {
        const baseWidth = currentZoom < 5 ? 1 : currentZoom < 7 ? 2 : 3;

        layers.push(new PathLayer({
            id: 'rivers-paths',
            data: cachedRivers,
            getPath: (d: RiverPath) => d.path,
            getColor: [60, 130, 200, 180],
            getWidth: (d: RiverPath) => baseWidth * (0.5 + d.flow * 0.5),
            widthUnits: 'pixels',
            widthMinPixels: 1,
            widthMaxPixels: 4,
            capRounded: true,
            jointRounded: true,
            pickable: false,
            parameters: { depthTest: false },
        }));
    }

    return layers;
}
