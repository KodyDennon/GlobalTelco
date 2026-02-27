// ── Terrain bitmap Web Worker ────────────────────────────────────────────────
// Offloads the expensive Gaussian-splat terrain rendering to a background
// thread so the main thread stays responsive during game initialization.
//
// Receives: cell data, color palette, cell spacing, pixels-per-degree
// Returns:  raw pixel buffer (Uint8ClampedArray) + dimensions via Transferable

interface CellInput {
    lat: number;
    lon: number;
    terrain: string;
}

interface WorkerRequest {
    cells: CellInput[];
    cellSpacingKm: number;
    colorPalette: Record<string, [number, number, number]>;
    pixelsPerDeg: number;
}

interface WorkerResponse {
    pixels: Uint8ClampedArray;
    width: number;
    height: number;
}

const OCEAN_TYPES = new Set(['OceanShallow', 'OceanDeep', 'OceanTrench', 'Ocean']);
const LAT_MIN = -85;
const LAT_MAX = 85;

function computeTerrainPixels(
    cells: CellInput[],
    cellSpacingKm: number,
    colorPalette: Record<string, [number, number, number]>,
    pixelsPerDeg: number
): WorkerResponse {
    const W = 360 * pixelsPerDeg;
    const H = 170 * pixelsPerDeg;
    const totalPixels = W * H;

    // Accumulation buffers
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

    // Pass 1: Gaussian splat each cell
    for (let ci = 0; ci < cells.length; ci++) {
        const cell = cells[ci];
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

    // Pass 2: Normalize
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

    // Pass 3: Edge-aware blur
    const blurRadius = Math.max(1, Math.round(baseSplatPx * 0.15));
    edgeAwareBlur(pixels, isLandMask, W, H, blurRadius);

    return { pixels, width: W, height: H };
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

// Worker message handler
self.onmessage = (e: MessageEvent<WorkerRequest>) => {
    const { cells, cellSpacingKm, colorPalette, pixelsPerDeg } = e.data;
    const result = computeTerrainPixels(cells, cellSpacingKm, colorPalette, pixelsPerDeg);

    // Transfer the pixel buffer (zero-copy) to avoid duplicating memory
    (self as any).postMessage(result, [result.pixels.buffer]);
};
