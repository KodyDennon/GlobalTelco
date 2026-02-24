import { BitmapLayer } from '@deck.gl/layers';
import type { Layer } from '@deck.gl/core';

/**
 * Creates the BitmapLayer for procedurally generated terrain.
 * For real-earth maps, returns null (MapLibre handles the base).
 */
export function createLandLayer(
    terrainCanvas: HTMLCanvasElement | null,
    isRealEarth: boolean
): Layer | null {
    if (isRealEarth || !terrainCanvas) return null;

    return new BitmapLayer({
        id: 'land-layer',
        image: terrainCanvas as any,
        bounds: [-180, -85, 180, 85] as [number, number, number, number],
        pickable: true,
        onClick: (info: any) => {
            window.dispatchEvent(new CustomEvent('entity-selected', { detail: { id: null, type: null } }));
            if (info.coordinate) {
                const [lon, lat] = info.coordinate;
                window.dispatchEvent(new CustomEvent('map-clicked', { detail: { lon, lat } }));
            }
        }
    });
}
