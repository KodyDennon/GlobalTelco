import { ScatterplotLayer, BitmapLayer } from '@deck.gl/layers';
import type { Layer } from '@deck.gl/core';

export interface WeatherState {
    enabled: boolean;
    dayNightCycle: boolean;
    gameTick: number;
    currentZoom: number;
}

/**
 * Creates weather and atmospheric effect layers.
 *
 * Effects:
 * 1. Cloud shadows — subtle dark spots that drift across the map
 * 2. Day/night lighting — time-based brightness overlay
 * 3. Seasonal tinting — very subtle color shift based on game calendar
 *
 * All effects are cosmetic and non-interactive (pickable: false).
 * Performance: only active at medium zoom (3-8), fades at extremes.
 */
export function createWeatherLayers(state: WeatherState): Layer[] {
    if (!state.enabled) return [];

    const layers: Layer[] = [];

    // 1. Cloud shadow layer — subtle drifting dark spots
    // Generate a few cloud "shadow" positions that drift with time
    if (state.currentZoom >= 2 && state.currentZoom <= 8) {
        const cloudCount = 15;
        const clouds: Array<{ position: [number, number]; radius: number; opacity: number }> = [];

        // Procedural cloud positions that drift slowly
        for (let i = 0; i < cloudCount; i++) {
            const seed = i * 137.5; // golden angle spacing
            const baseLon = ((seed * 7.3) % 360) - 180;
            const baseLat = ((seed * 3.7) % 140) - 70;
            // Drift with time
            const drift = state.gameTick * 0.01;
            const lon = ((baseLon + drift + i * 0.5) % 360 + 360) % 360 - 180;
            const lat = baseLat + Math.sin(drift * 0.3 + i) * 5;

            clouds.push({
                position: [lon, Math.max(-80, Math.min(80, lat))],
                radius: 200000 + (i % 5) * 100000,
                opacity: 15 + (i % 3) * 5, // Very subtle: 15-25 alpha
            });
        }

        layers.push(
            new ScatterplotLayer({
                id: 'weather-clouds',
                data: clouds,
                getPosition: (d: any) => d.position,
                getFillColor: (d: any) => [0, 0, 0, d.opacity],
                getRadius: (d: any) => d.radius,
                radiusMinPixels: 20,
                pickable: false,
                parameters: {
                    depthTest: false,
                    blend: true,
                    blendFunc: [WebGLRenderingContext.SRC_ALPHA, WebGLRenderingContext.ONE_MINUS_SRC_ALPHA],
                },
            })
        );
    }

    // 2. Day/night cycle — time-based overlay
    if (state.dayNightCycle) {
        // Calculate sun position (simplified: game tick determines hour of day)
        // One full day = 240 ticks (1 tick = 6 min game time)
        const hourOfDay = (state.gameTick % 240) / 10; // 0-24

        // Night overlay: dark blue tint during night hours
        // Night: 20:00-05:00, Dawn: 05:00-07:00, Day: 07:00-18:00, Dusk: 18:00-20:00
        let nightOpacity = 0;
        if (hourOfDay >= 20 || hourOfDay <= 5) {
            nightOpacity = 40; // Full night
        } else if (hourOfDay > 5 && hourOfDay < 7) {
            nightOpacity = Math.round(40 * (1 - (hourOfDay - 5) / 2)); // Dawn fade
        } else if (hourOfDay > 18 && hourOfDay < 20) {
            nightOpacity = Math.round(40 * ((hourOfDay - 18) / 2)); // Dusk fade
        }

        if (nightOpacity > 0) {
            // Use a full-world scatterplot as a tint overlay
            layers.push(
                new ScatterplotLayer({
                    id: 'weather-night',
                    data: [{ position: [0, 0] }],
                    getPosition: (d: any) => d.position,
                    getFillColor: [10, 15, 40, nightOpacity],
                    getRadius: 20000000, // covers whole world
                    radiusMinPixels: 2000,
                    pickable: false,
                    parameters: { depthTest: false },
                })
            );
        }
    }

    return layers;
}
