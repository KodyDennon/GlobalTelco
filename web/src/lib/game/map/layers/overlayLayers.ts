import { BitmapLayer, ScatterplotLayer, PolygonLayer } from '@deck.gl/layers';
import type { Layer } from '@deck.gl/core';
import type { City, Region, CellCoverage, SpectrumLicense, AllInfraNode } from '$lib/wasm/types';
import { CORP_COLORS } from '../constants';
import * as bridge from '$lib/wasm/bridge';
import { dataStore } from '../DataStore';
import { ZONE_CONFIGS, zoneRadii, type BuildingZone } from './buildingsLayer';

// ── Spectrum band color + coverage mappings ────────────────────────────────

/** RGBA color per frequency band for spectrum overlay coverage circles. */
const BAND_COLORS: Record<string, [number, number, number]> = {
    '700MHz':  [100, 200, 100],
    '850MHz':  [100, 200, 180],
    '1800MHz': [100, 150, 255],
    '2100MHz': [100, 220, 255],
    '2600MHz': [180, 100, 255],
    '3500MHz': [255, 100, 200],
    '28GHz':   [255, 180, 60],
    '39GHz':   [255, 100, 80],
};

/** Default grey for nodes with no assigned band. */
const UNASSIGNED_BAND_COLOR: [number, number, number] = [128, 128, 128];

/** Coverage radius in meters per frequency band. */
const BAND_COVERAGE_M: Record<string, number> = {
    '700MHz':  30000,
    '850MHz':  25000,
    '1800MHz': 10000,
    '2100MHz': 8000,
    '2600MHz': 5000,
    '3500MHz': 2000,
    '28GHz':   500,
    '39GHz':   300,
};

/** Default coverage radius for nodes without a recognized band. */
const DEFAULT_COVERAGE_M = 5000;

/** Wireless node types that emit coverage circles in the spectrum overlay. */
const WIRELESS_OVERLAY_TYPES = new Set(['MacroCell', 'SmallCell', 'CellTower', 'WirelessRelay']);

/**
 * Creates overlay visualization layers: terrain, demand, geographic risk, coverage,
 * ownership, market_share, spectrum, congestion, and traffic. Each uses ScatterplotLayer
 * with color gradients (except terrain which uses BitmapLayer, and market_share / spectrum
 * which use PolygonLayer).
 *
 * Note: congestion and traffic overlays are handled by infraLayer coloring.
 * This function handles terrain, demand, geographic risk, coverage, ownership,
 * market_share, and spectrum.
 */
export function createOverlayLayers(opts: {
    activeOverlay: string;
    terrainOverlayCanvas: HTMLCanvasElement | null;
    cities: City[];
    regions: Region[];
    cellRadiusM: number;
}): Layer[] {
    const { activeOverlay, terrainOverlayCanvas, cities, regions, cellRadiusM } = opts;

    const layers: Layer[] = [];
    const overlayRadius = cellRadiusM * 1.05;

    if (activeOverlay === 'terrain') {
        if (terrainOverlayCanvas) {
            layers.push(new BitmapLayer({
                id: 'overlay-terrain',
                image: terrainOverlayCanvas as any,
                bounds: [-180, -85, 180, 85] as [number, number, number, number],
                pickable: false
            }));
        }
        return layers;
    }

    if (activeOverlay === 'demand') {
        const demandCells: { position: [number, number]; color: [number, number, number, number] }[] = [];
        for (const city of cities) {
            const intensity = Math.min(1.0, city.telecom_demand / 500);
            const r = Math.floor(59 + intensity * 196);
            const g = Math.floor(130 * (1 - intensity));
            const b = Math.floor(246 * (1 - intensity));
            for (const cp of city.cell_positions) {
                demandCells.push({
                    position: [cp.lon, cp.lat],
                    color: [r, g, b, 150]
                });
            }
        }
        layers.push(new ScatterplotLayer({
            id: 'overlay-demand',
            data: demandCells,
            getPosition: (d: any) => d.position,
            getFillColor: (d: any) => d.color,
            getRadius: overlayRadius,
            radiusMinPixels: 6,
            pickable: false,
            parameters: { depthTest: false }
        }));
    }

    if (activeOverlay === 'disaster') {
        const regionRiskMap = new Map<number, number>();
        for (const r of regions) {
            regionRiskMap.set(r.id, r.disaster_risk);
        }
        const riskCells: { position: [number, number]; color: [number, number, number, number] }[] = [];
        for (const city of cities) {
            const risk = regionRiskMap.get(city.region_id) ?? 0;
            const intensity = Math.min(1.0, risk * 5);
            const r = Math.floor(intensity * 255);
            const g = Math.floor((1 - intensity) * 200);
            for (const cp of city.cell_positions) {
                riskCells.push({
                    position: [cp.lon, cp.lat],
                    color: [r, g, 50, 150]
                });
            }
        }
        layers.push(new ScatterplotLayer({
            id: 'overlay-disaster',
            data: riskCells,
            getPosition: (d: any) => d.position,
            getFillColor: (d: any) => d.color,
            getRadius: overlayRadius,
            radiusMinPixels: 6,
            pickable: false,
            parameters: { depthTest: false }
        }));
    }

    if (activeOverlay === 'coverage') {
        if (bridge.isInitialized()) {
            const coverageData = bridge.getCellCoverage();
            layers.push(new ScatterplotLayer({
                id: 'overlay-coverage',
                data: coverageData,
                getPosition: (d: CellCoverage) => [d.lon, d.lat],
                getFillColor: (d: CellCoverage) => {
                    const intensity = Math.min(1.0, d.signal_strength / 100);
                    return [Math.floor((1 - intensity) * 255), Math.floor(intensity * 200), 50, 150] as [number, number, number, number];
                },
                getRadius: overlayRadius,
                radiusMinPixels: 6,
                pickable: false,
                parameters: { depthTest: false }
            }));
        }
    }

    if (activeOverlay === 'ownership') {
        if (bridge.isInitialized()) {
            const coverageData = bridge.getCellCoverage();
            layers.push(new ScatterplotLayer({
                id: 'overlay-ownership',
                data: coverageData.filter(d => d.dominant_owner !== null),
                getPosition: (d: CellCoverage) => [d.lon, d.lat],
                getFillColor: (d: CellCoverage) => {
                    const corps = bridge.getAllCorporations();
                    const idx = corps.findIndex(c => c.id === d.dominant_owner);
                    const baseColor = CORP_COLORS[idx % CORP_COLORS.length];
                    return [...baseColor, 180] as [number, number, number, number];
                },
                getRadius: overlayRadius,
                radiusMinPixels: 6,
                pickable: false
            }));
        }
    }

    if (activeOverlay === 'market_share') {
        if (bridge.isInitialized() && regions.length > 0) {
            const corps = bridge.getAllCorporations();
            const corpIndex = new Map<number, number>();
            for (let i = 0; i < corps.length; i++) {
                corpIndex.set(corps[i].id, i);
            }

            // Count nodes per corporation per region using optimized DataStore
            const cellToRegion = new Map<number, number>();
            for (const city of cities) {
                for (const cp of city.cell_positions) {
                    cellToRegion.set(cp.index, city.region_id);
                }
            }

            const regionCorpCounts = dataStore.getRegionCorpCounts(cellToRegion);

            interface MarketShareRegion {
                polygon: [number, number][];
                color: [number, number, number, number];
            }
            const polygonData: MarketShareRegion[] = [];

            for (const region of regions) {
                if (!region.boundary_polygon || region.boundary_polygon.length < 3) continue;
                const counts = regionCorpCounts.get(region.id);
                if (!counts || counts.size === 0) continue;

                let maxCount = 0;
                let dominantCorpId = 0;
                for (const [corpId, count] of counts) {
                    if (count > maxCount) {
                        maxCount = count;
                        dominantCorpId = corpId;
                    }
                }

                const idx = corpIndex.get(dominantCorpId);
                const baseColor = idx !== undefined
                    ? CORP_COLORS[idx % CORP_COLORS.length]
                    : [160, 160, 160] as [number, number, number];

                polygonData.push({
                    polygon: region.boundary_polygon,
                    color: [baseColor[0], baseColor[1], baseColor[2], 80],
                });
            }

            if (polygonData.length > 0) {
                layers.push(new PolygonLayer({
                    id: 'overlay-market-share',
                    data: polygonData,
                    getPolygon: (d: MarketShareRegion) => d.polygon,
                    getFillColor: (d: MarketShareRegion) => d.color,
                    getLineColor: (d: MarketShareRegion) => [d.color[0], d.color[1], d.color[2], 140],
                    getLineWidth: 2,
                    lineWidthUnits: 'pixels',
                    filled: true,
                    stroked: true,
                    pickable: false,
                    parameters: { depthTest: false },
                } as any));
            }
        }
    }

    // ── Spectrum overlay ──────────────────────────────────────────────────────
    if (activeOverlay === 'spectrum') {
        if (bridge.isInitialized()) {
            layers.push(...createSpectrumOverlayLayers(regions, cities));
        }
    }

    // ── Coverage overlap (competitive) overlay ───────────────────────────────
    if (activeOverlay === 'coverage_overlap') {
        if (bridge.isInitialized()) {
            layers.push(...createCoverageOverlapLayers(cities, regions, cellRadiusM));
        }
    }

    // ── Congestion heatmap overlay ───────────────────────────────────────────
    if (activeOverlay === 'congestion') {
        if (bridge.isInitialized()) {
            const n = dataStore.nodes;
            const congestionPoints = [];
            
            for (let i = 0; i < n.count; i++) {
                const utilization = n.stats[i * 3 + 1];
                if (utilization > 0.1) {
                    congestionPoints.push({
                        position: [n.positions[i * 2], n.positions[i * 2 + 1]],
                        utilization,
                        color: [
                            Math.floor(Math.min(1, utilization * 1.5) * 255),
                            Math.floor(Math.max(0, 1 - utilization) * 200),
                            0,
                            160
                        ] as [number, number, number, number]
                    });
                }
            }

            if (congestionPoints.length > 0) {
                layers.push(new ScatterplotLayer({
                    id: 'overlay-congestion-heatmap',
                    data: congestionPoints,
                    getPosition: (d: any) => d.position,
                    getFillColor: (d: any) => d.color,
                    getRadius: overlayRadius * 1.5,
                    radiusMinPixels: 8,
                    pickable: false,
                    parameters: { depthTest: false }
                }));
            }
        }
    }

    // ── Traffic density overlay ──────────────────────────────────────────────
    if (activeOverlay === 'traffic') {
        if (bridge.isInitialized()) {
            const trafficFlows = bridge.getTrafficFlows();
            const trafficPoints = trafficFlows.node_flows
                .filter(f => f.utilization > 0.05)
                .map(f => {
                    const node = bridge.getNodeMetadata(f.id);
                    if (!node || !node.id) return null;
                    return {
                        position: [node.x, node.y],
                        utilization: f.utilization,
                        color: [0, 255, 255, Math.floor(f.utilization * 200)] as [number, number, number, number]
                    };
                })
                .filter(p => p !== null);

            if (trafficPoints.length > 0) {
                layers.push(new ScatterplotLayer({
                    id: 'overlay-traffic-density',
                    data: trafficPoints,
                    getPosition: (d: any) => d.position,
                    getFillColor: (d: any) => d.color,
                    getRadius: overlayRadius * 2,
                    radiusMinPixels: 10,
                    pickable: false,
                    parameters: { depthTest: false }
                }));
            }
        }
    }

    return layers;
}

// ── Spectrum overlay implementation ───────────────────────────────────────────

/**
 * Builds spectrum visualization layers:
 * (a) Licensed region polygons colored by dominant frequency band
 * (b) Unlicensed region polygons as faint dashed outlines (opportunity indicator)
 * (c) Wireless node coverage circles colored by frequency band
 * (d) Interference zones colored by severity (green/yellow/red)
 */
function createSpectrumOverlayLayers(regions: Region[], cities: City[]): Layer[] {
    const layers: Layer[] = [];
    const licenses = bridge.getSpectrumLicenses();

    // ── (a) Licensed region polygons colored by dominant frequency band ──────


    const regionBandCounts = new Map<number, Map<string, number>>();
    const licensedRegionIds = new Set<number>();
    for (const lic of licenses) {
        licensedRegionIds.add(lic.region_id);
        if (!regionBandCounts.has(lic.region_id)) {
            regionBandCounts.set(lic.region_id, new Map());
        }
        const counts = regionBandCounts.get(lic.region_id)!;
        counts.set(lic.band, (counts.get(lic.band) ?? 0) + 1);
    }

    interface SpectrumRegionPoly {
        polygon: [number, number][];
        color: [number, number, number, number];
    }
    const licensedPolys: SpectrumRegionPoly[] = [];
    const unlicensedPolys: SpectrumRegionPoly[] = [];

    for (const region of regions) {
        if (!region.boundary_polygon || region.boundary_polygon.length < 3) continue;
        const bandCounts = regionBandCounts.get(region.id);

        if (bandCounts && bandCounts.size > 0) {
            let maxCount = 0;
            let dominantBand = '';
            for (const [band, count] of bandCounts) {
                if (count > maxCount) {
                    maxCount = count;
                    dominantBand = band;
                }
            }
            const bandColor = BAND_COLORS[dominantBand] ?? UNASSIGNED_BAND_COLOR;
            licensedPolys.push({
                polygon: region.boundary_polygon,
                color: [bandColor[0], bandColor[1], bandColor[2], 51],
            });
        } else {
            unlicensedPolys.push({
                polygon: region.boundary_polygon,
                color: [80, 80, 80, 25],
            });
        }
    }

    if (licensedPolys.length > 0) {
        layers.push(new PolygonLayer({
            id: 'overlay-spectrum-regions',
            data: licensedPolys,
            getPolygon: (d: SpectrumRegionPoly) => d.polygon,
            getFillColor: (d: SpectrumRegionPoly) => d.color,
            getLineColor: (d: SpectrumRegionPoly) => [d.color[0], d.color[1], d.color[2], 100],
            getLineWidth: 1,
            lineWidthUnits: 'pixels',
            filled: true,
            stroked: true,
            pickable: false,
            parameters: { depthTest: false },
        } as any));
    }

    // ── (b) Unlicensed regions — faint dashed outlines ──────────────────────

    if (unlicensedPolys.length > 0) {
        layers.push(new PolygonLayer({
            id: 'overlay-spectrum-unlicensed',
            data: unlicensedPolys,
            getPolygon: (d: SpectrumRegionPoly) => d.polygon,
            getFillColor: (d: SpectrumRegionPoly) => d.color,
            getLineColor: [100, 100, 100, 60],
            getLineWidth: 2,
            lineWidthUnits: 'pixels',
            filled: true,
            stroked: true,
            pickable: false,
            parameters: { depthTest: false },
        } as any));
    }

    // ── (c) Wireless node coverage circles colored by frequency band ────────

    const cellToRegion = new Map<number, number>();
    for (const city of cities) {
        for (const cp of city.cell_positions) {
            cellToRegion.set(cp.index, city.region_id);
        }
    }

    const regionOwnerBands = new Map<string, string>();
    for (const lic of licenses) {
        const key = `${lic.region_id}-${lic.owner}`;
        if (!regionOwnerBands.has(key)) {
            regionOwnerBands.set(key, lic.band);
        }
    }

    interface CoverageCircle {
        position: [number, number];
        radius: number;
        color: [number, number, number, number];
        band: string;
    }
    const coverageCircles: CoverageCircle[] = [];

    const n = dataStore.nodes;
    const nodeTypes = bridge.getStaticDefinitions().node_types;

    for (let i = 0; i < n.count; i++) {
        const nodeType = nodeTypes[n.node_types[i]];
        if (!WIRELESS_OVERLAY_TYPES.has(nodeType)) continue;

        const cellIndex = n.cell_indices[i];
        const owner = n.owners[i];
        const regionId = cellToRegion.get(cellIndex);
        const ownerKey = regionId !== undefined ? `${regionId}-${owner}` : '';
        const band = regionOwnerBands.get(ownerKey) ?? '';

        const bandColor = band ? (BAND_COLORS[band] ?? UNASSIGNED_BAND_COLOR) : UNASSIGNED_BAND_COLOR;
        const coverageM = band ? (BAND_COVERAGE_M[band] ?? DEFAULT_COVERAGE_M) : DEFAULT_COVERAGE_M;

        coverageCircles.push({
            position: [n.positions[i * 2], n.positions[i * 2 + 1]],
            radius: coverageM,
            color: [bandColor[0], bandColor[1], bandColor[2], 80],
            band: band || 'unassigned',
        });
    }

    if (coverageCircles.length > 0) {
        layers.push(new ScatterplotLayer({
            id: 'overlay-spectrum-coverage',
            data: coverageCircles,
            getPosition: (d: CoverageCircle) => d.position,
            getFillColor: (d: CoverageCircle) => d.color,
            getRadius: (d: CoverageCircle) => d.radius,
            radiusMinPixels: 8,
            pickable: false,
            parameters: {
                depthTest: false,
                blend: true,
                blendFunc: [WebGLRenderingContext.SRC_ALPHA, WebGLRenderingContext.ONE_MINUS_SRC_ALPHA],
            },
        }));

        layers.push(new ScatterplotLayer({
            id: 'overlay-spectrum-coverage-ring',
            data: coverageCircles,
            getPosition: (d: CoverageCircle) => d.position,
            getFillColor: [0, 0, 0, 0],
            getLineColor: (d: CoverageCircle) => [d.color[0], d.color[1], d.color[2], 140],
            getLineWidth: 1,
            lineWidthUnits: 'pixels' as const,
            stroked: true,
            filled: false,
            getRadius: (d: CoverageCircle) => d.radius,
            radiusMinPixels: 8,
            pickable: false,
            parameters: { depthTest: false },
        }));

        // ── (d) Interference zones — severity-based coloring ────────────────

        const bandGroups = new Map<string, CoverageCircle[]>();
        for (const c of coverageCircles) {
            if (c.band === 'unassigned') continue;
            if (!bandGroups.has(c.band)) bandGroups.set(c.band, []);
            bandGroups.get(c.band)!.push(c);
        }

        interface InterferencePoint {
            position: [number, number];
            radius: number;
            severity: number;
        }
        const interferencePoints: InterferencePoint[] = [];

        for (const [, group] of bandGroups) {
            for (let i = 0; i < group.length; i++) {
                for (let j = i + 1; j < group.length; j++) {
                    const a = group[i];
                    const b = group[j];
                    const dLon = (b.position[0] - a.position[0]) * Math.PI / 180;
                    const dLat = (b.position[1] - a.position[1]) * Math.PI / 180;
                    const midLat = (a.position[1] + b.position[1]) / 2 * Math.PI / 180;
                    const R = 6371000;
                    const dx = dLon * Math.cos(midLat) * R;
                    const dy = dLat * R;
                    const dist = Math.sqrt(dx * dx + dy * dy);

                    const sumRadii = a.radius + b.radius;
                    if (dist < sumRadii) {
                        const midLon = (a.position[0] + b.position[0]) / 2;
                        const midLatDeg = (a.position[1] + b.position[1]) / 2;
                        const overlap = sumRadii - dist;
                        const severity = Math.min(1.0, overlap / sumRadii);
                        interferencePoints.push({
                            position: [midLon, midLatDeg],
                            radius: Math.max(overlap * 0.6, 1000),
                            severity,
                        });
                    }
                }
            }
        }

        if (interferencePoints.length > 0) {
            layers.push(new ScatterplotLayer({
                id: 'overlay-spectrum-interference',
                data: interferencePoints,
                getPosition: (d: InterferencePoint) => d.position,
                getFillColor: (d: InterferencePoint) => {
                    if (d.severity > 0.5) return [255, 60, 60, 80] as [number, number, number, number];
                    if (d.severity > 0.2) return [245, 180, 30, 60] as [number, number, number, number];
                    return [60, 200, 100, 40] as [number, number, number, number];
                },
                getRadius: (d: InterferencePoint) => d.radius,
                radiusMinPixels: 6,
                pickable: false,
                parameters: {
                    depthTest: false,
                    blend: true,
                    blendFunc: [WebGLRenderingContext.SRC_ALPHA, WebGLRenderingContext.ONE_MINUS_SRC_ALPHA],
                },
            }));

            layers.push(new ScatterplotLayer({
                id: 'overlay-spectrum-interference-ring',
                data: interferencePoints,
                getPosition: (d: InterferencePoint) => d.position,
                getFillColor: [0, 0, 0, 0],
                getLineColor: (d: InterferencePoint) => {
                    if (d.severity > 0.5) return [255, 80, 80, 160] as [number, number, number, number];
                    if (d.severity > 0.2) return [245, 200, 50, 120] as [number, number, number, number];
                    return [80, 200, 120, 80] as [number, number, number, number];
                },
                getLineWidth: 2,
                lineWidthUnits: 'pixels' as const,
                stroked: true,
                filled: false,
                getRadius: (d: InterferencePoint) => d.radius,
                radiusMinPixels: 6,
                pickable: false,
                parameters: { depthTest: false },
            }));
        }
    }

    return layers;
}

// ── Coverage overlap (competitive) overlay ──────────────────────────────────

/**
 * Builds the competitive coverage overlap visualization:
 * (a) Cell-level heatmap: cells where 2+ corporations have infrastructure,
 *     colored by competition intensity (blue = 2 corps, yellow = 3, red = 4+)
 * (b) Region-level competition borders: regions with multiple competitors get
 *     a stroked polygon outline showing the number of active competitors
 * (c) Expansion frontier: cells where only one corporation has recently built
 *     in a contested region, shown as faint markers indicating expansion reach
 */
function createCoverageOverlapLayers(
    cities: City[],
    regions: Region[],
    cellRadiusM: number,
): Layer[] {
    const layers: Layer[] = [];
    const corps = bridge.getAllCorporations();
    const playerCorpId = bridge.getPlayerCorpId();

    // Build corp index for color lookup
    const corpIndex = new Map<number, number>();
    for (let i = 0; i < corps.length; i++) {
        corpIndex.set(corps[i].id, i);
    }

    // Build cell_index -> region_id lookup and cell_index -> position lookup
    const cellToRegion = new Map<number, number>();
    const cellPositions = new Map<number, [number, number]>();
    for (const city of cities) {
        for (const cp of city.cell_positions) {
            cellToRegion.set(cp.index, city.region_id);
            cellPositions.set(cp.index, [cp.lon, cp.lat]);
        }
    }

    // Count distinct corporations per cell using optimized DataStore
    const cellCorpSets = dataStore.getCellCorpSets();

    // ── (a) Cell-level competition heatmap ──────────────────────────────────

    interface OverlapCell {
        position: [number, number];
        competitorCount: number;
        hasPlayer: boolean;
    }
    const overlapCells: OverlapCell[] = [];

    for (const [cellIndex, corpSet] of cellCorpSets) {
        if (corpSet.size < 2) continue; // only cells with 2+ corps
        const pos = cellPositions.get(cellIndex);
        if (!pos) continue;
        overlapCells.push({
            position: pos,
            competitorCount: corpSet.size,
            hasPlayer: corpSet.has(playerCorpId),
        });
    }

    if (overlapCells.length > 0) {
        const overlayRadius = cellRadiusM * 1.05;

        layers.push(new ScatterplotLayer({
            id: 'overlay-coverage-overlap',
            data: overlapCells,
            getPosition: (d: OverlapCell) => d.position,
            getFillColor: (d: OverlapCell) => {
                const count = d.competitorCount;
                // 2 corps: blue-cyan, 3: amber-yellow, 4+: red-hot
                if (count >= 4) return [239, 68, 68, 160] as [number, number, number, number];
                if (count === 3) return [245, 158, 11, 140] as [number, number, number, number];
                return [59, 130, 246, 120] as [number, number, number, number];
            },
            getRadius: overlayRadius,
            radiusMinPixels: 6,
            pickable: false,
            parameters: {
                depthTest: false,
                blend: true,
                blendFunc: [
                    WebGLRenderingContext.SRC_ALPHA,
                    WebGLRenderingContext.ONE_MINUS_SRC_ALPHA,
                ],
            },
        }));

        // Brighter ring around cells where the player is competing
        const playerContested = overlapCells.filter(c => c.hasPlayer);
        if (playerContested.length > 0) {
            layers.push(new ScatterplotLayer({
                id: 'overlay-coverage-overlap-player',
                data: playerContested,
                getPosition: (d: OverlapCell) => d.position,
                getFillColor: [0, 0, 0, 0],
                getLineColor: (d: OverlapCell) => {
                    const count = d.competitorCount;
                    if (count >= 4) return [255, 100, 100, 200] as [number, number, number, number];
                    if (count === 3) return [255, 200, 50, 180] as [number, number, number, number];
                    return [100, 180, 255, 160] as [number, number, number, number];
                },
                getLineWidth: 2,
                lineWidthUnits: 'pixels' as const,
                stroked: true,
                filled: false,
                getRadius: overlayRadius,
                radiusMinPixels: 6,
                pickable: false,
                parameters: { depthTest: false },
            }));
        }
    }

    // ── (b) Region competition borders ──────────────────────────────────────

    // Count distinct corps per region using optimized DataStore
    const regionCorpCounts = dataStore.getRegionCorpCounts(cellToRegion);
    const regionCorps = new Map<number, Set<number>>();
    for (const [regionId, counts] of regionCorpCounts) {
        regionCorps.set(regionId, new Set(counts.keys()));
    }

    interface CompetitionRegion {
        polygon: [number, number][];
        corpCount: number;
        color: [number, number, number, number];
    }
    const competitionRegions: CompetitionRegion[] = [];

    for (const region of regions) {
        if (!region.boundary_polygon || region.boundary_polygon.length < 3) continue;
        const corpSet = regionCorps.get(region.id);
        if (!corpSet || corpSet.size < 2) continue;

        const count = corpSet.size;
        let color: [number, number, number, number];
        if (count >= 4) color = [239, 68, 68, 100];
        else if (count === 3) color = [245, 158, 11, 80];
        else color = [59, 130, 246, 60];

        competitionRegions.push({
            polygon: region.boundary_polygon,
            corpCount: count,
            color,
        });
    }

    if (competitionRegions.length > 0) {
        layers.push(new PolygonLayer({
            id: 'overlay-competition-regions',
            data: competitionRegions,
            getPolygon: (d: CompetitionRegion) => d.polygon,
            getFillColor: (d: CompetitionRegion) => d.color,
            getLineColor: (d: CompetitionRegion) => [d.color[0], d.color[1], d.color[2], 180] as [number, number, number, number],
            getLineWidth: 2,
            lineWidthUnits: 'pixels',
            filled: true,
            stroked: true,
            pickable: false,
            parameters: { depthTest: false },
        } as any));
    }

    // ── (c) Expansion frontier — single-owner cells in contested regions ────

    interface ExpansionCell {
        position: [number, number];
        ownerColor: [number, number, number];
        isPlayer: boolean;
    }
    const expansionCells: ExpansionCell[] = [];

    for (const [cellIndex, corpSet] of cellCorpSets) {
        if (corpSet.size !== 1) continue; // only single-owner cells are expansion frontier
        const pos = cellPositions.get(cellIndex);
        if (!pos) continue;

        // Only mark cells in regions with multiple competing corps — these are
        // the outposts / frontier cells where a single corp has established
        // exclusive presence in a contested market.
        const regionId = cellToRegion.get(cellIndex);
        if (regionId === undefined) continue;
        const regionCorpSet = regionCorps.get(regionId);
        if (!regionCorpSet || regionCorpSet.size < 2) continue;

        const ownerId = corpSet.values().next().value;
        if (ownerId === undefined) continue;
        const idx = corpIndex.get(ownerId);
        const baseColor = idx !== undefined
            ? CORP_COLORS[idx % CORP_COLORS.length]
            : [160, 160, 160] as [number, number, number];

        expansionCells.push({
            position: pos,
            ownerColor: baseColor,
            isPlayer: ownerId === playerCorpId,
        });
    }

    if (expansionCells.length > 0) {
        const overlayRadius = cellRadiusM * 0.8;

        // Faint filled circles showing each corp's frontier presence
        layers.push(new ScatterplotLayer({
            id: 'overlay-expansion-frontier',
            data: expansionCells,
            getPosition: (d: ExpansionCell) => d.position,
            getFillColor: (d: ExpansionCell) => [
                d.ownerColor[0],
                d.ownerColor[1],
                d.ownerColor[2],
                d.isPlayer ? 80 : 45,
            ] as [number, number, number, number],
            getLineColor: (d: ExpansionCell) => [
                d.ownerColor[0],
                d.ownerColor[1],
                d.ownerColor[2],
                d.isPlayer ? 140 : 80,
            ] as [number, number, number, number],
            getLineWidth: 1,
            lineWidthUnits: 'pixels' as const,
            stroked: true,
            filled: true,
            getRadius: overlayRadius,
            radiusMinPixels: 4,
            pickable: false,
            parameters: { depthTest: false },
        }));
    }

    return layers;
}

// ── Density zone overlay ────────────────────────────────────────────────────

/** Colors for each density zone (semi-transparent fills). */
const DENSITY_ZONE_COLORS: Record<BuildingZone, [number, number, number, number]> = {
    downtown:          [220, 50, 50, 55],   // Red — dense urban core
    commercial:        [60, 120, 220, 50],   // Blue — commercial district
    residential_inner: [50, 180, 80, 45],    // Green — inner residential
    residential_outer: [50, 180, 80, 30],    // Lighter green — outer residential
    suburban:          [220, 190, 50, 25],    // Yellow — suburban fringe
};

/** Outline colors for density zone borders. */
const DENSITY_ZONE_OUTLINES: Record<BuildingZone, [number, number, number, number]> = {
    downtown:          [240, 70, 70, 160],
    commercial:        [80, 140, 240, 140],
    residential_inner: [70, 200, 100, 120],
    residential_outer: [70, 200, 100, 80],
    suburban:          [240, 210, 70, 70],
};

/**
 * Generate a polygon ring (circle approximation) at a given radius around
 * a city center. Returns a closed polygon array of [lon, lat] pairs.
 */
function generateZoneRing(
    centerLon: number,
    centerLat: number,
    radius: number,
    segments: number = 48,
): [number, number][] {
    const latCorr = 1 / Math.max(0.1, Math.cos((centerLat * Math.PI) / 180));
    const points: [number, number][] = [];
    for (let i = 0; i <= segments; i++) {
        const angle = (i / segments) * Math.PI * 2;
        points.push([
            centerLon + radius * Math.cos(angle) * latCorr,
            centerLat + radius * Math.sin(angle),
        ]);
    }
    return points;
}

/**
 * Creates density zone overlay layers showing concentric zone rings around cities.
 * Each zone (downtown, commercial, residential inner/outer, suburban) is
 * rendered as a semi-transparent filled polygon ring with a colored outline.
 *
 * Uses the same population-based zone radius calculations as buildingsLayer.ts.
 */
export function createDensityOverlayLayers(cities: City[]): Layer[] {
    const layers: Layer[] = [];

    if (cities.length === 0) return layers;

    interface DensityZonePoly {
        polygon: [number, number][];
        zone: BuildingZone;
        fillColor: [number, number, number, number];
        lineColor: [number, number, number, number];
        cityName: string;
    }

    const zonePolygons: DensityZonePoly[] = [];

    // Limit to cities with pop >= 10k (skip tiny hamlets for visual clarity)
    const visibleCities = cities.filter(c => c.population >= 10000 && Math.abs(c.y) <= 85);

    // Process from largest to smallest, cap at 60 cities for performance
    const sorted = [...visibleCities].sort((a, b) => b.population - a.population).slice(0, 60);

    for (const city of sorted) {
        // Generate zone rings from outermost to innermost (render order)
        // Iterate in reverse so outer zones are rendered first (behind inner zones)
        for (let zi = ZONE_CONFIGS.length - 1; zi >= 0; zi--) {
            const config = ZONE_CONFIGS[zi];
            const { outerR } = zoneRadii(config.zone, city.population);

            // Skip negligible zones
            if (outerR < 0.001) continue;

            const ring = generateZoneRing(city.x, city.y, outerR);

            zonePolygons.push({
                polygon: ring,
                zone: config.zone,
                fillColor: DENSITY_ZONE_COLORS[config.zone],
                lineColor: DENSITY_ZONE_OUTLINES[config.zone],
                cityName: city.name,
            });
        }
    }

    if (zonePolygons.length === 0) return layers;

    // Filled zone polygons
    layers.push(new PolygonLayer({
        id: 'overlay-density-zones-fill',
        data: zonePolygons,
        getPolygon: (d: DensityZonePoly) => d.polygon,
        getFillColor: (d: DensityZonePoly) => d.fillColor,
        getLineColor: (d: DensityZonePoly) => d.lineColor,
        getLineWidth: 1.5,
        lineWidthUnits: 'pixels',
        filled: true,
        stroked: true,
        pickable: false,
        parameters: {
            depthTest: false,
            blend: true,
            blendFunc: [
                WebGLRenderingContext.SRC_ALPHA,
                WebGLRenderingContext.ONE_MINUS_SRC_ALPHA,
            ],
        },
    } as any));

    return layers;
}
