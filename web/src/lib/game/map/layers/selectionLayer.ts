import { ScatterplotLayer, LineLayer } from '@deck.gl/layers';
import type { Layer } from '@deck.gl/core';
import * as bridge from '$lib/wasm/bridge';
import type { City } from '$lib/wasm/types';

/**
 * Creates a selection highlight ring (ScatterplotLayer for nodes/cities,
 * LineLayer for edges) around the currently selected entity.
 */
export function createSelectionLayer(
    selectedId: number | null,
    cachedCities: City[]
): Layer | null {
    if (selectedId === null || selectedId === undefined) return null;

    const infra = bridge.getAllInfrastructure();

    // Check infrastructure nodes
    const node = infra.nodes.find(n => n.id === selectedId);
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

    // Check infrastructure edges
    const edge = infra.edges.find(e => e.id === selectedId);
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

    // Check cities
    const city = cachedCities.find(c => c.id === selectedId);
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

/**
 * Creates edge build highlight layers: a source ring around the selected source node
 * and green rings around valid target nodes.
 */
export function createEdgeBuildHighlights(opts: {
    edgeSourceId: number | null;
    edgeTargetIds: Set<number>;
}): Layer[] {
    const { edgeSourceId, edgeTargetIds } = opts;

    if (edgeSourceId === null) return [];

    const infra = bridge.getAllInfrastructure();
    const layers: Layer[] = [];

    // Source node ring
    const sourceNode = infra.nodes.find(n => n.id === edgeSourceId);
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

    // Valid target node rings
    if (edgeTargetIds.size > 0) {
        const validTargets = infra.nodes
            .filter(n => edgeTargetIds.has(n.id))
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
