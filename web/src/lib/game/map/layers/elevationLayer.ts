// ── Elevation contour overlay layer ──────────────────────────────────────────
// Renders elevation-based color tinting and contour lines for procgen terrain.
// Higher elevation cells get a lighter/whiter tint; lower elevation cells get
// a richer/darker coloring. Contour lines trace boundaries between elevation
// bands (every 20% of max elevation range).
//
// Only renders in Procgen mode at zoom 3+. Toggle-able via 'elevation_contour'
// overlay option.

import { PolygonLayer, PathLayer } from '@deck.gl/layers';
import type { Layer } from '@deck.gl/core';
import { getCachedPolygons, type CellPolygon } from './vectorTerrainLayer';

// ── Types ──────────────────────────────────────────────────────────────────

interface ElevationTintedPolygon {
    polygon: [number, number][];
    fillColor: [number, number, number, number];
    elevation: number;
}

interface ElevationContourLine {
    path: [number, number][];
    elevationBand: number;
}

// ── Pseudo-elevation by terrain type ────────────────────────────────────────
// Mirrors the existing TERRAIN_ELEVATION in vectorTerrainLayer.ts.
// Values in 0..1 range where 0 = deep ocean and 1 = high mountain peak.

const TERRAIN_ELEVATION: Record<string, number> = {
    OceanDeep:    0.0,
    Ocean:        0.05,
    OceanShallow: 0.15,
    Coastal:      0.25,
    Rural:        0.35,
    Suburban:     0.40,
    Urban:        0.40,
    Desert:       0.45,
    Tundra:       0.55,
    Frozen:       0.60,
    Mountainous:  0.85,
};

const OCEAN_TYPES = new Set(['OceanShallow', 'OceanDeep', 'Ocean']);

// Contour band boundaries (every 20% of the 0..1 elevation range)
const CONTOUR_THRESHOLDS = [0.2, 0.4, 0.6, 0.8];

// Contour line colors per band (from low to high elevation)
const CONTOUR_COLORS: [number, number, number, number][] = [
    [60, 90, 140, 70],   // coastal/lowland boundary — subtle blue
    [90, 110, 100, 60],  // lowland/midland boundary — muted green
    [130, 120, 100, 55], // midland/highland boundary — warm tan
    [170, 160, 150, 65], // highland/mountain boundary — light grey
];

// ── Deterministic hash ────────────────────────────────────────────────────

function cellHash(index: number): number {
    let h = index * 2654435761;
    h = ((h >>> 16) ^ h) * 0x45d9f3b;
    h = ((h >>> 16) ^ h) * 0x45d9f3b;
    h = (h >>> 16) ^ h;
    return (h & 0xffff) / 0xffff;
}

// ── Elevation tint calculation ──────────────────────────────────────────────

/**
 * Apply elevation-based color modulation to a cell polygon.
 * Higher elevation = lighter/whiter tint overlaid on existing terrain color.
 * Lower elevation = richer/darker saturation.
 */
function computeElevationTint(
    poly: CellPolygon,
): ElevationTintedPolygon {
    const elevation = TERRAIN_ELEVATION[poly.terrain] ?? 0.4;
    const noise = cellHash(poly.index) * 0.06 - 0.03; // subtle per-cell variation
    const adjustedElev = Math.max(0, Math.min(1, elevation + noise));

    // Start from the polygon's existing fill color
    let [r, g, b] = poly.fillColor;

    if (OCEAN_TYPES.has(poly.terrain)) {
        // Ocean cells: deeper = darker blue-black, shallower = brighter blue
        const depthFactor = adjustedElev; // 0 (deep) → 0.15 (shallow)
        const darken = (0.15 - depthFactor) * 40;
        r = Math.max(0, Math.round(r - darken));
        g = Math.max(0, Math.round(g - darken));
        b = Math.max(0, Math.round(b - darken * 0.5));
    } else {
        // Land cells: elevation-based lightness shift
        // midpoint elevation = 0.4 (no change), higher = lighter, lower = richer
        const elevShift = (adjustedElev - 0.4) * 2.0; // -0.8 to +0.9

        if (elevShift > 0) {
            // Higher elevation: blend toward white/light grey
            const whiten = elevShift * 35;
            r = Math.min(255, Math.round(r + whiten * 0.9));
            g = Math.min(255, Math.round(g + whiten * 0.85));
            b = Math.min(255, Math.round(b + whiten * 0.8));
        } else {
            // Lower elevation: boost saturation (richer color)
            const richness = Math.abs(elevShift) * 20;
            // Boost the dominant channel, reduce others slightly
            const maxCh = Math.max(r, g, b);
            if (maxCh === g) {
                g = Math.min(255, Math.round(g + richness));
                r = Math.max(0, Math.round(r - richness * 0.3));
            } else if (maxCh === b) {
                b = Math.min(255, Math.round(b + richness));
            } else {
                r = Math.min(255, Math.round(r + richness * 0.5));
            }
        }
    }

    return {
        polygon: poly.polygon,
        fillColor: [r, g, b, 180],
        elevation: adjustedElev,
    };
}

// ── Contour line extraction ─────────────────────────────────────────────────

/**
 * Compute centroid of a polygon ring (excluding the closing duplicate vertex).
 */
function centroid(ring: [number, number][]): [number, number] {
    let cx = 0, cy = 0;
    const len = ring[ring.length - 1][0] === ring[0][0] && ring[ring.length - 1][1] === ring[0][1]
        ? ring.length - 1
        : ring.length;
    for (let i = 0; i < len; i++) {
        cx += ring[i][0];
        cy += ring[i][1];
    }
    return [cx / len, cy / len];
}

/**
 * Extract contour lines at elevation band boundaries.
 * A contour segment is drawn between two adjacent cells (sharing a Voronoi edge)
 * when they straddle an elevation threshold — one cell is below and the other
 * is at or above the threshold.
 *
 * The contour segment connects the midpoint between the two cell centers
 * (approximating the Voronoi edge position).
 */
function extractElevationContours(
    polygons: CellPolygon[],
): ElevationContourLine[] {
    if (polygons.length === 0) return [];

    // Build an index: cell_index → polygon for fast lookup
    const indexToPolygon = new Map<number, CellPolygon>();
    for (const poly of polygons) {
        indexToPolygon.set(poly.index, poly);
    }

    // Build centroid cache
    const centroidCache = new Map<number, [number, number]>();
    for (const poly of polygons) {
        centroidCache.set(poly.index, centroid(poly.polygon));
    }

    const contours: ElevationContourLine[] = [];
    const visitedPairs = new Set<string>();

    // For each cell, check neighbors for elevation band crossings
    for (const poly of polygons) {
        const elevA = TERRAIN_ELEVATION[poly.terrain] ?? 0.4;
        const centerA = centroidCache.get(poly.index)!;

        // We need the neighbor indices. The polygon doesn't store them directly,
        // but we can infer neighbors from the polygon ring vertices — they are
        // midpoints between this cell and its neighbors. Instead, we iterate
        // over all polygons and check proximity. For efficiency, we use the
        // polygon ring: each vertex of the Voronoi polygon is a midpoint to
        // a neighbor. Adjacent polygons share a vertex pair.

        // Simpler approach: iterate polygon pairs. Since we have the polygons
        // sorted, we check each pair only once using centroid distance.
        // For performance (up to 1000 polygons), this is acceptable.
    }

    // Alternative approach: for each pair of polygons whose centroids are close
    // enough (within 2x typical cell spacing), check if they cross a threshold.
    // This is O(n^2) but with n <= 800 (detail layer limit) it's fast enough.

    // First pass: find max inter-centroid distance for neighbors
    // Use a simple spatial hash for efficiency
    const gridSize = 5.0; // degrees — coarse spatial hash
    const spatialHash = new Map<string, CellPolygon[]>();

    for (const poly of polygons) {
        const c = centroidCache.get(poly.index)!;
        const gx = Math.floor(c[0] / gridSize);
        const gy = Math.floor(c[1] / gridSize);
        // Insert into this cell and all 8 neighbors for border coverage
        for (let dx = -1; dx <= 1; dx++) {
            for (let dy = -1; dy <= 1; dy++) {
                const key = `${gx + dx},${gy + dy}`;
                if (!spatialHash.has(key)) spatialHash.set(key, []);
                spatialHash.get(key)!.push(poly);
            }
        }
    }

    // Maximum neighbor distance (squared) — roughly 2x the typical cell extent
    // For ~2000 cells globally, cell spacing is ~4-5 degrees
    const maxDistSq = 100; // ~10 degrees max

    for (const poly of polygons) {
        const elevA2 = TERRAIN_ELEVATION[poly.terrain] ?? 0.4;
        const cA = centroidCache.get(poly.index)!;
        const gx = Math.floor(cA[0] / gridSize);
        const gy = Math.floor(cA[1] / gridSize);
        const key = `${gx},${gy}`;
        const nearby = spatialHash.get(key) ?? [];

        for (const other of nearby) {
            if (other.index <= poly.index) continue; // avoid duplicates
            const pairKey = `${poly.index}-${other.index}`;
            if (visitedPairs.has(pairKey)) continue;
            visitedPairs.add(pairKey);

            const cB = centroidCache.get(other.index)!;
            const dx = cA[0] - cB[0];
            const dy = cA[1] - cB[1];
            if (dx * dx + dy * dy > maxDistSq) continue;

            const elevB = TERRAIN_ELEVATION[other.terrain] ?? 0.4;

            // Check each threshold
            for (let ti = 0; ti < CONTOUR_THRESHOLDS.length; ti++) {
                const threshold = CONTOUR_THRESHOLDS[ti];
                // One cell below threshold, other at or above
                if ((elevA2 < threshold && elevB >= threshold) ||
                    (elevB < threshold && elevA2 >= threshold)) {
                    // Contour segment at the midpoint between centroids
                    const mid: [number, number] = [
                        (cA[0] + cB[0]) / 2,
                        (cA[1] + cB[1]) / 2,
                    ];
                    // Create a short perpendicular segment
                    const perpDx = -(cB[1] - cA[1]);
                    const perpDy = cB[0] - cA[0];
                    const perpLen = Math.sqrt(perpDx * perpDx + perpDy * perpDy);
                    if (perpLen < 0.001) continue;
                    const halfLen = perpLen * 0.35; // segment half-length
                    const nx = (perpDx / perpLen) * halfLen;
                    const ny = (perpDy / perpLen) * halfLen;

                    contours.push({
                        path: [
                            [mid[0] - nx, mid[1] - ny],
                            [mid[0] + nx, mid[1] + ny],
                        ],
                        elevationBand: ti,
                    });
                }
            }
        }
    }

    return contours;
}

// ── Cached data ────────────────────────────────────────────────────────────

let cachedTintedPolygons: ElevationTintedPolygon[] | null = null;
let cachedContourLines: ElevationContourLine[] | null = null;

/**
 * Build elevation layer data from cached vector terrain polygons.
 * Call after buildVectorTerrainData().
 */
export function buildElevationData(): void {
    const allPolygons = getCachedPolygons();
    if (!allPolygons || allPolygons.length === 0) {
        cachedTintedPolygons = [];
        cachedContourLines = [];
        return;
    }

    // Compute elevation-tinted polygons
    cachedTintedPolygons = allPolygons.map(computeElevationTint);

    // Extract contour lines (use sampling for performance)
    const maxCells = 1000;
    const step = allPolygons.length > maxCells ? Math.ceil(allPolygons.length / maxCells) : 1;
    const sampled: CellPolygon[] = [];
    for (let i = 0; i < allPolygons.length; i += step) {
        sampled.push(allPolygons[i]);
    }
    cachedContourLines = extractElevationContours(sampled);
}

/** Dispose cached elevation data to free memory. */
export function disposeElevationData(): void {
    cachedTintedPolygons = null;
    cachedContourLines = null;
}

/** Check if elevation data is ready. */
export function hasElevationData(): boolean {
    return cachedTintedPolygons !== null && cachedTintedPolygons.length > 0;
}

// ── Layer creation ─────────────────────────────────────────────────────────

/**
 * Create elevation contour overlay layers for Procgen mode.
 * Only rendered at zoom 3+ when the 'elevation_contour' overlay is active.
 *
 * @param visible - Whether the elevation_contour overlay is active.
 * @param currentZoom - Current map zoom level.
 * @param isRealEarth - Whether in Real Earth mode (skip rendering).
 * @returns Array of deck.gl layers.
 */
export function createElevationContourLayers(
    visible: boolean,
    currentZoom: number,
    isRealEarth: boolean,
): Layer[] {
    if (!visible || isRealEarth) return [];
    if (currentZoom < 3) return [];

    const layers: Layer[] = [];

    // Fade in between zoom 3 and 4
    const fadeIn = Math.min(1.0, currentZoom - 3);

    // 1. Elevation-tinted polygon overlay (semi-transparent over base terrain)
    if (cachedTintedPolygons && cachedTintedPolygons.length > 0) {
        layers.push(new PolygonLayer({
            id: 'elevation-tint-overlay',
            data: cachedTintedPolygons,
            getPolygon: (d: ElevationTintedPolygon) => d.polygon,
            getFillColor: (d: ElevationTintedPolygon) => {
                const c = d.fillColor;
                return [c[0], c[1], c[2], Math.round(c[3] * fadeIn)];
            },
            stroked: false,
            filled: true,
            pickable: false,
            parameters: { depthTest: false },
            updateTriggers: {
                getFillColor: [currentZoom],
            },
        }));
    }

    // 2. Contour lines at elevation band boundaries
    if (cachedContourLines && cachedContourLines.length > 0) {
        layers.push(new PathLayer({
            id: 'elevation-contour-lines',
            data: cachedContourLines,
            getPath: (d: ElevationContourLine) => d.path,
            getColor: (d: ElevationContourLine) => {
                const base = CONTOUR_COLORS[d.elevationBand] ?? CONTOUR_COLORS[0];
                return [base[0], base[1], base[2], Math.round(base[3] * fadeIn)];
            },
            getWidth: 1.2,
            widthUnits: 'pixels',
            widthMinPixels: 0.5,
            widthMaxPixels: 2.0,
            pickable: false,
            parameters: { depthTest: false },
            updateTriggers: {
                getColor: [currentZoom],
            },
        }));
    }

    return layers;
}
