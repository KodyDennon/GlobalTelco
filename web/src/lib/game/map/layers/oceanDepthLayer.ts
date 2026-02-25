// ── Ocean depth visualization layer ─────────────────────────────────────────
// Renders ocean cells with a depth-based blue gradient and subtle contour lines
// when the 'ocean_depth' overlay is active. Reuses cached polygon geometry from
// vectorTerrainLayer to avoid recomputing Voronoi tessellation.

import { PolygonLayer, PathLayer } from '@deck.gl/layers';
import type { Layer } from '@deck.gl/core';
import { getCachedPolygons, type CellPolygon } from './vectorTerrainLayer';

// ── Types ──────────────────────────────────────────────────────────────────

interface OceanDepthPolygon {
    polygon: [number, number][];
    terrain: string;
    index: number;
    fillColor: [number, number, number, number];
    depthLevel: number; // 0 = shallow, 1 = medium, 2 = deep
}

interface DepthContourSegment {
    path: [number, number][];
}

// ── Ocean classification and depth colors ────────────────────────────────

const OCEAN_TYPES = new Set(['OceanShallow', 'OceanDeep', 'Ocean']);

/** Depth-based fill colors for ocean cells. */
const OCEAN_DEPTH_COLORS: Record<string, [number, number, number, number]> = {
    OceanShallow: [18, 42, 82, 230],   // #122a52 — lighter blue
    Ocean:        [8, 26, 42, 240],     // #081a2a — medium depth
    OceanDeep:    [4, 13, 21, 250],     // #040d15 — very deep
};

/** Depth level classification: 0 = shallow, 1 = medium, 2 = deep. */
function getDepthLevel(terrain: string): number {
    if (terrain === 'OceanShallow') return 0;
    if (terrain === 'Ocean') return 1;
    return 2; // OceanDeep
}

/** Simple deterministic hash for per-cell variation. */
function cellHash(index: number): number {
    let h = index * 2654435761;
    h = ((h >>> 16) ^ h) * 0x45d9f3b;
    h = ((h >>> 16) ^ h) * 0x45d9f3b;
    h = (h >>> 16) ^ h;
    return (h & 0xffff) / 0xffff; // 0..1
}

/** Compute ocean cell fill color with subtle per-cell noise variation. */
function computeOceanFillColor(
    terrain: string,
    index: number,
): [number, number, number, number] {
    const base = OCEAN_DEPTH_COLORS[terrain] ?? OCEAN_DEPTH_COLORS['OceanDeep'];
    let [r, g, b, a] = base;

    // Per-cell noise for subtle variation (+/-6 per channel)
    const noise = cellHash(index);
    const variation = (noise - 0.5) * 12;

    r = Math.max(0, Math.min(255, Math.round(r + variation)));
    g = Math.max(0, Math.min(255, Math.round(g + variation * 0.8)));
    b = Math.max(0, Math.min(255, Math.round(b + variation * 0.6)));

    return [r, g, b, a];
}

// ── Cached data ────────────────────────────────────────────────────────────

let cachedOceanPolygons: OceanDepthPolygon[] | null = null;
let cachedContours: DepthContourSegment[] | null = null;

/**
 * Build ocean depth visualization data from the cached vector terrain polygons.
 * Filters to ocean-only cells and computes depth contour lines at boundaries
 * between different ocean depth levels.
 */
export function buildOceanDepthData(cells?: undefined): void {
    const allPolygons = getCachedPolygons();
    if (!allPolygons || allPolygons.length === 0) {
        cachedOceanPolygons = [];
        cachedContours = [];
        return;
    }

    // Filter to ocean cells and compute depth colors
    cachedOceanPolygons = [];
    for (const poly of allPolygons) {
        if (!OCEAN_TYPES.has(poly.terrain)) continue;

        cachedOceanPolygons.push({
            polygon: poly.polygon,
            terrain: poly.terrain,
            index: poly.index,
            fillColor: computeOceanFillColor(poly.terrain, poly.index),
            depthLevel: getDepthLevel(poly.terrain),
        });
    }

    // Build contour lines at depth-level boundaries.
    // For each ocean polygon, walk its ring and check if neighboring ocean cells
    // have a different depth level. We approximate this by checking if consecutive
    // polygon edges border cells of different depth.
    //
    // Since we don't have direct polygon-to-neighbor mapping here, we use the
    // midpoint approach: for each edge of an ocean polygon, if the polygon's
    // depth level differs from a threshold, we add contour segments at
    // depth-level transitions.
    //
    // Simpler approach: build a spatial index of depth levels and find boundaries.
    // For performance, we use the polygon ring edges directly — consecutive ring
    // vertices that lie on boundaries between depth zones create contour lines.

    cachedContours = [];

    // Build an index map: cell index -> depth level
    const depthIndex = new Map<number, number>();
    for (const op of cachedOceanPolygons) {
        depthIndex.set(op.index, op.depthLevel);
    }

    // For contour extraction, iterate all cached polygons and find edges
    // between ocean cells of different depth levels. The polygon ring vertices
    // are midpoints between the cell and its neighbors (from Voronoi computation).
    // Adjacent ring vertices correspond to adjacent neighbors.
    //
    // We use the original allPolygons data since it includes neighbor info via
    // the polygon ring structure. For each ocean cell, consecutive ring edges
    // where the neighboring cell has a different depth level indicate a contour.
    //
    // Since we can't directly map ring vertices to specific neighbors without
    // the original cell data, we'll use a simpler strategy: for each pair of
    // adjacent ocean cells with different depth levels, generate a contour segment
    // using their shared polygon edge (the edge connecting two consecutive midpoints).

    // Approach: for each ocean polygon, every edge of its ring is shared with
    // exactly one neighbor. If that neighbor has a different depth level, the edge
    // is a contour line. We deduplicate by only emitting from the cell with the
    // lower index.

    // Build a lookup from polygon edge midpoint (quantized) to depth level
    // to detect shared edges. This is an approximation that works well for
    // regular grids.

    // Instead, we'll use a more direct approach: group polygons by depth level
    // and find boundary edges between groups using a vertex hash approach.

    const edgeSet = new Set<string>();

    for (const op of cachedOceanPolygons) {
        const ring = op.polygon;
        // Walk ring edges (ring is closed, so last vertex == first vertex)
        for (let i = 0; i < ring.length - 1; i++) {
            const [x1, y1] = ring[i];
            const [x2, y2] = ring[i + 1];

            // Quantize vertices to detect shared edges
            const key1 = `${x1.toFixed(4)},${y1.toFixed(4)}`;
            const key2 = `${x2.toFixed(4)},${y2.toFixed(4)}`;

            // Canonical edge key (sorted vertex keys for dedup)
            const edgeKey = key1 < key2
                ? `${key1}|${key2}|${op.depthLevel}`
                : `${key2}|${key1}|${op.depthLevel}`;

            // Check if the reverse depth already registered this edge
            // We want edges where two different depth levels share a boundary
            for (let dl = 0; dl < 3; dl++) {
                if (dl === op.depthLevel) continue;
                const reverseKey = key1 < key2
                    ? `${key1}|${key2}|${dl}`
                    : `${key2}|${key1}|${dl}`;

                if (edgeSet.has(reverseKey)) {
                    // This edge is shared between two different depth levels — it's a contour
                    cachedContours.push({
                        path: [[x1, y1], [x2, y2]],
                    });
                    break;
                }
            }

            edgeSet.add(edgeKey);
        }
    }
}

/**
 * Create the ocean depth visualization layers.
 *
 * @param visible - Whether the ocean_depth overlay is currently active.
 * @returns Array of deck.gl layers (empty if not visible or no data).
 */
export function createOceanDepthLayers(visible: boolean): Layer[] {
    if (!visible || !cachedOceanPolygons || cachedOceanPolygons.length === 0) {
        return [];
    }

    const layers: Layer[] = [];

    // 1. Ocean depth fill polygons — overlaid on top of base terrain
    layers.push(new PolygonLayer({
        id: 'ocean-depth-fill',
        data: cachedOceanPolygons,
        getPolygon: (d: OceanDepthPolygon) => d.polygon,
        getFillColor: (d: OceanDepthPolygon) => d.fillColor,
        stroked: false,
        filled: true,
        pickable: false,
        parameters: { depthTest: false },
        updateTriggers: {
            getFillColor: [cachedOceanPolygons.length],
        },
    }));

    // 2. Depth contour lines — thin, semi-transparent lines at depth boundaries
    if (cachedContours && cachedContours.length > 0) {
        layers.push(new PathLayer({
            id: 'ocean-depth-contours',
            data: cachedContours,
            getPath: (d: DepthContourSegment) => d.path,
            getColor: [60, 120, 180, 50],
            getWidth: 1,
            widthUnits: 'pixels',
            widthMinPixels: 0.5,
            widthMaxPixels: 1.5,
            pickable: false,
            parameters: { depthTest: false },
        }));
    }

    return layers;
}

/**
 * Dispose cached ocean depth data to free memory.
 */
export function disposeOceanDepthData(): void {
    cachedOceanPolygons = null;
    cachedContours = null;
}
