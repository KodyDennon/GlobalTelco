import { Deck } from '@deck.gl/core';
import { GeoJsonLayer, ArcLayer, ScatterplotLayer, TextLayer, PathLayer } from '@deck.gl/layers';

import * as bridge from '$lib/wasm/bridge';
import type { GridCell, City, Region, CorpSummary, CellCoverage, TrafficFlows } from '$lib/wasm/types';
import type { IconName } from '$lib/assets/icons';
import { GridPathfinder, needsTerrainRouting } from './GridPathfinder';
import { tooltipData } from '$lib/stores/uiState';

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
            onViewStateChange: ({ viewState }) => {
                // Update tooltips or LOD based on zoom if needed
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

        const layers = [
            this.createLandLayer(),
            this.createBordersLayer(),
            ...this.createOverlayLayers(),
            ...this.createInfrastructureLayers(),
            this.createCitiesLayer(),
            this.createPathfindingPreviewLayer()
        ].filter(Boolean);

        this.deck.setProps({ layers });
    }

    private createLandLayer() {
        // We can use a ScatterplotLayer to render the hex cells as a base map
        // For a more performant approach with many cells, we could use GridCellLayer or SolidPolygonLayer
        return new ScatterplotLayer({
            id: 'land-layer',
            data: this.cachedCells.filter(c => c.terrain !== 'Ocean' && c.terrain !== 'OceanDeep' && Math.abs(c.lat) <= 85),
            getPosition: d => [d.lon, d.lat],
            getFillColor: d => [20, 30, 45, 200], // Dark base color
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
            getPath: d => d.polygon,
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
                    color: [...color, opacity]
                });
            }
        }

        return [
            new ArcLayer({
                id: 'infra-edges',
                data: edgesData,
                getSourcePosition: d => d.sourcePosition,
                getTargetPosition: d => d.targetPosition,
                getSourceColor: d => d.color,
                getTargetColor: d => d.color,
                getWidth: d => d.width,
                pickable: true,
                autoHighlight: true,
                onClick: ({ object }) => {
                    if (object) {
                        window.dispatchEvent(new CustomEvent('entity-selected', { detail: { id: object.id, type: 'edge' } }));
                    }
                }
            }),
            new ScatterplotLayer({
                id: 'infra-nodes',
                data: nodesData,
                getPosition: d => d.position,
                getFillColor: d => d.color,
                getRadius: 10000,
                radiusMinPixels: 4,
                radiusMaxPixels: 20,
                pickable: true,
                autoHighlight: true,
                onClick: ({ object }) => {
                    if (object) {
                        const event = new CustomEvent('entity-selected', { detail: { id: object.id, type: 'node' } });
                        window.dispatchEvent(event);
                    }
                }
            })
        ];
    }

    private createCitiesLayer() {
        const gtgCities = this.cachedCities.filter(c => Math.abs(c.y) <= 85);

        return new ScatterplotLayer({
            id: 'cities-glow',
            data: gtgCities,
            getPosition: d => [d.x, d.y],
            getFillColor: [255, 230, 150, 100], // Glow color
            getRadius: d => Math.log10(Math.max(d.population, 10)) * 15000,
            pickable: true,
            autoHighlight: true,
            onClick: ({ object }) => {
                if (object) {
                    window.dispatchEvent(new CustomEvent('entity-selected', { detail: { id: object.id, type: 'city' } }));
                }
            },
            parameters: {
                blend: true,
                blendFunc: [WebGLRenderingContext.SRC_ALPHA, WebGLRenderingContext.ONE], // Additive blending
            }
        });
    }

    private createOverlayLayers() {
        const layers = [];

        if (this.activeOverlay === 'demand') {
            layers.push(new ScatterplotLayer({
                id: 'overlay-demand',
                data: this.cachedRegions,
                getPosition: d => [d.center_lon, d.center_lat],
                getFillColor: d => {
                    const pop = d.population ?? 0;
                    const intensity = Math.min(1.0, pop / 500000);
                    return [Math.floor(intensity * 255), 50, Math.floor((1 - intensity) * 255), 115];
                },
                getRadius: d => Math.sqrt(d.cell_count) * 80000 * 0.4,
                pickable: false,
                parameters: { depthTest: false }
            }));
        }

        if (this.activeOverlay === 'disaster') {
            layers.push(new ScatterplotLayer({
                id: 'overlay-disaster',
                data: this.cachedRegions,
                getPosition: d => [d.center_lon, d.center_lat],
                getFillColor: d => {
                    const risk = d.disaster_risk ?? 0;
                    const intensity = Math.min(1.0, risk * 5);
                    return [Math.floor(intensity * 255), Math.floor((1 - intensity) * 180), 0, 115];
                },
                getRadius: d => Math.sqrt(d.cell_count) * 80000 * 0.4,
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
                    getPosition: d => [d.lon, d.lat],
                    getFillColor: d => {
                        const intensity = Math.min(1.0, d.signal_strength / 100);
                        return [Math.floor((1 - intensity) * 255), Math.floor(intensity * 200), 50, 150];
                    },
                    getRadius: 80000 * 1.05,
                    pickable: false,
                    parameters: { depthTest: false }
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

    private createPathfindingPreviewLayer() {
        if (this.currentEdgeSourceId === null || !this.hoveredEntity || this.hoveredEntity.type !== 'node') {
            return null;
        }

        const sourceNode = bridge.getAllInfrastructure().nodes.find(n => n.id === this.currentEdgeSourceId);
        const targetNode = this.hoveredEntity.object; // The node we're hovering over

        if (!sourceNode || !targetNode || sourceNode.id === targetNode.id) return null;

        const srcCell = (sourceNode as any).cell;
        const tgtCell = (targetNode as any).cell;

        // Ensure nodes have cell associations (assuming they do in WASM, but fallback to direct line if not)
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
                else if (pickInfo.layer.id === 'cities-glow') type = 'city';

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
