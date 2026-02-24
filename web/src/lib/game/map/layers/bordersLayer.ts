import { PathLayer } from '@deck.gl/layers';
import type { Layer } from '@deck.gl/core';
import type { Region } from '$lib/wasm/types';

/**
 * Creates a PathLayer for procedurally generated region borders.
 * For real-earth maps, returns null (MapLibre vector tiles handle borders).
 */
export function createBordersLayer(
    regions: Region[],
    isRealEarth: boolean
): Layer | null {
    if (isRealEarth) return null;

    const borderData = regions
        .filter(r => r.boundary_polygon?.length > 2)
        .map(r => ({
            polygon: r.boundary_polygon.map(p => [p[0], p[1]]),
            name: r.name
        }));

    return new PathLayer({
        id: 'region-borders',
        data: borderData,
        getPath: (d: any) => d.polygon,
        getColor: [140, 160, 200, 120],
        getWidth: 1.5,
        widthUnits: 'pixels'
    });
}
