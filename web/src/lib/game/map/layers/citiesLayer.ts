import { ScatterplotLayer, IconLayer } from '@deck.gl/layers';
import type { Layer } from '@deck.gl/core';
import type { City } from '$lib/wasm/types';
import type { IconMapping } from './infraLayer';

/**
 * Creates city glow (ScatterplotLayer with additive blending) and city icons
 * (IconLayer with ScatterplotLayer fallback). City tier is determined by
 * population thresholds: hamlet / town / city / metropolis / megalopolis.
 */
export function createCitiesLayer(opts: {
    cities: City[];
    iconAtlas: HTMLCanvasElement | null;
    iconMapping: Record<string, IconMapping>;
    iconAtlasReady: boolean;
}): Layer[] {
    const { cities, iconAtlas, iconMapping, iconAtlasReady } = opts;

    const gtgCities = cities.filter(c => Math.abs(c.y) <= 85).map(c => {
        let tier = 'hamlet';
        if (c.population > 5000000) tier = 'megalopolis';
        else if (c.population > 1000000) tier = 'metropolis';
        else if (c.population > 250000) tier = 'city';
        else if (c.population > 50000) tier = 'town';
        return { ...c, tier };
    });

    const layers: Layer[] = [
        // City glow — warm light halo
        new ScatterplotLayer({
            id: 'cities-glow',
            data: gtgCities,
            getPosition: (d: any) => [d.x, d.y],
            getFillColor: [255, 210, 120, 140],
            getRadius: (d: any) => Math.log10(Math.max(d.population, 10)) * 22000,
            radiusMinPixels: 3,
            pickable: false,
            parameters: {
                blend: true,
                blendFunc: [WebGLRenderingContext.SRC_ALPHA, WebGLRenderingContext.ONE],
            }
        })
    ];

    // City icons with atlas
    if (iconAtlasReady && iconAtlas) {
        layers.push(new IconLayer({
            id: 'cities-icons',
            data: gtgCities,
            getPosition: (d: any) => [d.x, d.y],
            getIcon: (d: any) => d.tier,
            iconAtlas: iconAtlas as any,
            iconMapping: iconMapping,
            getSize: (d: any) => Math.log10(Math.max(d.population, 10)) * 8,
            sizeMinPixels: 16,
            sizeMaxPixels: 48,
            getColor: [255, 255, 255, 255],
            pickable: true,
            autoHighlight: true,
            onClick: ({ object }: any) => {
                if (object) {
                    window.dispatchEvent(new CustomEvent('entity-selected', { detail: { id: object.id, type: 'city' } }));
                }
            }
        }));
    } else {
        // Fallback dots
        layers.push(new ScatterplotLayer({
            id: 'cities-dots-fallback',
            data: gtgCities,
            getPosition: (d: any) => [d.x, d.y],
            getFillColor: [255, 220, 150, 255],
            getRadius: (d: any) => Math.log10(Math.max(d.population, 10)) * 8000,
            radiusMinPixels: 4,
            radiusMaxPixels: 16,
            pickable: true,
            onClick: ({ object }: any) => {
                if (object) {
                    window.dispatchEvent(new CustomEvent('entity-selected', { detail: { id: object.id, type: 'city' } }));
                }
            }
        }));
    }

    return layers;
}
