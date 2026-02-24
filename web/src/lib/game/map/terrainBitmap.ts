// ── Terrain bitmap builder ──────────────────────────────────────────────────
// Pre-renders all grid cells onto an equirectangular Canvas image.
// Each cell becomes a filled ellipse of terrain color, suitable for
// deck.gl BitmapLayer or MapLibre raster source.

import type { GridCell } from '$lib/wasm/types';

/**
 * Render grid cells onto an equirectangular canvas.
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
    // Canvas resolution: 2px per degree gives a 720x340 canvas (lat -85 to 85)
    const PIXELS_PER_DEG = 2;
    const W = 360 * PIXELS_PER_DEG;  // 720
    const H = 170 * PIXELS_PER_DEG;  // 340
    const LAT_MIN = -85;
    const LAT_MAX = 85;

    const canvas = document.createElement('canvas');
    canvas.width = W;
    canvas.height = H;
    const ctx = canvas.getContext('2d')!;

    // Fill with deep ocean base color
    const oceanColor = colorPalette['OceanDeep'] || colorPalette['Ocean'] || [6, 12, 32];
    ctx.fillStyle = `rgb(${oceanColor[0]},${oceanColor[1]},${oceanColor[2]})`;
    ctx.fillRect(0, 0, W, H);

    // Convert cell radius from km to approximate pixel radius
    // At equator: 1 degree ~ 111km, so cellRadiusKm / 111 = degrees, * PIXELS_PER_DEG = pixels
    const baseDegRadius = cellRadiusKm / 111;
    const basePixelRadius = baseDegRadius * PIXELS_PER_DEG * 1.3; // 1.3 overlap factor for gapless coverage

    for (const cell of cells) {
        if (Math.abs(cell.lat) > 85) continue;

        const color = colorPalette[cell.terrain] || oceanColor;

        // Convert lon/lat to pixel coordinates
        const px = ((cell.lon + 180) / 360) * W;
        const py = ((LAT_MAX - cell.lat) / (LAT_MAX - LAT_MIN)) * H;

        // At higher latitudes, longitude degrees are compressed — expand the x-radius
        const latScale = 1 / Math.max(Math.cos(cell.lat * Math.PI / 180), 0.15);
        const rX = basePixelRadius * latScale;
        const rY = basePixelRadius;

        ctx.fillStyle = `rgb(${color[0]},${color[1]},${color[2]})`;
        ctx.beginPath();
        ctx.ellipse(px, py, rX, rY, 0, 0, Math.PI * 2);
        ctx.fill();
    }

    return canvas;
}
