// ── Icon atlas builder ──────────────────────────────────────────────────────
// Builds a single Canvas spritesheet from all SVG icons at init time.
// The atlas + mapping are consumed by deck.gl IconLayer.

import { icons } from '$lib/assets/icons';

/** Layout for a single icon within the atlas spritesheet. */
export interface IconMapping {
    x: number;
    y: number;
    width: number;
    height: number;
    mask: boolean;
}

/** Pixel size of each icon cell in the atlas. */
const ICON_SIZE = 64;

/** Number of icon columns per row in the atlas grid. */
const ICONS_PER_ROW = 8;

/**
 * Build a single Canvas spritesheet from all SVG icons.
 *
 * Returns a Promise that resolves when all icons have been rasterized onto
 * the canvas. The mapping is available immediately (synchronous layout),
 * but the canvas content is only complete after the returned promise resolves.
 */
export async function buildIconAtlas(): Promise<{
    canvas: HTMLCanvasElement;
    mapping: Record<string, IconMapping>;
}> {
    const names = Object.keys(icons);
    const cols = ICONS_PER_ROW;
    const rows = Math.ceil(names.length / cols);

    const canvas = document.createElement('canvas');
    canvas.width = cols * ICON_SIZE;
    canvas.height = rows * ICON_SIZE;
    const ctx = canvas.getContext('2d')!;

    const mapping: Record<string, IconMapping> = {};
    const promises: Promise<void>[] = [];

    for (let i = 0; i < names.length; i++) {
        const name = names[i];
        const col = i % cols;
        const row = Math.floor(i / cols);
        const x = col * ICON_SIZE;
        const y = row * ICON_SIZE;

        mapping[name] = { x, y, width: ICON_SIZE, height: ICON_SIZE, mask: false };

        const svg = (icons as Record<string, string>)[name];
        const blob = new Blob([svg], { type: 'image/svg+xml;charset=utf-8' });
        const url = URL.createObjectURL(blob);

        promises.push(new Promise<void>((resolve) => {
            const img = new Image();
            img.onload = () => {
                ctx.drawImage(img, x, y, ICON_SIZE, ICON_SIZE);
                URL.revokeObjectURL(url);
                resolve();
            };
            img.onerror = () => {
                // Draw a fallback colored square so missing icons are visible
                ctx.fillStyle = '#666';
                ctx.fillRect(x + 8, y + 8, ICON_SIZE - 16, ICON_SIZE - 16);
                URL.revokeObjectURL(url);
                resolve();
            };
            img.src = url;
        }));
    }

    await Promise.all(promises);

    return { canvas, mapping };
}
