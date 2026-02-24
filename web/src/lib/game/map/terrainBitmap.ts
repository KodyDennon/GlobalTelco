// ── Terrain bitmap builder ──────────────────────────────────────────────────
// Renders grid cells onto an equirectangular Canvas using Gaussian-splat
// interpolation with edge-aware blur. Heavy computation is offloaded to a
// Web Worker so the main thread stays responsive during initialization.

import type { GridCell } from '$lib/wasm/types';

// ── Resolution scaling ──────────────────────────────────────────────────────
// Pixels-per-degree controls the bitmap resolution and directly affects
// computation time (quadratic scaling). Quality tiers:
//   low    → 4 ppd → 1440×680  (~1M pixels)   — fastest, ~0.5s in worker
//   medium → 6 ppd → 2160×1020 (~2.2M pixels)  — balanced, ~1.5s in worker
//   high   → 8 ppd → 2880×1360 (~3.9M pixels)  — best quality, ~3s in worker

const PPD_BY_QUALITY: Record<string, number> = {
    low: 4,
    medium: 6,
    high: 8,
};

/** Resolve pixels-per-degree for a given quality tier. */
export function getPixelsPerDeg(quality: 'low' | 'medium' | 'high' = 'medium'): number {
    return PPD_BY_QUALITY[quality] ?? 6;
}

// ── Worker pool (singleton) ─────────────────────────────────────────────────
// A single reusable worker handles all terrain bitmap requests. Requests are
// serialized (one at a time) which is fine since we only build bitmaps at
// init and on overlay toggle.

let workerInstance: Worker | null = null;
let workerSupported: boolean | null = null;

function getWorker(): Worker | null {
    if (workerSupported === false) return null;

    if (workerInstance) return workerInstance;

    try {
        workerInstance = new Worker(
            new URL('./terrainBitmap.worker.ts', import.meta.url),
            { type: 'module' }
        );
        workerSupported = true;
        return workerInstance;
    } catch {
        // Workers unavailable (SSR, restricted context, etc.)
        workerSupported = false;
        return null;
    }
}

/** Clean up the worker when the application unmounts. */
export function disposeTerrainWorker(): void {
    if (workerInstance) {
        workerInstance.terminate();
        workerInstance = null;
    }
}

// ── Public API ──────────────────────────────────────────────────────────────

/**
 * Build a terrain bitmap asynchronously using a Web Worker.
 *
 * The heavy Gaussian-splat computation runs entirely off the main thread.
 * The pixel buffer is transferred back (zero-copy) and painted onto a
 * canvas on the main thread.
 *
 * Falls back to synchronous computation if Web Workers are unavailable.
 */
export async function buildTerrainBitmapAsync(
    cells: GridCell[],
    cellSpacingKm: number,
    colorPalette: Record<string, [number, number, number]>,
    quality: 'low' | 'medium' | 'high' = 'medium'
): Promise<HTMLCanvasElement> {
    const pixelsPerDeg = getPixelsPerDeg(quality);
    const worker = getWorker();

    if (worker) {
        // Worker path: offload computation
        const cellData = cells.map(c => ({
            lat: c.lat,
            lon: c.lon,
            terrain: c.terrain
        }));

        const result = await new Promise<{ pixels: Uint8ClampedArray; width: number; height: number }>((resolve, reject) => {
            const onMessage = (e: MessageEvent) => {
                worker.removeEventListener('message', onMessage);
                worker.removeEventListener('error', onError);
                resolve(e.data);
            };
            const onError = (e: ErrorEvent) => {
                worker.removeEventListener('message', onMessage);
                worker.removeEventListener('error', onError);
                reject(new Error(`Terrain worker error: ${e.message}`));
            };

            worker.addEventListener('message', onMessage);
            worker.addEventListener('error', onError);

            worker.postMessage({
                cells: cellData,
                cellSpacingKm,
                colorPalette,
                pixelsPerDeg,
            });
        });

        return pixelsToCanvas(result.pixels, result.width, result.height);
    }

    // Fallback: synchronous computation on main thread
    return buildTerrainBitmapSync(cells, cellSpacingKm, colorPalette, pixelsPerDeg);
}

// ── Canvas creation (main thread only) ──────────────────────────────────────

function pixelsToCanvas(pixels: Uint8ClampedArray, w: number, h: number): HTMLCanvasElement {
    const canvas = document.createElement('canvas');
    canvas.width = w;
    canvas.height = h;
    const ctx = canvas.getContext('2d')!;
    const imageData = new ImageData(pixels, w, h);
    ctx.putImageData(imageData, 0, 0);
    return canvas;
}

// ── Synchronous fallback ────────────────────────────────────────────────────
// Used when Web Workers are unavailable. Identical algorithm to the worker.

const OCEAN_TYPES = new Set(['OceanShallow', 'OceanDeep', 'Ocean']);
const LAT_MIN = -85;
const LAT_MAX = 85;

function buildTerrainBitmapSync(
    cells: GridCell[],
    cellSpacingKm: number,
    colorPalette: Record<string, [number, number, number]>,
    pixelsPerDeg: number
): HTMLCanvasElement {
    const W = 360 * pixelsPerDeg;
    const H = 170 * pixelsPerDeg;
    const totalPixels = W * H;

    const weightR = new Float32Array(totalPixels);
    const weightG = new Float32Array(totalPixels);
    const weightB = new Float32Array(totalPixels);
    const weightSum = new Float32Array(totalPixels);
    const landWeight = new Float32Array(totalPixels);

    const oceanColor = colorPalette['OceanDeep'] || colorPalette['Ocean'] || [8, 18, 42];
    const cellRadiusDeg = cellSpacingKm / 111;
    const baseSplatPx = cellRadiusDeg * pixelsPerDeg * 1.8;
    const sigma = baseSplatPx * 0.45;
    const sigmaSquared2 = 2.0 * sigma * sigma;
    const invSigmaSquared2 = 1.0 / sigmaSquared2;

    for (const cell of cells) {
        if (Math.abs(cell.lat) > 85) continue;

        const color = colorPalette[cell.terrain] || oceanColor;
        const r = color[0];
        const g = color[1];
        const b = color[2];
        const isOcean = OCEAN_TYPES.has(cell.terrain);

        const cx = ((cell.lon + 180) / 360) * W;
        const cy = ((LAT_MAX - cell.lat) / (LAT_MAX - LAT_MIN)) * H;

        const cosLat = Math.cos(cell.lat * (Math.PI / 180));
        const latScale = 1.0 / Math.max(cosLat, 0.15);
        const splatRx = baseSplatPx * latScale;
        const splatRy = baseSplatPx;

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

    const pixels = new Uint8ClampedArray(totalPixels * 4);
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
            isLandMask[i] = landWeight[i] > 0 ? 1 : 0;
        } else {
            pixels[pi] = oceanColor[0];
            pixels[pi + 1] = oceanColor[1];
            pixels[pi + 2] = oceanColor[2];
            pixels[pi + 3] = 255;
            isLandMask[i] = 0;
        }
    }

    const blurRadius = Math.max(1, Math.round(baseSplatPx * 0.15));
    edgeAwareBlur(pixels, isLandMask, W, H, blurRadius);

    return pixelsToCanvas(pixels, W, H);
}

function edgeAwareBlur(
    pixels: Uint8ClampedArray,
    mask: Uint8Array,
    w: number,
    h: number,
    radius: number
): void {
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
