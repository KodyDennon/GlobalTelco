// ── Procgen road network layer ──────────────────────────────────────────────
// Generates visible road connections between cities for procedurally generated
// worlds. Roads serve as fiber routing corridors and visual urban fabric.
//
// Highway connections: between cities in the same region (intra-region).
// Inter-region highways: between nearest cities of adjacent regions.
// Rendered as deck.gl PathLayer with grey coloring.
//
// Terrain-aware routing (Gap #27):
// When terrain data is available (GridCell[]), inter-city roads use A*
// pathfinding on the Voronoi cell grid to route around obstacles:
//   - Ocean cells: NEVER crossed (impassable)
//   - Mountain cells: cost multiplier x3 (roads wind around)
//   - Desert cells: slight cost increase
// Highways between major cities use lower terrain penalties (mostly straight).
// Secondary roads get higher penalties (wind through terrain).
// Falls back to straight lines if no terrain data is available.

import { PathLayer } from '@deck.gl/layers';
import type { Layer } from '@deck.gl/core';
import type { City, Region, GridCell } from '$lib/wasm/types';

// ── Types ──────────────────────────────────────────────────────────────────

interface RoadSegment {
    path: [number, number][];
    type: 'highway' | 'regional';
}

// ── Terrain cost functions for road routing ────────────────────────────────

const OCEAN_TERRAINS = new Set(['OceanShallow', 'OceanDeep', 'Ocean']);

/**
 * Road traversal cost for A* pathfinding.
 * @param terrain The terrain type string.
 * @param isHighway If true, use lower penalties (highways are more direct).
 */
function roadTraversalCost(terrain: string, isHighway: boolean): number {
    if (OCEAN_TERRAINS.has(terrain)) return Infinity; // NEVER cross water

    switch (terrain) {
        case 'Urban': return 1.0;
        case 'Suburban': return 1.0;
        case 'Rural': return 1.1;
        case 'Coastal': return 1.2;
        case 'Desert': return isHighway ? 1.3 : 1.8;
        case 'Tundra': return isHighway ? 1.5 : 2.5;
        case 'Frozen': return isHighway ? 2.0 : 3.0;
        case 'Mountainous': return isHighway ? 2.0 : 3.0;
        default: return 1.5;
    }
}

// ── A* pathfinding for terrain-aware roads ────────────────────────────────

/**
 * Haversine-approximate degree distance between two lat/lon points (for heuristic).
 */
function degDist(lat1: number, lon1: number, lat2: number, lon2: number): number {
    const dlat = lat1 - lat2;
    const dlon = lon1 - lon2;
    const cosLat = Math.cos(((lat1 + lat2) / 2) * Math.PI / 180);
    return Math.sqrt(dlat * dlat + dlon * dlon * cosLat * cosLat);
}

/**
 * Find the nearest cell index to a given (lon, lat) position.
 * Uses simple linear scan (cells array is typically 1000-5000 cells).
 */
function findNearestCell(lon: number, lat: number, cells: GridCell[]): number {
    let bestIdx = 0;
    let bestDist = Infinity;
    for (let i = 0; i < cells.length; i++) {
        const d = degDist(lat, lon, cells[i].lat, cells[i].lon);
        if (d < bestDist) {
            bestDist = d;
            bestIdx = i;
        }
    }
    return bestIdx;
}

/**
 * A* pathfinding on the cell grid from srcCell to dstCell.
 * Returns an array of [lon, lat] waypoints following cell centers.
 * If no path is found, returns a straight line fallback.
 */
function findRoadPath(
    srcCell: number,
    dstCell: number,
    cells: GridCell[],
    isHighway: boolean,
): [number, number][] {
    if (srcCell === dstCell) {
        const c = cells[srcCell];
        return c ? [[c.lon, c.lat]] : [];
    }

    const src = cells[srcCell];
    const dst = cells[dstCell];
    if (!src || !dst) return [[src?.lon ?? 0, src?.lat ?? 0], [dst?.lon ?? 0, dst?.lat ?? 0]];

    const cellCount = cells.length;
    const gScore = new Float64Array(cellCount).fill(Infinity);
    const fScore = new Float64Array(cellCount).fill(Infinity);
    const cameFrom = new Int32Array(cellCount).fill(-1);
    const closed = new Uint8Array(cellCount);

    // Open list (simple array -- fine for our grid sizes)
    const open: { cell: number; f: number }[] = [];
    gScore[srcCell] = 0;
    fScore[srcCell] = degDist(src.lat, src.lon, dst.lat, dst.lon);
    open.push({ cell: srcCell, f: fScore[srcCell] });

    let found = false;
    let iterations = 0;
    const maxIterations = Math.min(cellCount * 2, 15000);

    while (open.length > 0 && iterations < maxIterations) {
        iterations++;

        // Find min-f node
        let minIdx = 0;
        for (let i = 1; i < open.length; i++) {
            if (open[i].f < open[minIdx].f) minIdx = i;
        }
        const current = open[minIdx];
        open[minIdx] = open[open.length - 1];
        open.pop();

        if (current.cell === dstCell) {
            found = true;
            break;
        }

        if (closed[current.cell]) continue;
        closed[current.cell] = 1;

        const currentData = cells[current.cell];
        if (!currentData) continue;

        for (const neighborIdx of currentData.neighbors) {
            if (closed[neighborIdx]) continue;
            const neighbor = cells[neighborIdx];
            if (!neighbor) continue;

            const terrainCost = roadTraversalCost(neighbor.terrain, isHighway);
            if (!isFinite(terrainCost)) continue; // Impassable (ocean)

            const dist = degDist(currentData.lat, currentData.lon, neighbor.lat, neighbor.lon);
            const moveCost = dist * terrainCost;
            const tentativeG = gScore[current.cell] + moveCost;

            if (tentativeG < gScore[neighborIdx]) {
                gScore[neighborIdx] = tentativeG;
                cameFrom[neighborIdx] = current.cell;
                const h = degDist(neighbor.lat, neighbor.lon, dst.lat, dst.lon);
                fScore[neighborIdx] = tentativeG + h;
                open.push({ cell: neighborIdx, f: fScore[neighborIdx] });
            }
        }
    }

    // Reconstruct path
    if (found) {
        const path: number[] = [];
        let current = dstCell;
        while (current !== -1) {
            path.push(current);
            current = cameFrom[current];
        }
        path.reverse();

        // Simplify path (Douglas-Peucker)
        const simplified = simplifyPath(path, cells, 0.25);
        return simplified.map(i => [cells[i].lon, cells[i].lat] as [number, number]);
    }

    // No path found -- fall back to straight line
    return [[src.lon, src.lat], [dst.lon, dst.lat]];
}

/**
 * Douglas-Peucker simplification on a cell-index path.
 */
function simplifyPath(path: number[], cells: GridCell[], tolerance: number): number[] {
    if (path.length <= 3) return path;

    const points = path.map(i => ({ idx: i, lon: cells[i].lon, lat: cells[i].lat }));
    const keep = new Uint8Array(points.length);
    keep[0] = 1;
    keep[points.length - 1] = 1;

    dpSimplify(points, 0, points.length - 1, tolerance, keep);

    const result: number[] = [];
    for (let i = 0; i < points.length; i++) {
        if (keep[i]) result.push(points[i].idx);
    }
    return result;
}

function dpSimplify(
    points: Array<{ idx: number; lon: number; lat: number }>,
    start: number, end: number, tolerance: number,
    keep: Uint8Array,
): void {
    if (end - start < 2) return;

    let maxDist = 0;
    let maxIdx = start;

    const sx = points[start].lon, sy = points[start].lat;
    const ex = points[end].lon, ey = points[end].lat;
    const dx = ex - sx, dy = ey - sy;
    const lenSq = dx * dx + dy * dy;

    for (let i = start + 1; i < end; i++) {
        const px = points[i].lon - sx, py = points[i].lat - sy;
        let dist: number;
        if (lenSq === 0) {
            dist = Math.sqrt(px * px + py * py);
        } else {
            const t = Math.max(0, Math.min(1, (px * dx + py * dy) / lenSq));
            const projX = px - t * dx, projY = py - t * dy;
            dist = Math.sqrt(projX * projX + projY * projY);
        }
        if (dist > maxDist) {
            maxDist = dist;
            maxIdx = i;
        }
    }

    if (maxDist > tolerance) {
        keep[maxIdx] = 1;
        dpSimplify(points, start, maxIdx, tolerance, keep);
        dpSimplify(points, maxIdx, end, tolerance, keep);
    }
}

// ── Road generation ────────────────────────────────────────────────────────

/**
 * Generate road segments connecting cities within and between regions.
 * Deterministic: same cities + regions always produce the same roads.
 * When cells are provided, uses A* terrain-aware routing instead of straight lines.
 */
function generateRoads(cities: City[], regions: Region[], cells: GridCell[]): RoadSegment[] {
    if (cities.length === 0 || regions.length === 0) return [];

    const segments: RoadSegment[] = [];
    const hasTerrainData = cells.length > 0;

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

    /**
     * Create a road segment between two cities, using terrain routing if available.
     */
    function makeRoad(cityA: City, cityB: City, type: 'highway' | 'regional'): RoadSegment {
        if (hasTerrainData) {
            const srcCell = findNearestCell(cityA.x, cityA.y, cells);
            const dstCell = findNearestCell(cityB.x, cityB.y, cells);
            const isHighway = type === 'highway';
            const path = findRoadPath(srcCell, dstCell, cells, isHighway);

            // Ensure road starts and ends at exact city positions
            if (path.length > 0) {
                path[0] = [cityA.x, cityA.y];
                path[path.length - 1] = [cityB.x, cityB.y];
            }

            return { path, type };
        }
        return {
            path: [[cityA.x, cityA.y], [cityB.x, cityB.y]],
            type,
        };
    }

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
                    segments.push(makeRoad(bestFrom, bestTo, 'highway'));
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
                    segments.push(makeRoad(largeCities[i], largeCities[j], 'highway'));
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
                    segments.push(makeRoad(bestCityA, bestCityB, 'regional'));
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
 * @param cells Optional grid cells for terrain-aware routing. If empty, roads are straight lines.
 */
export function buildRoadData(cities: City[], regions: Region[], cells: GridCell[] = []): void {
    cachedRoads = generateRoads(cities, regions, cells);
}

/** Dispose cached road data to free memory. */
export function disposeRoadData(): void {
    cachedRoads = null;
}

// ── Layer creation ─────────────────────────────────────────────────────────

/**
 * Create road network layers for Procgen mode.
 * Returns deck.gl PathLayers for highways and regional roads.
 * For Real Earth mode, roads come from MapLibre vector tiles -- return empty.
 */
export function createRoadsLayers(
    isRealEarth: boolean,
    currentZoom: number,
): Layer[] {
    if (isRealEarth) return [];
    if (!cachedRoads || cachedRoads.length === 0) return [];

    const layers: Layer[] = [];

    // Highway roads -- visible at zoom 3+
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

    // Regional roads -- visible at zoom 5+
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
