import { PathLayer, ScatterplotLayer, TextLayer, IconLayer, ColumnLayer } from '@deck.gl/layers';
import { TripsLayer } from '@deck.gl/geo-layers';
import { CollisionFilterExtension } from '@deck.gl/extensions';
import type { Layer } from '@deck.gl/core';

import * as bridge from '$lib/wasm/bridge';
import type { AllInfraNode, DeploymentMethod } from '$lib/wasm/types';
import { CORP_COLORS, EDGE_STYLES, NODE_TIER_SIZE, NETWORK_TIER_LABEL, toIconKey } from '../constants';
import { catmullRomSpline } from '../spline';
import type { ActiveDisaster } from '../../WeatherLayer';

// ── Types ────────────────────────────────────────────────────────────────────

export interface IconMapping {
    x: number;
    y: number;
    width: number;
    height: number;
    mask: boolean;
}

// ── Constants ────────────────────────────────────────────────────────────────

/** Network tier numeric rank for LOD filtering and column heights. */
const TIER_RANK: Record<string, number> = {
    Local: 1,
    Regional: 2,
    National: 3,
    Continental: 4,
    GlobalBackbone: 5,
};

/** Column extrusion height (meters) by network tier for 2.5D view. */
const COLUMN_HEIGHT: Record<string, number> = {
    Local: 200,
    Regional: 500,
    National: 1000,
    Continental: 2000,
    GlobalBackbone: 3000,
};

/** Coverage radius (meters) for wireless node types when hovered. */
const COVERAGE_RADIUS: Record<string, number> = {
    CellTower: 15000,
    WirelessRelay: 8000,
    SatelliteGround: 200000,
};

/** Node types considered wireless for coverage display. */
const WIRELESS_TYPES = new Set(Object.keys(COVERAGE_RADIUS));

/**
 * Determine base pixel width for an edge type based on strand count / capacity class.
 * Drop → 1, Distribution/local → 2, Feeder/metro → 3, National → 4, Backbone/submarine → 5, Future → 6.
 */
export function edgeWidthByType(edgeType: string): number {
    switch (edgeType) {
        // Drop cable — thinnest single strand
        case 'DropCable':
            return 1;

        // Distribution fiber & local/access cables — 2px
        case 'DistributionFiber':
        case 'Copper':
        case 'FiberLocal':
        case 'TelegraphWire':
        case 'CoaxialCable':
        case 'CopperTrunkLine':
            return 2;

        // Feeder fiber & metro/regional — 3px
        case 'FeederFiber':
        case 'FiberRegional':
        case 'Microwave':
        case 'MicrowaveLink':
        case 'FiberMetro':
        case 'LongDistanceCopper':
        case 'Satellite':
        case 'SatelliteLEOLink':
        case 'EarlySatelliteLink':
            return 3;

        // National / long-haul — 4px
        case 'FiberNational':
        case 'FiberLongHaul':
            return 4;

        // DWDM backbone & submarine — 5px
        case 'DWDM_Backbone':
        case 'Submarine':
        case 'SubseaFiberCable':
        case 'SubseaTelegraphCable':
            return 5;

        // Near-future — 6px
        case 'QuantumFiberLink':
        case 'TerahertzBeam':
        case 'LaserInterSatelliteLink':
            return 6;

        default:
            return 2;
    }
}

/** Minimum tier rank visible at each zoom bracket. */
function minTierForZoom(zoom: number): number {
    if (zoom < 3) return 4;   // zoom 0-3: T4+ only (Continental, GlobalBackbone)
    if (zoom < 5) return 3;   // zoom 3-5: T3+ (National and above)
    return 1;                  // zoom 5+: everything
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/**
 * Offset a polyline path slightly to the right of its direction of travel.
 * This creates visual separation between a cable and the road it follows,
 * so both are visible at high zoom levels.
 *
 * The offset is in degrees, scaled to produce roughly 3-5 pixel offset
 * at the given zoom level.
 */
function offsetPathFromCenterline(
    path: [number, number][],
    zoom: number,
): [number, number][] {
    if (path.length < 2) return path;

    // Offset in degrees — smaller at higher zoom (more zoomed in = finer detail).
    // At zoom 8: ~0.0003 deg (~33m), zoom 9: ~0.00015 deg (~17m), zoom 10: ~0.00008 deg (~9m)
    const offsetDeg = 0.0012 / Math.pow(2, zoom - 6);

    const result: [number, number][] = [];

    for (let i = 0; i < path.length; i++) {
        // Compute the tangent direction at this point
        let dx: number, dy: number;

        if (i === 0) {
            dx = path[1][0] - path[0][0];
            dy = path[1][1] - path[0][1];
        } else if (i === path.length - 1) {
            dx = path[i][0] - path[i - 1][0];
            dy = path[i][1] - path[i - 1][1];
        } else {
            // Average of incoming and outgoing tangent
            dx = path[i + 1][0] - path[i - 1][0];
            dy = path[i + 1][1] - path[i - 1][1];
        }

        // Compute the perpendicular (right-hand normal)
        const len = Math.sqrt(dx * dx + dy * dy);
        if (len < 1e-12) {
            result.push(path[i]);
            continue;
        }

        // Normal to the right: rotate tangent 90 degrees clockwise
        const nx = dy / len;
        const ny = -dx / len;

        result.push([
            path[i][0] + nx * offsetDeg,
            path[i][1] + ny * offsetDeg,
        ]);
    }

    return result;
}

/** Get the corp color for an owner ID. Falls back to grey if unknown.
 *  Validates that the returned color has valid non-zero RGB values. */
function getCorpColor(ownerId: number, corpIndex: Map<number, number>): [number, number, number] {
    const idx = corpIndex.get(ownerId);
    if (idx !== undefined) {
        const color = CORP_COLORS[idx % CORP_COLORS.length];
        // Guard against corrupted/empty color entries
        if (color && color[0] + color[1] + color[2] > 0) return color;
        console.warn(`[infraLayer] CORP_COLORS[${idx % CORP_COLORS.length}] returned invalid color for corp ${ownerId}:`, color);
    }
    return [160, 160, 160];
}

/** Validate RGBA color values are in the 0-255 range and not all zeros.
 *  Returns the color clamped to valid range, or a visible fallback if all zeros. */
function validateRGBA(color: [number, number, number, number]): [number, number, number, number] {
    const r = Math.max(0, Math.min(255, Math.round(color[0])));
    const g = Math.max(0, Math.min(255, Math.round(color[1])));
    const b = Math.max(0, Math.min(255, Math.round(color[2])));
    const a = Math.max(0, Math.min(255, Math.round(color[3])));
    // If RGB are all zero and alpha is non-zero, the node would be invisible — fallback to grey
    if (r === 0 && g === 0 && b === 0 && a > 0) {
        return [160, 160, 160, a];
    }
    return [r, g, b, a];
}

/** Compute tinted node color based on health and construction state. */
function getNodeDisplayColor(
    node: AllInfraNode,
    baseColor: [number, number, number],
    isCongestion: boolean,
    isTraffic: boolean,
    trafficNodeFlowMap: Map<number, number> | null,
): [number, number, number, number] {
    // Overlay modes take priority
    if (isCongestion) {
        const util = node.utilization || 0;
        return [
            Math.floor(Math.min(1, util * 2) * 255),
            Math.floor(Math.max(0, 1 - util) * 200),
            0,
            255,
        ];
    }
    if (isTraffic && trafficNodeFlowMap) {
        const util = trafficNodeFlowMap.get(node.id);
        if (util !== undefined) {
            return [
                Math.floor(Math.min(1, util * 2) * 255),
                Math.floor(Math.max(0, 1 - util) * 200),
                0,
                180,
            ];
        }
        return [100, 100, 100, 30];
    }

    // Construction / health states
    if (node.under_construction) {
        return [baseColor[0], baseColor[1], baseColor[2], 150];
    }
    const health = node.health ?? 1;
    if (health <= 0) {
        // Offline
        return [100, 100, 100, 150];
    }
    if (health < 0.2) {
        // Damaged — red tint
        return [239, 68, 68, 220];
    }
    if (health < 0.5) {
        // Degraded — amber tint
        return [245, 158, 11, 220];
    }
    return [baseColor[0], baseColor[1], baseColor[2], 255];
}

// ── Trip path generation for TripsLayer ──────────────────────────────────────

interface TripDatum {
    path: [number, number][];
    timestamps: number[];
    color: [number, number, number, number];
}

/** Generate animated trip paths for high-traffic edges.
 *  Particles now follow the edge's spline path for visual consistency. */
function buildTrips(
    edges: ProcessedEdge[],
    _currentTime: number,
): TripDatum[] {
    const trips: TripDatum[] = [];
    for (const edge of edges) {
        const util = edge.utilization;
        if (util <= 0.7) continue; // only high traffic gets particles

        // Use the pre-computed spline path so particles follow curves
        const path = edge.path;
        if (path.length < 2) continue;

        // Distribute timestamps evenly along the path
        const duration = 200; // trip loop duration in time units
        const timestamps: number[] = [];
        for (let i = 0; i < path.length; i++) {
            timestamps.push((i / (path.length - 1)) * duration);
        }

        trips.push({
            path,
            timestamps,
            color: [255, 255, 255, 200],
        });
    }
    return trips;
}

// ── Processed data types ─────────────────────────────────────────────────────

interface ProcessedNode {
    id: number;
    position: [number, number];
    color: [number, number, number, number];
    icon: string;
    tierSize: number;
    tierLabel: string;
    network_level: string;
    node_type: string;
    under_construction: boolean;
    health: number;
    utilization: number;
    max_throughput: number;
    current_load: number;
    owner: number;
    owner_name: string;
    tierRank: number;
    x: number;
    y: number;
    isPlayer: boolean;
}

interface ProcessedEdge {
    id: number;
    sourcePosition: [number, number];
    targetPosition: [number, number];
    /** Tessellated spline path (or straight-line fallback) for PathLayer rendering. */
    path: [number, number][];
    color: [number, number, number, number];
    width: number;
    edge_type: string;
    utilization: number;
    bandwidth: number;
    current_load: number;
    length_km: number;
    health: number;
    source: number;
    target: number;
    src_x: number;
    src_y: number;
    dst_x: number;
    dst_y: number;
    tierRank: number;
    /** Raw waypoints from sim data (empty if none). */
    waypoints: [number, number][];
    /** Deployment method: Aerial (on poles) or Underground (buried). */
    deployment: DeploymentMethod;
    owner: number;
    owner_name: string;
    isPlayer: boolean;
}

// ── Main export ──────────────────────────────────────────────────────────────

/**
 * Creates all infrastructure visualization layers:
 * - Edge lines (with congestion/traffic overlay coloring)
 * - Animated TripsLayer for high-traffic edges
 * - Pulsing glow for medium-traffic edges
 * - Node icons (IconLayer) or 2.5D columns (ColumnLayer at high pitch)
 * - Construction pulsing rings, degraded/damaged tinting
 * - Hover glow, connected-edge highlights, coverage radius circles
 * - Infrastructure labels at high zoom with collision deconfliction
 * - LOD filtering by zoom level
 * - Tier badge labels
 */
export function createInfraLayers(opts: {
    iconAtlas: HTMLCanvasElement | null;
    iconMapping: Record<string, IconMapping>;
    iconAtlasReady: boolean;
    activeOverlay: string;
    currentZoom: number;
    currentTime: number;
    pitch: number;
    hoveredNodeId: number | null;
    playerCorpId?: number;
    activeDisasters?: ActiveDisaster[];
}): Layer[] {
    const {
        iconAtlas,
        iconMapping,
        iconAtlasReady,
        activeOverlay,
        currentZoom,
        currentTime,
        pitch,
        hoveredNodeId,
        playerCorpId,
        activeDisasters,
    } = opts;

    // ── Gather raw data ──────────────────────────────────────────────────────
    const corps = bridge.getAllCorporations();
    const corpIndex = new Map<number, number>();
    for (let i = 0; i < corps.length; i++) {
        corpIndex.set(corps[i].id, i);
    }

    const isCongestion = activeOverlay === 'congestion';
    const isTraffic = activeOverlay === 'traffic';

    let trafficEdgeFlowMap: Map<number, { utilization: number; color: [number, number, number]; opacity: number }> | null = null;
    let trafficNodeFlowMap: Map<number, number> | null = null;

    if (isTraffic && bridge.isInitialized()) {
        const trafficFlows = bridge.getTrafficFlows();
        trafficEdgeFlowMap = new Map();
        trafficNodeFlowMap = new Map();
        for (const f of trafficFlows.edge_flows) {
            let color: [number, number, number];
            let opacity: number;
            if (f.utilization > 1.0) { color = [255, 34, 34]; opacity = 153; }
            else if (f.utilization > 0.8) { color = [255, Math.floor((1 - (f.utilization - 0.8) / 0.2) * 80), 0]; opacity = 127; }
            else if (f.utilization > 0.5) { color = [0, 255, 255]; opacity = 100; }
            else { color = [59, 130, 246]; opacity = 50; }
            trafficEdgeFlowMap.set(f.id, { utilization: f.utilization, color, opacity });
        }
        for (const f of trafficFlows.node_flows) {
            trafficNodeFlowMap.set(f.id, f.utilization);
        }
    }

    // ── LOD: minimum tier for current zoom ───────────────────────────────────
    const minTier = minTierForZoom(currentZoom);

    // ── Process all infrastructure through a single pass ─────────────────────
    // Use getAllInfrastructure for unified owner info, but fall back to per-corp
    // iteration to preserve existing corp-color semantics.

    const allEdges: ProcessedEdge[] = [];
    const allNodes: ProcessedNode[] = [];

    for (let i = 0; i < corps.length; i++) {
        const corp = corps[i];
        const baseColor = CORP_COLORS[i % CORP_COLORS.length];
        const infra = bridge.getInfrastructureList(corp.id);

        for (const edge of infra.edges) {
            const tierRank = Math.max(
                TIER_RANK[edge.edge_type] || 1,
                // Estimate edge tier from type name
                edgeTierRank(edge.edge_type),
            );
            if (tierRank < minTier) continue; // LOD cull

            const style = EDGE_STYLES[edge.edge_type] || { color: baseColor, width: 2 };
            let color: [number, number, number] = style.color;
            let opacity = 255;

            if (isCongestion) {
                const util = edge.utilization || 0;
                color = [
                    Math.floor(Math.min(1, util * 2) * 255),
                    Math.floor(Math.max(0, 1 - util) * 200),
                    0,
                ];
            } else if (isTraffic && trafficEdgeFlowMap) {
                const flow = trafficEdgeFlowMap.get(edge.id);
                if (flow) {
                    color = flow.color;
                    opacity = flow.opacity;
                } else {
                    color = [100, 100, 100]; opacity = 20;
                }
            }

            // Base width from strand-count / capacity class
            const baseWidth = edgeWidthByType(edge.edge_type);

            // Medium traffic: wider, brighter glow
            const util = edge.utilization || 0;
            let width = baseWidth;
            if (!isCongestion && !isTraffic && util > 0.3 && util <= 0.7) {
                width = baseWidth * 1.8;
                opacity = Math.min(255, opacity + 40);
            }

            // Competitor visual hierarchy: reduce opacity and width for non-player corps
            const isPlayerEdge = playerCorpId !== undefined && corp.id === playerCorpId;
            if (!isPlayerEdge && !isCongestion && !isTraffic) {
                opacity = Math.floor(opacity * 0.7);
                width = width * 0.8;
            }

            // Health-based color tinting (green > 0.8, amber 0.5-0.8, red < 0.5)
            const health = edge.health ?? 1;
            if (!isCongestion && !isTraffic) {
                if (health < 0.5) {
                    // Red tint for damaged edges
                    color = [
                        Math.min(255, Math.floor(color[0] * 0.3 + 239 * 0.7)),
                        Math.min(255, Math.floor(color[1] * 0.3 + 68 * 0.7)),
                        Math.min(255, Math.floor(color[2] * 0.3 + 68 * 0.7)),
                    ];
                } else if (health < 0.8) {
                    // Amber tint for degraded edges
                    color = [
                        Math.min(255, Math.floor(color[0] * 0.5 + 245 * 0.5)),
                        Math.min(255, Math.floor(color[1] * 0.5 + 158 * 0.5)),
                        Math.min(255, Math.floor(color[2] * 0.5 + 11 * 0.5)),
                    ];
                }
                // health >= 0.8: keep original color (healthy)
            }

            // Build the display path: spline if waypoints exist, straight line otherwise
            const rawWaypoints: [number, number][] = (edge as any).waypoints ?? [];
            const deployment: DeploymentMethod = (edge as any).deployment ?? 'Underground';

            let path: [number, number][];
            if (rawWaypoints.length >= 2) {
                // Waypoints already include src→dst route; tessellate as spline
                path = catmullRomSpline(rawWaypoints);
            } else {
                // Fallback: straight line from source to target
                path = [
                    [edge.src_x, edge.src_y],
                    [edge.dst_x, edge.dst_y],
                ];
            }

            // At zoom > 7, offset cables that follow roads slightly from the road
            // centerline (3-5px equivalent in degrees) so both road and cable are
            // visible. An edge "follows a road" if it has 3+ waypoints (road-routed).
            if (currentZoom > 7 && rawWaypoints.length >= 3) {
                path = offsetPathFromCenterline(path, currentZoom);
            }

            allEdges.push({
                id: edge.id,
                sourcePosition: [edge.src_x, edge.src_y],
                targetPosition: [edge.dst_x, edge.dst_y],
                path,
                color: [color[0], color[1], color[2], opacity],
                width,
                edge_type: edge.edge_type,
                utilization: edge.utilization || 0,
                bandwidth: edge.bandwidth || 0,
                current_load: edge.current_load || 0,
                length_km: edge.length_km || 0,
                health: edge.health ?? 1,
                source: edge.source,
                target: edge.target,
                src_x: edge.src_x,
                src_y: edge.src_y,
                dst_x: edge.dst_x,
                dst_y: edge.dst_y,
                tierRank,
                waypoints: rawWaypoints,
                deployment,
                owner: corp.id,
                owner_name: corp.name,
                isPlayer: isPlayerEdge,
            });
        }

        for (const node of infra.nodes) {
            const tierRank = TIER_RANK[node.network_level] || 1;
            if (tierRank < minTier) continue; // LOD cull

            const nodeColor = validateRGBA(getNodeDisplayColor(
                { ...node, owner: corp.id, owner_name: corp.name } as AllInfraNode,
                baseColor,
                isCongestion,
                isTraffic,
                trafficNodeFlowMap,
            ));

            // Competitor visual hierarchy: reduce opacity for non-player corps
            const isPlayerNode = playerCorpId !== undefined && corp.id === playerCorpId;
            if (!isPlayerNode && !isCongestion && !isTraffic) {
                nodeColor[3] = Math.min(nodeColor[3], 200);
            }

            allNodes.push({
                id: node.id,
                position: [node.x, node.y],
                color: nodeColor,
                icon: toIconKey(node.node_type),
                tierSize: NODE_TIER_SIZE[node.network_level] || 32,
                tierLabel: NETWORK_TIER_LABEL[node.network_level] || '',
                network_level: node.network_level,
                node_type: node.node_type,
                under_construction: node.under_construction,
                health: node.health ?? 1,
                utilization: node.utilization || 0,
                max_throughput: node.max_throughput || 0,
                current_load: node.current_load || 0,
                owner: corp.id,
                owner_name: corp.name,
                tierRank,
                x: node.x,
                y: node.y,
                isPlayer: isPlayerNode,
            });
        }
    }

    // ── Build layer array ────────────────────────────────────────────────────
    const layers: Layer[] = [];

    // ── 1. Edge paths (spline curves or straight lines) ────────────────────
    // At high zoom (5+), Aerial edges get dashed style (poles), Underground stays solid.
    const highZoom = currentZoom >= 5;

    layers.push(new PathLayer({
        id: 'infra-edges',
        data: allEdges,
        getPath: (d: ProcessedEdge) => d.path,
        getColor: (d: ProcessedEdge) => d.color,
        getWidth: (d: ProcessedEdge) => d.width,
        widthUnits: 'pixels',
        widthMinPixels: 1,
        widthMaxPixels: 12,
        jointRounded: true,
        capRounded: true,
        // Dashed lines for Aerial deployment at high zoom
        getDashArray: highZoom
            ? (d: ProcessedEdge) => d.deployment === 'Aerial' ? [8, 4] : [0, 0]
            : [0, 0],
        dashJustified: true,
        pickable: true,
        autoHighlight: true,
        onClick: ({ object }: any) => {
            if (object) {
                window.dispatchEvent(new CustomEvent('entity-selected', {
                    detail: { id: object.id, type: 'edge' },
                }));
            }
        },
    }));

    // ── 2. Medium-traffic pulsing glow edges (0.3 < util <= 0.7) ─────────────
    const mediumTrafficEdges = allEdges.filter(e => {
        const u = e.utilization;
        return u > 0.3 && u <= 0.7;
    });
    if (mediumTrafficEdges.length > 0) {
        // Pulse factor oscillates between 0.4 and 1.0
        const pulse = 0.4 + 0.6 * (0.5 + 0.5 * Math.sin(currentTime * 0.003));
        layers.push(new PathLayer({
            id: 'infra-edges-medium-glow',
            data: mediumTrafficEdges,
            getPath: (d: ProcessedEdge) => d.path,
            getColor: (d: ProcessedEdge) => [
                Math.min(255, d.color[0] + 60),
                Math.min(255, d.color[1] + 60),
                Math.min(255, d.color[2] + 60),
                Math.floor(80 * pulse),
            ],
            getWidth: (d: ProcessedEdge) => d.width * 2.5,
            widthUnits: 'pixels',
            jointRounded: true,
            capRounded: true,
            pickable: false,
            parameters: {
                blend: true,
                blendFunc: [WebGLRenderingContext.SRC_ALPHA, WebGLRenderingContext.ONE],
            },
        }));
    }

    // ── 3. TripsLayer animated data flow for high-traffic edges ───────────────
    const trips = buildTrips(allEdges, currentTime);
    if (trips.length > 0) {
        const tripDuration = 200;
        const loopTime = currentTime % tripDuration;

        layers.push(new TripsLayer({
            id: 'infra-edge-trips',
            data: trips,
            getPath: (d: TripDatum) => d.path,
            getTimestamps: (d: TripDatum) => d.timestamps,
            getColor: (d: TripDatum) => d.color,
            getWidth: 4,
            widthMinPixels: 2,
            widthMaxPixels: 8,
            currentTime: loopTime,
            trailLength: 80,
            fadeTrail: true,
            pickable: false,
            parameters: {
                blend: true,
                blendFunc: [WebGLRenderingContext.SRC_ALPHA, WebGLRenderingContext.ONE],
            },
        } as any));
    }

    // ── 3b. Cable vulnerability highlights for active disasters ─────────────
    if (activeDisasters && activeDisasters.length > 0) {
        const atRiskEdges = findAtRiskEdges(allEdges, activeDisasters);
        if (atRiskEdges.length > 0) {
            // Pulsing amber/red glow for at-risk edges
            const riskPulse = 0.5 + 0.5 * Math.sin(currentTime * 0.006);

            layers.push(new PathLayer({
                id: 'infra-edges-at-risk',
                data: atRiskEdges,
                getPath: (d: AtRiskEdge) => d.edge.path,
                getColor: (d: AtRiskEdge) => {
                    const alpha = Math.floor((100 + 80 * riskPulse) * d.riskLevel);
                    if (d.riskType === 'submarine') {
                        // Wave interference: blue-white
                        return [80, 160, 255, alpha];
                    }
                    // Amber warning for aerial/underground
                    return [245, 158, 11, alpha];
                },
                getWidth: (d: AtRiskEdge) => d.edge.width * 2.5,
                widthUnits: 'pixels',
                jointRounded: true,
                capRounded: true,
                pickable: false,
                getDashArray: (d: AtRiskEdge) =>
                    d.riskType === 'submarine' ? [6, 4, 2, 4] : [0, 0],
                dashJustified: true,
                parameters: {
                    blend: true,
                    blendFunc: [WebGLRenderingContext.SRC_ALPHA, WebGLRenderingContext.ONE],
                },
            }));
        }
    }

    // ── 4. Construction pulsing rings ────────────────────────────────────────
    const constructionNodes = allNodes.filter(n => n.under_construction);
    if (constructionNodes.length > 0) {
        // Animated radius: oscillates between 0.7x and 1.3x base
        const pulseRadius = 0.7 + 0.6 * (0.5 + 0.5 * Math.sin(currentTime * 0.005));
        layers.push(new ScatterplotLayer({
            id: 'infra-construction-rings',
            data: constructionNodes,
            getPosition: (d: ProcessedNode) => d.position,
            getFillColor: [59, 130, 246, 0],
            getLineColor: [59, 130, 246, 160],
            getLineWidth: 2,
            lineWidthUnits: 'pixels',
            stroked: true,
            filled: false,
            getRadius: (d: ProcessedNode) => d.tierSize * 800 * pulseRadius,
            radiusMinPixels: 10,
            radiusMaxPixels: 40,
            pickable: false,
            parameters: { depthTest: false },
        }));
    }

    // ── 5. Node rendering: 2.5D columns (pitch > 10) or flat icons ───────────
    const use3D = pitch > 10;

    if (use3D) {
        // ColumnLayer for extruded 2.5D nodes
        layers.push(new ColumnLayer({
            id: 'infra-nodes-columns',
            data: allNodes,
            diskResolution: 12,
            radius: 8000,
            extruded: true,
            getPosition: (d: ProcessedNode) => d.position,
            getFillColor: (d: ProcessedNode) => d.color,
            getElevation: (d: ProcessedNode) => {
                const h = COLUMN_HEIGHT[d.network_level] || 200;
                return d.isPlayer ? h : h * 0.85;
            },
            pickable: true,
            autoHighlight: true,
            onClick: ({ object }: any) => {
                if (object) {
                    window.dispatchEvent(new CustomEvent('entity-selected', {
                        detail: { id: object.id, type: 'node' },
                    }));
                }
            },
        } as any));

        // Billboard icons on top of columns
        if (iconAtlasReady && iconAtlas) {
            layers.push(new IconLayer({
                id: 'infra-nodes-column-icons',
                data: allNodes,
                getPosition: (d: ProcessedNode) => d.position,
                getIcon: (d: ProcessedNode) => d.icon,
                iconAtlas: iconAtlas as any,
                iconMapping: iconMapping,
                getSize: (d: ProcessedNode) => d.tierSize * (d.isPlayer ? 0.8 : 0.68),
                sizeMinPixels: 10,
                sizeMaxPixels: 48,
                getColor: (d: ProcessedNode) => d.isPlayer ? [255, 255, 255, 230] : [255, 255, 255, 180],
                // Elevate icon above column top
                getPixelOffset: [0, -20],
                pickable: false,
                billboard: true,
            }));
        }
    } else {
        // Flat icon layer (default)
        if (iconAtlasReady && iconAtlas) {
            layers.push(new IconLayer({
                id: 'infra-nodes',
                data: allNodes,
                getPosition: (d: ProcessedNode) => d.position,
                getIcon: (d: ProcessedNode) => d.icon,
                iconAtlas: iconAtlas as any,
                iconMapping: iconMapping,
                getSize: (d: ProcessedNode) => d.isPlayer ? d.tierSize : d.tierSize * 0.85,
                sizeMinPixels: 12,
                sizeMaxPixels: 72,
                getColor: (d: ProcessedNode) => d.color,
                pickable: true,
                autoHighlight: true,
                onClick: ({ object }: any) => {
                    if (object) {
                        window.dispatchEvent(new CustomEvent('entity-selected', {
                            detail: { id: object.id, type: 'node' },
                        }));
                    }
                },
            }));
        } else {
            // Fallback colored dots when atlas not ready
            layers.push(new ScatterplotLayer({
                id: 'infra-nodes-fallback',
                data: allNodes,
                getPosition: (d: ProcessedNode) => d.position,
                getFillColor: (d: ProcessedNode) => validateRGBA(d.color),
                getRadius: (d: ProcessedNode) => d.isPlayer ? d.tierSize * 500 : d.tierSize * 425,
                radiusMinPixels: 6,
                radiusMaxPixels: 24,
                pickable: true,
                onClick: ({ object }: any) => {
                    if (object) {
                        window.dispatchEvent(new CustomEvent('entity-selected', {
                            detail: { id: object.id, type: 'node' },
                        }));
                    }
                },
            }));
        }
    }

    // ── 6. Hover effects ─────────────────────────────────────────────────────
    if (hoveredNodeId !== null) {
        const hoveredNode = allNodes.find(n => n.id === hoveredNodeId);
        if (hoveredNode) {
            const hoverColor = getCorpColor(hoveredNode.owner, corpIndex);

            // Bright glow ring around hovered node
            layers.push(new ScatterplotLayer({
                id: 'infra-hover-glow',
                data: [hoveredNode],
                getPosition: (d: ProcessedNode) => d.position,
                getFillColor: [...hoverColor, 40],
                getLineColor: [...hoverColor, 200],
                getLineWidth: 3,
                lineWidthUnits: 'pixels',
                stroked: true,
                filled: true,
                getRadius: hoveredNode.tierSize * 1200,
                radiusMinPixels: 20,
                radiusMaxPixels: 60,
                pickable: false,
                parameters: {
                    depthTest: false,
                    blend: true,
                    blendFunc: [WebGLRenderingContext.SRC_ALPHA, WebGLRenderingContext.ONE],
                },
            }));

            // Find directly connected edges
            const connectedEdges = allEdges.filter(
                e => e.source === hoveredNodeId || e.target === hoveredNodeId,
            );

            if (connectedEdges.length > 0) {
                // Highlight connected edges in bright accent color (follows spline path)
                layers.push(new PathLayer({
                    id: 'infra-hover-edges',
                    data: connectedEdges,
                    getPath: (d: ProcessedEdge) => d.path,
                    getColor: [255, 255, 100, 200],
                    getWidth: 4,
                    widthUnits: 'pixels',
                    jointRounded: true,
                    capRounded: true,
                    pickable: false,
                    parameters: { depthTest: false },
                }));

                // Find connected neighbor node IDs
                const neighborIds = new Set<number>();
                for (const e of connectedEdges) {
                    if (e.source === hoveredNodeId) neighborIds.add(e.target);
                    else neighborIds.add(e.source);
                }
                neighborIds.delete(hoveredNodeId);

                const neighborNodes = allNodes.filter(n => neighborIds.has(n.id));
                if (neighborNodes.length > 0) {
                    layers.push(new ScatterplotLayer({
                        id: 'infra-hover-neighbors',
                        data: neighborNodes,
                        getPosition: (d: ProcessedNode) => d.position,
                        getFillColor: [255, 255, 255, 0],
                        getLineColor: [255, 255, 100, 120],
                        getLineWidth: 2,
                        lineWidthUnits: 'pixels',
                        stroked: true,
                        filled: false,
                        getRadius: (d: ProcessedNode) => d.tierSize * 800,
                        radiusMinPixels: 10,
                        radiusMaxPixels: 40,
                        pickable: false,
                        parameters: { depthTest: false },
                    }));
                }
            }

            // Coverage radius for wireless nodes
            const coverageR = COVERAGE_RADIUS[hoveredNode.node_type];
            if (coverageR) {
                layers.push(new ScatterplotLayer({
                    id: 'infra-hover-coverage',
                    data: [hoveredNode],
                    getPosition: (d: ProcessedNode) => d.position,
                    getFillColor: [59, 130, 246, 25],
                    getLineColor: [59, 130, 246, 140],
                    getLineWidth: 2,
                    lineWidthUnits: 'pixels',
                    stroked: true,
                    filled: true,
                    getRadius: coverageR,
                    radiusMinPixels: 30,
                    pickable: false,
                    parameters: { depthTest: false },
                }));
            }
        }
    }

    // ── 7. Tier badge labels (visible at zoom > 5) ───────────────────────────
    if (currentZoom > 5) {
        layers.push(new TextLayer({
            id: 'node-tier-labels',
            data: allNodes,
            getPosition: (d: ProcessedNode) => d.position,
            getText: (d: ProcessedNode) => d.tierLabel,
            getSize: 10,
            getColor: [255, 255, 255, 180],
            getPixelOffset: [14, -14],
            fontFamily: 'Inter, sans-serif',
            fontWeight: 'bold',
            parameters: { depthTest: false },
        }));
    }

    // ── 8. Infrastructure labels at high zoom (owner + type) ─────────────────
    if (currentZoom > 7) {
        layers.push(new TextLayer({
            id: 'infra-owner-labels',
            data: allNodes,
            getPosition: (d: ProcessedNode) => d.position,
            getText: (d: ProcessedNode) => `${d.owner_name}\n${d.node_type}`,
            getSize: 11,
            getColor: (d: ProcessedNode) => {
                const c = getCorpColor(d.owner, corpIndex);
                return [c[0], c[1], c[2], 220];
            },
            getPixelOffset: [0, 18],
            fontFamily: 'Inter, sans-serif',
            fontWeight: 'normal',
            getTextAnchor: 'middle',
            getAlignmentBaseline: 'top',
            parameters: { depthTest: false },
            extensions: [new CollisionFilterExtension()],
            collisionEnabled: true,
            getCollisionPriority: (d: ProcessedNode) => d.tierRank,
            collisionTestProps: {
                sizeScale: 2,
            },
        } as any));
    }

    return layers;
}

// ── Cable vulnerability detection ────────────────────────────────────────────

interface AtRiskEdge {
    edge: ProcessedEdge;
    riskType: 'aerial' | 'underground' | 'submarine';
    riskLevel: number; // 0-1 severity multiplier
    disasterType: string;
}

/** Submarine edge types. */
const SUBMARINE_TYPES = new Set([
    'Submarine', 'SubseaFiberCable', 'SubseaTelegraphCable',
]);

/**
 * Determine vulnerability for deployment type vs disaster type.
 * - Aerial: vulnerable to storms, ice, wind (Hurricane, Landslide, storm-like)
 * - Underground: vulnerable to earthquakes, floods
 * - Submarine: vulnerable to earthquakes (underwater), anchor strikes
 */
function getVulnerability(
    deployment: DeploymentMethod,
    edgeType: string,
    disasterType: string,
): { vulnerable: boolean; riskType: 'aerial' | 'underground' | 'submarine' } | null {
    const lower = disasterType.toLowerCase();
    const isSubmarine = SUBMARINE_TYPES.has(edgeType);

    if (isSubmarine) {
        // Submarine cables: vulnerable to earthquakes and storms
        if (lower.includes('earthquake') || lower.includes('hurricane') ||
            lower.includes('typhoon') || lower.includes('storm')) {
            return { vulnerable: true, riskType: 'submarine' };
        }
        return null;
    }

    if (deployment === 'Aerial') {
        // Aerial: vulnerable to storms, ice, wind, hurricanes, landslides
        if (lower.includes('hurricane') || lower.includes('typhoon') ||
            lower.includes('storm') || lower.includes('thunder') ||
            lower.includes('ice') || lower.includes('blizzard') ||
            lower.includes('landslide') || lower.includes('cyclone')) {
            return { vulnerable: true, riskType: 'aerial' };
        }
        return null;
    }

    // Underground
    if (lower.includes('earthquake') || lower.includes('flood')) {
        return { vulnerable: true, riskType: 'underground' };
    }
    return null;
}

/**
 * Find all edges that are at risk from active disasters.
 * Uses a simple distance check between edge midpoint and disaster center.
 */
function findAtRiskEdges(
    allEdges: ProcessedEdge[],
    disasters: ActiveDisaster[],
): AtRiskEdge[] {
    const results: AtRiskEdge[] = [];
    const MAX_DIST_DEG = 5; // approximate degrees threshold for disaster influence

    for (const edge of allEdges) {
        // Edge midpoint
        const midLon = (edge.src_x + edge.dst_x) / 2;
        const midLat = (edge.src_y + edge.dst_y) / 2;

        for (const disaster of disasters) {
            const dlat = midLat - disaster.lat;
            const dlon = midLon - disaster.lon;
            const dist = Math.sqrt(dlat * dlat + dlon * dlon);
            const effectRadius = MAX_DIST_DEG * disaster.severity;

            if (dist > effectRadius) continue;

            const vuln = getVulnerability(edge.deployment, edge.edge_type, disaster.disasterType);
            if (!vuln) continue;

            const proximityFactor = 1 - (dist / effectRadius);
            results.push({
                edge,
                riskType: vuln.riskType,
                riskLevel: proximityFactor * disaster.severity,
                disasterType: disaster.disasterType,
            });
            break; // Only mark once per edge (use the first matching disaster)
        }
    }

    return results;
}

// ── Edge tier estimation ─────────────────────────────────────────────────────

/**
 * Estimate tier rank for an edge type based on name convention.
 * Local/access cables = T1, Regional/metro = T2, National/long-haul = T3,
 * Continental/submarine = T4, Global backbone/future = T5.
 */
function edgeTierRank(edgeType: string): number {
    switch (edgeType) {
        // T1 — Local / access
        case 'Copper':
        case 'FiberLocal':
        case 'TelegraphWire':
        case 'DropCable':
        case 'DistributionFiber':
            return 1;

        // T2 — Regional / metro
        case 'Microwave':
        case 'MicrowaveLink':
        case 'FiberRegional':
        case 'FeederFiber':
        case 'FiberMetro':
        case 'CoaxialCable':
        case 'CopperTrunkLine':
            return 2;

        // T3 — National / long-haul
        case 'FiberNational':
        case 'FiberLongHaul':
        case 'LongDistanceCopper':
            return 3;

        // T4 — Continental / submarine / satellite
        case 'Satellite':
        case 'SatelliteLEOLink':
        case 'EarlySatelliteLink':
        case 'Submarine':
        case 'SubseaFiberCable':
        case 'SubseaTelegraphCable':
        case 'DWDM_Backbone':
            return 4;

        // T5 — Global backbone / near-future
        case 'QuantumFiberLink':
        case 'TerahertzBeam':
        case 'LaserInterSatelliteLink':
            return 5;

        default:
            return 1;
    }
}
