// ── Waypoint Editor Preview Layers ───────────────────────────────────────────
// Creates deck.gl layers for the post-build waypoint editor:
// - Spline path preview (PathLayer) showing the edited route
// - Draggable waypoint handles (ScatterplotLayer, bright yellow)
// - Hover/drag highlight on the active waypoint

import { PathLayer, ScatterplotLayer } from '@deck.gl/layers';
import type { Layer } from '@deck.gl/core';
import { catmullRomSpline } from '../../map/spline';

export interface WaypointEditorState {
    /** Whether the editor is active. */
    editing: boolean;
    /** The edge ID being edited. */
    edgeId: number | null;
    /** Current waypoints being edited. */
    waypoints: [number, number][];
    /** Index of waypoint currently being dragged, or null. */
    draggingIndex: number | null;
    /** Current cursor position on the map. */
    cursorPosition: [number, number] | null;
}

/**
 * Create deck.gl layers for the waypoint editor overlay.
 * Returns an empty array if the editor is not active.
 */
export function createWaypointEditorLayers(state: WaypointEditorState): Layer[] {
    if (!state.editing || state.waypoints.length < 2) return [];

    const layers: Layer[] = [];

    // ── 1. Build the spline preview path ─────────────────────────────────
    let previewPath: [number, number][];
    if (state.waypoints.length >= 3) {
        previewPath = catmullRomSpline(state.waypoints, 12);
    } else {
        previewPath = [state.waypoints[0], state.waypoints[1]];
    }

    if (previewPath.length >= 2) {
        // Edited route path
        layers.push(new PathLayer({
            id: 'waypoint-editor-path',
            data: [{ path: previewPath }],
            getPath: (d: { path: [number, number][] }) => d.path,
            getColor: [96, 165, 250, 200], // blue
            getWidth: 3,
            widthUnits: 'pixels',
            widthMinPixels: 2,
            widthMaxPixels: 6,
            jointRounded: true,
            capRounded: true,
            pickable: false,
            parameters: { depthTest: false },
        }));

        // Glow
        layers.push(new PathLayer({
            id: 'waypoint-editor-path-glow',
            data: [{ path: previewPath }],
            getPath: (d: { path: [number, number][] }) => d.path,
            getColor: [96, 165, 250, 40],
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

    // ── 2. Waypoint handles (draggable circles) ─────────────────────────
    const waypointData = state.waypoints.map((wp, i) => ({
        position: wp,
        index: i,
        isDragging: i === state.draggingIndex,
        isEndpoint: i === 0 || i === state.waypoints.length - 1,
    }));

    // Outer ring handles
    layers.push(new ScatterplotLayer({
        id: 'waypoint-editor-handles',
        data: waypointData,
        getPosition: (d: any) => d.position,
        getFillColor: (d: any) => d.isDragging
            ? [255, 255, 100, 255]
            : d.isEndpoint
                ? [255, 200, 60, 220]
                : [255, 230, 80, 220],
        getLineColor: (d: any) => d.isDragging
            ? [255, 255, 255, 255]
            : [180, 160, 40, 255],
        getLineWidth: 2,
        lineWidthUnits: 'pixels',
        stroked: true,
        filled: true,
        getRadius: (d: any) => d.isDragging ? 18000 : 14000,
        radiusMinPixels: 6,
        radiusMaxPixels: 14,
        pickable: true,
        autoHighlight: true,
        highlightColor: [255, 255, 100, 255],
        parameters: { depthTest: false },
        onClick: ({ object }: any) => {
            // Right-click is handled via contextmenu event at the component level
        },
        onHover: ({ object }: any) => {
            // Change cursor to grab when hovering a handle
            const canvas = document.querySelector('.map-container canvas') as HTMLCanvasElement;
            if (canvas) {
                canvas.style.cursor = object ? 'grab' : '';
            }
        },
        onDragStart: ({ object }: any) => {
            if (object) {
                window.dispatchEvent(new CustomEvent('waypoint-drag-start', {
                    detail: { waypointIndex: object.index }
                }));
            }
        },
        onDrag: () => {
            // Handled by map-mousemove in the WaypointEditor component
        },
        onDragEnd: () => {
            window.dispatchEvent(new CustomEvent('waypoint-drag-end', {}));
        },
    }));

    // Waypoint index labels (visible when zoomed in)
    // Omitted for performance — the handles are self-explanatory

    return layers;
}
