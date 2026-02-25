// ── Procgen road network layer ──────────────────────────────────────────────
// Generates visible road connections between cities for procedurally generated
// worlds. Roads serve as fiber routing corridors and visual urban fabric.
//
// Highway connections: between cities in the same region (intra-region).
// Inter-region highways: between nearest cities of adjacent regions.
// Rendered as deck.gl PathLayer with grey coloring.
//
// Intra-city streets (Phase 3.2.2):
// Each city generates a local street network based on layout style:
//   Grid (American): rectangular block grid, main avenues wider
//   Radial (European): central plaza, ring roads, radial avenues
//   Organic (Asian/Old-World): irregular winding streets, narrow alleys
//   Mixed: historic organic core, newer outer areas grid
// Layout style is deterministic: region_id % 4.
// Street density scales with population tier (hamlet/town/city/metropolis).
// Streets render as PathLayer with thinner lines than highways, at zoom 7+.
//
// Bridge markers (Phase 3.2.4):
// Where roads cross rivers, a different color/width marks the bridge.
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
import { getCachedRiverPaths } from './riversLayer';

// ── Types ──────────────────────────────────────────────────────────────────

interface RoadSegment {
    path: [number, number][];
    type: 'highway' | 'regional';
}

/** Intra-city street segment with multi-point path and width. */
export interface CityStreet {
    path: [number, number][];
    type: 'avenue' | 'main' | 'residential' | 'alley';
    width: number;
}

/** Bridge marker where a road or street crosses a river. */
interface BridgeMarker {
    path: [number, number][];
}

/** City layout style, determined by region_id % 4. */
type CityLayoutStyle = 'grid' | 'radial' | 'organic' | 'mixed';

/** Population tier for street density scaling. */
type StreetPopTier = 'hamlet' | 'town' | 'city' | 'metropolis';

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

// ── Intra-city street generation (Phase 3.2.2) ─────────────────────────────

/** Degrees per meter at equator (approximate). 1 degree ~ 111,320m */
const DEG_PER_METER = 1 / 111_320;

/** Maximum total city street segments across all cities (performance budget). */
const MAX_CITY_STREETS = 8000;

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
 */
function streetSeed(city: City): number {
    const lonBits = Math.floor((city.x + 180) * 10000);
    const latBits = Math.floor((city.y + 90) * 10000);
    return (lonBits * 2654435761 + latBits * 340573321 + 12345) | 0;
}

/**
 * Determine city layout style from region_id (deterministic hash).
 */
function getLayoutStyle(regionId: number): CityLayoutStyle {
    const styles: CityLayoutStyle[] = ['grid', 'radial', 'organic', 'mixed'];
    return styles[((regionId % 4) + 4) % 4];
}

/**
 * Determine street population tier from population count.
 */
function getStreetPopTier(population: number): StreetPopTier {
    if (population >= 1_000_000) return 'metropolis';
    if (population >= 250_000) return 'city';
    if (population >= 50_000) return 'town';
    return 'hamlet';
}

/**
 * Compute the city radius in degrees based on population.
 * Larger cities have wider street networks.
 */
function cityRadiusDeg(population: number): number {
    if (population >= 5_000_000) return 0.28;
    if (population >= 1_000_000) return 0.18;
    if (population >= 250_000) return 0.10;
    if (population >= 50_000) return 0.05;
    return 0.025;
}

/**
 * Generate intra-city streets for a single city based on its layout style and
 * population tier. Returns an array of CityStreet segments.
 *
 * Street density scales with population:
 *   Hamlet (<50k): main road + a few side streets
 *   Town (50k-250k): basic grid/radial core + residential streets
 *   City (250k-1M): full street network, commercial district
 *   Metropolis (1M+): dense multi-center network
 */
export function generateCityStreets(city: City, layoutStyle: CityLayoutStyle): CityStreet[] {
    const tier = getStreetPopTier(city.population);
    const rng = mulberry32(streetSeed(city));
    const latCorr = 1 / Math.max(0.1, Math.cos((city.y * Math.PI) / 180));
    const radius = cityRadiusDeg(city.population);

    const streets: CityStreet[] = [];

    switch (layoutStyle) {
        case 'grid':
            generateGridStreets(city, rng, latCorr, radius, tier, streets);
            break;
        case 'radial':
            generateRadialStreets(city, rng, latCorr, radius, tier, streets);
            break;
        case 'organic':
            generateOrganicStreets(city, rng, latCorr, radius, tier, streets);
            break;
        case 'mixed': {
            // Organic inner core (~40% of radius), grid outer area
            const innerR = radius * 0.4;
            generateOrganicStreets(city, rng, latCorr, innerR, tier, streets);
            generateGridStreets(city, rng, latCorr, radius, tier, streets, innerR);
            break;
        }
    }

    return streets;
}

/**
 * Grid (American) layout: rectangular block grid with main avenues wider.
 * Grid is rotated by a seed-based angle for visual variety.
 * @param skipInnerR If > 0, skip streets within this radius (for mixed layout).
 */
function generateGridStreets(
    city: City,
    rng: () => number,
    latCorr: number,
    radius: number,
    tier: StreetPopTier,
    out: CityStreet[],
    skipInnerR: number = 0,
): void {
    const gridAngle = (rng() - 0.5) * 0.4; // +/- ~23 degrees rotation
    const cos = Math.cos(gridAngle);
    const sin = Math.sin(gridAngle);

    // Spacing in degrees
    const avenueSpacing = 500 * DEG_PER_METER;  // ~500m between avenues
    const mainSpacing = 250 * DEG_PER_METER;     // ~250m between main streets
    const resSpacing = 120 * DEG_PER_METER;      // ~120m between residential streets

    // Grid extent based on tier
    let extent = radius * 0.6;
    if (tier === 'hamlet') extent = radius * 0.25;
    else if (tier === 'town') extent = radius * 0.4;
    else if (tier === 'metropolis') extent = radius * 0.8;

    // Helper: rotate and translate a local point to world coordinates
    const toWorld = (lx: number, ly: number): [number, number] => [
        city.x + (lx * cos - ly * sin) * latCorr,
        city.y + (lx * sin + ly * cos),
    ];

    // Helper: check if a point is within the skip radius
    const inSkipZone = (lx: number, ly: number): boolean => {
        if (skipInnerR <= 0) return false;
        return Math.sqrt(lx * lx + ly * ly) < skipInnerR;
    };

    // E-W avenues (type: avenue, widest)
    const numAvenues = Math.floor((extent * 2) / avenueSpacing);
    for (let i = 0; i <= numAvenues; i++) {
        const yOff = -extent + i * avenueSpacing;
        if (inSkipZone(0, yOff)) continue;
        const from = toWorld(-extent, yOff);
        const to = toWorld(extent, yOff);
        out.push({ path: [from, to], type: 'avenue', width: 3 });
    }

    // N-S main streets
    const numMains = Math.floor((extent * 2) / mainSpacing);
    const mainStep = tier === 'hamlet' ? Math.max(1, Math.floor(numMains / 3)) :
                     tier === 'town' ? Math.max(1, Math.floor(numMains / 8)) : 1;
    for (let i = 0; i <= numMains; i += mainStep) {
        const xOff = -extent + i * mainSpacing;
        if (inSkipZone(xOff, 0)) continue;
        const from = toWorld(xOff, -extent);
        const to = toWorld(xOff, extent);
        out.push({ path: [from, to], type: 'main', width: 2 });
    }

    // Residential cross-streets (city and metropolis only)
    if (tier === 'city' || tier === 'metropolis') {
        const numRes = Math.floor((extent * 2) / resSpacing);
        // Limit for performance: every 2nd for city, every 1st for metropolis
        const resStep = tier === 'city' ? 2 : 1;
        for (let i = 0; i <= numRes; i += resStep) {
            const yOff = -extent + i * resSpacing;
            if (inSkipZone(0, yOff)) continue;
            // Only draw if not overlapping an avenue
            const nearAvenue = Math.abs(yOff % avenueSpacing) < resSpacing * 0.3;
            if (nearAvenue) continue;
            const from = toWorld(-extent, yOff);
            const to = toWorld(extent, yOff);
            out.push({ path: [from, to], type: 'residential', width: 1 });
        }
    }

    // Alley network (metropolis only, between main streets in downtown area)
    if (tier === 'metropolis') {
        const downtownR = extent * 0.3;
        const alleySpacing = 80 * DEG_PER_METER; // ~80m
        const numAlleys = Math.floor((downtownR * 2) / alleySpacing);
        for (let i = 0; i <= numAlleys; i++) {
            const xOff = -downtownR + i * alleySpacing;
            const from = toWorld(xOff, -downtownR);
            const to = toWorld(xOff, downtownR);
            out.push({ path: [from, to], type: 'alley', width: 0.5 });
        }
    }
}

/**
 * Radial (European) layout: central plaza, ring roads at increasing radius,
 * radial avenues from center to edge.
 */
function generateRadialStreets(
    city: City,
    rng: () => number,
    latCorr: number,
    radius: number,
    tier: StreetPopTier,
    out: CityStreet[],
): void {
    const ringSpacing = 300 * DEG_PER_METER; // ~300m between ring roads

    let maxRadius = radius * 0.6;
    let numRadials = 8;
    if (tier === 'hamlet') { maxRadius = radius * 0.2; numRadials = 4; }
    else if (tier === 'town') { maxRadius = radius * 0.35; numRadials = 6; }
    else if (tier === 'metropolis') { maxRadius = radius * 0.8; numRadials = 12; }

    const numRings = Math.max(1, Math.floor(maxRadius / ringSpacing));
    const radialBaseAngle = rng() * Math.PI * 2;
    const radialAngleStep = (Math.PI * 2) / numRadials;

    // Ring roads -- approximated as multi-point paths (smoother arcs)
    const arcSegments = Math.max(20, numRadials * 3);
    for (let ring = 1; ring <= numRings; ring++) {
        const r = ring * ringSpacing;
        const ringType = ring <= 2 ? 'avenue' as const : ring <= numRings * 0.6 ? 'main' as const : 'residential' as const;
        const ringWidth = ring <= 2 ? 2.5 : ring <= numRings * 0.6 ? 1.5 : 1;
        const angleStep = (Math.PI * 2) / arcSegments;

        // Build the ring as a single multi-point path for smoother rendering
        const ringPath: [number, number][] = [];
        for (let seg = 0; seg <= arcSegments; seg++) {
            const a = seg * angleStep;
            ringPath.push([
                city.x + r * Math.cos(a) * latCorr,
                city.y + r * Math.sin(a),
            ]);
        }
        out.push({ path: ringPath, type: ringType, width: ringWidth });
    }

    // Radial avenues from plaza to edge
    for (let i = 0; i < numRadials; i++) {
        const angle = radialBaseAngle + i * radialAngleStep;
        const startR = ringSpacing * 0.3; // start from plaza edge
        const from: [number, number] = [
            city.x + startR * Math.cos(angle) * latCorr,
            city.y + startR * Math.sin(angle),
        ];
        const to: [number, number] = [
            city.x + maxRadius * Math.cos(angle) * latCorr,
            city.y + maxRadius * Math.sin(angle),
        ];
        out.push({ path: [from, to], type: 'avenue', width: 2.5 });
    }

    // Residential streets between radials (city and metropolis)
    if (tier === 'city' || tier === 'metropolis') {
        const halfAngle = radialAngleStep / 2;
        for (let i = 0; i < numRadials; i++) {
            const midAngle = radialBaseAngle + i * radialAngleStep + halfAngle;
            const startR = ringSpacing * 2;
            if (startR > maxRadius) continue;
            const from: [number, number] = [
                city.x + startR * Math.cos(midAngle) * latCorr,
                city.y + startR * Math.sin(midAngle),
            ];
            const to: [number, number] = [
                city.x + maxRadius * Math.cos(midAngle) * latCorr,
                city.y + maxRadius * Math.sin(midAngle),
            ];
            out.push({ path: [from, to], type: 'residential', width: 1 });
        }
    }

    // Alley network in the very center (metropolis only)
    if (tier === 'metropolis') {
        const plazaR = ringSpacing * 1.5;
        const numAlleyRadials = 6;
        for (let i = 0; i < numAlleyRadials; i++) {
            const angle = radialBaseAngle + (i / numAlleyRadials) * Math.PI * 2 + 0.3;
            const from: [number, number] = [
                city.x + plazaR * 0.2 * Math.cos(angle) * latCorr,
                city.y + plazaR * 0.2 * Math.sin(angle),
            ];
            const to: [number, number] = [
                city.x + plazaR * Math.cos(angle) * latCorr,
                city.y + plazaR * Math.sin(angle),
            ];
            out.push({ path: [from, to], type: 'alley', width: 0.5 });
        }
    }
}

/**
 * Organic (Asian/Old-World) layout: irregular winding streets using random walks.
 * Main roads meander from center outward; branches create narrow alleys.
 */
function generateOrganicStreets(
    city: City,
    rng: () => number,
    latCorr: number,
    radius: number,
    tier: StreetPopTier,
    out: CityStreet[],
): void {
    let numMainRoads = 5;
    let numBranches = 3;
    let stepsPerRoad = 10;

    if (tier === 'hamlet') { numMainRoads = 2; numBranches = 1; stepsPerRoad = 5; }
    else if (tier === 'town') { numMainRoads = 3; numBranches = 2; stepsPerRoad = 7; }
    else if (tier === 'metropolis') { numMainRoads = 8; numBranches = 4; stepsPerRoad = 14; }

    const stepLen = radius * 0.11;
    const maxR = radius * 0.7;

    for (let road = 0; road < numMainRoads; road++) {
        let angle = (road / numMainRoads) * Math.PI * 2 + (rng() - 0.5) * 0.5;
        let cx = city.x;
        let cy = city.y;

        // Build multi-point path for this main road
        const mainPath: [number, number][] = [[cx, cy]];

        for (let step = 0; step < stepsPerRoad; step++) {
            angle += (rng() - 0.5) * 0.7;
            const dx = stepLen * Math.cos(angle) * latCorr;
            const dy = stepLen * Math.sin(angle);
            const nx = cx + dx;
            const ny = cy + dy;

            const distFromCenter = Math.sqrt(
                ((nx - city.x) / latCorr) ** 2 + (ny - city.y) ** 2
            );
            if (distFromCenter > maxR) break;

            mainPath.push([nx, ny]);

            // Branch off at intervals
            if (step > 0 && step % 2 === 0) {
                for (let br = 0; br < numBranches; br++) {
                    const brAngle = angle + (rng() - 0.5) * 2.0 + (br % 2 === 0 ? Math.PI / 3 : -Math.PI / 3);
                    let bx = nx;
                    let by = ny;
                    const branchSteps = 2 + Math.floor(rng() * 3);
                    const branchStep = stepLen * 0.55;
                    let brDir = brAngle;
                    const branchPath: [number, number][] = [[bx, by]];

                    for (let bs = 0; bs < branchSteps; bs++) {
                        brDir += (rng() - 0.5) * 1.0;
                        const nbx = bx + branchStep * Math.cos(brDir) * latCorr;
                        const nby = by + branchStep * Math.sin(brDir);
                        const brDist = Math.sqrt(
                            ((nbx - city.x) / latCorr) ** 2 + (nby - city.y) ** 2
                        );
                        if (brDist > maxR) break;
                        branchPath.push([nbx, nby]);
                        bx = nbx;
                        by = nby;
                    }

                    if (branchPath.length >= 2) {
                        // Inner branches are residential, outer branches are alleys
                        const branchDist = Math.sqrt(
                            ((nx - city.x) / latCorr) ** 2 + (ny - city.y) ** 2
                        );
                        const isInner = branchDist < maxR * 0.5;
                        out.push({
                            path: branchPath,
                            type: isInner ? 'residential' : 'alley',
                            width: isInner ? 1 : 0.5,
                        });
                    }
                }
            }

            cx = nx;
            cy = ny;
        }

        if (mainPath.length >= 2) {
            // First two segments from center are avenues, rest are main streets
            const isMainAvenue = road < 3;
            out.push({
                path: mainPath,
                type: isMainAvenue ? 'avenue' : 'main',
                width: isMainAvenue ? 2.5 : 1.5,
            });
        }
    }
}

// ── Bridge marker detection (Phase 3.2.4) ──────────────────────────────────

/**
 * Detect where road/street paths cross river paths and produce bridge markers.
 * Uses segment-segment intersection between road paths and river paths.
 * Bridge markers are short path segments centered on the crossing point.
 */
function detectBridgeMarkers(
    roads: RoadSegment[],
    cityStreets: CityStreet[],
): BridgeMarker[] {
    const rivers = getCachedRiverPaths();
    if (!rivers || rivers.length === 0) return [];

    const markers: BridgeMarker[] = [];
    const MAX_BRIDGES = 500; // performance cap

    // Pre-collect all river segments with bounding boxes
    interface RiverSeg { ax: number; ay: number; bx: number; by: number }
    const riverSegs: RiverSeg[] = [];
    for (const river of rivers) {
        for (let i = 0; i < river.path.length - 1; i++) {
            riverSegs.push({
                ax: river.path[i][0],
                ay: river.path[i][1],
                bx: river.path[i + 1][0],
                by: river.path[i + 1][1],
            });
        }
    }
    if (riverSegs.length === 0) return [];

    // Check all road segments against all river segments
    const checkPaths = (paths: [number, number][][]): void => {
        for (const path of paths) {
            for (let i = 0; i < path.length - 1 && markers.length < MAX_BRIDGES; i++) {
                const p1x = path[i][0], p1y = path[i][1];
                const p2x = path[i + 1][0], p2y = path[i + 1][1];
                for (const rs of riverSegs) {
                    const ix = segmentIntersection(p1x, p1y, p2x, p2y, rs.ax, rs.ay, rs.bx, rs.by);
                    if (ix) {
                        // Create a short bridge marker path across the crossing
                        const roadDx = p2x - p1x;
                        const roadDy = p2y - p1y;
                        const roadLen = Math.sqrt(roadDx * roadDx + roadDy * roadDy);
                        if (roadLen < 1e-10) continue;
                        const bridgeHalf = Math.min(roadLen * 0.3, 0.005); // half-length of bridge marker
                        const ndx = roadDx / roadLen;
                        const ndy = roadDy / roadLen;
                        markers.push({
                            path: [
                                [ix[0] - ndx * bridgeHalf, ix[1] - ndy * bridgeHalf],
                                [ix[0] + ndx * bridgeHalf, ix[1] + ndy * bridgeHalf],
                            ],
                        });
                    }
                }
            }
        }
    };

    // Check highway/regional roads
    checkPaths(roads.map(r => r.path));

    // Check city streets (only avenues and main streets to limit count)
    checkPaths(
        cityStreets
            .filter(s => s.type === 'avenue' || s.type === 'main')
            .map(s => s.path)
    );

    return markers;
}

/**
 * Segment-segment intersection test.
 * Returns the intersection point [x, y] or null if segments don't cross.
 */
function segmentIntersection(
    ax: number, ay: number, bx: number, by: number,
    cx: number, cy: number, dx: number, dy: number,
): [number, number] | null {
    const abx = bx - ax, aby = by - ay;
    const cdx = dx - cx, cdy = dy - cy;
    const denom = abx * cdy - aby * cdx;
    if (Math.abs(denom) < 1e-12) return null; // parallel

    const acx = cx - ax, acy = cy - ay;
    const t = (acx * cdy - acy * cdx) / denom;
    const u = (acx * aby - acy * abx) / denom;

    if (t >= 0 && t <= 1 && u >= 0 && u <= 1) {
        return [ax + t * abx, ay + t * aby];
    }
    return null;
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
let cachedCityStreets: CityStreet[] | null = null;
let cachedBridgeMarkers: BridgeMarker[] | null = null;

/**
 * Build and cache road network data, intra-city streets, and bridge markers.
 * Call once after cities/regions are loaded (and after river data is built).
 * @param cells Optional grid cells for terrain-aware routing. If empty, roads are straight lines.
 */
export function buildRoadData(cities: City[], regions: Region[], cells: GridCell[] = []): void {
    cachedRoads = generateRoads(cities, regions, cells);

    // Generate intra-city streets for all cities, largest first, within budget
    const sorted = [...cities]
        .filter(c => Math.abs(c.y) <= 85)
        .sort((a, b) => b.population - a.population);

    const allStreets: CityStreet[] = [];
    for (const city of sorted) {
        if (allStreets.length >= MAX_CITY_STREETS) break;
        const layout = getLayoutStyle(city.region_id);
        const streets = generateCityStreets(city, layout);
        const remaining = MAX_CITY_STREETS - allStreets.length;
        const toAdd = streets.slice(0, remaining);
        for (const s of toAdd) allStreets.push(s);
    }
    cachedCityStreets = allStreets;

    // Detect bridge markers where roads/streets cross rivers
    cachedBridgeMarkers = detectBridgeMarkers(cachedRoads, cachedCityStreets);
}

/** Dispose cached road data to free memory. */
export function disposeRoadData(): void {
    cachedRoads = null;
    cachedCityStreets = null;
    cachedBridgeMarkers = null;
}

/**
 * Get the cached city streets array (read-only).
 * Used by buildingsLayer for building alignment to street geometry.
 */
export function getCachedCityStreets(): CityStreet[] | null {
    return cachedCityStreets;
}

// ── Layer creation ─────────────────────────────────────────────────────────

/**
 * Create road network layers for Procgen mode.
 * Returns deck.gl PathLayers for highways, regional roads, intra-city streets,
 * and bridge markers at river crossings.
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

    // Intra-city streets -- visible at zoom 7+
    if (currentZoom >= 7 && cachedCityStreets && cachedCityStreets.length > 0) {
        // Avenues: widest city streets
        const avenues = cachedCityStreets.filter(s => s.type === 'avenue');
        if (avenues.length > 0) {
            layers.push(new PathLayer({
                id: 'city-streets-avenue',
                data: avenues,
                getPath: (d: CityStreet) => d.path,
                getColor: [88, 95, 112, 150],
                getWidth: (d: CityStreet) => d.width,
                widthUnits: 'pixels',
                widthMinPixels: 1,
                widthMaxPixels: 4,
                pickable: false,
                parameters: { depthTest: false },
                capRounded: true,
                jointRounded: true,
            }));
        }

        // Main streets
        const mains = cachedCityStreets.filter(s => s.type === 'main');
        if (mains.length > 0) {
            layers.push(new PathLayer({
                id: 'city-streets-main',
                data: mains,
                getPath: (d: CityStreet) => d.path,
                getColor: [72, 78, 92, 130],
                getWidth: (d: CityStreet) => d.width,
                widthUnits: 'pixels',
                widthMinPixels: 1,
                widthMaxPixels: 3,
                pickable: false,
                parameters: { depthTest: false },
                capRounded: true,
                jointRounded: true,
            }));
        }

        // Residential streets -- visible at zoom 8+
        if (currentZoom >= 8) {
            const residential = cachedCityStreets.filter(s => s.type === 'residential');
            if (residential.length > 0) {
                layers.push(new PathLayer({
                    id: 'city-streets-residential',
                    data: residential,
                    getPath: (d: CityStreet) => d.path,
                    getColor: [58, 62, 74, 100],
                    getWidth: (d: CityStreet) => d.width,
                    widthUnits: 'pixels',
                    widthMinPixels: 1,
                    widthMaxPixels: 2,
                    pickable: false,
                    parameters: { depthTest: false },
                    capRounded: true,
                    jointRounded: true,
                }));
            }
        }

        // Alleys -- visible at zoom 9+
        if (currentZoom >= 9) {
            const alleys = cachedCityStreets.filter(s => s.type === 'alley');
            if (alleys.length > 0) {
                layers.push(new PathLayer({
                    id: 'city-streets-alley',
                    data: alleys,
                    getPath: (d: CityStreet) => d.path,
                    getColor: [50, 54, 64, 80],
                    getWidth: (d: CityStreet) => d.width,
                    widthUnits: 'pixels',
                    widthMinPixels: 1,
                    widthMaxPixels: 1,
                    pickable: false,
                    parameters: { depthTest: false },
                    capRounded: true,
                    jointRounded: true,
                }));
            }
        }
    }

    // Bridge markers -- visible at zoom 5+ (where roads cross rivers)
    if (currentZoom >= 5 && cachedBridgeMarkers && cachedBridgeMarkers.length > 0) {
        layers.push(new PathLayer({
            id: 'road-bridges',
            data: cachedBridgeMarkers,
            getPath: (d: BridgeMarker) => d.path,
            getColor: [140, 130, 110, 200],  // warm stone/concrete color
            getWidth: 5,
            widthUnits: 'pixels',
            widthMinPixels: 2,
            widthMaxPixels: 8,
            pickable: false,
            parameters: { depthTest: false },
            capRounded: false, // squared ends for bridge appearance
            jointRounded: false,
        }));
    }

    return layers;
}
