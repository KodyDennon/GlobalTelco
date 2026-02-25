// ── Procgen road network layer ──────────────────────────────────────────────
// Generates visible road connections between cities for procedurally generated
// worlds. Roads serve as fiber routing corridors and visual urban fabric.
//
// Highway connections: between cities in the same region (intra-region).
// Inter-region highways: between nearest cities of adjacent regions.
// Rendered as deck.gl PathLayer with grey coloring.

import { PathLayer } from '@deck.gl/layers';
import type { Layer } from '@deck.gl/core';
import type { City, Region } from '$lib/wasm/types';

// ── Types ──────────────────────────────────────────────────────────────────

interface RoadSegment {
    path: [number, number][];
    type: 'highway' | 'regional';
}

// ── Road generation ────────────────────────────────────────────────────────

/**
 * Generate road segments connecting cities within and between regions.
 * Deterministic: same cities + regions always produce the same roads.
 */
function generateRoads(cities: City[], regions: Region[]): RoadSegment[] {
    if (cities.length === 0 || regions.length === 0) return [];

    const segments: RoadSegment[] = [];

    // Build region -> cities lookup
    const regionCities = new Map<number, City[]>();
    for (const city of cities) {
        const list = regionCities.get(city.region_id) ?? [];
        list.push(city);
        regionCities.set(city.region_id, list);
    }

    // Deduplicate edges: track connection keys to avoid duplicate road segments
    const connected = new Set<string>();
    const edgeKey = (a: number, b: number): string => {
        const lo = Math.min(a, b);
        const hi = Math.max(a, b);
        return `${lo}-${hi}`;
    };

    // ── Intra-region highways: connect all cities within the same region ──
    // Use a minimum spanning tree approach (Prim's) to avoid O(n^2) roads
    // for regions with many cities, then add extra edges for large cities.
    for (const [, rCities] of regionCities) {
        if (rCities.length < 2) continue;

        // Sort by population descending for deterministic processing
        const sorted = [...rCities].sort((a, b) => b.population - a.population);

        // Prim's MST: start from the largest city
        const inTree = new Set<number>();
        inTree.add(sorted[0].id);
        const remaining = new Set(sorted.slice(1).map(c => c.id));
        const cityById = new Map(sorted.map(c => [c.id, c]));

        while (remaining.size > 0) {
            let bestDist = Infinity;
            let bestFrom: City | null = null;
            let bestTo: City | null = null;

            for (const inId of inTree) {
                const fromCity = cityById.get(inId)!;
                for (const outId of remaining) {
                    const toCity = cityById.get(outId)!;
                    const dx = fromCity.x - toCity.x;
                    const dy = fromCity.y - toCity.y;
                    const dist = dx * dx + dy * dy;
                    if (dist < bestDist) {
                        bestDist = dist;
                        bestFrom = fromCity;
                        bestTo = toCity;
                    }
                }
            }

            if (bestFrom && bestTo) {
                const key = edgeKey(bestFrom.id, bestTo.id);
                if (!connected.has(key)) {
                    connected.add(key);
                    segments.push({
                        path: [[bestFrom.x, bestFrom.y], [bestTo.x, bestTo.y]],
                        type: 'highway',
                    });
                }
                inTree.add(bestTo.id);
                remaining.delete(bestTo.id);
            } else {
                break;
            }
        }

        // Extra connections: connect large cities (pop > 250k) to each other
        // if they are not already connected. Limits to 3 extra connections
        // per city to avoid visual clutter.
        const largeCities = sorted.filter(c => c.population > 250000);
        for (let i = 0; i < largeCities.length; i++) {
            let extraCount = 0;
            for (let j = i + 1; j < largeCities.length && extraCount < 3; j++) {
                const key = edgeKey(largeCities[i].id, largeCities[j].id);
                if (!connected.has(key)) {
                    connected.add(key);
                    segments.push({
                        path: [[largeCities[i].x, largeCities[i].y], [largeCities[j].x, largeCities[j].y]],
                        type: 'highway',
                    });
                    extraCount++;
                }
            }
        }
    }

    // ── Inter-region highways: connect nearest city pairs between regions ──
    // For each region, find the nearest city in every other region and
    // connect to the closest 2-3 neighbors (to avoid excessive connections).
    const regionIds = [...regionCities.keys()];
    for (let i = 0; i < regionIds.length; i++) {
        const rIdA = regionIds[i];
        const citiesA = regionCities.get(rIdA)!;
        // Find the largest city in this region as the representative hub
        const hubA = citiesA.reduce((best, c) => c.population > best.population ? c : best, citiesA[0]);

        // Compute distances to all other regions' hubs
        const otherRegions: { regionId: number; hub: City; dist: number }[] = [];
        for (let j = 0; j < regionIds.length; j++) {
            if (i === j) continue;
            const rIdB = regionIds[j];
            const citiesB = regionCities.get(rIdB)!;
            const hubB = citiesB.reduce((best, c) => c.population > best.population ? c : best, citiesB[0]);
            const dx = hubA.x - hubB.x;
            const dy = hubA.y - hubB.y;
            otherRegions.push({ regionId: rIdB, hub: hubB, dist: dx * dx + dy * dy });
        }

        // Connect to the 3 nearest region hubs
        otherRegions.sort((a, b) => a.dist - b.dist);
        const maxLinks = Math.min(3, otherRegions.length);
        for (let k = 0; k < maxLinks; k++) {
            const { hub: hubB } = otherRegions[k];
            // Find the closest city pair between the two regions
            let bestDist = Infinity;
            let bestCityA: City | null = null;
            let bestCityB: City | null = null;

            const citiesBInRegion = regionCities.get(otherRegions[k].regionId)!;
            for (const cA of citiesA) {
                for (const cB of citiesBInRegion) {
                    const dx = cA.x - cB.x;
                    const dy = cA.y - cB.y;
                    const dist = dx * dx + dy * dy;
                    if (dist < bestDist) {
                        bestDist = dist;
                        bestCityA = cA;
                        bestCityB = cB;
                    }
                }
            }

            if (bestCityA && bestCityB) {
                const key = edgeKey(bestCityA.id, bestCityB.id);
                if (!connected.has(key)) {
                    connected.add(key);
                    segments.push({
                        path: [[bestCityA.x, bestCityA.y], [bestCityB.x, bestCityB.y]],
                        type: 'regional',
                    });
                }
            }
        }
    }

    return segments;
}

// ── Cached data ────────────────────────────────────────────────────────────

let cachedRoads: RoadSegment[] | null = null;

/**
 * Build and cache road network data. Call once after cities/regions are loaded.
 */
export function buildRoadData(cities: City[], regions: Region[]): void {
    cachedRoads = generateRoads(cities, regions);
}

/** Dispose cached road data to free memory. */
export function disposeRoadData(): void {
    cachedRoads = null;
}

// ── Layer creation ─────────────────────────────────────────────────────────

/**
 * Create road network layers for Procgen mode.
 * Returns deck.gl PathLayers for highways and regional roads.
 * For Real Earth mode, roads come from MapLibre vector tiles — return empty.
 */
export function createRoadsLayers(
    isRealEarth: boolean,
    currentZoom: number,
): Layer[] {
    if (isRealEarth) return [];
    if (!cachedRoads || cachedRoads.length === 0) return [];

    const layers: Layer[] = [];

    // Highway roads — visible at zoom 3+
    if (currentZoom >= 3) {
        const highways = cachedRoads.filter(r => r.type === 'highway');
        if (highways.length > 0) {
            layers.push(new PathLayer({
                id: 'roads-highway',
                data: highways,
                getPath: (d: RoadSegment) => d.path,
                getColor: [74, 85, 104, 140],
                getWidth: 3,
                widthUnits: 'pixels',
                widthMinPixels: 1,
                widthMaxPixels: 5,
                pickable: false,
                parameters: { depthTest: false },
                capRounded: true,
                jointRounded: true,
            }));
        }
    }

    // Regional roads — visible at zoom 5+
    if (currentZoom >= 5) {
        const regional = cachedRoads.filter(r => r.type === 'regional');
        if (regional.length > 0) {
            layers.push(new PathLayer({
                id: 'roads-regional',
                data: regional,
                getPath: (d: RoadSegment) => d.path,
                getColor: [113, 128, 150, 110],
                getWidth: 2,
                widthUnits: 'pixels',
                widthMinPixels: 1,
                widthMaxPixels: 3,
                pickable: false,
                parameters: { depthTest: false },
                capRounded: true,
                jointRounded: true,
            }));
        }
    }

    return layers;
}
