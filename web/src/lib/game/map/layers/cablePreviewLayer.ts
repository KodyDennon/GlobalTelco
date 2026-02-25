// ── Cable Drawing Preview Layers ─────────────────────────────────────────────
// Creates deck.gl layers for the interactive cable drawing preview:
// - Spline path preview (PathLayer)
// - Waypoint handle dots (ScatterplotLayer)
// - Source node highlight ring (ScatterplotLayer)
// - Cursor-following extension line
// - Road snap highlight (bright cyan road segment)
// - Auto-route preview (dashed line along roads)

import { PathLayer, ScatterplotLayer } from '@deck.gl/layers';
import type { Layer } from '@deck.gl/core';
import { catmullRomSpline } from '../spline';

export interface CableDrawingState {
    /** All placed waypoints so far (lon, lat pairs). */
    waypoints: [number, number][];
    /** Current mouse cursor position on the map (lon, lat), or null if off-map. */
    cursorPosition: [number, number] | null;
    /** Deployment method affects visual style. */
    deployment: 'Aerial' | 'Underground';
    /** Position of the source node (first waypoint's origin). */
    sourceNodePos: [number, number] | null;
    /** Whether drawing is actively in progress. */
    isDrawing: boolean;
    /** If snapping to a road, the snapped position [lon, lat]. Null if not snapping. */
    roadSnapPosition?: [number, number] | null;
    /** The road segment being snapped to, as [fromLon, fromLat, toLon, toLat]. */
    roadSnapSegment?: [[number, number], [number, number]] | null;
    /** Whether the cursor is currently snapped to a road. */
    isSnappedToRoad?: boolean;
    /** Auto-route waypoints along roads (preview dashed line). */
    autoRouteWaypoints?: [number, number][] | null;
    /** Auto-route cost (road route, in currency). */
    autoRouteCost?: number | null;
    /** Direct route cost for comparison. */
    directRouteCost?: number | null;
    /** Whether auto-route is available and being shown. */
    autoRouteAvailable?: boolean;
}

/**
 * Create deck.gl layers for the cable drawing preview.
 * Returns an empty array if not currently drawing.
 */
export function createCablePreviewLayers(state: CableDrawingState): Layer[] {
    if (!state.isDrawing || state.waypoints.length === 0) return [];

    const layers: Layer[] = [];

    // ── 1. Build the spline preview path ─────────────────────────────────
    // Include cursor position as a temporary extension point
    const allPoints: [number, number][] = [...state.waypoints];
    if (state.cursorPosition) {
        allPoints.push(state.cursorPosition);
    }

    let previewPath: [number, number][];
    if (allPoints.length >= 3) {
        previewPath = catmullRomSpline(allPoints, 12);
    } else if (allPoints.length === 2) {
        // Straight line for just 2 points
        previewPath = [allPoints[0], allPoints[1]];
    } else {
        previewPath = allPoints;
    }

    if (previewPath.length >= 2) {
        // Main spline preview line
        const isAerial = state.deployment === 'Aerial';
        layers.push(new PathLayer({
            id: 'cable-preview-path',
            data: [{ path: previewPath }],
            getPath: (d: { path: [number, number][] }) => d.path,
            getColor: [251, 191, 36, 200], // amber
            getWidth: 3,
            widthUnits: 'pixels',
            widthMinPixels: 2,
            widthMaxPixels: 6,
            jointRounded: true,
            capRounded: true,
            getDashArray: isAerial ? [8, 4] : [0, 0],
            dashJustified: true,
            pickable: false,
            parameters: { depthTest: false },
        }));

        // Glow behind the path
        layers.push(new PathLayer({
            id: 'cable-preview-glow',
            data: [{ path: previewPath }],
            getPath: (d: { path: [number, number][] }) => d.path,
            getColor: [251, 191, 36, 50],
            getWidth: 8,
            widthUnits: 'pixels',
            jointRounded: true,
            capRounded: true,
            pickable: false,
            parameters: {
                depthTest: false,
                blend: true,
                blendFunc: [WebGLRenderingContext.SRC_ALPHA, WebGLRenderingContext.ONE],
            },
        }));
    }

    // ── 2. Committed waypoint handles ────────────────────────────────────
    if (state.waypoints.length > 0) {
        const waypointData = state.waypoints.map((wp, i) => ({
            position: wp,
            index: i,
        }));

        // Outer ring (blue border)
        layers.push(new ScatterplotLayer({
            id: 'cable-preview-waypoints',
            data: waypointData,
            getPosition: (d: { position: [number, number] }) => d.position,
            getFillColor: [255, 255, 255, 220],
            getLineColor: [59, 130, 246, 255],
            getLineWidth: 2,
            lineWidthUnits: 'pixels',
            stroked: true,
            filled: true,
            getRadius: 12000,
            radiusMinPixels: 5,
            radiusMaxPixels: 10,
            pickable: false,
            parameters: { depthTest: false },
        }));
    }

    // ── 3. Source node highlight ring ─────────────────────────────────────
    if (state.sourceNodePos) {
        layers.push(new ScatterplotLayer({
            id: 'cable-preview-source-ring',
            data: [{ position: state.sourceNodePos }],
            getPosition: (d: { position: [number, number] }) => d.position,
            getFillColor: [251, 191, 36, 30],
            getLineColor: [251, 191, 36, 255],
            getLineWidth: 3,
            lineWidthUnits: 'pixels',
            stroked: true,
            filled: true,
            getRadius: 40000,
            radiusMinPixels: 14,
            radiusMaxPixels: 28,
            pickable: false,
            parameters: { depthTest: false },
        }));
    }

    // ── 4. Cursor dot (where mouse is) ───────────────────────────────────
    if (state.cursorPosition) {
        // Use different color when snapped to road
        const isSnapped = state.isSnappedToRoad ?? false;
        layers.push(new ScatterplotLayer({
            id: 'cable-preview-cursor',
            data: [{ position: state.cursorPosition }],
            getPosition: (d: { position: [number, number] }) => d.position,
            getFillColor: isSnapped ? [0, 255, 200, 220] : [251, 191, 36, 160],
            getLineColor: isSnapped ? [0, 255, 200, 255] : [255, 255, 255, 200],
            getLineWidth: isSnapped ? 2 : 1,
            lineWidthUnits: 'pixels',
            stroked: true,
            filled: true,
            getRadius: isSnapped ? 10000 : 8000,
            radiusMinPixels: isSnapped ? 5 : 4,
            radiusMaxPixels: isSnapped ? 10 : 8,
            pickable: false,
            parameters: { depthTest: false },
        }));
    }

    // ── 5. Road snap highlight segment ──────────────────────────────────
    if (state.isSnappedToRoad && state.roadSnapSegment) {
        const seg = state.roadSnapSegment;
        layers.push(new PathLayer({
            id: 'cable-preview-road-snap',
            data: [{ path: [seg[0], seg[1]] }],
            getPath: (d: { path: [number, number][] }) => d.path,
            getColor: [0, 255, 200, 220],
            getWidth: 5,
            widthUnits: 'pixels',
            widthMinPixels: 3,
            widthMaxPixels: 8,
            capRounded: true,
            jointRounded: true,
            pickable: false,
            parameters: { depthTest: false },
        }));

        // Bright glow on snapped road segment
        layers.push(new PathLayer({
            id: 'cable-preview-road-snap-glow',
            data: [{ path: [seg[0], seg[1]] }],
            getPath: (d: { path: [number, number][] }) => d.path,
            getColor: [0, 255, 200, 60],
            getWidth: 12,
            widthUnits: 'pixels',
            capRounded: true,
            jointRounded: true,
            pickable: false,
            parameters: {
                depthTest: false,
                blend: true,
                blendFunc: [WebGLRenderingContext.SRC_ALPHA, WebGLRenderingContext.ONE],
            },
        }));
    }

    // ── 6. Auto-route preview (dashed line along roads) ─────────────────
    if (state.autoRouteAvailable && state.autoRouteWaypoints && state.autoRouteWaypoints.length >= 2) {
        let autoPath: [number, number][];
        if (state.autoRouteWaypoints.length >= 3) {
            autoPath = catmullRomSpline(state.autoRouteWaypoints, 8);
        } else {
            autoPath = [...state.autoRouteWaypoints];
        }

        // Dashed green preview line for the auto-route
        layers.push(new PathLayer({
            id: 'cable-preview-autoroute',
            data: [{ path: autoPath }],
            getPath: (d: { path: [number, number][] }) => d.path,
            getColor: [16, 185, 129, 180],
            getWidth: 3,
            widthUnits: 'pixels',
            widthMinPixels: 2,
            widthMaxPixels: 5,
            jointRounded: true,
            capRounded: true,
            getDashArray: [6, 4],
            dashJustified: true,
            pickable: false,
            parameters: { depthTest: false },
        }));

        // Subtle glow behind auto-route
        layers.push(new PathLayer({
            id: 'cable-preview-autoroute-glow',
            data: [{ path: autoPath }],
            getPath: (d: { path: [number, number][] }) => d.path,
            getColor: [16, 185, 129, 35],
            getWidth: 10,
            widthUnits: 'pixels',
            jointRounded: true,
            capRounded: true,
            pickable: false,
            parameters: {
                depthTest: false,
                blend: true,
                blendFunc: [WebGLRenderingContext.SRC_ALPHA, WebGLRenderingContext.ONE],
            },
        }));
    }

    return layers;
}
