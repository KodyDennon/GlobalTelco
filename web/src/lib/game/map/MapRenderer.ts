// ── MapRenderer (modular orchestrator) ──────────────────────────────────────
// Uses MapLibre GL as the base map with deck.gl as an overlay via MapboxOverlay.
// Real earth: satellite tiles + vector borders via MapLibre, game layers via deck.gl.
// Procgen: blank dark canvas via MapLibre, terrain bitmap + borders + game layers via deck.gl.

import maplibregl from 'maplibre-gl';
import { MapboxOverlay } from '@deck.gl/mapbox';
import { BitmapLayer, LineLayer, ScatterplotLayer, TextLayer, PathLayer, IconLayer, PolygonLayer } from '@deck.gl/layers';
import type { Layer } from '@deck.gl/core';
import { CollisionFilterExtension } from '@deck.gl/extensions';
import { get } from 'svelte/store';

import * as bridge from '$lib/wasm/bridge';
import type { City, Region, GridCell, CellCoverage } from '$lib/wasm/types';
import { dataStore } from './DataStore';

import { GridPathfinder, needsTerrainRouting } from '../GridPathfinder';
import { selectedEdgeType } from '$lib/stores/uiState';
import { annotations, routePlans, createAnnotationLayers } from '../AnnotationLayer';

import { createLandLayer } from './layers/landLayer';
import { createBordersLayer } from './layers/bordersLayer';
import { createInfraLayers } from './layers/infraLayer';
import { createVectorTerrainLayers, buildVectorTerrainData, disposeVectorTerrainData } from './layers/vectorTerrainLayer';
import { createOceanDepthLayers as createOceanDepthLayersFn, buildOceanDepthData, disposeOceanDepthData } from './layers/oceanDepthLayer';
import { createRoadsLayers, buildRoadData, disposeRoadData } from './layers/roadsLayer';
import { createBuildingsLayers, buildBuildingData, disposeBuildingData, updateBuildingCoverage, setRealEarthMode } from './layers/buildingsLayer';
import { createRiversLayers, buildRiverData, disposeRiverData } from './layers/riversLayer';
import { createTerrainDetailLayers, buildTerrainDetailData, disposeTerrainDetailData } from './layers/terrainDetailLayer';
import { createElevationContourLayers, buildElevationData, disposeElevationData } from './layers/elevationLayer';
import { createCityGlowLayer } from './layers/cityGlowLayer';
import { createCableGlowLayers } from './layers/cableGlowLayer';
import { createSubmarineCableRefLayers } from './layers/submarineCableRefLayer';
import type { IconMapping } from './iconAtlas';
import { buildIconAtlas } from './iconAtlas';
import { buildTerrainBitmapAsync, disposeTerrainWorker } from './terrainBitmap';
import { CORP_COLORS, SATELLITE_COLORS, TERRAIN_OVERLAY_COLORS, NODE_TIER_SIZE } from './constants';
import { satelliteMapStyle, procgenMapStyle } from './tileConfig';
import { handleMapMouseMove, setTooltipDisasters, type TooltipHit } from './tooltip';
import { createWeatherLayers, computeDisasterForecasts, convertWeatherForecasts, type ActiveDisaster, type ForecastDisaster } from '../WeatherLayer';
import { createCablePreviewLayers, type CableDrawingState } from './layers/cablePreviewLayer';
import { createWaypointEditorLayers, type WaypointEditorState } from './layers/waypointEditorLayer';
import { createOverlayLayers as createSpectrumAndOtherOverlays, createDensityOverlayLayers } from './layers/overlayLayers';
import { createSatelliteLayers } from './layers/satelliteLayer';
import { selectedBuildItem, buildCategory, buildMode, ghostPreviewInfo, TERRAIN_COST_MULTIPLIERS, viewport, zoomLevel } from '$lib/stores/uiState';
import type { BuildOption } from '$lib/wasm/types';

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
    private activeDisasters: ActiveDisaster[] = [];
    private forecastDisasters: ForecastDisaster[] = [];
    private lastForecastTick: number = -1;

    // Cable drawing preview state
    private cableDrawingState: CableDrawingState = {
        waypoints: [],
        cursorPosition: null,
        deployment: 'Underground',
        sourceNodePos: null,
        isDrawing: false,
    };

    // Waypoint editor state (post-build waypoint editing)
    private waypointEditorState: WaypointEditorState = {
        editing: false,
        edgeId: null,
        waypoints: [],
        draggingIndex: null,
        cursorPosition: null,
    };

    // Ghost preview state (cursor-following placement preview)
    private ghostPreviewPosition: [number, number] | null = null;

    // Cursor position for ghost preview layer
    private cursorLon: number = 0;
    private cursorLat: number = 0;
    private cursorOnMap: boolean = false;

    // Cached ghost preview validation
    private ghostTerrainType: string | null = null;
    private ghostValid: boolean = true;
    private ghostCost: number | null = null;
    private ghostBuildOptions: BuildOption[] = [];

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

    // Whether buildMap has completed
    private mapBuilt = false;

    constructor(container: HTMLElement, quality: 'low' | 'medium' | 'high' = 'medium') {
        this.quality = quality;
        this.container = container;

        // Determine world type before creating map
        this.isRealEarth = bridge.isInitialized() && bridge.isRealEarth();
        setRealEarthMode(this.isRealEarth);

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
            zoomLevel.set(this.currentZoom);
            setProjectionForZoom(this.currentZoom);
        });

        this.map.on('move', () => {
            if (!this.map) return;
            const b = this.map.getBounds();
            viewport.set({
                minX: b.getWest(),
                minY: b.getSouth(),
                maxX: b.getEast(),
                maxY: b.getNorth()
            });
        });

        // Expose camera state for BookmarkManager
        this.map.on('moveend', () => {
            if (!this.map) return;
            const center = this.map.getCenter();
            (window as any).__gtCamera = { lon: center.lng, lat: center.lat, zoom: this.map.getZoom() };
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

        // Double-click: dispatch for cable drawing completion
        this.map.on('dblclick', (e: maplibregl.MapMouseEvent) => {
            // Check if deck.gl picked a node at this point
            let pickedNode: any = null;
            if (this.overlay) {
                const pickInfo = this.overlay.pickObject({
                    x: e.point.x,
                    y: e.point.y,
                    radius: 8,
                });
                if (pickInfo && pickInfo.object && pickInfo.object.id !== undefined) {
                    pickedNode = pickInfo.object;
                }
            }
            window.dispatchEvent(new CustomEvent('map-dblclick', {
                detail: {
                    lon: e.lngLat.lng,
                    lat: e.lngLat.lat,
                    screenX: e.point.x,
                    screenY: e.point.y,
                    pickedNode,
                }
            }));
        });

        // Right-click: open radial build menu at cursor position
        this.map.on('contextmenu', (e: maplibregl.MapMouseEvent) => {
            e.preventDefault();
            // Calculate screen position relative to the window (not the map container)
            const rect = container.getBoundingClientRect();
            window.dispatchEvent(new CustomEvent('map-contextmenu', {
                detail: {
                    screenX: rect.left + e.point.x,
                    screenY: rect.top + e.point.y,
                    lon: e.lngLat.lng,
                    lat: e.lngLat.lat,
                }
            }));
        });

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

        // NOTE: Animation loop is NOT started here. It starts after buildMap()
        // completes, so we don't waste frames rendering empty layers.
    }

    // ── Map initialization ──────────────────────────────────────────────────

    async buildMap(): Promise<void> {
        if (!bridge.isInitialized()) return;

        // Wait for MapLibre GL style to finish loading before querying WASM data
        await this.mapReadyPromise;

        // Query WASM for cell/city/region data
        const cells = bridge.getGridCells();
        this.cachedCities = bridge.getCities();
        this.cachedRegions = bridge.getRegions();
        this.cachedCells = cells;

        // Initialize DataStore for binary infrastructure data
        await dataStore.init();

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

        // Run terrain build, icon atlas, and pathfinder init in parallel.
        // For procgen mode: build vector terrain polygons (fast geometry, no worker needed).
        // For real earth mode: build bitmap terrain (worker-offloaded Gaussian splat).
        if (this.isRealEarth) {
            const [terrainResult, iconResult] = await Promise.all([
                buildTerrainBitmapAsync(cells, this.cellSpacingKm, SATELLITE_COLORS, this.quality),
                buildIconAtlas(),
            ]);
            this.terrainCanvas = terrainResult;
            this.iconAtlas = iconResult.canvas;
            this.iconMapping = iconResult.mapping;
            this.iconAtlasReady = true;
        } else {
            // Procgen: vector terrain polygons + icon atlas in parallel
            const iconResult = await buildIconAtlas();
            buildVectorTerrainData(cells);
            // Build ocean depth data from cached vector terrain polygons
            buildOceanDepthData();
            // Build river/lake/coastline glow, terrain detail, and elevation contour data
            buildRiverData(cells);
            buildTerrainDetailData();
            buildElevationData();
            this.iconAtlas = iconResult.canvas;
            this.iconMapping = iconResult.mapping;
            this.iconAtlasReady = true;
        }

        // Pathfinder init is fast (<10ms) — runs after await to avoid race conditions
        this.pathfinder.init(cells);

        // Build road network and building footprint data (both modes)
        buildRoadData(this.cachedCities, this.cachedRegions);
        buildBuildingData(this.cachedCities);

        // Mark map as ready and do initial render
        this.mapBuilt = true;
        this.renderLayers();

        // Start the animation loop now that we have content to render
        this.startAnimationLoop();
    }

    // ── Animation loop (started after buildMap) ─────────────────────────────

    private startAnimationLoop(): void {
        if (this.animationFrameId !== null) return; // already running

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
    }

    // ── Layer assembly ──────────────────────────────────────────────────────

    private renderLayers(): void {
        if (!this.overlay || !this.mapBuilt) return;

        // Terrain base: vector polygons for procgen, bitmap for real earth
        const terrainLayers: (Layer | Layer[] | null)[] = this.isRealEarth
            ? [createLandLayer(this.terrainCanvas, this.isRealEarth)]
            : [createVectorTerrainLayers()];

        const layers: (Layer | Layer[] | null)[] = [
            // 1. Terrain (bitmap or vector polygons + coastlines)
            ...terrainLayers,
            // 2. Ocean depth (procgen)
            ...this.createOceanDepthLayers(),
            // 2b. Terrain detail patterns (mountain contours, desert bands, urban grid — procgen zoom 4+)
            ...createTerrainDetailLayers(this.currentZoom, this.isRealEarth),
            // 2c. Elevation contour overlay (procgen zoom 3+, toggle-able)
            ...createElevationContourLayers(
                this.activeOverlay === 'elevation_contour',
                this.currentZoom,
                this.isRealEarth,
            ),
            // 2d. Rivers, lakes, and coastline glow (procgen zoom 3+)
            ...createRiversLayers(this.currentZoom, this.isRealEarth),
            // 3. Roads (procgen PathLayer — real earth roads come from MapLibre vector tiles)
            ...createRoadsLayers(this.isRealEarth, this.currentZoom),
            // 4. Buildings (procedural footprints around cities, demand overlay at zoom 7+)
            ...createBuildingsLayers(this.currentZoom, this.activeOverlay === 'demand' && this.currentZoom >= 7),
            // 5. Borders
            createBordersLayer(this.cachedRegions, this.isRealEarth),
            // 6. Overlays (population, demand, terrain, etc.)
            ...this.createOverlayLayers(),
            // 6a. Submarine cable reference (real earth mode only, zoom 0-7)
            ...createSubmarineCableRefLayers(
                this.activeOverlay === 'submarine_reference',
                this.isRealEarth,
                this.currentZoom,
            ),
            // 6b. City ambient glow (population-proportional warm glow — rendered before city icons)
            ...createCityGlowLayer(this.cachedCities, this.currentZoom),
            // 7. Cities (glow, icons, labels)
            this.createCitiesLayer(),
            this.createLabelsLayer(),
            this.createRegionLabelsLayer(),
            // 7b. Cable glow (low zoom) + pole dots (high zoom aerial) — below main infra
            ...createCableGlowLayers(
                [], // Legacy argument, now using DataStore internally
                this.currentZoom,
            ),
            // Sync binary data before rendering infra layers
            (() => { 
                let minLevel = 0;
                if (this.currentZoom < 3) minLevel = 3;
                else if (this.currentZoom < 5) minLevel = 2;
                else if (this.currentZoom < 7) minLevel = 1;
                dataStore.sync(minLevel); 
                return []; 
            })(),
            // 8. Infrastructure (nodes, edges — above cities)
            ...createInfraLayers({
                iconAtlas: this.iconAtlas,
                iconMapping: this.iconMapping,
                iconAtlasReady: this.iconAtlasReady,
                activeOverlay: this.activeOverlay,
                currentZoom: this.currentZoom,
                currentTime: this.currentTime,
                pitch: this.pitch,
                hoveredNodeId: this.hoveredNodeId,
                playerCorpId: bridge.isInitialized() ? bridge.getPlayerCorpId() : undefined,
                activeDisasters: this.activeDisasters,
                bounds: (() => {
                    if (!this.map) return undefined;
                    const b = this.map.getBounds();
                    return [b.getWest(), b.getSouth(), b.getEast(), b.getNorth()] as [number, number, number, number];
                })(),
            }),
            // 8b. Satellite overlay (orbital positions, ISL links, coverage footprints)
            ...createSatelliteLayers(this.activeOverlay === 'satellite', this.currentZoom),
            // 9. Cable drawing preview (between infra and selection)
            ...createCablePreviewLayers(this.cableDrawingState),
            // 10. Annotations and weather
            ...createAnnotationLayers(get(annotations), get(routePlans)),
            ...createWeatherLayers({
                enabled: this.weatherEnabled,
                dayNightCycle: this.dayNightCycle,
                gameTick: this.gameTick,
                currentZoom: this.currentZoom,
            }, this.activeDisasters, this.forecastDisasters),
            // 11. Selection/hover highlights (topmost)
            this.createSelectionLayer(),
            ...this.createEdgeBuildHighlights(),
            this.createPathfindingPreviewLayer(),
            // 12. Ghost preview for node placement (above everything)
            ...this.createGhostPreviewLayer(),
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
    // When the 'ocean_depth' overlay is active, renders ocean cells with an
    // enhanced depth-based blue gradient and subtle contour lines at depth
    // boundaries. Reuses polygon geometry from vectorTerrainLayer.

    private createOceanDepthLayers(): Layer[] {
        const visible = this.activeOverlay === 'ocean_depth' && !this.isRealEarth;
        return createOceanDepthLayersFn(visible);
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

        if (this.activeOverlay === 'population') {
            const popCells: { position: [number, number]; color: [number, number, number, number]; pop: number }[] = [];
            // Find max population for normalization
            const maxPop = Math.max(1, ...this.cachedCities.map(c => c.population));
            for (const city of this.cachedCities) {
                // Logarithmic scale: log(pop) normalized to [0, 1]
                const rawIntensity = Math.log10(Math.max(city.population, 10)) / Math.log10(Math.max(maxPop, 10));
                const intensity = Math.min(1.0, rawIntensity);
                // Dark purple (sparse) → bright yellow (dense)
                const r = Math.floor(40 + intensity * 215);
                const g = Math.floor(20 + intensity * 215);
                const b = Math.floor(80 * (1 - intensity));
                const alpha = Math.floor(100 + intensity * 155);
                const cellPop = Math.round(city.population / Math.max(city.cell_positions.length, 1));
                for (const cp of city.cell_positions) {
                    popCells.push({
                        position: [cp.lon, cp.lat],
                        color: [r, g, b, alpha],
                        pop: cellPop,
                    });
                }
            }
            layers.push(new ScatterplotLayer({
                id: 'overlay-population',
                data: popCells,
                getPosition: (d: any) => d.position,
                getFillColor: (d: any) => d.color,
                getRadius: overlayRadius,
                radiusMinPixels: 6,
                pickable: false,
                parameters: { depthTest: false }
            }));
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
                    pickable: false,
                    parameters: { depthTest: false }
                }));
            }
        }

        if (this.activeOverlay === 'market_share') {
            if (bridge.isInitialized() && this.cachedRegions.length > 0) {
                const corps = bridge.getAllCorporations();
                const corpIndex = new Map<number, number>();
                for (let i = 0; i < corps.length; i++) {
                    corpIndex.set(corps[i].id, i);
                }

                // Optimization: Use DataStore to get region corp counts
                const cellToRegion = new Map<number, number>();
                for (const city of this.cachedCities) {
                    for (const cp of city.cell_positions) {
                        cellToRegion.set(cp.index, city.region_id);
                    }
                }

                const regionCorpCounts = dataStore.getRegionCorpCounts(cellToRegion);

                // Build polygon data for regions with a dominant corporation
                interface MarketShareRegion {
                    polygon: [number, number][];
                    color: [number, number, number, number];
                    regionName: string;
                    dominantCorpName: string;
                    nodeCount: number;
                }
                const polygonData: MarketShareRegion[] = [];

                for (const region of this.cachedRegions) {
                    if (!region.boundary_polygon || region.boundary_polygon.length < 3) continue;
                    const counts = regionCorpCounts.get(region.id);
                    if (!counts || counts.size === 0) continue;

                    // Find dominant corporation (most nodes)
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

                    const dominantCorp = corps.find(c => c.id === dominantCorpId);

                    polygonData.push({
                        polygon: region.boundary_polygon,
                        color: [baseColor[0], baseColor[1], baseColor[2], 80],
                        regionName: region.name,
                        dominantCorpName: dominantCorp?.name ?? 'Unknown',
                        nodeCount: maxCount,
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

        if (this.activeOverlay === 'coverage_overlap') {
            if (bridge.isInitialized()) {
                const corps = bridge.getAllCorporations();
                const playerCorpId = bridge.getPlayerCorpId();

                // Build corp index for color lookup
                const corpIndex = new Map<number, number>();
                for (let i = 0; i < corps.length; i++) {
                    corpIndex.set(corps[i].id, i);
                }

                // Build cell -> region lookup and cell -> position lookup
                const cellToRegion = new Map<number, number>();
                const cellPositions = new Map<number, [number, number]>();
                for (const city of this.cachedCities) {
                    for (const cp of city.cell_positions) {
                        cellToRegion.set(cp.index, city.region_id);
                        cellPositions.set(cp.index, [cp.lon, cp.lat]);
                    }
                }

                // Optimization: Use DataStore to get cell corp sets and region corp counts
                const cellCorpSets = dataStore.getCellCorpSets();
                const regionCorpCounts = dataStore.getRegionCorpCounts(cellToRegion);
                const regionCorps = new Map<number, Set<number>>();
                for (const [regionId, counts] of regionCorpCounts) {
                    regionCorps.set(regionId, new Set(counts.keys()));
                }

                // (a) Cell-level competition heatmap: cells with 2+ corps
                interface OverlapCell {
                    position: [number, number];
                    competitorCount: number;
                    hasPlayer: boolean;
                }
                const overlapCells: OverlapCell[] = [];
                for (const [cellIndex, corpSet] of cellCorpSets) {
                    if (corpSet.size < 2) continue;
                    const pos = cellPositions.get(cellIndex);
                    if (!pos) continue;
                    overlapCells.push({
                        position: pos,
                        competitorCount: corpSet.size,
                        hasPlayer: corpSet.has(playerCorpId),
                    });
                }

                if (overlapCells.length > 0) {
                    layers.push(new ScatterplotLayer({
                        id: 'overlay-coverage-overlap',
                        data: overlapCells,
                        getPosition: (d: OverlapCell) => d.position,
                        getFillColor: (d: OverlapCell) => {
                            const count = d.competitorCount;
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
                            lineWidthUnits: 'pixels',
                            stroked: true,
                            filled: false,
                            getRadius: overlayRadius,
                            radiusMinPixels: 6,
                            pickable: false,
                            parameters: { depthTest: false },
                        }));
                    }
                }

                // (b) Region competition borders (already computed above as regionCorps)

                interface CompetitionRegion {
                    polygon: [number, number][];
                    corpCount: number;
                    color: [number, number, number, number];
                }
                const competitionRegions: CompetitionRegion[] = [];
                for (const region of this.cachedRegions) {
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

                // (c) Expansion frontier: single-owner cells in contested regions
                interface ExpansionCell {
                    position: [number, number];
                    ownerColor: [number, number, number];
                    isPlayer: boolean;
                }
                const expansionCells: ExpansionCell[] = [];
                for (const [cellIndex, corpSet] of cellCorpSets) {
                    if (corpSet.size !== 1) continue;
                    const pos = cellPositions.get(cellIndex);
                    if (!pos) continue;
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
                    const frontierRadius = this.cellRadiusM * 0.8;
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
                        lineWidthUnits: 'pixels',
                        stroked: true,
                        filled: true,
                        getRadius: frontierRadius,
                        radiusMinPixels: 4,
                        pickable: false,
                        parameters: { depthTest: false },
                    }));
                }
            }
        }

        if (this.activeOverlay === 'spectrum') {
            if (bridge.isInitialized()) {
                layers.push(...createSpectrumAndOtherOverlays({
                    activeOverlay: 'spectrum',
                    terrainOverlayCanvas: null,
                    cities: this.cachedCities,
                    regions: this.cachedRegions,
                    cellRadiusM: this.cellRadiusM,
                }));
            }
        }

        if (this.activeOverlay === 'density') {
            layers.push(...createDensityOverlayLayers(this.cachedCities));
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
            outlineWidth: 2,
            outlineColor: [0, 0, 0, 200],
            fontSettings: { sdf: true, buffer: 8 },
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
            outlineWidth: 3,
            outlineColor: [0, 0, 0, 160],
            fontSettings: { sdf: true, buffer: 8 },
            parameters: { depthTest: false }
        });
    }

    // ── Selection highlight ─────────────────────────────────────────────────

    private createSelectionLayer(): Layer | null {
        if (this.selectedId === null || this.selectedId === undefined) return null;

        // Optimization: Use targeted metadata query instead of scanning all infrastructure
        const node = bridge.getNodeMetadata(this.selectedId);
        if (node && node.id) {
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

        const edge = bridge.getEdgeMetadata(this.selectedId);
        if (edge && edge.id) {
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

        const layers: Layer[] = [];

        // Optimization: Targeted query
        const sourceNode = bridge.getNodeMetadata(this.currentEdgeSourceId);
        if (sourceNode && sourceNode.id) {
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
            // Optimization: Batch query for valid targets
            const targetIds = Array.from(this.edgeTargetIds);
            const validTargets = bridge.getNodesMetadata(targetIds)
                .map((n: any) => ({ position: [n.x, n.y], id: n.id }));

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

    // ── Ghost preview layer (cursor-following placement preview) ────────────

    private createGhostPreviewLayer(): Layer[] {
        // Only show when in node placement mode with a selected build item
        const currentBuildMode = get(buildMode);
        const currentBuildItem = get(selectedBuildItem);
        const currentBuildCat = get(buildCategory);

        if (currentBuildMode !== 'node' || !currentBuildItem || currentBuildCat !== 'node' || !this.cursorOnMap) {
            ghostPreviewInfo.set({ terrainType: null, cost: null, valid: true, costMultiplier: 1.0 });
            return [];
        }

        if (!bridge.isInitialized()) return [];

        const layers: Layer[] = [];

        // Determine node sizing from the network level of the selected item
        const NODE_TYPE_LEVEL: Record<string, string> = {
            CellTower: 'Local',
            WirelessRelay: 'Local',
            CentralOffice: 'Regional',
            ExchangePoint: 'Regional',
            DataCenter: 'National',
            BackboneRouter: 'Continental',
            SatelliteGround: 'GlobalBackbone',
            SubmarineLanding: 'GlobalBackbone',
        };
        const networkLevel = NODE_TYPE_LEVEL[currentBuildItem] ?? 'Local';
        const tierSize = NODE_TIER_SIZE[networkLevel] ?? 24;

        // Get player corp color
        const playerCorpId = bridge.getPlayerCorpId();
        const corps = bridge.getAllCorporations();
        const corpIdx = corps.findIndex(c => c.id === playerCorpId);
        const baseColor: [number, number, number] = corpIdx >= 0
            ? CORP_COLORS[corpIdx % CORP_COLORS.length]
            : [16, 185, 129];

        // Find terrain at cursor position from cached cells
        const cursorTerrain = this.getTerrainAtCursor();
        this.ghostTerrainType = cursorTerrain;

        // Validate placement based on terrain
        const isValid = this.validateNodePlacement(currentBuildItem, cursorTerrain);
        this.ghostValid = isValid;

        // Get cost estimate from cached build options
        const costInfo = this.ghostBuildOptions.find(o => o.node_type === currentBuildItem);
        this.ghostCost = costInfo?.cost ?? null;

        // Update ghost preview store for HUD display
        const multiplier = cursorTerrain ? (TERRAIN_COST_MULTIPLIERS[cursorTerrain] ?? 1.0) : 1.0;
        ghostPreviewInfo.set({
            terrainType: cursorTerrain,
            cost: this.ghostCost,
            valid: isValid,
            costMultiplier: multiplier,
        });

        // Choose color based on validity: green tint for valid, red tint for invalid
        const ghostColor: [number, number, number, number] = isValid
            ? [
                Math.min(255, Math.floor(baseColor[0] * 0.6 + 16 * 0.4)),
                Math.min(255, Math.floor(baseColor[1] * 0.6 + 185 * 0.4)),
                Math.min(255, Math.floor(baseColor[2] * 0.6 + 129 * 0.4)),
                100,
            ]
            : [
                Math.min(255, Math.floor(baseColor[0] * 0.3 + 239 * 0.7)),
                Math.min(255, Math.floor(baseColor[1] * 0.3 + 68 * 0.7)),
                Math.min(255, Math.floor(baseColor[2] * 0.3 + 68 * 0.7)),
                100,
            ];

        // Pulsing factor for subtle animation
        const pulse = 0.7 + 0.3 * (0.5 + 0.5 * Math.sin(this.currentTime * 4.0));
        const pulseAlpha = Math.floor(ghostColor[3] * pulse);

        // Main ghost node circle (semi-transparent, pulsing)
        layers.push(new ScatterplotLayer({
            id: 'ghost-preview-node',
            data: [{ position: [this.cursorLon, this.cursorLat] as [number, number] }],
            getPosition: (d: { position: [number, number] }) => d.position,
            getFillColor: [ghostColor[0], ghostColor[1], ghostColor[2], pulseAlpha],
            getLineColor: isValid
                ? [16, 185, 129, Math.floor(200 * pulse)]
                : [239, 68, 68, Math.floor(200 * pulse)],
            getLineWidth: 2,
            lineWidthUnits: 'pixels',
            stroked: true,
            filled: true,
            getRadius: tierSize * 600,
            radiusMinPixels: 12,
            radiusMaxPixels: 40,
            pickable: false,
            parameters: { depthTest: false },
        }));

        // Outer pulsing ring
        layers.push(new ScatterplotLayer({
            id: 'ghost-preview-ring',
            data: [{ position: [this.cursorLon, this.cursorLat] as [number, number] }],
            getPosition: (d: { position: [number, number] }) => d.position,
            getFillColor: [0, 0, 0, 0],
            getLineColor: isValid
                ? [16, 185, 129, Math.floor(120 * pulse)]
                : [239, 68, 68, Math.floor(120 * pulse)],
            getLineWidth: 1,
            lineWidthUnits: 'pixels',
            stroked: true,
            filled: false,
            getRadius: tierSize * 900 * pulse,
            radiusMinPixels: 18,
            radiusMaxPixels: 60,
            pickable: false,
            parameters: { depthTest: false },
        }));

        // Cost + terrain label at cursor
        const labelParts: string[] = [];
        if (cursorTerrain) {
            labelParts.push(cursorTerrain);
        }
        if (this.ghostCost !== null) {
            labelParts.push(`$${this.formatCompactCost(this.ghostCost)}`);
        }
        if (!isValid) {
            labelParts.push('INVALID');
        }

        if (labelParts.length > 0) {
            layers.push(new TextLayer({
                id: 'ghost-preview-label',
                data: [{ position: [this.cursorLon, this.cursorLat] as [number, number], text: labelParts.join(' | ') }],
                getPosition: (d: { position: [number, number] }) => d.position,
                getText: (d: { text: string }) => d.text,
                getSize: 11,
                getColor: isValid ? [16, 185, 129, 220] : [239, 68, 68, 220],
                getPixelOffset: [0, 28],
                fontFamily: 'monospace',
                fontWeight: 'bold',
                getTextAnchor: 'middle',
                getAlignmentBaseline: 'top',
                parameters: { depthTest: false },
            }));
        }

        return layers;
    }

    /** Find the terrain type of the cell nearest to the cursor position. */
    private getTerrainAtCursor(): string | null {
        // Optimization: Use targeted bridge query instead of scanning all cells in JS
        return bridge.getTerrainAt(this.cursorLon, this.cursorLat);
    }

    /** Validate whether a node type can be placed on the given terrain. */
    private validateNodePlacement(nodeType: string, terrain: string | null): boolean {
        if (!terrain) return true; // Unknown terrain = allow

        const OCEAN_TERRAINS = new Set(['OceanShallow', 'OceanDeep', 'OceanTrench', 'Ocean']);
        const COASTAL_TERRAINS = new Set(['Coastal']);
        const LAND_TERRAINS = new Set(['Urban', 'Suburban', 'Rural', 'Mountainous', 'Desert', 'Tundra', 'Frozen']);
        const isOcean = OCEAN_TERRAINS.has(terrain);
        const isCoastal = COASTAL_TERRAINS.has(terrain);
        const isLand = LAND_TERRAINS.has(terrain);

        switch (nodeType) {
            case 'SubmarineLanding':
                return isCoastal;
            case 'SatelliteGround':
                return isLand || isCoastal;
            case 'CellTower':
            case 'WirelessRelay':
            case 'CentralOffice':
            case 'ExchangePoint':
            case 'DataCenter':
            case 'BackboneRouter':
                return isLand || isCoastal;
            default:
                return !isOcean;
        }
    }

    /** Format a cost number compactly (e.g., 1500000 -> "1.5M"). */
    private formatCompactCost(cost: number): string {
        if (cost >= 1_000_000_000) return `${(cost / 1_000_000_000).toFixed(1)}B`;
        if (cost >= 1_000_000) return `${(cost / 1_000_000).toFixed(1)}M`;
        if (cost >= 1_000) return `${(cost / 1_000).toFixed(0)}K`;
        return cost.toFixed(0);
    }

    /** Update ghost preview build options at cursor (throttled). */
    updateGhostBuildOptions(): void {
        if (!bridge.isInitialized()) return;
        const currentBuildMode = get(buildMode);
        if (currentBuildMode !== 'node' || !this.cursorOnMap) {
            this.ghostBuildOptions = [];
            return;
        }
        this.ghostBuildOptions = bridge.getBuildableNodes(this.cursorLon, this.cursorLat);
    }

    /** Update cursor position for ghost preview. Called from handleMouseMove. */
    updateCursorPosition(lon: number, lat: number): void {
        this.cursorLon = lon;
        this.cursorLat = lat;
        this.cursorOnMap = true;
    }

    /** Mark cursor as off the map (hides ghost preview). */
    clearCursorPosition(): void {
        this.cursorOnMap = false;
    }

    // ── Pathfinding preview ─────────────────────────────────────────────────

    private createPathfindingPreviewLayer(): Layer | null {
        if (this.currentEdgeSourceId === null || !this.hoveredEntity || this.hoveredEntity.type !== 'node') {
            return null;
        }

        // Optimization: Use targeted metadata queries
        const sourceNode = bridge.getNodeMetadata(this.currentEdgeSourceId);
        const targetNode = bridge.getNodeMetadata(this.hoveredEntity!.object?.id);

        if (!sourceNode || !sourceNode.id || !targetNode || !targetNode.id || sourceNode.id === targetNode.id) return null;

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
        // Refresh building coverage status based on current infrastructure
        this.refreshBuildingCoverage();
        this.renderLayers();
    }

    /** Update building coverage/connection status from current infrastructure nodes. */
    private refreshBuildingCoverage(): void {
        if (!bridge.isInitialized()) return;
        // Optimization: updateBuildingCoverage now uses dataStore.nodes internally
        updateBuildingCoverage();
    }

    updateCities(): void {
        if (!bridge.isInitialized()) return;
        this.cachedCities = bridge.getCities();
        // Rebuild road and building data when cities change
        buildRoadData(this.cachedCities, this.cachedRegions);
        buildBuildingData(this.cachedCities);
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

        // Lazy-build the terrain overlay bitmap on first use
        if (overlayType === 'terrain' && !this.terrainOverlayCanvas && this.cachedCells.length > 0) {
            buildTerrainBitmapAsync(
                this.cachedCells,
                this.cellSpacingKm,
                TERRAIN_OVERLAY_COLORS,
                this.quality
            ).then((canvas) => {
                this.terrainOverlayCanvas = canvas;
                // Re-render now that the overlay bitmap is ready
                if (this.activeOverlay === 'terrain') {
                    this.renderLayers();
                }
            });
        }

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

    /** Update active disaster events for weather and vulnerability visualization. */
    setActiveDisasters(disasters: ActiveDisaster[]): void {
        this.activeDisasters = disasters;
        setTooltipDisasters(disasters);
        // No explicit renderLayers() call needed — the animation loop handles it
    }

    /** Update forecast disasters (recomputed every 10 ticks for stability).
     *  Uses server-side weather forecasts when available, falls back to client-side heuristic. */
    updateForecasts(regions: import('$lib/wasm/types').Region[], currentTick: number): void {
        // Only recompute every 10 ticks to avoid flicker (seed includes tick)
        const bucket = Math.floor(currentTick / 10);
        if (bucket === this.lastForecastTick) return;
        this.lastForecastTick = bucket;

        // Try server-side weather forecasts first (authoritative)
        let serverForecasts: ForecastDisaster[] = [];
        try {
            const weatherData = bridge.getWeatherForecasts();
            if (weatherData.length > 0) {
                serverForecasts = convertWeatherForecasts(weatherData, regions, currentTick);
            }
        } catch {
            // Fall through to client-side
        }

        // Client-side disaster risk heuristic (supplement)
        const clientForecasts = computeDisasterForecasts(regions, bucket);

        // Merge: server forecasts take priority
        const coveredRegions = new Set(serverForecasts.map(f => f.regionId));
        const supplemental = clientForecasts.filter(f => !coveredRegions.has(f.regionId));

        this.forecastDisasters = [...serverForecasts, ...supplemental]
            .sort((a, b) => b.probability - a.probability)
            .slice(0, 8);
    }

    /** Get the current forecast disasters (for use by parent components). */
    getForecasts(): ForecastDisaster[] {
        return this.forecastDisasters;
    }

    /** Update the cable drawing preview state and re-render. */
    setCableDrawingState(state: CableDrawingState): void {
        this.cableDrawingState = state;
        this.renderLayers();
    }

    /** Pick a deck.gl object at the given screen coordinates. Returns pick info or null. */
    pickObject(x: number, y: number, radius: number = 4): any {
        if (!this.overlay) return null;
        return this.overlay.pickObject({ x, y, radius });
    }

    /** Convert screen pixel coordinates to map lon/lat. Returns [lon, lat] or null. */
    screenToLngLat(x: number, y: number): [number, number] | null {
        if (!this.map) return null;
        const lngLat = this.map.unproject([x, y]);
        return [lngLat.lng, lngLat.lat];
    }

    /** Get the underlying MapLibre map instance (for direct access when needed). */
    getMap(): maplibregl.Map | null {
        return this.map;
    }

    /** Fly the camera to a specific lon/lat with an optional zoom level. */
    flyTo(lon: number, lat: number, zoom?: number): void {
        if (!this.map) return;
        this.map.flyTo({
            center: [lon, lat],
            zoom: zoom ?? this.map.getZoom(),
            duration: 800,
        });
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

        // Dispatch map-mousemove with geo coordinates for cable drawing + ghost preview
        if (this.map) {
            const rect = this.container.getBoundingClientRect();
            const x = e.clientX - rect.left;
            const y = e.clientY - rect.top;
            const lngLat = this.map.unproject([x, y]);

            // Update cursor position for ghost preview layer
            this.updateCursorPosition(lngLat.lng, lngLat.lat);

            window.dispatchEvent(new CustomEvent('map-mousemove', {
                detail: {
                    lon: lngLat.lng,
                    lat: lngLat.lat,
                    screenX: e.clientX,
                    screenY: e.clientY,
                }
            }));
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

        // Clean up the terrain worker, vector terrain data, ocean depth, roads, buildings, rivers, and terrain detail
        disposeTerrainWorker();
        disposeVectorTerrainData();
        disposeOceanDepthData();
        disposeRoadData();
        disposeBuildingData();
        disposeRiverData();
        disposeTerrainDetailData();
        disposeElevationData();
    }
}
