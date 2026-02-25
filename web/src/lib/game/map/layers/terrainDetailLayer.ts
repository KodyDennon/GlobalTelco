// ── Terrain detail pattern layer ─────────────────────────────────────────────
// Adds subtle visual detail within large Voronoi terrain cells at zoom 4+:
// - Mountain cells: thin contour lines in a slightly lighter shade
// - Desert cells: subtle bands of lighter/darker color
// - Forest/Rural cells: enhanced green variation noise
// - Urban cells: subtle grey grid overlay pattern
//
// These are extremely subtle effects (opacity 0.1-0.3) that add depth without
// overpowering the base terrain. Procgen mode only.

import { PathLayer, PolygonLayer } from '@deck.gl/layers';
import type { Layer } from '@deck.gl/core';
import { getCachedPolygons, type CellPolygon } from './vectorTerrainLayer';

// ── Types ──────────────────────────────────────────────────────────────────

interface ContourLine {
    path: [number, number][];
    color: [number, number, number, number];
}

interface DetailPolygon {
    polygon: [number, number][];
    fillColor: [number, number, number, number];
}

// ── Constants ──────────────────────────────────────────────────────────────

const MOUNTAIN_TERRAINS = new Set(['Mountainous']);
const DESERT_TERRAINS = new Set(['Desert']);
const FOREST_TERRAINS = new Set(['Rural']);
const URBAN_TERRAINS = new Set(['Urban']);

// ── Utilities ──────────────────────────────────────────────────────────────

/** Deterministic hash for cell-based randomness. */
function cellHash(index: number): number {
    let h = index * 2654435761;
    h = ((h >>> 16) ^ h) * 0x45d9f3b;
    h = ((h >>> 16) ^ h) * 0x45d9f3b;
    h = (h >>> 16) ^ h;
    return (h & 0xffff) / 0xffff;
}

/** Second hash for independent noise channel. */
function cellHash2(index: number): number {
    let h = (index + 37) * 1664525;
    h = ((h >>> 16) ^ h) * 0x119de1f3;
    h = ((h >>> 16) ^ h) * 0x119de1f3;
    h = (h >>> 16) ^ h;
    return (h & 0xffff) / 0xffff;
}

/**
 * Compute the centroid of a polygon ring.
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
 * Interpolate between two points: (1-t)*a + t*b.
 */
function lerp2(a: [number, number], b: [number, number], t: number): [number, number] {
    return [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t];
}

// ── Detail generators ──────────────────────────────────────────────────────

/**
 * Mountain contour lines: shrink the polygon ring toward the center
 * at 40% and 70% to create two contour rings.
 */
function generateMountainContours(poly: CellPolygon): ContourLine[] {
    const lines: ContourLine[] = [];
    const center = centroid(poly.polygon);
    const noise = cellHash(poly.index);

    // Inner contour at 40% from center
    const ring40: [number, number][] = [];
    const ringLen = poly.polygon[poly.polygon.length - 1][0] === poly.polygon[0][0]
        ? poly.polygon.length - 1
        : poly.polygon.length;

    for (let i = 0; i < ringLen; i++) {
        ring40.push(lerp2(center, poly.polygon[i], 0.4 + noise * 0.1));
    }
    if (ring40.length >= 3) {
        ring40.push(ring40[0]); // close
        // Slightly lighter than base mountain color
        const alpha = Math.round(25 + noise * 20);
        lines.push({
            path: ring40,
            color: [155, 140, 120, alpha],
        });
    }

    // Outer contour at 70% from center
    const ring70: [number, number][] = [];
    for (let i = 0; i < ringLen; i++) {
        ring70.push(lerp2(center, poly.polygon[i], 0.7 + noise * 0.08));
    }
    if (ring70.length >= 3) {
        ring70.push(ring70[0]);
        const alpha = Math.round(20 + noise * 15);
        lines.push({
            path: ring70,
            color: [145, 130, 110, alpha],
        });
    }

    return lines;
}

/**
 * Desert bands: divide the polygon into two halves with a slightly
 * different shade, creating a subtle banding effect.
 */
function generateDesertBands(poly: CellPolygon): DetailPolygon[] {
    const bands: DetailPolygon[] = [];
    const center = centroid(poly.polygon);
    const noise = cellHash(poly.index);
    const noise2 = cellHash2(poly.index);

    // Create an inner polygon at 55% from center — lighter band
    const ringLen = poly.polygon[poly.polygon.length - 1][0] === poly.polygon[0][0]
        ? poly.polygon.length - 1
        : poly.polygon.length;

    const inner: [number, number][] = [];
    for (let i = 0; i < ringLen; i++) {
        inner.push(lerp2(center, poly.polygon[i], 0.55 + noise * 0.1));
    }
    if (inner.length >= 3) {
        inner.push(inner[0]);
        // Slightly lighter desert shade
        const lightness = noise2 > 0.5 ? 15 : -10;
        bands.push({
            polygon: inner,
            fillColor: [
                Math.min(255, 155 + lightness),
                Math.min(255, 130 + lightness),
                Math.min(255, 85 + Math.round(lightness * 0.5)),
                Math.round(20 + noise * 15),
            ],
        });
    }

    return bands;
}

/**
 * Forest/Rural green variation: a smaller inner polygon with a
 * different shade of green to add variation within the cell.
 */
function generateForestVariation(poly: CellPolygon): DetailPolygon[] {
    const details: DetailPolygon[] = [];
    const center = centroid(poly.polygon);
    const noise = cellHash(poly.index);
    const noise2 = cellHash2(poly.index);

    const ringLen = poly.polygon[poly.polygon.length - 1][0] === poly.polygon[0][0]
        ? poly.polygon.length - 1
        : poly.polygon.length;

    // Offset center slightly for asymmetry
    const offsetCenter: [number, number] = [
        center[0] + (noise - 0.5) * 0.3 * (poly.polygon[1]?.[0] ?? center[0]) * 0.01,
        center[1] + (noise2 - 0.5) * 0.3 * (poly.polygon[1]?.[1] ?? center[1]) * 0.01,
    ];

    const inner: [number, number][] = [];
    for (let i = 0; i < ringLen; i++) {
        const t = 0.45 + noise * 0.15;
        inner.push(lerp2(offsetCenter, poly.polygon[i], t));
    }
    if (inner.length >= 3) {
        inner.push(inner[0]);
        // Variation: darker or lighter green
        const shift = noise2 > 0.5 ? 18 : -12;
        details.push({
            polygon: inner,
            fillColor: [
                Math.max(0, 55 + Math.round(shift * 0.3)),
                Math.max(0, Math.min(255, 105 + shift)),
                Math.max(0, 58 + Math.round(shift * 0.2)),
                Math.round(25 + noise * 20),
            ],
        });
    }

    return details;
}

/**
 * Urban grid overlay: create a few short line segments in a grid-like
 * pattern within the cell to suggest city blocks.
 */
function generateUrbanGrid(poly: CellPolygon): ContourLine[] {
    const lines: ContourLine[] = [];
    const center = centroid(poly.polygon);
    const noise = cellHash(poly.index);

    // Find approximate polygon extent
    let minX = Infinity, maxX = -Infinity, minY = Infinity, maxY = -Infinity;
    const ringLen = poly.polygon[poly.polygon.length - 1][0] === poly.polygon[0][0]
        ? poly.polygon.length - 1
        : poly.polygon.length;

    for (let i = 0; i < ringLen; i++) {
        const [x, y] = poly.polygon[i];
        if (x < minX) minX = x;
        if (x > maxX) maxX = x;
        if (y < minY) minY = y;
        if (y > maxY) maxY = y;
    }

    const dx = (maxX - minX);
    const dy = (maxY - minY);
    if (dx < 0.001 || dy < 0.001) return lines;

    // Scale grid to ~30% of cell extent from center
    const gridScale = 0.3;
    const gridDx = dx * gridScale;
    const gridDy = dy * gridScale;

    // Rotation angle based on cell hash
    const angle = noise * Math.PI * 0.5;
    const cos = Math.cos(angle);
    const sin = Math.sin(angle);

    const gridColor: [number, number, number, number] = [140, 140, 150, Math.round(20 + noise * 20)];

    // Generate 3-4 horizontal-ish and 3-4 vertical-ish lines
    const lineCount = 3 + Math.floor(noise * 2);
    for (let i = 0; i < lineCount; i++) {
        const t = (i / (lineCount - 1)) - 0.5; // -0.5 to 0.5

        // Horizontal line (rotated)
        const hx1 = -gridDx * 0.5;
        const hy = t * gridDy;
        const hx2 = gridDx * 0.5;

        const p1: [number, number] = [
            center[0] + hx1 * cos - hy * sin,
            center[1] + hx1 * sin + hy * cos,
        ];
        const p2: [number, number] = [
            center[0] + hx2 * cos - hy * sin,
            center[1] + hx2 * sin + hy * cos,
        ];
        lines.push({ path: [p1, p2], color: gridColor });

        // Vertical line (rotated)
        const vx = t * gridDx;
        const vy1 = -gridDy * 0.5;
        const vy2 = gridDy * 0.5;

        const p3: [number, number] = [
            center[0] + vx * cos - vy1 * sin,
            center[1] + vx * sin + vy1 * cos,
        ];
        const p4: [number, number] = [
            center[0] + vx * cos - vy2 * sin,
            center[1] + vx * sin + vy2 * cos,
        ];
        lines.push({ path: [p3, p4], color: gridColor });
    }

    return lines;
}

// ── Cached data ────────────────────────────────────────────────────────────

let cachedContourLines: ContourLine[] | null = null;
let cachedDetailPolygons: DetailPolygon[] | null = null;

/**
 * Build terrain detail data from the cached vector terrain polygons.
 * Call after buildVectorTerrainData().
 */
export function buildTerrainDetailData(): void {
    const allPolygons = getCachedPolygons();
    if (!allPolygons || allPolygons.length === 0) {
        cachedContourLines = [];
        cachedDetailPolygons = [];
        return;
    }

    const contourLines: ContourLine[] = [];
    const detailPolygons: DetailPolygon[] = [];

    // Limit processing for performance — sample cells if too many
    const maxCells = 800;
    const step = allPolygons.length > maxCells ? Math.ceil(allPolygons.length / maxCells) : 1;

    for (let i = 0; i < allPolygons.length; i += step) {
        const poly = allPolygons[i];

        if (MOUNTAIN_TERRAINS.has(poly.terrain)) {
            contourLines.push(...generateMountainContours(poly));
        } else if (DESERT_TERRAINS.has(poly.terrain)) {
            detailPolygons.push(...generateDesertBands(poly));
        } else if (FOREST_TERRAINS.has(poly.terrain)) {
            detailPolygons.push(...generateForestVariation(poly));
        } else if (URBAN_TERRAINS.has(poly.terrain)) {
            contourLines.push(...generateUrbanGrid(poly));
        }
    }

    cachedContourLines = contourLines;
    cachedDetailPolygons = detailPolygons;
}

/** Dispose cached terrain detail data. */
export function disposeTerrainDetailData(): void {
    cachedContourLines = null;
    cachedDetailPolygons = null;
}

// ── Layer creation ─────────────────────────────────────────────────────────

/**
 * Create terrain detail pattern layers.
 * Only rendered in Procgen mode at zoom 4+.
 *
 * @param currentZoom - Current map zoom level.
 * @param isRealEarth - Whether in Real Earth mode (skip rendering).
 * @returns Array of deck.gl layers.
 */
export function createTerrainDetailLayers(currentZoom: number, isRealEarth: boolean): Layer[] {
    if (isRealEarth) return [];
    if (currentZoom < 4) return [];

    const layers: Layer[] = [];

    // Fade in between zoom 4 and 5
    const fadeIn = Math.min(1.0, (currentZoom - 4));

    // 1. Detail polygons (desert bands, forest variation)
    if (cachedDetailPolygons && cachedDetailPolygons.length > 0) {
        layers.push(new PolygonLayer({
            id: 'terrain-detail-polygons',
            data: cachedDetailPolygons,
            getPolygon: (d: DetailPolygon) => d.polygon,
            getFillColor: (d: DetailPolygon) => {
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

    // 2. Contour lines (mountain contours, urban grid)
    if (cachedContourLines && cachedContourLines.length > 0) {
        layers.push(new PathLayer({
            id: 'terrain-detail-contours',
            data: cachedContourLines,
            getPath: (d: ContourLine) => d.path,
            getColor: (d: ContourLine) => {
                const c = d.color;
                return [c[0], c[1], c[2], Math.round(c[3] * fadeIn)];
            },
            getWidth: 0.8,
            widthUnits: 'pixels',
            widthMinPixels: 0.5,
            widthMaxPixels: 1.5,
            pickable: false,
            parameters: { depthTest: false },
            updateTriggers: {
                getColor: [currentZoom],
            },
        }));
    }

    return layers;
}
