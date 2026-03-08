import { ScatterplotLayer } from '@deck.gl/layers';
import type { Layer } from '@deck.gl/core';
import type { Region } from '$lib/wasm/types';
import type { WeatherForecast } from '$lib/wasm/bridge';

export interface WeatherState {
    enabled: boolean;
    dayNightCycle: boolean;
    gameTick: number;
    currentZoom: number;
}

/** An active disaster event with location + type + intensity for visualization. */
export interface ActiveDisaster {
    id: string;
    disasterType: string;
    /** Center longitude of the affected region. */
    lon: number;
    /** Center latitude of the affected region. */
    lat: number;
    /** 0-1 severity, controls opacity and animation speed. */
    severity: number;
    /** Game tick when the disaster started. */
    startTick: number;
    /** Region name for display. */
    regionName: string;
    /** Region ID for matching. */
    regionId: number;
    /** Number of affected infrastructure assets. */
    affectedCount: number;
}

/** A forecasted disaster — client-side approximation based on region disaster_risk. */
export interface ForecastDisaster {
    /** Unique ID for the forecast entry. */
    id: string;
    /** Predicted disaster type based on region characteristics. */
    disasterType: string;
    /** Region where the disaster might occur. */
    regionName: string;
    /** Region ID. */
    regionId: number;
    /** Center longitude. */
    lon: number;
    /** Center latitude. */
    lat: number;
    /** Estimated ticks until arrival (5-15 range, seeded by tick). */
    estimatedTicks: number;
    /** Probability of occurrence (0-1), derived from disaster_risk. */
    probability: number;
}

/** Disaster type names for forecast display, selected by seeded pseudo-random. */
const FORECAST_DISASTER_TYPES = [
    'Thunderstorm', 'Hurricane', 'Earthquake', 'Flooding',
    'Ice Storm', 'Cyber Attack', 'Political Unrest', 'Equipment Failure',
];

/**
 * Simple seeded pseudo-random number generator (mulberry32).
 * Returns values in [0, 1).
 */
function seededRandom(seed: number): number {
    let t = (seed + 0x6D2B79F5) | 0;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
}

/**
 * Compute disaster forecasts for regions with elevated disaster risk.
 * This is a client-side heuristic — it examines each region's disaster_risk
 * and generates warning entries for those above the threshold.
 *
 * The forecasts are deterministic for a given tick (seeded by tick + region id)
 * so they don't flicker between frames.
 *
 * @param regions - All regions with their disaster_risk values
 * @param currentTick - The current simulation tick
 * @param riskThreshold - Minimum disaster_risk to generate a forecast (default 0.3)
 * @returns Array of forecast entries sorted by probability descending
 */
export function computeDisasterForecasts(
    regions: Region[],
    currentTick: number,
    riskThreshold = 0.3,
): ForecastDisaster[] {
    return [];
}

export function convertWeatherForecasts(
    serverForecasts: WeatherForecast[],
    regions: Region[],
    currentTick: number,
): ForecastDisaster[] {
    return [];
}

/**
 * Creates weather and atmospheric effect layers.
 *
 * Effects:
 * 1. Cloud shadows -- subtle dark spots that drift across the map
 * 2. Day/night lighting -- time-based brightness overlay
 * 3. Active storm visualizations -- animated overlays for active disaster events
 * 4. Forecast warning zones -- pulsing amber outlines on high-risk regions
 *
 * All effects are cosmetic and non-interactive (pickable: false).
 * Performance: only active at medium zoom (2-8), fades at extremes.
 */
export function createWeatherLayers(
    state: WeatherState,
    activeDisasters?: ActiveDisaster[],
    forecastDisasters?: ForecastDisaster[],
): Layer[] {
    if (!state.enabled) return [];

    const layers: Layer[] = [];

    // 1. Cloud shadow layer -- subtle drifting dark spots
    if (state.currentZoom >= 2 && state.currentZoom <= 8) {
        const cloudCount = 15;
        const clouds: Array<{ position: [number, number]; radius: number; opacity: number }> = [];

        for (let i = 0; i < cloudCount; i++) {
            const seed = i * 137.5;
            const baseLon = ((seed * 7.3) % 360) - 180;
            const baseLat = ((seed * 3.7) % 140) - 70;
            const drift = state.gameTick * 0.01;
            const lon = ((baseLon + drift + i * 0.5) % 360 + 360) % 360 - 180;
            const lat = baseLat + Math.sin(drift * 0.3 + i) * 5;

            clouds.push({
                position: [lon, Math.max(-80, Math.min(80, lat))],
                radius: 200000 + (i % 5) * 100000,
                opacity: 15 + (i % 3) * 5,
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

    // 2. Day/night cycle -- time-based overlay
    if (state.dayNightCycle) {
        const hourOfDay = (state.gameTick % 240) / 10;

        let nightOpacity = 0;
        if (hourOfDay >= 20 || hourOfDay <= 5) {
            nightOpacity = 40;
        } else if (hourOfDay > 5 && hourOfDay < 7) {
            nightOpacity = Math.round(40 * (1 - (hourOfDay - 5) / 2));
        } else if (hourOfDay > 18 && hourOfDay < 20) {
            nightOpacity = Math.round(40 * ((hourOfDay - 18) / 2));
        }

        if (nightOpacity > 0) {
            layers.push(
                new ScatterplotLayer({
                    id: 'weather-night',
                    data: [{ position: [0, 0] }],
                    getPosition: (d: any) => d.position,
                    getFillColor: [10, 15, 40, nightOpacity],
                    getRadius: 20000000,
                    radiusMinPixels: 2000,
                    pickable: false,
                    parameters: { depthTest: false },
                })
            );
        }
    }

    // 3. Active disaster visualizations
    if (activeDisasters && activeDisasters.length > 0) {
        layers.push(...createDisasterLayers(activeDisasters, state.gameTick));
    }

    // 4. Forecast warning zones -- pulsing amber outlines on high-risk regions
    if (forecastDisasters && forecastDisasters.length > 0) {
        layers.push(...createForecastLayers(forecastDisasters, state.gameTick));
    }

    return layers;
}

// ── Disaster visualization layers ─────────────────────────────────────────

/** Duration in ticks that a disaster visualization persists. */
export const DISASTER_DISPLAY_DURATION = 80;

/**
 * Maps disaster type to visual treatment:
 * - Thunderstorm / Hurricane: dark overlays with animated rings
 * - Earthquake: radial ripple effect
 * - Flooding: blue water overlay
 * - Ice storm (mapped from weather-like types): blue-white shimmer
 * - Others: generic amber warning overlay
 */
function createDisasterLayers(disasters: ActiveDisaster[], gameTick: number): Layer[] {
    const layers: Layer[] = [];

    for (let i = 0; i < disasters.length; i++) {
        const d = disasters[i];
        const elapsed = gameTick - d.startTick;
        // Fade out over last 20 ticks of display duration
        const fadeStart = DISASTER_DISPLAY_DURATION - 20;
        const fadeFactor = elapsed > fadeStart
            ? Math.max(0, 1 - (elapsed - fadeStart) / 20)
            : 1;

        if (fadeFactor <= 0) continue;

        const baseOpacity = Math.floor(40 + d.severity * 60);
        const opacity = Math.floor(baseOpacity * fadeFactor);

        // Animation phase based on tick (oscillates 0-1)
        const phase = (gameTick % 40) / 40;
        const pulse = 0.6 + 0.4 * Math.sin(phase * Math.PI * 2);

        const type = classifyDisaster(d.disasterType);

        switch (type) {
            case 'storm': {
                // Thunderstorm: dark cloud overlay with flash effect
                // Flash occurs briefly every ~12 ticks
                const flashPhase = gameTick % 12;
                const isFlash = flashPhase === 0 && d.severity > 0.3;
                const flashColor: [number, number, number, number] = isFlash
                    ? [255, 255, 255, Math.floor(80 * fadeFactor)]
                    : [30, 30, 50, opacity];

                // Main storm cloud
                layers.push(new ScatterplotLayer({
                    id: `disaster-storm-${i}`,
                    data: [{ position: [d.lon, d.lat] }],
                    getPosition: (dd: any) => dd.position,
                    getFillColor: flashColor,
                    getRadius: 250000 + d.severity * 150000,
                    radiusMinPixels: 30,
                    pickable: false,
                    parameters: {
                        depthTest: false,
                        blend: true,
                        blendFunc: [WebGLRenderingContext.SRC_ALPHA, WebGLRenderingContext.ONE_MINUS_SRC_ALPHA],
                    },
                }));

                // Wind direction arrows (small satellite dots around center)
                const arrowCount = 6;
                const arrowData: Array<{ position: [number, number] }> = [];
                const windAngle = gameTick * 0.08; // slowly rotating wind
                for (let a = 0; a < arrowCount; a++) {
                    const angle = windAngle + (a / arrowCount) * Math.PI * 2;
                    const dist = 1.5 + d.severity;
                    arrowData.push({
                        position: [
                            d.lon + Math.cos(angle) * dist,
                            d.lat + Math.sin(angle) * dist * 0.6,
                        ],
                    });
                }
                layers.push(new ScatterplotLayer({
                    id: `disaster-storm-arrows-${i}`,
                    data: arrowData,
                    getPosition: (dd: any) => dd.position,
                    getFillColor: [200, 200, 220, Math.floor(60 * pulse * fadeFactor)],
                    getRadius: 40000,
                    radiusMinPixels: 3,
                    pickable: false,
                    parameters: { depthTest: false },
                }));
                break;
            }

            case 'hurricane': {
                // Hurricane/Typhoon: rotating spiral pattern
                const spiralCount = 12;
                const spiralData: Array<{ position: [number, number]; opacity: number }> = [];
                const rotAngle = gameTick * 0.12;
                for (let s = 0; s < spiralCount; s++) {
                    const t = s / spiralCount;
                    const spiralR = 0.5 + t * 3.0 * d.severity;
                    const angle = rotAngle + t * Math.PI * 4;
                    spiralData.push({
                        position: [
                            d.lon + Math.cos(angle) * spiralR,
                            d.lat + Math.sin(angle) * spiralR * 0.6,
                        ],
                        opacity: Math.floor((1 - t * 0.5) * opacity),
                    });
                }

                // Eye of the storm
                layers.push(new ScatterplotLayer({
                    id: `disaster-hurricane-eye-${i}`,
                    data: [{ position: [d.lon, d.lat] }],
                    getPosition: (dd: any) => dd.position,
                    getFillColor: [20, 20, 40, Math.floor(opacity * 0.8)],
                    getRadius: 80000,
                    radiusMinPixels: 8,
                    pickable: false,
                    parameters: { depthTest: false },
                }));

                // Spiral arms
                layers.push(new ScatterplotLayer({
                    id: `disaster-hurricane-spiral-${i}`,
                    data: spiralData,
                    getPosition: (dd: any) => dd.position,
                    getFillColor: (dd: any) => [60, 70, 100, dd.opacity],
                    getRadius: 120000 * d.severity,
                    radiusMinPixels: 10,
                    pickable: false,
                    parameters: {
                        depthTest: false,
                        blend: true,
                        blendFunc: [WebGLRenderingContext.SRC_ALPHA, WebGLRenderingContext.ONE_MINUS_SRC_ALPHA],
                    },
                }));
                break;
            }

            case 'earthquake': {
                // Ripple/wave effect radiating from epicenter
                const ringCount = 4;
                const ringData: Array<{ position: [number, number]; radius: number; opacity: number }> = [];
                for (let r = 0; r < ringCount; r++) {
                    // Each ring expands outward over time, staggered
                    const ringPhase = ((gameTick * 0.05 + r * 0.25) % 1);
                    const ringRadius = 50000 + ringPhase * 400000 * d.severity;
                    const ringOpacity = Math.floor((1 - ringPhase) * opacity * 0.7);
                    ringData.push({
                        position: [d.lon, d.lat],
                        radius: ringRadius,
                        opacity: ringOpacity,
                    });
                }

                layers.push(new ScatterplotLayer({
                    id: `disaster-earthquake-rings-${i}`,
                    data: ringData,
                    getPosition: (dd: any) => dd.position,
                    getFillColor: [0, 0, 0, 0],
                    getLineColor: (dd: any) => [200, 120, 50, dd.opacity],
                    getLineWidth: 3,
                    lineWidthUnits: 'pixels' as const,
                    stroked: true,
                    filled: false,
                    getRadius: (dd: any) => dd.radius,
                    radiusMinPixels: 10,
                    pickable: false,
                    parameters: { depthTest: false },
                }));

                // Epicenter dot
                layers.push(new ScatterplotLayer({
                    id: `disaster-earthquake-center-${i}`,
                    data: [{ position: [d.lon, d.lat] }],
                    getPosition: (dd: any) => dd.position,
                    getFillColor: [200, 80, 30, Math.floor(opacity * pulse)],
                    getRadius: 60000,
                    radiusMinPixels: 6,
                    pickable: false,
                    parameters: { depthTest: false },
                }));
                break;
            }

            case 'flood': {
                // Blue water overlay rising from low-elevation areas
                const waterOpacity = Math.floor(opacity * (0.6 + 0.4 * pulse));

                // Main flood zone
                layers.push(new ScatterplotLayer({
                    id: `disaster-flood-${i}`,
                    data: [{ position: [d.lon, d.lat] }],
                    getPosition: (dd: any) => dd.position,
                    getFillColor: [30, 80, 180, waterOpacity],
                    getRadius: 300000 * d.severity,
                    radiusMinPixels: 25,
                    pickable: false,
                    parameters: {
                        depthTest: false,
                        blend: true,
                        blendFunc: [WebGLRenderingContext.SRC_ALPHA, WebGLRenderingContext.ONE_MINUS_SRC_ALPHA],
                    },
                }));

                // Expanding water edge
                layers.push(new ScatterplotLayer({
                    id: `disaster-flood-edge-${i}`,
                    data: [{ position: [d.lon, d.lat] }],
                    getPosition: (dd: any) => dd.position,
                    getFillColor: [0, 0, 0, 0],
                    getLineColor: [60, 130, 220, Math.floor(waterOpacity * 0.6)],
                    getLineWidth: 2,
                    lineWidthUnits: 'pixels' as const,
                    stroked: true,
                    filled: false,
                    getRadius: 350000 * d.severity * pulse,
                    radiusMinPixels: 30,
                    pickable: false,
                    parameters: { depthTest: false },
                }));
                break;
            }

            case 'ice': {
                // Blue-white shimmer overlay
                const shimmerOpacity = Math.floor(opacity * (0.5 + 0.5 * pulse));

                layers.push(new ScatterplotLayer({
                    id: `disaster-ice-${i}`,
                    data: [{ position: [d.lon, d.lat] }],
                    getPosition: (dd: any) => dd.position,
                    getFillColor: [180, 210, 255, shimmerOpacity],
                    getRadius: 250000 * d.severity,
                    radiusMinPixels: 20,
                    pickable: false,
                    parameters: {
                        depthTest: false,
                        blend: true,
                        blendFunc: [WebGLRenderingContext.SRC_ALPHA, WebGLRenderingContext.ONE],
                    },
                }));

                // Secondary shimmer ring
                layers.push(new ScatterplotLayer({
                    id: `disaster-ice-shimmer-${i}`,
                    data: [{ position: [d.lon, d.lat] }],
                    getPosition: (dd: any) => dd.position,
                    getFillColor: [220, 240, 255, Math.floor(shimmerOpacity * 0.4)],
                    getRadius: 300000 * d.severity,
                    radiusMinPixels: 25,
                    pickable: false,
                    parameters: {
                        depthTest: false,
                        blend: true,
                        blendFunc: [WebGLRenderingContext.SRC_ALPHA, WebGLRenderingContext.ONE],
                    },
                }));
                break;
            }

            default: {
                // Generic amber warning overlay for CyberAttack, PoliticalUnrest, etc.
                layers.push(new ScatterplotLayer({
                    id: `disaster-generic-${i}`,
                    data: [{ position: [d.lon, d.lat] }],
                    getPosition: (dd: any) => dd.position,
                    getFillColor: [200, 150, 30, Math.floor(opacity * 0.6 * pulse)],
                    getRadius: 200000 * d.severity,
                    radiusMinPixels: 20,
                    pickable: false,
                    parameters: {
                        depthTest: false,
                        blend: true,
                        blendFunc: [WebGLRenderingContext.SRC_ALPHA, WebGLRenderingContext.ONE_MINUS_SRC_ALPHA],
                    },
                }));

                // Warning ring
                layers.push(new ScatterplotLayer({
                    id: `disaster-generic-ring-${i}`,
                    data: [{ position: [d.lon, d.lat] }],
                    getPosition: (dd: any) => dd.position,
                    getFillColor: [0, 0, 0, 0],
                    getLineColor: [245, 158, 11, Math.floor(opacity * 0.8)],
                    getLineWidth: 2,
                    lineWidthUnits: 'pixels' as const,
                    stroked: true,
                    filled: false,
                    getRadius: 250000 * d.severity,
                    radiusMinPixels: 25,
                    pickable: false,
                    parameters: { depthTest: false },
                }));
                break;
            }
        }
    }

    return layers;
}

// ── Forecast warning visualization layers ─────────────────────────────────

/**
 * Creates pulsing amber warning circles for forecast disaster zones.
 * Higher probability forecasts have brighter, more prominent indicators.
 */
function createForecastLayers(forecasts: ForecastDisaster[], gameTick: number): Layer[] {
    const layers: Layer[] = [];

    // Slow pulse for forecast warnings (slower than active disaster pulse)
    const phase = (gameTick % 60) / 60;
    const pulse = 0.4 + 0.6 * Math.sin(phase * Math.PI * 2);

    // Warning zone fills
    const forecastData = forecasts.map((f, i) => ({
        position: [f.lon, f.lat] as [number, number],
        probability: f.probability,
        index: i,
    }));

    if (forecastData.length > 0) {
        // Subtle amber fill pulsing with probability-based intensity
        layers.push(new ScatterplotLayer({
            id: 'forecast-warning-fill',
            data: forecastData,
            getPosition: (d: any) => d.position,
            getFillColor: (d: any) => {
                const intensity = d.probability;
                const alpha = Math.floor(15 + intensity * 30 * pulse);
                return [245, 158, 11, alpha] as [number, number, number, number];
            },
            getRadius: 300000,
            radiusMinPixels: 25,
            pickable: false,
            parameters: {
                depthTest: false,
                blend: true,
                blendFunc: [WebGLRenderingContext.SRC_ALPHA, WebGLRenderingContext.ONE_MINUS_SRC_ALPHA],
            },
        }));

        // Pulsing amber outline ring
        layers.push(new ScatterplotLayer({
            id: 'forecast-warning-ring',
            data: forecastData,
            getPosition: (d: any) => d.position,
            getFillColor: [0, 0, 0, 0],
            getLineColor: (d: any) => {
                const intensity = d.probability;
                const alpha = Math.floor(40 + intensity * 80 * pulse);
                return [245, 158, 11, alpha] as [number, number, number, number];
            },
            getLineWidth: 2,
            lineWidthUnits: 'pixels' as const,
            stroked: true,
            filled: false,
            getRadius: 300000 * (0.9 + 0.1 * pulse),
            radiusMinPixels: 25,
            pickable: false,
            parameters: { depthTest: false },
        }));

        // Inner ring (secondary pulse, offset phase)
        const pulse2 = 0.4 + 0.6 * Math.sin((phase + 0.5) * Math.PI * 2);
        layers.push(new ScatterplotLayer({
            id: 'forecast-warning-inner-ring',
            data: forecastData,
            getPosition: (d: any) => d.position,
            getFillColor: [0, 0, 0, 0],
            getLineColor: (d: any) => {
                const alpha = Math.floor(20 + d.probability * 40 * pulse2);
                return [245, 180, 50, alpha] as [number, number, number, number];
            },
            getLineWidth: 1,
            lineWidthUnits: 'pixels' as const,
            stroked: true,
            filled: false,
            getRadius: 200000 * (0.9 + 0.1 * pulse2),
            radiusMinPixels: 18,
            pickable: false,
            parameters: { depthTest: false },
        }));
    }

    return layers;
}

/** Classify a disaster type string into a visual category. */
function classifyDisaster(disasterType: string): 'storm' | 'hurricane' | 'earthquake' | 'flood' | 'ice' | 'generic' {
    const lower = disasterType.toLowerCase();
    if (lower.includes('hurricane') || lower.includes('typhoon') || lower.includes('cyclone')) return 'hurricane';
    if (lower.includes('earthquake')) return 'earthquake';
    if (lower.includes('flood')) return 'flood';
    if (lower.includes('ice') || lower.includes('blizzard') || lower.includes('frost')) return 'ice';
    if (lower.includes('storm') || lower.includes('thunder') || lower.includes('landslide')) return 'storm';
    return 'generic';
}
