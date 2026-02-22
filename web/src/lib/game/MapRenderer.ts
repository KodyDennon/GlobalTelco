import { Deck } from '@deck.gl/core';
import { ArcLayer, ScatterplotLayer, TextLayer, PathLayer, IconLayer } from '@deck.gl/layers';

import * as bridge from '$lib/wasm/bridge';
import type { GridCell, City, Region, CorpSummary, CellCoverage, TrafficFlows } from '$lib/wasm/types';

import { GridPathfinder, needsTerrainRouting } from './GridPathfinder';
import { tooltipData } from '$lib/stores/uiState';
import { icons } from '$lib/assets/icons';

// Premium Dark Mode / Neon colors
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

const EDGE_STYLES: Record<string, { color: [number, number, number], width: number, dashed?: boolean }> = {
    FiberLocal: { color: [34, 211, 160], width: 2 },
    FiberRegional: { color: [96, 165, 250], width: 3 },
    FiberNational: { color: [129, 140, 248], width: 5 },
    Copper: { color: [217, 119, 6], width: 1 },
    Microwave: { color: [34, 211, 238], width: 2, dashed: true },
    Satellite: { color: [251, 191, 36], width: 3, dashed: true },
    Submarine: { color: [59, 130, 246], width: 5 }
};

// Helper to convert raw SVG to data URI for Deck.gl IconLayer (UTF-8 safe)
function svgToDataUri(svg: string): string {
    return `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`;
}

const ICON_MAPPING = Object.fromEntries(
    Object.entries(icons).map(([name, svg]) => [
        name,
        {
            url: svgToDataUri(svg as string),
            width: 64,
            height: 64,
            mask: false
        }
    ])
);

// Tier-based sizing: higher tiers = more visually prominent nodes
const NODE_TIER_SIZE: Record<string, number> = {
    Local: 24,
    Regional: 32,
    National: 40,
    Continental: 48,
    Global: 56
};

const EDGE_TIER_WIDTH: Record<string, number> = {
    FiberLocal: 2,
    FiberRegional: 3,
    FiberNational: 5,
    Copper: 1,
    Microwave: 2,
    Satellite: 4,
    Submarine: 6
};

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

    constructor(container: HTMLElement, quality: 'low' | 'medium' | 'high' = 'medium') {
        this.quality = quality;
        this.container = container;

        // Initialize Deck.gl
        this.deck = new Deck({
            parent: container as HTMLDivElement,
            initialViewState: {
                longitude: 0,
                latitude: 20,
                zoom: 2,
                pitch: 0,
                bearing: 0,
                minZoom: 1,
                maxZoom: 12,
                maxPitch: 60
            },
            controller: true,
            layers: [],
            getTooltip: () => null, // We handle this manually in Svelte using UI State
            onClick: (info) => {
                // If clicked on nothing (ocean/background), deselect
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
            this.createPathfindingPreviewLayer()
        ].filter(Boolean).flat() as any[];

        this.deck.setProps({ layers });
    }

    private createLandLayer() {
        // We can use a ScatterplotLayer to render the hex cells as a base map
        // For a more performant approach with many cells, we could use GridCellLayer or SolidPolygonLayer
        return new ScatterplotLayer({
            id: 'land-layer',
            data: this.cachedCells.filter(c => c.terrain !== 'Ocean' && c.terrain !== 'OceanDeep' && Math.abs(c.lat) <= 85),
            getPosition: (d: GridCell) => [d.lon, d.lat],
            getFillColor: [20, 30, 45, 200], // Dark base color
            getRadius: 80000, // Roughly cell size in meters
            pickable: true,
            onClick: (info: any) => {
                // Deselect active entity
                window.dispatchEvent(new CustomEvent('entity-selected', { detail: { id: null, type: null } }));

                if (info.coordinate) {
                    // Dispatch map click with exact coordinates (free placement)
                    const [lon, lat] = info.coordinate;
                    const event = new CustomEvent('map-clicked', { detail: { lon, lat } });
                    window.dispatchEvent(event);
                }
            }
        });
    }

    private createBordersLayer() {
        if (bridge.isRealEarth()) return null;

        const borderData = this.cachedRegions.filter(r => r.boundary_polygon?.length > 2).map(r => ({
            polygon: r.boundary_polygon.map(p => [p[0], p[1]]),
            name: r.name
        }));

        return new PathLayer({
            id: 'region-borders',
            data: borderData,
            getPath: (d: any) => d.polygon,
            getColor: [100, 110, 140, 150],
            getWidth: 2,
            widthUnits: 'pixels'
        });
    }

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
                    icon: node.node_type.toLowerCase().replace(/_/g, '-'),
                    tierSize: NODE_TIER_SIZE[node.network_level] || 32
                });
            }
        }

        return [
            new ArcLayer({
                id: 'infra-edges',
                data: edgesData,
                getSourcePosition: (d: any) => d.sourcePosition,
                getTargetPosition: (d: any) => d.targetPosition,
                getSourceColor: (d: any) => d.color,
                getTargetColor: (d: any) => d.color,
                getWidth: (d: any) => d.width,
                pickable: true,
                autoHighlight: true,
                onClick: ({ object }: any) => {
                    if (object) {
                        window.dispatchEvent(new CustomEvent('entity-selected', { detail: { id: object.id, type: 'edge' } }));
                    }
                }
            }),
            new IconLayer({
                id: 'infra-nodes',
                data: nodesData,
                getPosition: (d: any) => d.position,
                getIcon: (d: any) => d.icon,
                iconAtlas: null, // We use individual data URIs
                iconMapping: ICON_MAPPING,
                getSize: (d: any) => d.tierSize,
                sizeMinPixels: 12,
                sizeMaxPixels: 72,
                getColor: (d: any) => d.color,
                pickable: true,
                autoHighlight: true,
                onClick: ({ object }: any) => {
                    if (object) {
                        const event = new CustomEvent('entity-selected', { detail: { id: object.id, type: 'node' } });
                        window.dispatchEvent(event);
                    }
                }
            })
        ];
    }

    private createCitiesLayer() {
        const gtgCities = this.cachedCities.filter(c => Math.abs(c.y) <= 85).map(c => {
            let tier = 'hamlet';
            if (c.population > 5000000) tier = 'megalopolis';
            else if (c.population > 1000000) tier = 'metropolis';
            else if (c.population > 250000) tier = 'city';
            else if (c.population > 50000) tier = 'town';
            return { ...c, tier };
        });

        return [
            new ScatterplotLayer({
                id: 'cities-glow',
                data: gtgCities,
                getPosition: (d: any) => [d.x, d.y],
                getFillColor: [255, 230, 150, 100], // Glow color
                getRadius: (d: any) => Math.log10(Math.max(d.population, 10)) * 15000,
                pickable: false,
                parameters: {
                    blend: true,
                    blendFunc: [WebGLRenderingContext.SRC_ALPHA, WebGLRenderingContext.ONE], // Additive blending
                }
            }),
            new IconLayer({
                id: 'cities-icons',
                data: gtgCities,
                getPosition: (d: any) => [d.x, d.y],
                getIcon: (d: any) => d.tier,
                iconAtlas: null,
                iconMapping: ICON_MAPPING,
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
            })
        ];
    }

    private createOverlayLayers() {
        const layers = [];

        if (this.activeOverlay === 'demand') {
            layers.push(new ScatterplotLayer({
                id: 'overlay-demand',
                data: this.cachedRegions,
                getPosition: (d: Region) => [d.center_lon, d.center_lat],
                getFillColor: (d: Region) => {
                    const pop = d.population ?? 0;
                    const intensity = Math.min(1.0, pop / 500000);
                    const color: [number, number, number, number] = [Math.floor(intensity * 255), 50, Math.floor((1 - intensity) * 255), 115];
                    return color;
                },
                getRadius: (d: Region) => Math.sqrt(d.cell_count) * 80000 * 0.4,
                pickable: false,
                parameters: { depthTest: false }
            }));
        }

        if (this.activeOverlay === 'disaster') {
            layers.push(new ScatterplotLayer({
                id: 'overlay-disaster',
                data: this.cachedRegions,
                getPosition: (d: Region) => [d.center_lon, d.center_lat],
                getFillColor: (d: Region) => {
                    const risk = d.disaster_risk ?? 0;
                    const intensity = Math.min(1.0, risk * 5);
                    const color: [number, number, number, number] = [Math.floor(intensity * 255), Math.floor((1 - intensity) * 180), 0, 115];
                    return color;
                },
                getRadius: (d: Region) => Math.sqrt(d.cell_count) * 80000 * 0.4,
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
                        const color: [number, number, number, number] = [Math.floor((1 - intensity) * 255), Math.floor(intensity * 200), 50, 150];
                        return color;
                    },
                    getRadius: 80000 * 1.05,
                    pickable: false,
                    parameters: { depthTest: false }
                }));
            }
        }

        if (this.activeOverlay === 'terrain') {
            const TERRAIN_COLORS: Record<string, [number, number, number]> = {
                Urban: [80, 80, 90],
                Suburban: [65, 70, 65],
                Rural: [40, 60, 40],
                Mountainous: [60, 60, 65],
                Desert: [70, 65, 45],
                Coastal: [45, 65, 70],
                OceanShallow: [15, 30, 55],
                OceanDeep: [5, 10, 25],
                Tundra: [60, 70, 75],
                Frozen: [75, 80, 85],
            };

            layers.push(new ScatterplotLayer({
                id: 'overlay-terrain',
                data: this.cachedCells,
                getPosition: (d: GridCell) => [d.lon, d.lat],
                getFillColor: (d: GridCell) => {
                    const color = TERRAIN_COLORS[d.terrain] || [50, 50, 50];
                    return [...color, 255] as [number, number, number, number];
                },
                getRadius: 85000,
                pickable: false
            }));
        }

        if (this.activeOverlay === 'ownership') {
            if (bridge.isInitialized()) {
                const coverageData = bridge.getCellCoverage(); // Use dominant owner from coverage
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
                    getRadius: 85000,
                    pickable: false
                }));
            }
        }

        return layers;
    }

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
        this.renderLayers();
    }

    setSelected(id: number | null) {
        this.selectedId = id;
        this.renderLayers();
    }

    private createSelectionLayer() {
        if (this.selectedId === null || this.selectedId === undefined) return null;

        const infra = bridge.getAllInfrastructure();
        const layers: any[] = [];

        // Search nodes
        const node = infra.nodes.find(n => n.id === this.selectedId);
        if (node) {
            layers.push(new ScatterplotLayer({
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
            }));
            return layers;
        }

        // Search edges — highlight with a brighter arc
        const edge = infra.edges.find(e => e.id === this.selectedId);
        if (edge) {
            layers.push(new ArcLayer({
                id: 'selection-highlight-edge',
                data: [edge],
                getSourcePosition: (d: any) => [d.src_x, d.src_y],
                getTargetPosition: (d: any) => [d.dst_x, d.dst_y],
                getSourceColor: [255, 255, 100, 220],
                getTargetColor: [255, 255, 100, 220],
                getWidth: 6,
                pickable: false,
                parameters: { depthTest: false }
            }));
            return layers;
        }

        // Search cities
        const city = this.cachedCities.find(c => c.id === this.selectedId);
        if (city) {
            layers.push(new ScatterplotLayer({
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
            }));
            return layers;
        }

        return null;
    }

    private createLabelsLayer() {
        if (this.currentZoom < 2) return null;

        // At lower zoom, only show major cities
        const minPop = this.currentZoom < 3 ? 500000 : this.currentZoom < 5 ? 100000 : 0;
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
            getColor: [255, 255, 255, 100],
            getAlignmentBaseline: 'center',
            fontFamily: 'Inter, sans-serif',
            fontWeight: 'bold',
            parameters: { depthTest: false }
        });
    }

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

        // Use terrain-aware pathfinding if both nodes have cell associations
        if (srcCell !== undefined && tgtCell !== undefined) {
            const edgeType = 'FiberLocal'; // hardcode for preview right now, could pull from uiState
            if (needsTerrainRouting(edgeType)) {
                const path = this.pathfinder.findPath(srcCell, tgtCell, edgeType);
                return new PathLayer({
                    id: 'pathfinding-preview',
                    data: [{ path }],
                    getPath: d => d.path,
                    getColor: [234, 179, 8, 200], // Yellow pulsing preview
                    getWidth: 3,
                    widthUnits: 'pixels',
                    dashJustified: true,
                    pickable: false,
                    getDashArray: [4, 2],
                    // extensions: [new PathStyleExtension({dash: true})] // requires additional import if we want true dashes
                });
            }
        }

        // Fallback or straight line edges (Microwave, Satellite)
        return new ArcLayer({
            id: 'pathfinding-preview-arc',
            data: [{ source: [sourceNode.x, sourceNode.y], target: [targetNode.x, targetNode.y] }],
            getSourcePosition: d => d.source,
            getTargetPosition: d => d.target,
            getSourceColor: [234, 179, 8, 200],
            getTargetColor: [234, 179, 8, 200],
            getWidth: 3,
            widthUnits: 'pixels',
            pickable: false
        });
    }

    setOverlay(overlayType: string) {
        this.activeOverlay = overlayType;
        this.renderLayers();
    }

    handleMouseMove(e: MouseEvent) {
        if (this.deck) {
            const pickInfo = this.deck.pickObject({ x: e.offsetX, y: e.offsetY, radius: 2 });
            let type: string | null = null;
            let object: any = null;

            if (pickInfo && pickInfo.object && pickInfo.layer) {
                if (pickInfo.layer.id === 'infra-nodes') type = 'node';
                else if (pickInfo.layer.id === 'infra-edges') type = 'edge';
                else if (pickInfo.layer.id === 'cities-icons') type = 'city';

                if (type) {
                    object = pickInfo.object;
                }
            }

            if (type && object) {
                if (!this.hoveredEntity || this.hoveredEntity.object.id !== object.id) {
                    this.hoveredEntity = { type, object };
                    this.renderLayers(); // re-render to show preview
                }

                // Show Svelte glassmorphism tooltip
                let content = '';
                if (type === 'city') content = `🏙️ ${object.name}\nPop: ${object.population.toLocaleString()}`;
                if (type === 'node') content = `📡 ${object.node_type}\nStatus: ${object.status || 'Active'}\nUtil: ${Math.round((object.utilization || 0) * 100)}%`;
                if (type === 'edge') content = `🔗 ${object.edge_type}\nLength: ${Math.round(object.length_km || 0)}km`;

                tooltipData.set({
                    x: e.clientX,
                    y: e.clientY,
                    content
                });
            } else {
                if (this.hoveredEntity) {
                    this.hoveredEntity = null;
                    this.renderLayers();
                }
                tooltipData.set(null);
            }
        }
    }

    dispose() {
        if (this.deck) {
            this.deck.finalize();
            this.deck = null;
        }
    }
}
