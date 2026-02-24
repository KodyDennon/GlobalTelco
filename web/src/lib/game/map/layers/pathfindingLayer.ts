import { PathLayer, LineLayer } from '@deck.gl/layers';
import type { Layer } from '@deck.gl/core';

import * as bridge from '$lib/wasm/bridge';
import { GridPathfinder, needsTerrainRouting } from '$lib/game/GridPathfinder';
import { selectedEdgeType } from '$lib/stores/uiState';
import { get } from 'svelte/store';

/**
 * Creates a terrain-aware pathfinding preview when hovering a target node
 * during edge building. Uses GridPathfinder for routed paths on land/submarine
 * edges, falls back to a straight line for wireless edge types.
 */
export function createPathfindingPreviewLayer(opts: {
    edgeSourceId: number | null;
    hoveredEntity: { type: string; object: any } | null;
    pathfinder: GridPathfinder;
}): Layer | null {
    const { edgeSourceId, hoveredEntity, pathfinder } = opts;

    if (edgeSourceId === null || !hoveredEntity || hoveredEntity.type !== 'node') {
        return null;
    }

    const allInfra = bridge.getAllInfrastructure();
    const sourceNode = allInfra.nodes.find(n => n.id === edgeSourceId);
    const targetNode = allInfra.nodes.find(n => n.id === hoveredEntity.object?.id);

    if (!sourceNode || !targetNode || sourceNode.id === targetNode.id) return null;

    const srcCell = sourceNode.cell_index;
    const tgtCell = targetNode.cell_index;

    if (srcCell !== undefined && tgtCell !== undefined) {
        const edgeType = get(selectedEdgeType);
        if (needsTerrainRouting(edgeType)) {
            const path = pathfinder.findPath(srcCell, tgtCell, edgeType);
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

    // Fallback: straight line for wireless edge types or when cell indices are unavailable
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
