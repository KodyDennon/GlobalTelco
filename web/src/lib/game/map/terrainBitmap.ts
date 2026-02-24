// ── Terrain bitmap builder ──────────────────────────────────────────────────
// Pre-renders all grid cells onto a high-resolution equirectangular Canvas
// using Gaussian-splat interpolation with edge-aware blur that preserves
// sharp land/ocean boundaries. Produces smooth, satellite-style terrain.

import type { GridCell } from '$lib/wasm/types';

// ── Internal constants ──────────────────────────────────────────────────────

/** Pixels per degree of longitude/latitude. 8 = 2880x1360 canvas. */
const PIXELS_PER_DEG = 8;

/** Canvas dimensions covering -180..180 lon, -85..85 lat. */
const W = 360 * PIXELS_PER_DEG; // 2880
const H = 170 * PIXELS_PER_DEG; // 1360

/** Latitude bounds (we clip polar regions). */
const LAT_MIN = -85;
const LAT_MAX = 85;

/** Ocean terrain types — used for edge-aware blur boundary detection. */
const OCEAN_TYPES = new Set(['OceanShallow', 'OceanDeep', 'Ocean']);

/**
 * Render grid cells onto a high-resolution equirectangular canvas using
 * Gaussian-splat interpolation with edge-aware blur for clean coastlines.
 *
 * @param cells - Array of grid cells with lat, lon, terrain fields
 * @param cellRadiusKm - Approximate radius of each cell in kilometres
 * @param colorPalette - Terrain type to RGB color mapping
 * @returns A canvas element suitable as a BitmapLayer image
 */
export function buildTerrainBitmap(
    cells: GridCell[],
    cellRadiusKm: number,
    colorPalette: Record<string, [number, number, number]>
): HTMLCanvasElement {
    // Accumulation buffers: weighted R/G/B and total weight per pixel
    const totalPixels = W * H;
    const weightR = new Float32Array(totalPixels);
    const weightG = new Float32Array(totalPixels);
    const weightB = new Float32Array(totalPixels);
    const weightSum = new Float32Array(totalPixels);

    // Track whether each pixel is "ocean" or "land" for edge-aware blur.
    // 1.0 = land weight, -1.0 = ocean weight, blended by Gaussian.
    const landWeight = new Float32Array(totalPixels);

    // Ocean base color — used as fallback where no cell data reaches
    const oceanColor = colorPalette['OceanDeep'] || colorPalette['Ocean'] || [8, 18, 42];

    // Convert cell radius from km to degrees, then to pixels.
    const cellRadiusDeg = cellRadiusKm / 111;

    // Splat radius in pixels — generous overlap so cells blend.
    const baseSplatPx = cellRadiusDeg * PIXELS_PER_DEG * 1.8;

    // Gaussian sigma — controls falloff softness.
    const sigma = baseSplatPx * 0.45;
    const sigmaSquared2 = 2.0 * sigma * sigma;
    const invSigmaSquared2 = 1.0 / sigmaSquared2;

    // ── Pass 1: Gaussian splat each cell ────────────────────────────────────

    for (const cell of cells) {
        if (Math.abs(cell.lat) > 85) continue;

        const color = colorPalette[cell.terrain] || oceanColor;
        const r = color[0];
        const g = color[1];
        const b = color[2];
        const isOcean = OCEAN_TYPES.has(cell.terrain);

        // Convert cell position to pixel coordinates
        const cx = ((cell.lon + 180) / 360) * W;
        const cy = ((LAT_MAX - cell.lat) / (LAT_MAX - LAT_MIN)) * H;

        // At higher latitudes, longitude degrees are narrower
        const cosLat = Math.cos(cell.lat * (Math.PI / 180));
        const latScale = 1.0 / Math.max(cosLat, 0.15);
        const splatRx = baseSplatPx * latScale;
        const splatRy = baseSplatPx;

        // Pixel bounding box for this splat
        const x0 = Math.max(0, Math.floor(cx - splatRx) - 1);
        const x1 = Math.min(W - 1, Math.ceil(cx + splatRx) + 1);
        const y0 = Math.max(0, Math.floor(cy - splatRy) - 1);
        const y1 = Math.min(H - 1, Math.ceil(cy + splatRy) + 1);

        const invRx2 = 1.0 / (splatRx * splatRx);
        const invRy2 = 1.0 / (splatRy * splatRy);

        for (let y = y0; y <= y1; y++) {
            const dy = y - cy;
            const dy2scaled = dy * dy * invRy2;
            const rowOffset = y * W;

            for (let x = x0; x <= x1; x++) {
                const dx = x - cx;
                const normDist2 = dx * dx * invRx2 + dy2scaled;
                if (normDist2 > 1.0) continue;

                const pixelDist2 = normDist2 * splatRy * splatRy;
                const w = Math.exp(-pixelDist2 * invSigmaSquared2);

                const idx = rowOffset + x;
                weightR[idx] += r * w;
                weightG[idx] += g * w;
                weightB[idx] += b * w;
                weightSum[idx] += w;
                landWeight[idx] += isOcean ? -w : w;
            }
        }
    }

    // ── Pass 2: Normalize accumulated colors ────────────────────────────────
    // Also build a land/ocean mask for edge-aware blur.

    const pixels = new Uint8ClampedArray(totalPixels * 4);
    // Mask: true = land pixel, false = ocean pixel
    const isLandMask = new Uint8Array(totalPixels);

    for (let i = 0; i < totalPixels; i++) {
        const w = weightSum[i];
        const pi = i * 4;
        if (w > 0.0001) {
            const invW = 1.0 / w;
            pixels[pi] = (weightR[i] * invW + 0.5) | 0;
            pixels[pi + 1] = (weightG[i] * invW + 0.5) | 0;
            pixels[pi + 2] = (weightB[i] * invW + 0.5) | 0;
            pixels[pi + 3] = 255;
            // Positive landWeight means more land than ocean influence
            isLandMask[i] = landWeight[i] > 0 ? 1 : 0;
        } else {
            pixels[pi] = oceanColor[0];
            pixels[pi + 1] = oceanColor[1];
            pixels[pi + 2] = oceanColor[2];
            pixels[pi + 3] = 255;
            isLandMask[i] = 0;
        }
    }

    // ── Pass 3: Edge-aware blur ─────────────────────────────────────────────
    // Single pass of a gentle blur that ONLY blends pixels of the same type
    // (land with land, ocean with ocean). This smooths terrain transitions
    // without bleeding ocean blue into coastlines or vice versa.

    const blurRadius = Math.max(1, Math.round(baseSplatPx * 0.15));
    edgeAwareBlur(pixels, isLandMask, W, H, blurRadius);

    // ── Pass 4: Render to canvas ────────────────────────────────────────────

    const canvas = document.createElement('canvas');
    canvas.width = W;
    canvas.height = H;
    const ctx = canvas.getContext('2d')!;

    const imageData = new ImageData(pixels, W, H);
    ctx.putImageData(imageData, 0, 0);

    return canvas;
}

// ── Edge-aware blur ─────────────────────────────────────────────────────────
// Only blurs pixels of the same terrain class (land/ocean). Preserves sharp
// coastline boundaries while smoothing within each region.

function edgeAwareBlur(
    pixels: Uint8ClampedArray,
    mask: Uint8Array,
    w: number,
    h: number,
    radius: number
): void {
    // Work on a copy so reads don't see partially-written values
    const src = new Uint8ClampedArray(pixels);

    for (let y = 0; y < h; y++) {
        for (let x = 0; x < w; x++) {
            const idx = y * w + x;
            const centerType = mask[idx];
            let sumR = 0, sumG = 0, sumB = 0, count = 0;

            const y0 = Math.max(0, y - radius);
            const y1 = Math.min(h - 1, y + radius);
            const x0 = Math.max(0, x - radius);
            const x1 = Math.min(w - 1, x + radius);

            for (let ny = y0; ny <= y1; ny++) {
                for (let nx = x0; nx <= x1; nx++) {
                    const nIdx = ny * w + nx;
                    // Only blend pixels of the same type
                    if (mask[nIdx] !== centerType) continue;
                    const pi = nIdx * 4;
                    sumR += src[pi];
                    sumG += src[pi + 1];
                    sumB += src[pi + 2];
                    count++;
                }
            }

            if (count > 0) {
                const pi = idx * 4;
                pixels[pi] = (sumR / count + 0.5) | 0;
                pixels[pi + 1] = (sumG / count + 0.5) | 0;
                pixels[pi + 2] = (sumB / count + 0.5) | 0;
            }
        }
    }
}
