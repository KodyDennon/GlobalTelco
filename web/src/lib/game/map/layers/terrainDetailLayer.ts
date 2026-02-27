// ── Terrain detail pattern layer ─────────────────────────────────────────────
// Adds sub-cell visual patterns to different terrain types for visual richness:
// - Mountain cells: multiple contour ridge lines with jitter + peak highlight
// - Forest/Rural cells: 2-3 inner canopy patches of varying green shades
// - Desert cells: 3 concentric dune bands + diagonal wind-streak lines
// - Ocean cells: depth gradient polygons (lighter near coast, darker deep)
// - Urban cells: dense grid pattern suggesting city blocks (60% fill)
// - Suburban cells: scattered block clusters
// - Coastal cells: shore-side gradient band
// - Tundra cells: crackle fracture pattern
// - Frozen cells: ice fracture lines
//
// Two visibility tiers:
//   - Ocean depth patterns render at all zoom levels (always-on depth feel)
//   - Land detail patterns fade in at zoom 3+ (progressive disclosure)
//
// All effects use low alpha (10-45) to stay subtle. Procgen mode only.

import { PathLayer, PolygonLayer } from '@deck.gl/layers';
import type { Layer } from '@deck.gl/core';
import { getCachedPolygons, type CellPolygon } from './vectorTerrainLayer';

// ── Types ──────────────────────────────────────────────────────────────────

interface ContourLine {
	path: [number, number][];
	color: [number, number, number, number];
	width?: number;
}

interface DetailPolygon {
	polygon: [number, number][];
	fillColor: [number, number, number, number];
}

// ── Terrain classification sets ──────────────────────────────────────────

const OCEAN_TYPES = new Set(['OceanShallow', 'OceanDeep', 'OceanTrench', 'Ocean']);

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

/** Third hash for a third independent noise channel. */
function cellHash3(index: number): number {
	let h = (index + 97) * 22695477;
	h = ((h >>> 16) ^ h) * 0x27d4eb2d;
	h = ((h >>> 16) ^ h) * 0x27d4eb2d;
	h = (h >>> 16) ^ h;
	return (h & 0xffff) / 0xffff;
}

/** Compute the centroid of a polygon ring. */
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

/** Interpolate between two points: (1-t)*a + t*b. */
function lerp2(a: [number, number], b: [number, number], t: number): [number, number] {
	return [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t];
}

/** Get polygon ring length (excluding closing duplicate vertex). */
function ringLength(ring: [number, number][]): number {
	if (ring.length < 2) return ring.length;
	return (ring[ring.length - 1][0] === ring[0][0] && ring[ring.length - 1][1] === ring[0][1])
		? ring.length - 1
		: ring.length;
}

/** Compute polygon bounding box. */
function polyBounds(ring: [number, number][]): { minX: number; maxX: number; minY: number; maxY: number; dx: number; dy: number } {
	let minX = Infinity, maxX = -Infinity, minY = Infinity, maxY = -Infinity;
	const len = ringLength(ring);
	for (let i = 0; i < len; i++) {
		const [x, y] = ring[i];
		if (x < minX) minX = x;
		if (x > maxX) maxX = x;
		if (y < minY) minY = y;
		if (y > maxY) maxY = y;
	}
	return { minX, maxX, minY, maxY, dx: maxX - minX, dy: maxY - minY };
}

/**
 * Create a scaled copy of the polygon ring around a center point.
 * @param ring - Original polygon ring.
 * @param center - Center point to scale toward.
 * @param t - Scale factor (0 = center, 1 = original).
 * @param jitter - Optional per-vertex jitter magnitude (0 = none).
 * @param seed - Hash seed for jitter.
 */
function scaledRing(
	ring: [number, number][],
	center: [number, number],
	t: number,
	jitter: number = 0,
	seed: number = 0,
): [number, number][] {
	const len = ringLength(ring);
	const result: [number, number][] = [];
	for (let i = 0; i < len; i++) {
		let pt = lerp2(center, ring[i], t);
		if (jitter > 0) {
			// Deterministic per-vertex jitter
			const vh = cellHash(seed * 1000 + i * 7);
			const vh2 = cellHash2(seed * 1000 + i * 7);
			pt = [
				pt[0] + (vh - 0.5) * jitter,
				pt[1] + (vh2 - 0.5) * jitter,
			];
		}
		result.push(pt);
	}
	result.push(result[0]); // close ring
	return result;
}

// ── Detail generators ──────────────────────────────────────────────────────

/**
 * Mountain cells: 4 concentric contour rings with jitter for ridge-like
 * appearance, plus an inner peak highlight polygon. Darker lines suggest
 * elevation contours; the peak polygon is a lighter tint.
 */
function generateMountainDetail(poly: CellPolygon, contours: ContourLine[], polygons: DetailPolygon[]): void {
	const center = centroid(poly.polygon);
	const noise = cellHash(poly.index);
	const noise2 = cellHash2(poly.index);
	const bounds = polyBounds(poly.polygon);
	const jitterMag = Math.min(bounds.dx, bounds.dy) * 0.06;

	// 4 contour rings at 30%, 50%, 70%, 88% from center
	const scales = [0.30, 0.50, 0.70, 0.88];
	const alphas = [35, 30, 25, 20];
	// Alternate slightly lighter and darker contour colors for variety
	const colors: [number, number, number][] = [
		[85, 72, 58],   // darker brown (inner peak)
		[100, 88, 70],  // medium brown
		[90, 78, 62],   // darker
		[110, 96, 78],  // lighter (outer)
	];

	for (let r = 0; r < scales.length; r++) {
		const scale = scales[r] + noise * 0.05;
		const ring = scaledRing(poly.polygon, center, scale, jitterMag * (1 - r * 0.2), poly.index + r);
		if (ring.length >= 4) {
			const alpha = Math.round(alphas[r] + noise2 * 10);
			contours.push({
				path: ring,
				color: [colors[r][0], colors[r][1], colors[r][2], alpha],
			});
		}
	}

	// Peak highlight: small lighter polygon at 20% from center
	const peakScale = 0.18 + noise * 0.08;
	const peak = scaledRing(poly.polygon, center, peakScale, jitterMag * 0.3, poly.index + 99);
	if (peak.length >= 4) {
		polygons.push({
			polygon: peak,
			fillColor: [160, 150, 140, Math.round(18 + noise * 12)],
		});
	}
}

/**
 * Desert cells: 3 concentric dune bands with alternating lighter/darker
 * fills, plus 2-3 diagonal wind-streak lines across the cell.
 */
function generateDesertDetail(poly: CellPolygon, contours: ContourLine[], polygons: DetailPolygon[]): void {
	const center = centroid(poly.polygon);
	const noise = cellHash(poly.index);
	const noise2 = cellHash2(poly.index);
	const noise3 = cellHash3(poly.index);

	// 3 concentric dune bands
	const bandScales = [0.85, 0.55, 0.30];
	const bandLightness = [
		noise2 > 0.5 ? 12 : -8,
		noise2 > 0.5 ? -10 : 14,
		noise2 > 0.5 ? 8 : -6,
	];

	for (let b = 0; b < bandScales.length; b++) {
		const scale = bandScales[b] + noise * 0.06;
		const ring = scaledRing(poly.polygon, center, scale, 0, poly.index + b);
		if (ring.length >= 4) {
			const ltn = bandLightness[b];
			polygons.push({
				polygon: ring,
				fillColor: [
					Math.max(0, Math.min(255, 155 + ltn)),
					Math.max(0, Math.min(255, 130 + ltn)),
					Math.max(0, Math.min(255, 85 + Math.round(ltn * 0.5))),
					Math.round(15 + noise * 12),
				],
			});
		}
	}

	// Wind-streak diagonal lines (2-3 lines)
	const bounds = polyBounds(poly.polygon);
	if (bounds.dx < 0.001 || bounds.dy < 0.001) return;

	const windAngle = noise3 * Math.PI; // random wind direction per cell
	const cos = Math.cos(windAngle);
	const sin = Math.sin(windAngle);
	const extent = Math.max(bounds.dx, bounds.dy) * 0.35;
	const lineCount = 2 + Math.floor(noise * 1.5); // 2 or 3

	for (let i = 0; i < lineCount; i++) {
		const offset = (i / (lineCount - 1) - 0.5) * extent * 0.6;
		const p1: [number, number] = [
			center[0] + (-extent) * cos - offset * sin,
			center[1] + (-extent) * sin + offset * cos,
		];
		const p2: [number, number] = [
			center[0] + extent * cos - offset * sin,
			center[1] + extent * sin + offset * cos,
		];
		contours.push({
			path: [p1, p2],
			color: [175, 150, 100, Math.round(12 + noise2 * 10)],
		});
	}
}

/**
 * Forest/Rural cells: 2-3 inner canopy patches of varying green shades
 * with offset centers for an organic tree-canopy feel.
 */
function generateForestDetail(poly: CellPolygon, polygons: DetailPolygon[]): void {
	const center = centroid(poly.polygon);
	const noise = cellHash(poly.index);
	const noise2 = cellHash2(poly.index);
	const noise3 = cellHash3(poly.index);
	const bounds = polyBounds(poly.polygon);

	const patchCount = 2 + Math.floor(noise3 * 1.5); // 2 or 3

	for (let p = 0; p < patchCount; p++) {
		// Each patch has a randomly offset center
		const pNoise = cellHash(poly.index * 10 + p);
		const pNoise2 = cellHash2(poly.index * 10 + p);

		const offsetCenter: [number, number] = [
			center[0] + (pNoise - 0.5) * bounds.dx * 0.25,
			center[1] + (pNoise2 - 0.5) * bounds.dy * 0.25,
		];

		const scale = 0.35 + pNoise * 0.2; // 35% to 55%
		const len = ringLength(poly.polygon);
		const inner: [number, number][] = [];
		for (let i = 0; i < len; i++) {
			inner.push(lerp2(offsetCenter, poly.polygon[i], scale));
		}
		if (inner.length >= 3) {
			inner.push(inner[0]);

			// Vary between darker forest green and lighter lime green
			const shift = (p % 2 === 0)
				? (noise2 > 0.5 ? 20 : -15)
				: (noise2 > 0.5 ? -12 : 16);

			polygons.push({
				polygon: inner,
				fillColor: [
					Math.max(0, Math.min(255, 50 + Math.round(shift * 0.4))),
					Math.max(0, Math.min(255, 100 + shift)),
					Math.max(0, Math.min(255, 52 + Math.round(shift * 0.3))),
					Math.round(18 + pNoise * 15),
				],
			});
		}
	}
}

/**
 * Urban cells: dense grid pattern (60% of cell extent) with 4-5 lines
 * in each direction, suggesting city blocks. Slightly rotated per cell.
 */
function generateUrbanDetail(poly: CellPolygon, contours: ContourLine[]): void {
	const center = centroid(poly.polygon);
	const noise = cellHash(poly.index);
	const bounds = polyBounds(poly.polygon);
	if (bounds.dx < 0.001 || bounds.dy < 0.001) return;

	// Grid covers 60% of cell extent
	const gridScale = 0.6;
	const gridDx = bounds.dx * gridScale;
	const gridDy = bounds.dy * gridScale;

	// Rotation angle based on cell hash
	const angle = noise * Math.PI * 0.4;
	const cos = Math.cos(angle);
	const sin = Math.sin(angle);

	const gridColor: [number, number, number, number] = [
		145, 140, 155, Math.round(25 + noise * 18),
	];

	// 4-5 lines each direction
	const lineCount = 4 + Math.floor(noise * 2);
	for (let i = 0; i < lineCount; i++) {
		const t = (i / (lineCount - 1)) - 0.5; // -0.5 to 0.5

		// Horizontal line (rotated)
		const hx1 = -gridDx * 0.5;
		const hy = t * gridDy;
		const hx2 = gridDx * 0.5;

		contours.push({
			path: [
				[center[0] + hx1 * cos - hy * sin, center[1] + hx1 * sin + hy * cos],
				[center[0] + hx2 * cos - hy * sin, center[1] + hx2 * sin + hy * cos],
			],
			color: gridColor,
		});

		// Vertical line (rotated)
		const vx = t * gridDx;
		const vy1 = -gridDy * 0.5;
		const vy2 = gridDy * 0.5;

		contours.push({
			path: [
				[center[0] + vx * cos - vy1 * sin, center[1] + vx * sin + vy1 * cos],
				[center[0] + vx * cos - vy2 * sin, center[1] + vx * sin + vy2 * cos],
			],
			color: gridColor,
		});
	}
}

/**
 * Suburban cells: a few scattered rectangular block clusters suggesting
 * residential neighborhoods. Less dense than urban grid.
 */
function generateSuburbanDetail(poly: CellPolygon, contours: ContourLine[]): void {
	const center = centroid(poly.polygon);
	const noise = cellHash(poly.index);
	const noise2 = cellHash2(poly.index);
	const bounds = polyBounds(poly.polygon);
	if (bounds.dx < 0.001 || bounds.dy < 0.001) return;

	// 2-3 small block clusters scattered around the cell
	const clusterCount = 2 + Math.floor(noise * 1.5);
	const clusterColor: [number, number, number, number] = [
		115, 115, 120, Math.round(18 + noise2 * 12),
	];

	for (let c = 0; c < clusterCount; c++) {
		const cn = cellHash(poly.index * 10 + c);
		const cn2 = cellHash2(poly.index * 10 + c);

		// Cluster center offset from cell center
		const cx = center[0] + (cn - 0.5) * bounds.dx * 0.4;
		const cy = center[1] + (cn2 - 0.5) * bounds.dy * 0.4;

		// Small 2x2 grid (4 short lines)
		const blockSize = Math.min(bounds.dx, bounds.dy) * 0.12;
		const angle = cn * Math.PI * 0.3;
		const cos = Math.cos(angle);
		const sin = Math.sin(angle);

		// 2 horizontal + 2 vertical
		for (let i = 0; i < 2; i++) {
			const t = (i - 0.5) * blockSize;
			contours.push({
				path: [
					[cx + (-blockSize * 0.5) * cos - t * sin, cy + (-blockSize * 0.5) * sin + t * cos],
					[cx + (blockSize * 0.5) * cos - t * sin, cy + (blockSize * 0.5) * sin + t * cos],
				],
				color: clusterColor,
			});
			contours.push({
				path: [
					[cx + t * cos - (-blockSize * 0.5) * sin, cy + t * sin + (-blockSize * 0.5) * cos],
					[cx + t * cos - (blockSize * 0.5) * sin, cy + t * sin + (blockSize * 0.5) * cos],
				],
				color: clusterColor,
			});
		}
	}
}

/**
 * Ocean cells: depth gradient inner polygon. Shallow ocean gets a slightly
 * lighter inner patch; deep ocean gets a darker inner patch. This creates
 * a per-cell depth feel visible at all zoom levels.
 */
function generateOceanDepthDetail(poly: CellPolygon, polygons: DetailPolygon[]): void {
	const center = centroid(poly.polygon);
	const noise = cellHash(poly.index);

	const isShallow = poly.terrain === 'OceanShallow';
	const isDeep = poly.terrain === 'OceanDeep' || poly.terrain === 'OceanTrench';

	// Inner gradient polygon at 50-65% from center
	const scale = 0.52 + noise * 0.13;
	const ring = scaledRing(poly.polygon, center, scale, 0, poly.index);
	if (ring.length < 4) return;

	if (isShallow) {
		// Lighter inner patch for shallow cells (coastal proximity feel)
		polygons.push({
			polygon: ring,
			fillColor: [22, 52, 96, Math.round(18 + noise * 14)],
		});
	} else if (isDeep) {
		// Darker inner patch for deep ocean
		polygons.push({
			polygon: ring,
			fillColor: [2, 8, 22, Math.round(20 + noise * 16)],
		});
	} else {
		// Medium ocean: subtle directional variation
		const shift = noise > 0.5 ? 6 : -6;
		polygons.push({
			polygon: ring,
			fillColor: [
				Math.max(0, 8 + shift),
				Math.max(0, 18 + shift),
				Math.max(0, 42 + shift),
				Math.round(14 + noise * 10),
			],
		});
	}
}

/**
 * Coastal cells: a shore-side gradient band along the edge closest to
 * the cell center, creating a land-sea transition feel.
 */
function generateCoastalDetail(poly: CellPolygon, polygons: DetailPolygon[]): void {
	const center = centroid(poly.polygon);
	const noise = cellHash(poly.index);

	// A large inner polygon with a blue-green tint suggesting water proximity
	const scale = 0.70 + noise * 0.10;
	const ring = scaledRing(poly.polygon, center, scale, 0, poly.index);
	if (ring.length < 4) return;

	polygons.push({
		polygon: ring,
		fillColor: [55, 105, 125, Math.round(14 + noise * 10)],
	});

	// Smaller inner ring with stronger sea tint
	const innerScale = 0.35 + noise * 0.10;
	const innerRing = scaledRing(poly.polygon, center, innerScale, 0, poly.index + 50);
	if (innerRing.length < 4) return;

	polygons.push({
		polygon: innerRing,
		fillColor: [45, 95, 120, Math.round(12 + noise * 8)],
	});
}

/**
 * Tundra cells: subtle crackle fracture lines suggesting frozen ground
 * with permafrost patterns.
 */
function generateTundraDetail(poly: CellPolygon, contours: ContourLine[]): void {
	const center = centroid(poly.polygon);
	const noise = cellHash(poly.index);
	const noise2 = cellHash2(poly.index);
	const bounds = polyBounds(poly.polygon);
	if (bounds.dx < 0.001 || bounds.dy < 0.001) return;

	// 3-4 crack lines radiating from near-center
	const crackCount = 3 + Math.floor(noise * 2);
	const crackColor: [number, number, number, number] = [
		95, 108, 122, Math.round(18 + noise2 * 12),
	];

	for (let i = 0; i < crackCount; i++) {
		const cn = cellHash(poly.index * 10 + i);
		const cn2 = cellHash2(poly.index * 10 + i);

		// Start point near center
		const sx = center[0] + (cn - 0.5) * bounds.dx * 0.15;
		const sy = center[1] + (cn2 - 0.5) * bounds.dy * 0.15;

		// End point toward polygon edge
		const angle = (i / crackCount) * Math.PI * 2 + noise * 0.5;
		const reach = Math.min(bounds.dx, bounds.dy) * (0.25 + cn * 0.15);
		const ex = sx + Math.cos(angle) * reach;
		const ey = sy + Math.sin(angle) * reach;

		// Add a mid-point bend for more natural cracking
		const mx = (sx + ex) / 2 + (cn2 - 0.5) * bounds.dx * 0.08;
		const my = (sy + ey) / 2 + (cn - 0.5) * bounds.dy * 0.08;

		contours.push({
			path: [[sx, sy], [mx, my], [ex, ey]],
			color: crackColor,
		});
	}
}

/**
 * Frozen cells: ice fracture lines — longer, straighter cracks with a
 * white-blue tint suggesting glacier or ice sheet fractures.
 */
function generateFrozenDetail(poly: CellPolygon, contours: ContourLine[]): void {
	const center = centroid(poly.polygon);
	const noise = cellHash(poly.index);
	const noise2 = cellHash2(poly.index);
	const bounds = polyBounds(poly.polygon);
	if (bounds.dx < 0.001 || bounds.dy < 0.001) return;

	// 2-3 longer fracture lines
	const crackCount = 2 + Math.floor(noise * 1.5);
	const crackColor: [number, number, number, number] = [
		170, 180, 200, Math.round(16 + noise2 * 10),
	];

	for (let i = 0; i < crackCount; i++) {
		const cn = cellHash(poly.index * 10 + i);
		const cn2 = cellHash2(poly.index * 10 + i);

		const angle = (i / crackCount) * Math.PI + noise * 0.8;
		const reach = Math.min(bounds.dx, bounds.dy) * (0.30 + cn * 0.15);

		const p1: [number, number] = [
			center[0] - Math.cos(angle) * reach * 0.4,
			center[1] - Math.sin(angle) * reach * 0.4,
		];
		const p2: [number, number] = [
			center[0] + Math.cos(angle) * reach * 0.6,
			center[1] + Math.sin(angle) * reach * 0.6,
		];

		// Slight bend
		const mx = (p1[0] + p2[0]) / 2 + (cn2 - 0.5) * bounds.dx * 0.04;
		const my = (p1[1] + p2[1]) / 2 + (cn - 0.5) * bounds.dy * 0.04;

		contours.push({
			path: [p1, [mx, my], p2],
			color: crackColor,
		});
	}
}

// ── Cached data ────────────────────────────────────────────────────────────

let cachedContourLines: ContourLine[] | null = null;
let cachedDetailPolygons: DetailPolygon[] | null = null;
let cachedOceanDetailPolygons: DetailPolygon[] | null = null;

/**
 * Build terrain detail data from the cached vector terrain polygons.
 * Call after buildVectorTerrainData().
 *
 * Processes ALL cells (no sampling) since per-cell detail generation is
 * lightweight (a few array pushes per cell). The total geometry count stays
 * manageable because each cell produces only 1-5 primitives.
 */
export function buildTerrainDetailData(): void {
	const allPolygons = getCachedPolygons();
	if (!allPolygons || allPolygons.length === 0) {
		cachedContourLines = [];
		cachedDetailPolygons = [];
		cachedOceanDetailPolygons = [];
		return;
	}

	const contourLines: ContourLine[] = [];
	const detailPolygons: DetailPolygon[] = [];
	const oceanDetailPolygons: DetailPolygon[] = [];

	for (let i = 0; i < allPolygons.length; i++) {
		const poly = allPolygons[i];
		const terrain = poly.terrain;

		if (terrain === 'Mountainous') {
			generateMountainDetail(poly, contourLines, detailPolygons);
		} else if (terrain === 'Desert') {
			generateDesertDetail(poly, contourLines, detailPolygons);
		} else if (terrain === 'Rural') {
			generateForestDetail(poly, detailPolygons);
		} else if (terrain === 'Urban') {
			generateUrbanDetail(poly, contourLines);
		} else if (terrain === 'Suburban') {
			generateSuburbanDetail(poly, contourLines);
		} else if (terrain === 'Coastal') {
			generateCoastalDetail(poly, detailPolygons);
		} else if (terrain === 'Tundra') {
			generateTundraDetail(poly, contourLines);
		} else if (terrain === 'Frozen') {
			generateFrozenDetail(poly, contourLines);
		} else if (OCEAN_TYPES.has(terrain)) {
			generateOceanDepthDetail(poly, oceanDetailPolygons);
		}
	}

	cachedContourLines = contourLines;
	cachedDetailPolygons = detailPolygons;
	cachedOceanDetailPolygons = oceanDetailPolygons;
}

/** Dispose cached terrain detail data. */
export function disposeTerrainDetailData(): void {
	cachedContourLines = null;
	cachedDetailPolygons = null;
	cachedOceanDetailPolygons = null;
}

// ── Layer creation ─────────────────────────────────────────────────────────

/**
 * Create terrain detail pattern layers.
 * - Ocean depth detail: visible at all zoom levels (always-on)
 * - Land detail patterns: fade in at zoom 3+ (progressive disclosure)
 * Only rendered in Procgen mode.
 *
 * @param currentZoom - Current map zoom level.
 * @param isRealEarth - Whether in Real Earth mode (skip rendering).
 * @returns Array of deck.gl layers.
 */
export function createTerrainDetailLayers(currentZoom: number, isRealEarth: boolean): Layer[] {
	if (isRealEarth) return [];

	const layers: Layer[] = [];

	// Ocean depth detail polygons — always visible (subtle depth gradient)
	if (cachedOceanDetailPolygons && cachedOceanDetailPolygons.length > 0) {
		layers.push(new PolygonLayer({
			id: 'terrain-detail-ocean-depth',
			data: cachedOceanDetailPolygons,
			getPolygon: (d: DetailPolygon) => d.polygon,
			getFillColor: (d: DetailPolygon) => d.fillColor,
			stroked: false,
			filled: true,
			pickable: false,
			parameters: { depthTest: false },
			updateTriggers: {
				getFillColor: [cachedOceanDetailPolygons.length],
			},
		}));
	}

	// Land detail layers: fade in at zoom 3+
	if (currentZoom < 3) return layers;

	// Fade in between zoom 3 and 4
	const fadeIn = Math.min(1.0, currentZoom - 3);

	// Land detail polygons (desert bands, forest variation, coastal gradient)
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

	// Contour lines (mountain ridges, urban grid, suburban blocks, tundra/frozen cracks, desert wind-streaks)
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
