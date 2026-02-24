import { TextLayer } from '@deck.gl/layers';
import type { Layer } from '@deck.gl/core';
import type { City, Region } from '$lib/wasm/types';

/**
 * Creates city labels (TextLayer, zoom-dependent population filter) and
 * region labels (TextLayer, hidden at zoom > 5).
 */
export function createLabelLayers(opts: {
    cities: City[];
    regions: Region[];
    currentZoom: number;
}): Layer[] {
    const { cities, regions, currentZoom } = opts;
    const layers: Layer[] = [];

    // City labels — zoom-dependent population filter
    if (currentZoom >= 0.8) {
        const minPop = currentZoom < 1.5 ? 5000000
            : currentZoom < 2 ? 1000000
            : currentZoom < 3 ? 500000
            : currentZoom < 5 ? 100000
            : 0;
        const visibleCities = minPop > 0
            ? cities.filter(c => c.population >= minPop)
            : cities;

        layers.push(new TextLayer({
            id: 'city-labels',
            data: visibleCities,
            getPosition: (d: City) => [d.x, d.y],
            getText: (d: City) => d.name,
            getSize: 12,
            getColor: [255, 255, 255, 200],
            getAlignmentBaseline: 'bottom',
            getPixelOffset: [0, -10],
            fontFamily: 'Inter, sans-serif',
            parameters: { depthTest: false }
        }));
    }

    // Region labels — hidden at zoom > 5
    if (currentZoom <= 5) {
        layers.push(new TextLayer({
            id: 'region-labels',
            data: regions,
            getPosition: (d: Region) => [d.center_lon, d.center_lat],
            getText: (d: Region) => d.name,
            getSize: 18,
            getColor: [255, 255, 255, 80],
            getAlignmentBaseline: 'center',
            fontFamily: 'Inter, sans-serif',
            fontWeight: 'bold',
            parameters: { depthTest: false }
        }));
    }

    return layers;
}
