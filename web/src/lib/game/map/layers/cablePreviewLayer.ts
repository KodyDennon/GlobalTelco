// ── Cable Drawing Preview Layers ─────────────────────────────────────────────
// Creates deck.gl layers for the interactive cable drawing preview:
// - Spline path preview (PathLayer)
// - Waypoint handle dots (ScatterplotLayer)
// - Source node highlight ring (ScatterplotLayer)
// - Cursor-following extension line

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
        layers.push(new ScatterplotLayer({
            id: 'cable-preview-cursor',
            data: [{ position: state.cursorPosition }],
            getPosition: (d: { position: [number, number] }) => d.position,
            getFillColor: [251, 191, 36, 160],
            getLineColor: [255, 255, 255, 200],
            getLineWidth: 1,
            lineWidthUnits: 'pixels',
            stroked: true,
            filled: true,
            getRadius: 8000,
            radiusMinPixels: 4,
            radiusMaxPixels: 8,
            pickable: false,
            parameters: { depthTest: false },
        }));
    }

    return layers;
}
