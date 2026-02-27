import { Deck } from '@deck.gl/core';
import { BitmapLayer, LineLayer, ScatterplotLayer, TextLayer, PathLayer, IconLayer } from '@deck.gl/layers';

import * as bridge from '$lib/wasm/bridge';
import type { GridCell, City, Region, CellCoverage } from '$lib/wasm/types';

import { GridPathfinder, needsTerrainRouting } from './GridPathfinder';
import { tooltipData, selectedEdgeType } from '$lib/stores/uiState';
import { get } from 'svelte/store';
import { icons } from '$lib/assets/icons';

// ── Color palettes ──────────────────────────────────────────────────────────

const CORP_COLORS = [
    [16, 185, 129], // Emerald
    [59, 130, 246], // Blue
    [245, 158, 11], // Amber
    [239, 68, 68],  // Red
    [139, 92, 246], // Violet
    [236, 72, 153], // Pink
    [20, 184, 166], // Teal
    [249, 115, 22]  // Orange
];

const EDGE_STYLES: Record<string, { color: [number, number, number], width: number }> = {
    FiberLocal: { color: [34, 211, 160], width: 2 },
    FiberRegional: { color: [96, 165, 250], width: 3 },
    FiberNational: { color: [129, 140, 248], width: 5 },
    Copper: { color: [217, 119, 6], width: 1 },
    Microwave: { color: [34, 211, 238], width: 2 },
    Satellite: { color: [251, 191, 36], width: 3 },
    Submarine: { color: [59, 130, 246], width: 5 }
};

// Night-earth satellite palette for terrain
const SATELLITE_COLORS: Record<string, [number, number, number]> = {
    Urban:        [55, 55, 72],
    Suburban:     [42, 45, 52],
    Rural:        [24, 42, 26],
    Mountainous:  [48, 42, 35],
    Desert:       [62, 52, 32],
    Coastal:      [28, 48, 58],
    Tundra:       [48, 56, 64],
    Frozen:       [62, 70, 78],
    OceanShallow: [12, 24, 52],
    OceanDeep:    [6, 12, 32],
    OceanTrench:  [2, 5, 16],
    Ocean:        [6, 12, 32],
};

// Brighter terrain colors for the terrain overlay toggle
const TERRAIN_OVERLAY_COLORS: Record<string, [number, number, number]> = {
    Urban:        [110, 110, 135],
    Suburban:     [85, 95, 85],
    Rural:        [50, 95, 50],
    Mountainous:  [95, 85, 72],
    Desert:       [125, 108, 68],
    Coastal:      [55, 100, 115],
    OceanShallow: [22, 55, 100],
    OceanDeep:    [8, 18, 50],
    OceanTrench:  [3, 8, 25],
    Ocean:        [8, 18, 50],
    Tundra:       [85, 100, 115],
    Frozen:       [110, 120, 130],
};

// Tier-based sizing: keyed by Rust NetworkLevel enum Debug names
const NODE_TIER_SIZE: Record<string, number> = {
    Local: 24, Regional: 32, National: 40, Continental: 48, GlobalBackbone: 56
};

const NETWORK_TIER_LABEL: Record<string, string> = {
    Local: 'T1', Regional: 'T2', National: 'T3', Continental: 'T4', GlobalBackbone: 'T5'
};

// ── Icon atlas builder ──────────────────────────────────────────────────────

// Convert Rust CamelCase enum variant to kebab-case icon key
function toIconKey(camelCase: string): string {
    return camelCase.replace(/([a-z])([A-Z])/g, '$1-$2').toLowerCase();
}

const ICON_SIZE = 64;
const ICONS_PER_ROW = 8;

/** Build a single Canvas spritesheet from all SVG icons at init time.
 *  Returns { canvas, mapping } where mapping is suitable for deck.gl IconLayer iconMapping. */
function buildIconAtlas(): { canvas: HTMLCanvasElement; mapping: Record<string, { x: number; y: number; width: number; height: number; mask: boolean }> } {
    const names = Object.keys(icons);
    const cols = ICONS_PER_ROW;
    const rows = Math.ceil(names.length / cols);
    const canvas = document.createElement('canvas');
    canvas.width = cols * ICON_SIZE;
    canvas.height = rows * ICON_SIZE;
    const ctx = canvas.getContext('2d')!;

    const mapping: Record<string, { x: number; y: number; width: number; height: number; mask: boolean }> = {};

    // We'll draw synchronously using SVG data URIs as Image sources
    // But Image loading is async — so we return immediately with a blank canvas
    // and fill it in as images load. deck.gl will re-render when the atlas updates.
    const promises: Promise<void>[] = [];

    for (let i = 0; i < names.length; i++) {
        const name = names[i];
        const col = i % cols;
        const row = Math.floor(i / cols);
        const x = col * ICON_SIZE;
        const y = row * ICON_SIZE;

        mapping[name] = { x, y, width: ICON_SIZE, height: ICON_SIZE, mask: false };

        const svg = (icons as Record<string, string>)[name];
        const blob = new Blob([svg], { type: 'image/svg+xml;charset=utf-8' });
        const url = URL.createObjectURL(blob);

        promises.push(new Promise<void>((resolve) => {
            const img = new Image();
            img.onload = () => {
                ctx.drawImage(img, x, y, ICON_SIZE, ICON_SIZE);
                URL.revokeObjectURL(url);
                resolve();
            };
            img.onerror = () => {
                // Draw a fallback colored square
                ctx.fillStyle = '#666';
                ctx.fillRect(x + 8, y + 8, ICON_SIZE - 16, ICON_SIZE - 16);
                URL.revokeObjectURL(url);
                resolve();
            };
            img.src = url;
        }));
    }

    // The atlas will be populated async but we return immediately.
    // deck.gl will pick up the canvas content on next render cycle.
    Promise.all(promises).then(() => {
        // Canvas is now fully painted — deck.gl will use it on next setProps
    });

    return { canvas, mapping };
}

// ── Terrain bitmap builder ──────────────────────────────────────────────────

/** Pre-render all grid cells onto an equirectangular Canvas image.
 *  Each cell becomes a filled circle of terrain color.
 *  Returns a canvas suitable for BitmapLayer `image`. */
function buildTerrainBitmap(
    cells: GridCell[],
    cellRadiusKm: number,
    colorPalette: Record<string, [number, number, number]>
): HTMLCanvasElement {
    // Canvas resolution: 1px per ~0.5 degrees gives a 720x340 canvas
    // For better quality, use 2px per degree → 720x340
    const PIXELS_PER_DEG = 2;
    const W = 360 * PIXELS_PER_DEG; // 720
    const H = 170 * PIXELS_PER_DEG; // 340 (for lat -85 to 85)
    const LAT_MIN = -85;
    const LAT_MAX = 85;

    const canvas = document.createElement('canvas');
    canvas.width = W;
    canvas.height = H;
    const ctx = canvas.getContext('2d')!;

    // Fill with deep ocean base color
    const oceanColor = colorPalette['OceanDeep'] || colorPalette['Ocean'] || [6, 12, 32];
    ctx.fillStyle = `rgb(${oceanColor[0]},${oceanColor[1]},${oceanColor[2]})`;
    ctx.fillRect(0, 0, W, H);

    // Convert cell radius from km to approximate pixel radius
    // At equator: 1 degree ≈ 111km, so cellRadiusKm / 111 = degrees, * PIXELS_PER_DEG = pixels
    const baseDegRadius = cellRadiusKm / 111;
    const basePixelRadius = baseDegRadius * PIXELS_PER_DEG * 1.3; // 1.3 = overlap factor for gapless coverage

    for (const cell of cells) {
        if (Math.abs(cell.lat) > 85) continue;

        const color = colorPalette[cell.terrain] || oceanColor;

        // Convert lon/lat to pixel coordinates
        const px = ((cell.lon + 180) / 360) * W;
        const py = ((LAT_MAX - cell.lat) / (LAT_MAX - LAT_MIN)) * H;

        // At higher latitudes, longitude degrees are "compressed" — expand the radius
        const latScale = 1 / Math.max(Math.cos(cell.lat * Math.PI / 180), 0.15);
        const rX = basePixelRadius * latScale;
        const rY = basePixelRadius;

        ctx.fillStyle = `rgb(${color[0]},${color[1]},${color[2]})`;
        ctx.beginPath();
        ctx.ellipse(px, py, rX, rY, 0, 0, Math.PI * 2);
        ctx.fill();
    }

    return canvas;
}

// ── MapRenderer class ───────────────────────────────────────────────────────

export class MapRenderer {
    private deck: Deck | null = null;
    private container: HTMLElement;

    private pathfinder = new GridPathfinder();
    private cachedCells: GridCell[] = [];
    private cachedRegions: Region[] = [];
    private cachedCities: City[] = [];

    private activeOverlay: string = 'none';
    private quality: 'low' | 'medium' | 'high' = 'medium';

    private hoveredEntity: any = null;
    private selectedId: number | null = null;
    private currentEdgeSourceId: number | null = null;
    private currentZoom: number = 2;
    private edgeTargetIds: Set<number> = new Set();

    private cellRadiusM: number = 120000;
    private cellSpacingKm: number = 120;

    // Pre-built assets
    private terrainCanvas: HTMLCanvasElement | null = null;
    private terrainOverlayCanvas: HTMLCanvasElement | null = null;
    private iconAtlas: HTMLCanvasElement | null = null;
    private iconMapping: Record<string, { x: number; y: number; width: number; height: number; mask: boolean }> = {};
    private iconAtlasReady = false;

    constructor(container: HTMLElement, quality: 'low' | 'medium' | 'high' = 'medium') {
        this.quality = quality;
        this.container = container;

        // Build icon atlas immediately
        const { canvas: atlasCanvas, mapping } = buildIconAtlas();
        this.iconAtlas = atlasCanvas;
        this.iconMapping = mapping;

        // Mark atlas ready after a tick (images load async)
        setTimeout(() => {
            this.iconAtlasReady = true;
            this.renderLayers();
        }, 500);

        this.deck = new Deck({
            parent: container as HTMLDivElement,
            initialViewState: {
                longitude: 0,
                latitude: 20,
                zoom: 2,
                pitch: 0,
                bearing: 0,
                minZoom: 0.5,
                maxZoom: 12,
                maxPitch: 60
            },
            controller: true,
            layers: [],
            getTooltip: () => null,
            onClick: (info) => {
                if (!info.object) {
                    window.dispatchEvent(new CustomEvent('entity-selected', { detail: { id: null, type: null } }));
                }
            },
            onViewStateChange: ({ viewState }: any) => {
                this.currentZoom = viewState.zoom;
            }
        });
    }

    async buildMap() {
        if (!bridge.isInitialized()) return;

        const cells = bridge.getGridCells();
        this.cachedCities = bridge.getCities();
        this.cachedRegions = bridge.getRegions();
        this.cachedCells = cells;

        const worldInfo = bridge.getWorldInfo();
        if (worldInfo.cell_spacing_km && worldInfo.cell_spacing_km > 0) {
            this.cellSpacingKm = worldInfo.cell_spacing_km;
            this.cellRadiusM = worldInfo.cell_spacing_km * 1000 * 0.85;
        } else if (cells.length > 0) {
            const surfaceArea = 4 * Math.PI * 6371 * 6371;
            const areaPerCell = surfaceArea / cells.length;
            this.cellSpacingKm = Math.sqrt(areaPerCell);
            this.cellRadiusM = this.cellSpacingKm * 1000 * 0.85;
        }

        // Pre-render terrain bitmap (smooth, no dots)
        this.terrainCanvas = buildTerrainBitmap(cells, this.cellSpacingKm, SATELLITE_COLORS);
        this.terrainOverlayCanvas = buildTerrainBitmap(cells, this.cellSpacingKm, TERRAIN_OVERLAY_COLORS);

        this.pathfinder.init(cells);
        this.renderLayers();
    }

    private renderLayers() {
        if (!this.deck) return;

        const layers: any[] = [
            this.createLandLayer(),
            this.createBordersLayer(),
            ...this.createOverlayLayers(),
            ...this.createInfrastructureLayers(),
            this.createCitiesLayer(),
            this.createLabelsLayer(),
            this.createRegionLabelsLayer(),
            this.createSelectionLayer(),
            this.createEdgeBuildHighlights(),
            this.createPathfindingPreviewLayer()
        ].filter(Boolean).flat() as any[];

        this.deck.setProps({ layers });
    }

    // ── Terrain: smooth bitmap ──────────────────────────────────────────────

    private createLandLayer() {
        if (!this.terrainCanvas) return null;

        return new BitmapLayer({
            id: 'land-layer',
            image: this.terrainCanvas as any,
            bounds: [-180, -85, 180, 85] as [number, number, number, number],
            pickable: true,
            onClick: (info: any) => {
                window.dispatchEvent(new CustomEvent('entity-selected', { detail: { id: null, type: null } }));
                if (info.coordinate) {
                    const [lon, lat] = info.coordinate;
                    window.dispatchEvent(new CustomEvent('map-clicked', { detail: { lon, lat } }));
                }
            }
        });
    }

    // ── Borders: very subtle ────────────────────────────────────────────────

    private createBordersLayer() {
        if (bridge.isRealEarth()) return null;

        const borderData = this.cachedRegions
            .filter(r => r.boundary_polygon?.length > 2)
            .map(r => ({
                polygon: r.boundary_polygon.map(p => [p[0], p[1]]),
                name: r.name
            }));

        return new PathLayer({
            id: 'region-borders',
            data: borderData,
            getPath: (d: any) => d.polygon,
            getColor: [80, 95, 130, 80],
            getWidth: 1,
            widthUnits: 'pixels'
        });
    }

    // ── Infrastructure ──────────────────────────────────────────────────────

    private createInfrastructureLayers() {
        const corps = bridge.getAllCorporations();
        const edgesData: any[] = [];
        const nodesData: any[] = [];
        const isCongestion = this.activeOverlay === 'congestion';
        const isTraffic = this.activeOverlay === 'traffic';
        let trafficFlows: any = null;

        if (isTraffic && bridge.isInitialized()) {
            trafficFlows = bridge.getTrafficFlows();
        }

        for (let i = 0; i < corps.length; i++) {
            const corp = corps[i];
            const baseColor = CORP_COLORS[i % CORP_COLORS.length];
            const infra = bridge.getInfrastructureList(corp.id);

            for (const edge of infra.edges) {
                const style = EDGE_STYLES[edge.edge_type] || { color: baseColor, width: 2 };
                let color = style.color;
                let opacity = 255;

                if (isCongestion) {
                    const util = edge.utilization || 0;
                    color = [
                        Math.floor(Math.min(1, util * 2) * 255),
                        Math.floor(Math.max(0, 1 - util) * 200),
                        0
                    ];
                } else if (isTraffic && trafficFlows) {
                    const flow = trafficFlows.edge_flows.find((f: any) => f.id === edge.id);
                    if (flow) {
                        const util = flow.utilization;
                        if (util > 1.0) { color = [255, 34, 34]; opacity = 153; }
                        else if (util > 0.8) { color = [255, Math.floor((1 - (util - 0.8) / 0.2) * 80), 0]; opacity = 127; }
                        else if (util > 0.5) { color = [0, 255, 255]; opacity = 100; }
                        else { color = [59, 130, 246]; opacity = 50; }
                    } else {
                        color = [100, 100, 100]; opacity = 20;
                    }
                }

                edgesData.push({
                    ...edge,
                    sourcePosition: [edge.src_x, edge.src_y],
                    targetPosition: [edge.dst_x, edge.dst_y],
                    color: [...color, opacity],
                    width: style.width
                });
            }

            for (const node of infra.nodes) {
                let color = baseColor;
                let opacity = 255;
                if (isCongestion) {
                    const util = node.utilization || 0;
                    color = [
                        Math.floor(Math.min(1, util * 2) * 255),
                        Math.floor(Math.max(0, 1 - util) * 200),
                        0
                    ];
                } else if (isTraffic && trafficFlows) {
                    const flow = trafficFlows.node_flows.find((f: any) => f.id === node.id);
                    if (flow) {
                        const util = flow.utilization;
                        color = [Math.floor(Math.min(1, util * 2) * 255), Math.floor(Math.max(0, 1 - util) * 200), 0];
                        opacity = 180;
                    } else { color = [100, 100, 100]; opacity = 30; }
                }

                nodesData.push({
                    ...node,
                    position: [node.x, node.y],
                    color: [...color, opacity],
                    icon: toIconKey(node.node_type),
                    tierSize: NODE_TIER_SIZE[node.network_level] || 32,
                    tierLabel: NETWORK_TIER_LABEL[node.network_level] || ''
                });
            }
        }

        const layers: any[] = [
            new LineLayer({
                id: 'infra-edges',
                data: edgesData,
                getSourcePosition: (d: any) => d.sourcePosition,
                getTargetPosition: (d: any) => d.targetPosition,
                getColor: (d: any) => d.color,
                getWidth: (d: any) => d.width,
                widthUnits: 'pixels',
                pickable: true,
                autoHighlight: true,
                onClick: ({ object }: any) => {
                    if (object) {
                        window.dispatchEvent(new CustomEvent('entity-selected', { detail: { id: object.id, type: 'edge' } }));
                    }
                }
            })
        ];

        // Only add icon layer when atlas is ready
        if (this.iconAtlasReady && this.iconAtlas) {
            layers.push(new IconLayer({
                id: 'infra-nodes',
                data: nodesData,
                getPosition: (d: any) => d.position,
                getIcon: (d: any) => d.icon,
                iconAtlas: this.iconAtlas as any,
                iconMapping: this.iconMapping,
                getSize: (d: any) => d.tierSize,
                sizeMinPixels: 12,
                sizeMaxPixels: 72,
                getColor: (d: any) => d.color,
                pickable: true,
                autoHighlight: true,
                onClick: ({ object }: any) => {
                    if (object) {
                        window.dispatchEvent(new CustomEvent('entity-selected', { detail: { id: object.id, type: 'node' } }));
                    }
                }
            }));
        } else {
            // Fallback: colored dots while atlas loads
            layers.push(new ScatterplotLayer({
                id: 'infra-nodes-fallback',
                data: nodesData,
                getPosition: (d: any) => d.position,
                getFillColor: (d: any) => d.color,
                getRadius: (d: any) => d.tierSize * 500,
                radiusMinPixels: 6,
                radiusMaxPixels: 24,
                pickable: true,
                onClick: ({ object }: any) => {
                    if (object) {
                        window.dispatchEvent(new CustomEvent('entity-selected', { detail: { id: object.id, type: 'node' } }));
                    }
                }
            }));
        }

        // Tier badge labels (visible at zoom > 5)
        if (this.currentZoom > 5) {
            layers.push(new TextLayer({
                id: 'node-tier-labels',
                data: nodesData,
                getPosition: (d: any) => d.position,
                getText: (d: any) => d.tierLabel,
                getSize: 10,
                getColor: [255, 255, 255, 180],
                getPixelOffset: [14, -14],
                fontFamily: 'Inter, sans-serif',
                fontWeight: 'bold',
                parameters: { depthTest: false }
            }));
        }

        return layers;
    }

    // ── Cities ──────────────────────────────────────────────────────────────

    private createCitiesLayer() {
        const gtgCities = this.cachedCities.filter(c => Math.abs(c.y) <= 85).map(c => {
            let tier = 'hamlet';
            if (c.population > 5000000) tier = 'megalopolis';
            else if (c.population > 1000000) tier = 'metropolis';
            else if (c.population > 250000) tier = 'city';
            else if (c.population > 50000) tier = 'town';
            return { ...c, tier };
        });

        const layers: any[] = [
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
        if (this.iconAtlasReady && this.iconAtlas) {
            layers.push(new IconLayer({
                id: 'cities-icons',
                data: gtgCities,
                getPosition: (d: any) => [d.x, d.y],
                getIcon: (d: any) => d.tier,
                iconAtlas: this.iconAtlas as any,
                iconMapping: this.iconMapping,
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

    // ── Overlays ────────────────────────────────────────────────────────────

    private createOverlayLayers() {
        const layers: any[] = [];
        const overlayRadius = this.cellRadiusM * 1.05;

        if (this.activeOverlay === 'terrain') {
            if (this.terrainOverlayCanvas) {
                layers.push(new BitmapLayer({
                    id: 'overlay-terrain',
                    image: this.terrainOverlayCanvas as any,
                    bounds: [-180, -85, 180, 85] as [number, number, number, number],
                    pickable: false
                }));
            }
            return layers;
        }

        if (this.activeOverlay === 'demand') {
            const demandCells: { position: [number, number]; color: [number, number, number, number] }[] = [];
            for (const city of this.cachedCities) {
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

        if (this.activeOverlay === 'disaster') {
            const regionRiskMap = new Map<number, number>();
            for (const r of this.cachedRegions) {
                regionRiskMap.set(r.id, r.disaster_risk);
            }
            const riskCells: { position: [number, number]; color: [number, number, number, number] }[] = [];
            for (const city of this.cachedCities) {
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

        if (this.activeOverlay === 'coverage') {
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

        if (this.activeOverlay === 'ownership') {
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

        return layers;
    }

    // ── Public API ──────────────────────────────────────────────────────────

    updateInfrastructure() {
        if (!bridge.isInitialized()) return;
        this.renderLayers();
    }

    updateCities() {
        if (!bridge.isInitialized()) return;
        this.cachedCities = bridge.getCities();
        this.renderLayers();
    }

    highlightEdgeSource(nodeId: number | null) {
        this.currentEdgeSourceId = nodeId;
        if (nodeId === null) {
            this.edgeTargetIds.clear();
        }
        this.renderLayers();
    }

    setEdgeTargets(targets: Array<{ target_id: number }>) {
        this.edgeTargetIds = new Set(targets.map(t => t.target_id));
        this.renderLayers();
    }

    setSelected(id: number | null) {
        this.selectedId = id;
        this.renderLayers();
    }

    setOverlay(overlayType: string) {
        this.activeOverlay = overlayType;
        this.renderLayers();
    }

    // ── Selection highlight ─────────────────────────────────────────────────

    private createSelectionLayer() {
        if (this.selectedId === null || this.selectedId === undefined) return null;

        const infra = bridge.getAllInfrastructure();

        const node = infra.nodes.find(n => n.id === this.selectedId);
        if (node) {
            return new ScatterplotLayer({
                id: 'selection-highlight',
                data: [{ position: [node.x, node.y] }],
                getPosition: (d: any) => d.position,
                getFillColor: [255, 255, 255, 0],
                getLineColor: [255, 255, 255, 200],
                getLineWidth: 2,
                lineWidthUnits: 'pixels',
                stroked: true,
                filled: false,
                getRadius: 25000,
                parameters: { depthTest: false }
            });
        }

        const edge = infra.edges.find(e => e.id === this.selectedId);
        if (edge) {
            return new LineLayer({
                id: 'selection-highlight-edge',
                data: [edge],
                getSourcePosition: (d: any) => [d.src_x, d.src_y],
                getTargetPosition: (d: any) => [d.dst_x, d.dst_y],
                getColor: [255, 255, 100, 220],
                getWidth: 6,
                widthUnits: 'pixels',
                pickable: false,
                parameters: { depthTest: false }
            });
        }

        const city = this.cachedCities.find(c => c.id === this.selectedId);
        if (city) {
            return new ScatterplotLayer({
                id: 'selection-highlight',
                data: [{ position: [city.x, city.y] }],
                getPosition: (d: any) => d.position,
                getFillColor: [255, 255, 255, 0],
                getLineColor: [255, 255, 255, 200],
                getLineWidth: 2,
                lineWidthUnits: 'pixels',
                stroked: true,
                filled: false,
                getRadius: Math.log10(Math.max(city.population, 10)) * 25000,
                parameters: { depthTest: false }
            });
        }

        return null;
    }

    // ── Edge build highlights ───────────────────────────────────────────────

    private createEdgeBuildHighlights() {
        if (this.currentEdgeSourceId === null) return null;

        const infra = bridge.getAllInfrastructure();
        const layers: any[] = [];

        const sourceNode = infra.nodes.find(n => n.id === this.currentEdgeSourceId);
        if (sourceNode) {
            layers.push(new ScatterplotLayer({
                id: 'edge-source-ring',
                data: [{ position: [sourceNode.x, sourceNode.y] }],
                getPosition: (d: any) => d.position,
                getFillColor: [59, 130, 246, 40],
                getLineColor: [59, 130, 246, 255],
                getLineWidth: 3,
                lineWidthUnits: 'pixels',
                stroked: true,
                filled: true,
                getRadius: 35000,
                parameters: { depthTest: false }
            }));
        }

        if (this.edgeTargetIds.size > 0) {
            const validTargets = infra.nodes
                .filter(n => this.edgeTargetIds.has(n.id))
                .map(n => ({ position: [n.x, n.y], id: n.id }));

            if (validTargets.length > 0) {
                layers.push(new ScatterplotLayer({
                    id: 'edge-valid-targets',
                    data: validTargets,
                    getPosition: (d: any) => d.position,
                    getFillColor: [16, 185, 129, 30],
                    getLineColor: [16, 185, 129, 200],
                    getLineWidth: 2,
                    lineWidthUnits: 'pixels',
                    stroked: true,
                    filled: true,
                    getRadius: 25000,
                    pickable: false,
                    parameters: { depthTest: false }
                }));
            }
        }

        return layers;
    }

    // ── Labels ──────────────────────────────────────────────────────────────

    private createLabelsLayer() {
        if (this.currentZoom < 0.8) return null;

        const minPop = this.currentZoom < 1.5 ? 5000000
            : this.currentZoom < 2 ? 1000000
            : this.currentZoom < 3 ? 500000
            : this.currentZoom < 5 ? 100000
            : 0;
        const visibleCities = minPop > 0
            ? this.cachedCities.filter(c => c.population >= minPop)
            : this.cachedCities;

        return new TextLayer({
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
        });
    }

    private createRegionLabelsLayer() {
        if (this.currentZoom > 5) return null;

        return new TextLayer({
            id: 'region-labels',
            data: this.cachedRegions,
            getPosition: (d: Region) => [d.center_lon, d.center_lat],
            getText: (d: Region) => d.name,
            getSize: 18,
            getColor: [255, 255, 255, 80],
            getAlignmentBaseline: 'center',
            fontFamily: 'Inter, sans-serif',
            fontWeight: 'bold',
            parameters: { depthTest: false }
        });
    }

    // ── Pathfinding preview ─────────────────────────────────────────────────

    private createPathfindingPreviewLayer() {
        if (this.currentEdgeSourceId === null || !this.hoveredEntity || this.hoveredEntity.type !== 'node') {
            return null;
        }

        const allInfra = bridge.getAllInfrastructure();
        const sourceNode = allInfra.nodes.find(n => n.id === this.currentEdgeSourceId);
        const targetNode = allInfra.nodes.find(n => n.id === this.hoveredEntity.object?.id);

        if (!sourceNode || !targetNode || sourceNode.id === targetNode.id) return null;

        const srcCell = sourceNode.cell_index;
        const tgtCell = targetNode.cell_index;

        if (srcCell !== undefined && tgtCell !== undefined) {
            const edgeType = get(selectedEdgeType);
            if (needsTerrainRouting(edgeType)) {
                const path = this.pathfinder.findPath(srcCell, tgtCell, edgeType);
                return new PathLayer({
                    id: 'pathfinding-preview',
                    data: [{ path }],
                    getPath: (d: any) => d.path,
                    getColor: [234, 179, 8, 200],
                    getWidth: 3,
                    widthUnits: 'pixels',
                    dashJustified: true,
                    pickable: false,
                    getDashArray: [4, 2],
                });
            }
        }

        return new LineLayer({
            id: 'pathfinding-preview-line',
            data: [{ source: [sourceNode.x, sourceNode.y], target: [targetNode.x, targetNode.y] }],
            getSourcePosition: (d: any) => d.source,
            getTargetPosition: (d: any) => d.target,
            getColor: [234, 179, 8, 200],
            getWidth: 3,
            widthUnits: 'pixels',
            pickable: false
        });
    }

    // ── Tooltip / mouse handling ────────────────────────────────────────────

    handleMouseMove(e: MouseEvent) {
        if (!this.deck) return;

        const pickInfo = this.deck.pickObject({ x: e.offsetX, y: e.offsetY, radius: 2 });
        let type: string | null = null;
        let object: any = null;

        if (pickInfo && pickInfo.object && pickInfo.layer) {
            if (pickInfo.layer.id === 'infra-nodes' || pickInfo.layer.id === 'infra-nodes-fallback') type = 'node';
            else if (pickInfo.layer.id === 'infra-edges') type = 'edge';
            else if (pickInfo.layer.id === 'cities-icons' || pickInfo.layer.id === 'cities-dots-fallback') type = 'city';

            if (type) object = pickInfo.object;
        }

        if (type && object) {
            if (!this.hoveredEntity || this.hoveredEntity.object.id !== object.id) {
                this.hoveredEntity = { type, object };
                this.renderLayers();
            }

            let content = '';
            if (type === 'city') {
                const sat = object.infrastructure_satisfaction !== undefined
                    ? `\nSatisfaction: ${Math.round(object.infrastructure_satisfaction * 100)}%`
                    : '';
                const demand = object.telecom_demand !== undefined
                    ? `\nDemand: ${Math.round(object.telecom_demand)}`
                    : '';
                content = `${object.name}\nPopulation: ${object.population.toLocaleString()}${demand}${sat}`;
            }
            if (type === 'node') {
                const health = object.health !== undefined ? `\nHealth: ${Math.round(object.health * 100)}%` : '';
                const throughput = object.max_throughput ? `\nThroughput: ${Math.round(object.max_throughput)}` : '';
                const owner = object.owner_name ? `\nOwner: ${object.owner_name}` : '';
                const building = object.under_construction ? ' (building...)' : '';
                content = `${object.node_type}${building}\nUtil: ${Math.round((object.utilization || 0) * 100)}%${health}${throughput}${owner}`;
            }
            if (type === 'edge') {
                const bw = object.bandwidth ? `\nBandwidth: ${Math.round(object.bandwidth)}` : '';
                const load = object.current_load !== undefined ? `\nLoad: ${Math.round(object.current_load)}` : '';
                const health = object.health !== undefined ? `\nHealth: ${Math.round(object.health * 100)}%` : '';
                content = `${object.edge_type}\nLength: ${Math.round(object.length_km || 0)}km${bw}${load}${health}`;
            }

            tooltipData.set({ x: e.clientX, y: e.clientY, content });
        } else {
            if (this.hoveredEntity) {
                this.hoveredEntity = null;
                this.renderLayers();
            }
            tooltipData.set(null);
        }
    }

    dispose() {
        if (this.deck) {
            this.deck.finalize();
            this.deck = null;
        }
    }
}
