// ── Procedural building footprint layer ─────────────────────────────────────
// Generates building footprints (small rectangle polygons) around city centers
// for both Real Earth and Procgen modes. Buildings serve as demand point
// visualization and urban fabric.
//
// Building generation is deterministic: seeded from city position so the same
// city always produces the same building layout.
//
// Population tiers determine building count and ring radii:
//   Hamlet (<50k): 20-50 buildings
//   Town (50k-250k): 50-200 buildings
//   City (250k-1M): 200-500 buildings
//   Metropolis (1M-5M): 500-1000 buildings
//   Megalopolis (5M+): 1000-2000 buildings
//
// Buildings are arranged in concentric rings:
//   Core: Dense large rectangles (commercial)
//   Inner: Medium rectangles (mixed use)
//   Outer: Small rectangles (residential, spaced)

import { PolygonLayer } from '@deck.gl/layers';
import type { Layer } from '@deck.gl/core';
import type { City } from '$lib/wasm/types';

// ── Types ──────────────────────────────────────────────────────────────────

interface BuildingFootprint {
    polygon: [number, number][];
    color: [number, number, number, number];
}

// ── Constants ──────────────────────────────────────────────────────────────

/** Maximum total buildings rendered at once (performance budget). */
const MAX_VISIBLE_BUILDINGS = 5000;

/** Building zone ring definitions. */
interface ZoneConfig {
    /** Fraction of total buildings in this zone. */
    fraction: number;
    /** Building width range in degrees (approximate meters at equator). */
    minSizeDeg: number;
    maxSizeDeg: number;
    /** Building height/width aspect ratio range. */
    minAspect: number;
    maxAspect: number;
    /** Base color RGBA. */
    color: [number, number, number, number];
}

const ZONE_CORE: ZoneConfig = {
    fraction: 0.2,
    minSizeDeg: 0.00027,  // ~30m
    maxSizeDeg: 0.00054,  // ~60m
    minAspect: 0.6,
    maxAspect: 1.4,
    color: [32, 36, 48, 200],
};

const ZONE_INNER: ZoneConfig = {
    fraction: 0.35,
    minSizeDeg: 0.000135, // ~15m
    maxSizeDeg: 0.00027,  // ~30m
    minAspect: 0.5,
    maxAspect: 1.6,
    color: [28, 32, 44, 190],
};

const ZONE_OUTER: ZoneConfig = {
    fraction: 0.45,
    minSizeDeg: 0.000072, // ~8m
    maxSizeDeg: 0.000135, // ~15m
    minAspect: 0.7,
    maxAspect: 1.3,
    color: [24, 28, 40, 180],
};

const ZONES = [ZONE_CORE, ZONE_INNER, ZONE_OUTER];

// ── Seeded PRNG (deterministic per city) ──────────────────────────────────

/**
 * Simple mulberry32 PRNG seeded from a 32-bit integer.
 * Returns a function that produces the next pseudorandom float in [0, 1).
 */
function mulberry32(seed: number): () => number {
    let state = seed | 0;
    return () => {
        state = (state + 0x6D2B79F5) | 0;
        let t = Math.imul(state ^ (state >>> 15), 1 | state);
        t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
        return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
    };
}

/**
 * Create a deterministic seed from a city's position.
 * Combines lon and lat into a single 32-bit integer.
 */
function citySeed(city: City): number {
    // Mix lat/lon bits for a good spread
    const lonBits = Math.floor((city.x + 180) * 10000);
    const latBits = Math.floor((city.y + 90) * 10000);
    return (lonBits * 2654435761 + latBits * 340573321) | 0;
}

// ── Building count by population tier ─────────────────────────────────────

function buildingCount(population: number): number {
    if (population >= 5000000) return 1000 + Math.min(1000, Math.floor(population / 10000000 * 1000));
    if (population >= 1000000) return 500 + Math.floor((population - 1000000) / 4000000 * 500);
    if (population >= 250000) return 200 + Math.floor((population - 250000) / 750000 * 300);
    if (population >= 50000) return 50 + Math.floor((population - 50000) / 200000 * 150);
    return 20 + Math.floor(population / 50000 * 30);
}

// ── Ring radii (in degrees) by population tier ───────────────────────────

interface CityRings {
    coreRadius: number;
    innerRadius: number;
    outerRadius: number;
}

/**
 * Compute ring radii for a city based on population.
 * Returns radii in degrees longitude (approximate, not lat-corrected).
 */
function cityRings(population: number): CityRings {
    // Base radii in degrees (at equator: 1 deg ~ 111km, 0.001 deg ~ 111m)
    // We use much smaller radii — these are for building placement, not coverage.
    // Scale: 0.5km radius = 0.0045 deg for a small city
    if (population >= 5000000) {
        return { coreRadius: 0.018, innerRadius: 0.035, outerRadius: 0.055 };
    }
    if (population >= 1000000) {
        return { coreRadius: 0.012, innerRadius: 0.025, outerRadius: 0.04 };
    }
    if (population >= 250000) {
        return { coreRadius: 0.008, innerRadius: 0.018, outerRadius: 0.03 };
    }
    if (population >= 50000) {
        return { coreRadius: 0.005, innerRadius: 0.012, outerRadius: 0.02 };
    }
    // Hamlet
    return { coreRadius: 0.003, innerRadius: 0.007, outerRadius: 0.012 };
}

// ── Building generation ─────────────────────────────────────────────────

/**
 * Generate a single rectangular building footprint polygon.
 * The rectangle is positioned at (cx, cy) with given width/height in degrees,
 * rotated by `angle` radians.
 */
function makeRect(
    cx: number,
    cy: number,
    halfW: number,
    halfH: number,
    angle: number,
): [number, number][] {
    const cos = Math.cos(angle);
    const sin = Math.sin(angle);

    // 4 corners of the rectangle, rotated
    const corners: [number, number][] = [
        [cx + (-halfW * cos - -halfH * sin), cy + (-halfW * sin + -halfH * cos)],
        [cx + (halfW * cos - -halfH * sin), cy + (halfW * sin + -halfH * cos)],
        [cx + (halfW * cos - halfH * sin), cy + (halfW * sin + halfH * cos)],
        [cx + (-halfW * cos - halfH * sin), cy + (-halfW * sin + halfH * cos)],
    ];
    // Close the ring
    corners.push(corners[0]);
    return corners;
}

/**
 * Generate all building footprints for a single city.
 * Returns an array of BuildingFootprint objects.
 */
function generateCityBuildings(city: City): BuildingFootprint[] {
    const count = buildingCount(city.population);
    const rings = cityRings(city.population);
    const rng = mulberry32(citySeed(city));

    const buildings: BuildingFootprint[] = [];

    // Latitude correction factor: buildings at higher latitudes need wider
    // longitude span to appear the same size on screen
    const latCorrectionFactor = 1 / Math.max(0.1, Math.cos((city.y * Math.PI) / 180));

    const zoneRadii = [
        { zone: ZONES[0], innerR: 0, outerR: rings.coreRadius },
        { zone: ZONES[1], innerR: rings.coreRadius, outerR: rings.innerRadius },
        { zone: ZONES[2], innerR: rings.innerRadius, outerR: rings.outerRadius },
    ];

    for (const { zone, innerR, outerR } of zoneRadii) {
        const zoneCount = Math.floor(count * zone.fraction);

        for (let i = 0; i < zoneCount; i++) {
            // Random position within the ring (polar coordinates)
            const r = innerR + rng() * (outerR - innerR);
            const theta = rng() * Math.PI * 2;

            const bx = city.x + r * Math.cos(theta) * latCorrectionFactor;
            const by = city.y + r * Math.sin(theta);

            // Random building size within zone constraints
            const sizeW = zone.minSizeDeg + rng() * (zone.maxSizeDeg - zone.minSizeDeg);
            const aspect = zone.minAspect + rng() * (zone.maxAspect - zone.minAspect);
            const sizeH = sizeW * aspect;

            // Apply lat correction to width
            const halfW = (sizeW * latCorrectionFactor) / 2;
            const halfH = sizeH / 2;

            // Random rotation (slight, mostly grid-aligned with some randomness)
            // Buildings tend to align to a grid, so quantize to ~90 degree increments
            // with small random offset
            const baseAngle = Math.floor(rng() * 4) * (Math.PI / 2);
            const angleJitter = (rng() - 0.5) * 0.3; // +/- ~17 degrees
            const angle = baseAngle + angleJitter;

            // Color variation: slight per-building tint
            const colorVariation = Math.floor((rng() - 0.5) * 12);
            const color: [number, number, number, number] = [
                Math.max(0, Math.min(255, zone.color[0] + colorVariation)),
                Math.max(0, Math.min(255, zone.color[1] + colorVariation)),
                Math.max(0, Math.min(255, zone.color[2] + colorVariation)),
                zone.color[3],
            ];

            buildings.push({
                polygon: makeRect(bx, by, halfW, halfH, angle),
                color,
            });
        }
    }

    return buildings;
}

// ── Cached data ────────────────────────────────────────────────────────────

let cachedBuildings: BuildingFootprint[] | null = null;
let cachedShadows: BuildingFootprint[] | null = null;
let cachedCityCount: number = 0;

/**
 * Build and cache building footprint data from city array.
 * Call once after cities are loaded from WASM.
 *
 * Enforces the MAX_VISIBLE_BUILDINGS budget by generating buildings for
 * the largest cities first and stopping when the budget is reached.
 */
export function buildBuildingData(cities: City[]): void {
    if (cities.length === 0) {
        cachedBuildings = [];
        cachedShadows = [];
        cachedCityCount = 0;
        return;
    }

    // Process cities from largest to smallest so we prioritize big cities
    const sorted = [...cities]
        .filter(c => Math.abs(c.y) <= 85)
        .sort((a, b) => b.population - a.population);

    const allBuildings: BuildingFootprint[] = [];
    const allShadows: BuildingFootprint[] = [];

    for (const city of sorted) {
        if (allBuildings.length >= MAX_VISIBLE_BUILDINGS) break;

        const cityBuildings = generateCityBuildings(city);
        const remaining = MAX_VISIBLE_BUILDINGS - allBuildings.length;
        const toAdd = cityBuildings.slice(0, remaining);

        for (const b of toAdd) {
            allBuildings.push(b);

            // Generate shadow: offset polygon slightly SE and darker
            const shadowOffset = 0.00003; // ~3m offset
            const latCorr = 1 / Math.max(0.1, Math.cos((city.y * Math.PI) / 180));
            const shadowPoly = b.polygon.map(([lon, lat]) => [
                lon + shadowOffset * latCorr,
                lat - shadowOffset,
            ] as [number, number]);

            allShadows.push({
                polygon: shadowPoly,
                color: [8, 10, 16, 120],
            });
        }
    }

    cachedBuildings = allBuildings;
    cachedShadows = allShadows;
    cachedCityCount = cities.length;
}

/** Dispose cached building data to free memory. */
export function disposeBuildingData(): void {
    cachedBuildings = null;
    cachedShadows = null;
    cachedCityCount = 0;
}

// ── Layer creation ─────────────────────────────────────────────────────────

/**
 * Create building footprint layers.
 * Returns deck.gl PolygonLayers for shadows (underneath) and building fills.
 * Visible at zoom 7+.
 */
export function createBuildingsLayers(currentZoom: number): Layer[] {
    if (currentZoom < 7) return [];
    if (!cachedBuildings || cachedBuildings.length === 0) return [];

    const layers: Layer[] = [];

    // 1. Shadow layer (offset darker polygons underneath buildings)
    if (cachedShadows && cachedShadows.length > 0) {
        layers.push(new PolygonLayer({
            id: 'buildings-shadows',
            data: cachedShadows,
            getPolygon: (d: BuildingFootprint) => d.polygon,
            getFillColor: (d: BuildingFootprint) => d.color,
            stroked: false,
            filled: true,
            pickable: false,
            parameters: { depthTest: false },
            updateTriggers: {
                getFillColor: [cachedCityCount],
            },
        }));
    }

    // 2. Building fill layer
    layers.push(new PolygonLayer({
        id: 'buildings-fill',
        data: cachedBuildings,
        getPolygon: (d: BuildingFootprint) => d.polygon,
        getFillColor: (d: BuildingFootprint) => d.color,
        stroked: false,
        filled: true,
        pickable: false,
        parameters: { depthTest: false },
        updateTriggers: {
            getFillColor: [cachedCityCount],
        },
    }));

    return layers;
}
