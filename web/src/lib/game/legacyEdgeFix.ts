// ── Legacy Edge Auto-Fix Tool ────────────────────────────────────────────────
// Retroactively routes old straight-line edges through terrain-optimal paths
// using the GridPathfinder. Edges with only 2 waypoints (or empty waypoints)
// are considered "legacy" and can be auto-routed.

import type { AllInfraEdge } from '$lib/wasm/types';
import { GridPathfinder, needsTerrainRouting } from './GridPathfinder';

export interface FixResult {
    edgeId: number;
    newWaypoints: [number, number][];
}

/**
 * Find all legacy edges (those with 0 or 2 waypoints) and compute
 * terrain-optimal routes for them using the GridPathfinder.
 *
 * @param edges - All infrastructure edges
 * @param pathfinder - Initialized GridPathfinder instance
 * @returns Array of { edgeId, newWaypoints } pairs for edges that were re-routed
 */
export function fixLegacyEdges(
    edges: AllInfraEdge[],
    pathfinder: GridPathfinder,
): FixResult[] {
    const results: FixResult[] = [];

    for (const edge of edges) {
        // Skip edges that already have custom waypoints (3+ points means user has edited them)
        const wpCount = edge.waypoints?.length ?? 0;
        if (wpCount > 2) continue;

        // Only route edge types that benefit from terrain routing
        if (!needsTerrainRouting(edge.edge_type)) continue;

        // Need valid cell indices for pathfinding
        if (edge.src_cell === undefined || edge.dst_cell === undefined) continue;
        if (edge.src_cell === edge.dst_cell) continue;

        // Run pathfinder
        const path = pathfinder.findPath(edge.src_cell, edge.dst_cell, edge.edge_type);

        // Only include if pathfinder found a meaningful route (more than 2 points)
        if (path.length > 2) {
            results.push({
                edgeId: edge.id,
                newWaypoints: path,
            });
        }
    }

    return results;
}
