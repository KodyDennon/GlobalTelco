// ── City glow layer ─────────────────────────────────────────────────────────
// Renders cities as warm glowing blobs at low zoom levels (0-7).
// Population-proportional alpha and radius. Fades out at zoom 5+.
// Uses additive blending for a "night earth city lights" effect.
//
// This layer renders BEFORE city icons/dots so the glow sits underneath.

import { ScatterplotLayer } from '@deck.gl/layers';
import type { Layer } from '@deck.gl/core';
import type { City } from '$lib/wasm/types';

// ── Types ──────────────────────────────────────────────────────────────────

interface CityGlowDatum {
    x: number;
    y: number;
    population: number;
    alpha: number;
    radius: number;
}

// ── Layer creation ─────────────────────────────────────────────────────────

/**
 * Create the city glow ScatterplotLayer with population-proportional size
 * and zoom-dependent fade.
 *
 * At zoom 0-4: full glow intensity.
 * At zoom 5-8: fade out (multiply alpha by (8 - zoom) / 4, clamped 0-1).
 * At zoom 8+: invisible.
 *
 * @param cities - Array of City objects from WASM.
 * @param currentZoom - Current map zoom level.
 * @returns Array containing the glow layer, or empty array if zoom too high.
 */
export function createCityGlowLayer(cities: City[], currentZoom: number): Layer[] {
    if (currentZoom >= 8) return [];

    // Compute zoom fade factor
    // zoom 0-4: fadeFactor = 1.0
    // zoom 5-8: fadeFactor linearly from 0.75 to 0.0
    let fadeFactor = 1.0;
    if (currentZoom >= 5) {
        fadeFactor = Math.max(0, (8 - currentZoom) / 4);
    }

    if (fadeFactor <= 0) return [];

    const glowData: CityGlowDatum[] = [];

    for (const city of cities) {
        if (Math.abs(city.y) > 85) continue;

        const pop = Math.max(city.population, 100);

        // Alpha proportional to population: small cities = 40, large = 150
        const popNorm = Math.min(1.0, Math.log10(pop) / 7.5); // log10(30M) ~ 7.5
        const baseAlpha = 40 + popNorm * 110; // 40..150
        const alpha = Math.round(baseAlpha * fadeFactor);

        // Radius proportional to sqrt(population) * 0.3 (in meters)
        const radius = Math.sqrt(pop) * 0.3;

        if (alpha > 0) {
            glowData.push({
                x: city.x,
                y: city.y,
                population: pop,
                alpha,
                radius,
            });
        }
    }

    if (glowData.length === 0) return [];

    return [
        new ScatterplotLayer({
            id: 'city-glow-ambient',
            data: glowData,
            getPosition: (d: CityGlowDatum) => [d.x, d.y],
            getFillColor: (d: CityGlowDatum) => [255, 180, 60, d.alpha],
            getRadius: (d: CityGlowDatum) => d.radius,
            radiusMinPixels: 4,
            radiusMaxPixels: 60,
            pickable: false,
            parameters: {
                depthTest: false,
                blend: true,
                blendFunc: [WebGLRenderingContext.SRC_ALPHA, WebGLRenderingContext.ONE],
            },
            updateTriggers: {
                getFillColor: [currentZoom],
                getRadius: [currentZoom],
            },
        }),
    ];
}
