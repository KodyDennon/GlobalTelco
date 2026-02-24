// ── MapRenderer (modular orchestrator) ──────────────────────────────────────
// Uses MapLibre GL as the base map with deck.gl as an overlay via MapboxOverlay.
// Real earth: satellite tiles + vector borders via MapLibre, game layers via deck.gl.
// Procgen: blank dark canvas via MapLibre, terrain bitmap + borders + game layers via deck.gl.

import maplibregl from 'maplibre-gl';
import { MapboxOverlay } from '@deck.gl/mapbox';
import { BitmapLayer, LineLayer, ScatterplotLayer, TextLayer, PathLayer, IconLayer } from '@deck.gl/layers';
import type { Layer } from '@deck.gl/core';
import { CollisionFilterExtension } from '@deck.gl/extensions';
import { get } from 'svelte/store';

import * as bridge from '$lib/wasm/bridge';
import type { City, Region, GridCell, CellCoverage } from '$lib/wasm/types';

import { GridPathfinder, needsTerrainRouting } from '../GridPathfinder';
import { selectedEdgeType } from '$lib/stores/uiState';
import { annotations, routePlans, createAnnotationLayers } from '../AnnotationLayer';

import { createLandLayer } from './layers/landLayer';
import { createBordersLayer } from './layers/bordersLayer';
import { createInfraLayers } from './layers/infraLayer';
import type { IconMapping } from './iconAtlas';
import { buildIconAtlas } from './iconAtlas';
import { buildTerrainBitmap } from './terrainBitmap';
import { CORP_COLORS, SATELLITE_COLORS, TERRAIN_OVERLAY_COLORS } from './constants';
import { satelliteMapStyle, procgenMapStyle } from './tileConfig';
import { handleMapMouseMove, type TooltipHit } from './tooltip';
import { createWeatherLayers } from '../WeatherLayer';

// ── MapRenderer class ───────────────────────────────────────────────────────

export class MapRenderer {
    private map: maplibregl.Map | null = null;
    private overlay: MapboxOverlay | null = null;
    private container: HTMLElement;

    private pathfinder = new GridPathfinder();
    private cachedCells: GridCell[] = [];
    private cachedRegions: Region[] = [];
    private cachedCities: City[] = [];

    private activeOverlay: string = 'none';
    private quality: 'low' | 'medium' | 'high' = 'medium';
    private isRealEarth: boolean = false;

    private hoveredEntity: TooltipHit | null = null;
    private selectedId: number | null = null;
    private currentEdgeSourceId: number | null = null;
    private currentZoom: number = 2;
    private edgeTargetIds: Set<number> = new Set();

    private cellRadiusM: number = 120000;
    private cellSpacingKm: number = 120;

    // Animation loop state
    private currentTime: number = 0;
    private animationFrameId: number | null = null;
    private pitch: number = 0;
    private animationFrameCounter: number = 0;

    // Hovered node tracking
    private hoveredNodeId: number | null = null;

    // Weather/atmosphere state
    private gameTick: number = 0;
    private weatherEnabled: boolean = true;
    private dayNightCycle: boolean = true;

    // Custom event listener references (for cleanup)
    private mapPanHandler: ((e: Event) => void) | null = null;
    private mapZoomHandler: ((e: Event) => void) | null = null;
    private mapResetViewHandler: ((e: Event) => void) | null = null;
    private mapTogglePitchHandler: ((e: Event) => void) | null = null;

    // Pre-built assets
    private terrainCanvas: HTMLCanvasElement | null = null;
    private terrainOverlayCanvas: HTMLCanvasElement | null = null;
    private iconAtlas: HTMLCanvasElement | null = null;
    private iconMapping: Record<string, IconMapping> = {};
    private iconAtlasReady = false;

    // Promise that resolves when MapLibre GL style is loaded
    private mapReadyPromise: Promise<void>;

    constructor(container: HTMLElement, quality: 'low' | 'medium' | 'high' = 'medium') {
        this.quality = quality;
        this.container = container;

        // Determine world type before creating map
        this.isRealEarth = bridge.isInitialized() && bridge.isRealEarth();

        // Build icon atlas immediately
        const { canvas: atlasCanvas, mapping } = buildIconAtlas();
        this.iconAtlas = atlasCanvas;
        this.iconMapping = mapping;

        // Mark atlas ready after a tick (images load async)
        setTimeout(() => {
            this.iconAtlasReady = true;
            this.renderLayers();
        }, 500);

        // Create MapLibre GL map
        const style = this.isRealEarth ? satelliteMapStyle : procgenMapStyle;
        this.map = new maplibregl.Map({
            container,
            style,
            center: [0, 20],
            zoom: 2,
            minZoom: 0.5,
            maxZoom: 10,
            maxPitch: 60,
            attributionControl: false,
        });

        // Create deck.gl overlay (added to map after style loads)
        this.overlay = new MapboxOverlay({
            interleaved: false,
            layers: [],
        });

        // Track zoom changes
        this.map.on('zoom', () => {
            this.currentZoom = this.map!.getZoom();
        });

        // Globe projection toggle based on zoom level
        const setProjectionForZoom = (zoom: number) => {
            if (!this.map || !this.map.isStyleLoaded()) return;
            this.map.setProjection({ type: zoom < 2.5 ? 'globe' : 'mercator' } as any);
        };

        // Defer overlay + projection until style is loaded
        this.mapReadyPromise = new Promise<void>((resolve) => {
            this.map!.once('style.load', () => {
                this.map!.addControl(this.overlay as any);
                setProjectionForZoom(this.map!.getZoom());
                resolve();
            });
        });

        this.map.on('zoom', () => {
            setProjectionForZoom(this.map!.getZoom());
        });

        // Track pitch changes
        this.map.on('pitch', () => {
            this.pitch = this.map!.getPitch();
        });

        // Handle clicks on empty map area (no deck.gl pick)
        this.map.on('click', (e: maplibregl.MapMouseEvent) => {
            // Check if deck.gl picked something at this point
            if (this.overlay) {
                const pickInfo = this.overlay.pickObject({
                    x: e.point.x,
                    y: e.point.y,
                    radius: 4,
                });
                if (pickInfo && pickInfo.object) {
                    // deck.gl layer onClick handlers will handle this
                    return;
                }
            }

            // No entity picked — dispatch deselect and map-clicked events
            window.dispatchEvent(new CustomEvent('entity-selected', {
                detail: { id: null, type: null }
            }));
            window.dispatchEvent(new CustomEvent('map-clicked', {
                detail: { lon: e.lngLat.lng, lat: e.lngLat.lat }
            }));
        });

        // ── Animation loop for TripsLayer ────────────────────────────────
        const animate = () => {
            this.currentTime += 0.005;
            this.animationFrameCounter++;
            // Re-render every 3 frames to avoid excessive re-renders
            if (this.animationFrameCounter % 3 === 0) {
                this.renderLayers();
            }
            this.animationFrameId = requestAnimationFrame(animate);
        };
        this.animationFrameId = requestAnimationFrame(animate);

        // ── Map navigation event handlers (dispatched by KeyboardManager) ─
        this.mapPanHandler = (e: Event) => {
            if (!this.map) return;
            const { direction } = (e as CustomEvent).detail;
            const panOffset = 100; // pixels
            const offsets: Record<string, [number, number]> = {
                up: [0, -panOffset],
                down: [0, panOffset],
                left: [-panOffset, 0],
                right: [panOffset, 0],
            };
            const offset = offsets[direction];
            if (offset) {
                this.map.panBy(offset, { duration: 200 });
            }
        };

        this.mapZoomHandler = (e: Event) => {
            if (!this.map) return;
            const { direction } = (e as CustomEvent).detail;
            if (direction === 'in') {
                this.map.zoomIn({ duration: 200 });
            } else if (direction === 'out') {
                this.map.zoomOut({ duration: 200 });
            }
        };

        this.mapResetViewHandler = () => {
            if (!this.map) return;
            this.map.flyTo({ center: [0, 20], zoom: 2, pitch: 0, bearing: 0, duration: 1000 });
        };

        this.mapTogglePitchHandler = () => {
            if (!this.map) return;
            const currentPitch = this.map.getPitch();
            const targetPitch = currentPitch > 10 ? 0 : 45;
            this.map.easeTo({ pitch: targetPitch, duration: 500 });
        };

        window.addEventListener('map-pan', this.mapPanHandler);
        window.addEventListener('map-zoom', this.mapZoomHandler);
        window.addEventListener('map-reset-view', this.mapResetViewHandler);
        window.addEventListener('map-toggle-pitch', this.mapTogglePitchHandler);
    }

    // ── Map initialization ──────────────────────────────────────────────────

    async buildMap(): Promise<void> {
        if (!bridge.isInitialized()) return;

        // Wait for MapLibre GL style to finish loading before querying WASM data
        await this.mapReadyPromise;

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

        // Pre-render terrain bitmaps (used for procgen land layer and terrain overlay)
        this.terrainCanvas = buildTerrainBitmap(cells, this.cellSpacingKm, SATELLITE_COLORS);
        this.terrainOverlayCanvas = buildTerrainBitmap(cells, this.cellSpacingKm, TERRAIN_OVERLAY_COLORS);

        this.pathfinder.init(cells);
        this.renderLayers();
    }

    // ── Layer assembly ──────────────────────────────────────────────────────

    private renderLayers(): void {
        if (!this.overlay) return;

        const layers: (Layer | Layer[] | null)[] = [
            createLandLayer(this.terrainCanvas, this.isRealEarth),
            createBordersLayer(this.cachedRegions, this.isRealEarth),
            ...this.createOceanDepthLayers(),
            ...this.createOverlayLayers(),
            ...createInfraLayers({
                iconAtlas: this.iconAtlas,
                iconMapping: this.iconMapping,
                iconAtlasReady: this.iconAtlasReady,
                activeOverlay: this.activeOverlay,
                currentZoom: this.currentZoom,
                currentTime: this.currentTime,
                pitch: this.pitch,
                hoveredNodeId: this.hoveredNodeId,
            }),
            this.createCitiesLayer(),
            this.createLabelsLayer(),
            this.createRegionLabelsLayer(),
            ...createAnnotationLayers(get(annotations), get(routePlans)),
            ...createWeatherLayers({
                enabled: this.weatherEnabled,
                dayNightCycle: this.dayNightCycle,
                gameTick: this.gameTick,
                currentZoom: this.currentZoom,
            }),
            this.createSelectionLayer(),
            ...this.createEdgeBuildHighlights(),
            this.createPathfindingPreviewLayer(),
        ];

        const filtered = layers.flat().filter(Boolean) as Layer[];
        this.overlay.setProps({ layers: filtered });
    }

    // ── Cities ──────────────────────────────────────────────────────────────

    private createCitiesLayer(): Layer[] | null {
        const gtgCities = this.cachedCities.filter(c => Math.abs(c.y) <= 85).map(c => {
            let tier = 'hamlet';
            if (c.population > 5000000) tier = 'megalopolis';
            else if (c.population > 1000000) tier = 'metropolis';
            else if (c.population > 250000) tier = 'city';
            else if (c.population > 50000) tier = 'town';
            return { ...c, tier };
        });

        const layers: Layer[] = [
            new ScatterplotLayer({
                id: 'cities-glow',
                data: gtgCities,
                getPosition: (d: any) => [d.x, d.y],
                getFillColor: [255, 210, 120, 90],
                getRadius: (d: any) => Math.log10(Math.max(d.population, 10)) * 8000,
                radiusMinPixels: 2,
                pickable: false,
                parameters: {
                    blend: true,
                    blendFunc: [WebGLRenderingContext.SRC_ALPHA, WebGLRenderingContext.ONE],
                }
            })
        ];

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
                        window.dispatchEvent(new CustomEvent('entity-selected', {
                            detail: { id: object.id, type: 'city' }
                        }));
                    }
                }
            }));
        } else {
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
                        window.dispatchEvent(new CustomEvent('entity-selected', {
                            detail: { id: object.id, type: 'city' }
                        }));
                    }
                }
            }));
        }

        return layers;
    }

    // ── Ocean depth shading (procgen worlds only) ─────────────────────────
    // Ocean depth is rendered as part of the terrain bitmap (buildTerrainBitmap)
    // which already paints OceanShallow/OceanDeep/Ocean cells with distinct colors.
    // No separate layer needed — the bitmap provides smooth, gapless coverage.

    private createOceanDepthLayers(): Layer[] {
        return [];
    }

    // ── Overlays ────────────────────────────────────────────────────────────

    private createOverlayLayers(): Layer[] {
        const layers: Layer[] = [];
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
                        return [
                            Math.floor((1 - intensity) * 255),
                            Math.floor(intensity * 200),
                            50,
                            150
                        ] as [number, number, number, number];
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

    // ── Labels ──────────────────────────────────────────────────────────────

    private createLabelsLayer(): Layer | null {
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
            extensions: [new CollisionFilterExtension()],
            collisionEnabled: true,
            getCollisionPriority: (d: City) => d.population,
            parameters: { depthTest: false }
        } as any);
    }

    private createRegionLabelsLayer(): Layer | null {
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

    // ── Selection highlight ─────────────────────────────────────────────────

    private createSelectionLayer(): Layer | null {
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

    private createEdgeBuildHighlights(): Layer[] {
        if (this.currentEdgeSourceId === null) return [];

        const infra = bridge.getAllInfrastructure();
        const layers: Layer[] = [];

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

    // ── Pathfinding preview ─────────────────────────────────────────────────

    private createPathfindingPreviewLayer(): Layer | null {
        if (this.currentEdgeSourceId === null || !this.hoveredEntity || this.hoveredEntity.type !== 'node') {
            return null;
        }

        const allInfra = bridge.getAllInfrastructure();
        const sourceNode = allInfra.nodes.find(n => n.id === this.currentEdgeSourceId);
        const targetNode = allInfra.nodes.find(n => n.id === this.hoveredEntity!.object?.id);

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
            data: [{
                source: [sourceNode.x, sourceNode.y],
                target: [targetNode.x, targetNode.y]
            }],
            getSourcePosition: (d: any) => d.source,
            getTargetPosition: (d: any) => d.target,
            getColor: [234, 179, 8, 200],
            getWidth: 3,
            widthUnits: 'pixels',
            pickable: false
        });
    }

    // ── Public API ──────────────────────────────────────────────────────────

    updateInfrastructure(): void {
        if (!bridge.isInitialized()) return;
        this.renderLayers();
    }

    updateCities(): void {
        if (!bridge.isInitialized()) return;
        this.cachedCities = bridge.getCities();
        this.renderLayers();
    }

    highlightEdgeSource(nodeId: number | null): void {
        this.currentEdgeSourceId = nodeId;
        if (nodeId === null) {
            this.edgeTargetIds.clear();
        }
        this.renderLayers();
    }

    setEdgeTargets(targets: Array<{ target_id: number }>): void {
        this.edgeTargetIds = new Set(targets.map(t => t.target_id));
        this.renderLayers();
    }

    setSelected(id: number | null): void {
        this.selectedId = id;
        this.renderLayers();
    }

    setOverlay(overlayType: string): void {
        this.activeOverlay = overlayType;
        this.renderLayers();
    }

    setGameTick(tick: number): void {
        this.gameTick = tick;
    }

    setWeatherEnabled(enabled: boolean): void {
        this.weatherEnabled = enabled;
        this.renderLayers();
    }

    setDayNightCycle(enabled: boolean): void {
        this.dayNightCycle = enabled;
        this.renderLayers();
    }

    handleMouseMove(e: MouseEvent): void {
        if (!this.overlay) return;

        this.hoveredEntity = handleMapMouseMove(
            e,
            (opts) => this.overlay!.pickObject(opts),
            this.hoveredEntity,
            () => this.renderLayers()
        );

        // Track hovered node ID for infraLayer highlights
        if (this.hoveredEntity && this.hoveredEntity.type === 'node') {
            this.hoveredNodeId = this.hoveredEntity.object?.id ?? null;
        } else {
            this.hoveredNodeId = null;
        }
    }

    dispose(): void {
        // Cancel animation frame
        if (this.animationFrameId !== null) {
            cancelAnimationFrame(this.animationFrameId);
            this.animationFrameId = null;
        }

        // Remove custom event listeners
        if (this.mapPanHandler) {
            window.removeEventListener('map-pan', this.mapPanHandler);
            this.mapPanHandler = null;
        }
        if (this.mapZoomHandler) {
            window.removeEventListener('map-zoom', this.mapZoomHandler);
            this.mapZoomHandler = null;
        }
        if (this.mapResetViewHandler) {
            window.removeEventListener('map-reset-view', this.mapResetViewHandler);
            this.mapResetViewHandler = null;
        }
        if (this.mapTogglePitchHandler) {
            window.removeEventListener('map-toggle-pitch', this.mapTogglePitchHandler);
            this.mapTogglePitchHandler = null;
        }

        if (this.overlay) {
            this.overlay.finalize();
            this.overlay = null;
        }
        if (this.map) {
            this.map.remove();
            this.map = null;
        }
    }
}
