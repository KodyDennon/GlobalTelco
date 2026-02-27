// ── Vector polygon terrain layer ────────────────────────────────────────────
// Renders procgen terrain as crisp deck.gl vector polygons instead of a blurry
// bitmap. Each grid cell becomes a Voronoi-approximated polygon colored by
// terrain type, with coastline edge highlights and elevation-based tinting.

import { PolygonLayer, PathLayer } from '@deck.gl/layers';
import type { Layer } from '@deck.gl/core';
import type { GridCell } from '$lib/wasm/types';
import { SATELLITE_COLORS } from '../constants';

// ── Types ──────────────────────────────────────────────────────────────────

interface CellPolygon {
    polygon: [number, number][];
    terrain: string;
    index: number;
    fillColor: [number, number, number, number];
}

interface CoastlineSegment {
    path: [number, number][];
}

// ── Ocean / terrain classification ─────────────────────────────────────────

const OCEAN_TYPES = new Set(['OceanShallow', 'OceanDeep', 'OceanTrench', 'Ocean']);

/** Pseudo-elevation by terrain type (0 = deep ocean, 1 = high mountain). */
const TERRAIN_ELEVATION: Record<string, number> = {
    OceanTrench:  -0.1,
    OceanDeep:    0.0,
    Ocean:        0.1,
    OceanShallow: 0.2,
    Coastal:      0.3,
    Rural:        0.4,
    Suburban:     0.45,
    Urban:        0.45,
    Desert:       0.4,
    Tundra:       0.5,
    Frozen:       0.55,
    Mountainous:  0.85,
};

// ── Color utilities ────────────────────────────────────────────────────────

/** Simple deterministic hash for per-cell color variation. */
function cellHash(index: number): number {
    let h = index * 2654435761;
    h = ((h >>> 16) ^ h) * 0x45d9f3b;
    h = ((h >>> 16) ^ h) * 0x45d9f3b;
    h = (h >>> 16) ^ h;
    return (h & 0xffff) / 0xffff; // 0..1
}

/**
 * Compute the fill color for a cell, applying:
 * - Base terrain color from SATELLITE_COLORS
 * - Per-cell noise variation (subtle, +/-8 per channel)
 * - Elevation tinting (mountains get a whiter tint, low land gets richer green)
 * - Ocean depth gradient (deeper = darker)
 */
function computeFillColor(
    terrain: string,
    index: number
): [number, number, number, number] {
    const base = SATELLITE_COLORS[terrain] ?? SATELLITE_COLORS['Ocean'] ?? [8, 18, 42];
    let [r, g, b] = base;

    const noise = cellHash(index);
    const variation = (noise - 0.5) * 16; // -8 to +8

    r = Math.round(r + variation);
    g = Math.round(g + variation);
    b = Math.round(b + variation);

    // Elevation-based tinting
    const elevation = TERRAIN_ELEVATION[terrain] ?? 0.4;

    if (OCEAN_TYPES.has(terrain)) {
        // Ocean depth: deeper cells are darker, shallower are brighter
        const depthFactor = elevation; // 0 (deep) to 0.2 (shallow)
        const brighten = depthFactor * 30;
        r = Math.round(r + brighten);
        g = Math.round(g + brighten);
        b = Math.round(b + brighten);
    } else if (terrain === 'Mountainous') {
        // Mountains: add a whiter/lighter tint suggesting snow or rock
        const snowFactor = 0.15 + (noise * 0.1);
        r = Math.round(r + snowFactor * 60);
        g = Math.round(g + snowFactor * 55);
        b = Math.round(b + snowFactor * 50);
    } else if (terrain === 'Rural') {
        // Lower land: richer green at low elevations
        const greenBoost = (1.0 - elevation) * 12;
        g = Math.round(g + greenBoost);
    }

    // Clamp all channels
    r = Math.max(0, Math.min(255, r));
    g = Math.max(0, Math.min(255, g));
    b = Math.max(0, Math.min(255, b));

    return [r, g, b, 255];
}

// ── Polygon computation ────────────────────────────────────────────────────

/**
 * Approximate the Voronoi polygon for a cell using the midpoints between
 * the cell center and each of its neighbors, ordered by angle.
 *
 * For cell C with neighbors N1..Nk:
 *   - midpoint_i = ((C.lon + Ni.lon)/2, (C.lat + Ni.lat)/2)
 *   - Sort midpoints by angle from C
 *   - The polygon is the ordered ring of midpoints (closed)
 *
 * This produces a tight, gap-free tessellation when applied to all cells.
 */
function computeCellPolygons(
    cells: GridCell[],
): CellPolygon[] {
    if (cells.length === 0) return [];

    // Build a lookup for cell positions by index for fast neighbor access
    const cellPositions = new Float64Array(cells.length * 2);
    const cellTerrains: string[] = new Array(cells.length);
    const indexToArrayPos = new Map<number, number>();

    for (let i = 0; i < cells.length; i++) {
        const c = cells[i];
        indexToArrayPos.set(c.index, i);
        cellPositions[i * 2] = c.lon;
        cellPositions[i * 2 + 1] = c.lat;
        cellTerrains[i] = c.terrain;
    }

    const polygons: CellPolygon[] = [];

    for (let i = 0; i < cells.length; i++) {
        const cell = cells[i];
        // Skip cells outside renderable latitude
        if (Math.abs(cell.lat) > 85) continue;

        const neighbors = cell.neighbors;
        if (!neighbors || neighbors.length < 3) continue;

        const clon = cell.lon;
        const clat = cell.lat;

        // Compute midpoints to each neighbor
        const midpoints: { lon: number; lat: number; angle: number }[] = [];

        for (const nIdx of neighbors) {
            const arrPos = indexToArrayPos.get(nIdx);
            if (arrPos === undefined) continue;

            const nlon = cellPositions[arrPos * 2];
            const nlat = cellPositions[arrPos * 2 + 1];

            // Handle antimeridian wrapping: if neighbor is more than 180 degrees
            // away in longitude, wrap it
            let dlon = nlon - clon;
            if (dlon > 180) dlon -= 360;
            if (dlon < -180) dlon += 360;
            const wrappedNlon = clon + dlon;

            const midLon = (clon + wrappedNlon) / 2;
            const midLat = (clat + nlat) / 2;
            const angle = Math.atan2(midLat - clat, midLon - clon);

            midpoints.push({ lon: midLon, lat: midLat, angle });
        }

        if (midpoints.length < 3) continue;

        // Sort by angle to get ordered polygon ring
        midpoints.sort((a, b) => a.angle - b.angle);

        // Close the ring
        const ring: [number, number][] = midpoints.map(m => [m.lon, m.lat]);
        ring.push(ring[0]);

        polygons.push({
            polygon: ring,
            terrain: cell.terrain,
            index: cell.index,
            fillColor: computeFillColor(cell.terrain, cell.index),
        });
    }

    return polygons;
}

/**
 * Extract coastline edges: segments where a land cell borders an ocean cell.
 * Returns line segments (pairs of midpoints) along the land-ocean boundary.
 */
function extractCoastlineEdges(cells: GridCell[]): CoastlineSegment[] {
    if (cells.length === 0) return [];

    const indexToArrayPos = new Map<number, number>();
    const cellPositions = new Float64Array(cells.length * 2);
    const cellTerrains: string[] = new Array(cells.length);

    for (let i = 0; i < cells.length; i++) {
        const c = cells[i];
        indexToArrayPos.set(c.index, i);
        cellPositions[i * 2] = c.lon;
        cellPositions[i * 2 + 1] = c.lat;
        cellTerrains[i] = c.terrain;
    }

    const segments: CoastlineSegment[] = [];

    // For each land cell, find neighbor pairs where one neighbor is ocean and
    // the next (in angular order around the cell) is also ocean or land.
    // The coastline segment lies between the midpoints of two consecutive
    // neighbors where the cell straddles the land-ocean boundary.

    for (let i = 0; i < cells.length; i++) {
        const cell = cells[i];
        if (Math.abs(cell.lat) > 85) continue;
        const isLand = !OCEAN_TYPES.has(cell.terrain);
        if (!isLand) continue; // Only process from the land side

        const neighbors = cell.neighbors;
        if (!neighbors || neighbors.length < 3) continue;

        const clon = cell.lon;
        const clat = cell.lat;

        // Compute midpoints and whether each neighbor is ocean
        interface NeighborInfo {
            midLon: number;
            midLat: number;
            angle: number;
            isOcean: boolean;
        }

        const neighborInfos: NeighborInfo[] = [];

        for (const nIdx of neighbors) {
            const arrPos = indexToArrayPos.get(nIdx);
            if (arrPos === undefined) continue;

            let nlon = cellPositions[arrPos * 2];
            const nlat = cellPositions[arrPos * 2 + 1];

            let dlon = nlon - clon;
            if (dlon > 180) dlon -= 360;
            if (dlon < -180) dlon += 360;
            const wrappedNlon = clon + dlon;

            const midLon = (clon + wrappedNlon) / 2;
            const midLat = (clat + nlat) / 2;
            const angle = Math.atan2(midLat - clat, midLon - clon);
            const nTerrain = cellTerrains[arrPos];
            const isOcean = OCEAN_TYPES.has(nTerrain);

            neighborInfos.push({ midLon, midLat, angle, isOcean });
        }

        if (neighborInfos.length < 3) continue;
        neighborInfos.sort((a, b) => a.angle - b.angle);

        // Walk around the cell: wherever we transition from an ocean neighbor
        // to a land neighbor (or vice versa), we need a coastline segment
        // connecting the midpoints on either side of the transition.
        // Simpler approach: for each pair of consecutive neighbors where one
        // is ocean, draw a segment between those two midpoints (the Voronoi edge
        // between the cell and its ocean neighbor).
        for (let j = 0; j < neighborInfos.length; j++) {
            const curr = neighborInfos[j];
            const next = neighborInfos[(j + 1) % neighborInfos.length];

            if (curr.isOcean || next.isOcean) {
                // If the current neighbor is ocean, the Voronoi edge between
                // the midpoints of curr and next is on the coastline
                if (curr.isOcean !== next.isOcean) {
                    // Boundary transition — this is a coastline vertex pair
                    // We want the edge running along the boundary between this
                    // cell and its ocean neighbor. The edge goes from one
                    // midpoint to the next midpoint in the polygon ring.
                    segments.push({
                        path: [
                            [curr.midLon, curr.midLat],
                            [next.midLon, next.midLat],
                        ],
                    });
                }
            }
        }
    }

    return segments;
}

// ── Cached data ────────────────────────────────────────────────────────────

let cachedPolygons: CellPolygon[] | null = null;
let cachedCoastlines: CoastlineSegment[] | null = null;
let cachedCellCount: number = 0;

/**
 * Build and cache the vector terrain geometry from grid cells.
 * Call this once during map initialization (replaces the bitmap build step).
 */
export function buildVectorTerrainData(cells: GridCell[]): void {
    cachedPolygons = computeCellPolygons(cells);
    cachedCoastlines = extractCoastlineEdges(cells);
    cachedCellCount = cells.length;
}

/** Check if vector terrain data has been built. */
export function hasVectorTerrainData(): boolean {
    return cachedPolygons !== null && cachedPolygons.length > 0;
}

/** Get the cached cell polygons for reuse by other layers (e.g. ocean depth). */
export function getCachedPolygons(): CellPolygon[] | null {
    return cachedPolygons;
}

// Re-export CellPolygon type for use by ocean depth layer
export type { CellPolygon };

/** Dispose cached terrain data to free memory. */
export function disposeVectorTerrainData(): void {
    cachedPolygons = null;
    cachedCoastlines = null;
    cachedCellCount = 0;
}

// ── Layer creation ─────────────────────────────────────────────────────────

/**
 * Create the vector terrain layers for Procgen mode.
 * Returns an array of layers: [terrain polygons, coastline paths].
 *
 * @returns Array of deck.gl layers, or empty array if data not ready.
 */
export function createVectorTerrainLayers(): Layer[] {
    if (!cachedPolygons || cachedPolygons.length === 0) return [];

    const layers: Layer[] = [];

    // 1. Terrain polygon fill layer — SolidPolygonLayer via PolygonLayer (no strokes)
    layers.push(new PolygonLayer({
        id: 'vector-terrain-fill',
        data: cachedPolygons,
        getPolygon: (d: CellPolygon) => d.polygon,
        getFillColor: (d: CellPolygon) => d.fillColor,
        // No stroke on individual cells for clean look — coastlines handled separately
        stroked: false,
        filled: true,
        pickable: true,
        // Ensure terrain renders below everything else
        parameters: { depthTest: false },
        onClick: (info: any) => {
            window.dispatchEvent(new CustomEvent('entity-selected', {
                detail: { id: null, type: null }
            }));
            if (info.coordinate) {
                const [lon, lat] = info.coordinate;
                window.dispatchEvent(new CustomEvent('map-clicked', {
                    detail: { lon, lat }
                }));
            }
        },
        updateTriggers: {
            getFillColor: [cachedCellCount],
        },
    }));

    // 2. Coastline edge highlights
    if (cachedCoastlines && cachedCoastlines.length > 0) {
        layers.push(new PathLayer({
            id: 'vector-terrain-coastlines',
            data: cachedCoastlines,
            getPath: (d: CoastlineSegment) => d.path,
            getColor: [40, 80, 120, 140],
            getWidth: 1.5,
            widthUnits: 'pixels',
            pickable: false,
            parameters: { depthTest: false },
        }));
    }

    return layers;
}
