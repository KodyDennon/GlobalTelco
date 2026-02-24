import { LineLayer, ScatterplotLayer, TextLayer, IconLayer } from '@deck.gl/layers';
import type { Layer } from '@deck.gl/core';

import * as bridge from '$lib/wasm/bridge';
import { CORP_COLORS, EDGE_STYLES, NODE_TIER_SIZE, NETWORK_TIER_LABEL, toIconKey } from '../constants';

export interface IconMapping {
    x: number;
    y: number;
    width: number;
    height: number;
    mask: boolean;
}

/**
 * Creates infrastructure edge (LineLayer) and node (IconLayer with ScatterplotLayer fallback) layers,
 * plus tier badge labels (TextLayer at zoom > 5).
 * Supports congestion and traffic overlay color modes.
 */
export function createInfraLayers(opts: {
    iconAtlas: HTMLCanvasElement | null;
    iconMapping: Record<string, IconMapping>;
    iconAtlasReady: boolean;
    activeOverlay: string;
    currentZoom: number;
}): Layer[] {
    const { iconAtlas, iconMapping, iconAtlasReady, activeOverlay, currentZoom } = opts;

    const corps = bridge.getAllCorporations();
    const edgesData: any[] = [];
    const nodesData: any[] = [];
    const isCongestion = activeOverlay === 'congestion';
    const isTraffic = activeOverlay === 'traffic';
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
            let color: [number, number, number] | number[] = style.color;
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
            let color: [number, number, number] | number[] = baseColor;
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
                } else {
                    color = [100, 100, 100]; opacity = 30;
                }
            }

            nodesData.push({
                ...node,
                position: [node.x, node.y],
                color: [...color, opacity],
                icon: toIconKey(node.node_type),
                tierSize: NODE_TIER_SIZE[node.network_level] || 32,
                tierLabel: NETWORK_TIER_LABEL[node.network_level] || ''
            });
        }
    }

    const layers: Layer[] = [
        new LineLayer({
            id: 'infra-edges',
            data: edgesData,
            getSourcePosition: (d: any) => d.sourcePosition,
            getTargetPosition: (d: any) => d.targetPosition,
            getColor: (d: any) => d.color,
            getWidth: (d: any) => d.width,
            widthUnits: 'pixels',
            pickable: true,
            autoHighlight: true,
            onClick: ({ object }: any) => {
                if (object) {
                    window.dispatchEvent(new CustomEvent('entity-selected', { detail: { id: object.id, type: 'edge' } }));
                }
            }
        })
    ];

    // Icon layer when atlas is ready, otherwise fallback colored dots
    if (iconAtlasReady && iconAtlas) {
        layers.push(new IconLayer({
            id: 'infra-nodes',
            data: nodesData,
            getPosition: (d: any) => d.position,
            getIcon: (d: any) => d.icon,
            iconAtlas: iconAtlas as any,
            iconMapping: iconMapping,
            getSize: (d: any) => d.tierSize,
            sizeMinPixels: 12,
            sizeMaxPixels: 72,
            getColor: (d: any) => d.color,
            pickable: true,
            autoHighlight: true,
            onClick: ({ object }: any) => {
                if (object) {
                    window.dispatchEvent(new CustomEvent('entity-selected', { detail: { id: object.id, type: 'node' } }));
                }
            }
        }));
    } else {
        layers.push(new ScatterplotLayer({
            id: 'infra-nodes-fallback',
            data: nodesData,
            getPosition: (d: any) => d.position,
            getFillColor: (d: any) => d.color,
            getRadius: (d: any) => d.tierSize * 500,
            radiusMinPixels: 6,
            radiusMaxPixels: 24,
            pickable: true,
            onClick: ({ object }: any) => {
                if (object) {
                    window.dispatchEvent(new CustomEvent('entity-selected', { detail: { id: object.id, type: 'node' } }));
                }
            }
        }));
    }

    // Tier badge labels (visible at zoom > 5)
    if (currentZoom > 5) {
        layers.push(new TextLayer({
            id: 'node-tier-labels',
            data: nodesData,
            getPosition: (d: any) => d.position,
            getText: (d: any) => d.tierLabel,
            getSize: 10,
            getColor: [255, 255, 255, 180],
            getPixelOffset: [14, -14],
            fontFamily: 'Inter, sans-serif',
            fontWeight: 'bold',
            parameters: { depthTest: false }
        }));
    }

    return layers;
}
