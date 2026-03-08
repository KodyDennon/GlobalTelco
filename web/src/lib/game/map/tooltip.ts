// ── Tooltip / mouse handling ────────────────────────────────────────────────
// Extracts entity pick + tooltip formatting from the map renderer.

import { tooltipData } from '$lib/stores/uiState';

/** Layer ID to entity type mapping for deck.gl pick results. */
const LAYER_TYPE_MAP: Record<string, string> = {
    'infra-nodes': 'node',
    'infra-nodes-fallback': 'node',
    'infra-edges': 'edge',
    'cities-icons': 'city',
    'cities-dots-fallback': 'city',
    'buildings-fill': 'building',
};

/** Result of a tooltip pick — either an entity hit or null (no hit). */
export interface TooltipHit {
    type: string;
    object: any;
}

/**
 * Handle mouse move over the map: pick the topmost entity under the cursor,
 * format tooltip content, and update the tooltipData store.
 *
 * @param e - The browser MouseEvent
 * @param pickObject - A function that calls deck.pickObject (or overlay.pickObject)
 * @param hoveredEntity - The currently hovered entity (for change detection)
 * @param renderLayers - Callback to re-render layers when hover state changes
 * @returns The new hovered entity, or null if nothing is under the cursor
 */
export function handleMapMouseMove(
    e: MouseEvent,
    pickObject: (opts: { x: number; y: number; radius: number }) => any,
    hoveredEntity: TooltipHit | null,
    renderLayers: () => void
): TooltipHit | null {
    const pickInfo = pickObject({ x: e.offsetX, y: e.offsetY, radius: 2 });

    let type: string | null = null;
    let object: any = null;

    if (pickInfo && pickInfo.object && pickInfo.layer) {
        type = LAYER_TYPE_MAP[pickInfo.layer.id] ?? null;
        if (type) object = pickInfo.object;
    }

    if (type && object) {
        if (!hoveredEntity || hoveredEntity.object.id !== object.id) {
            renderLayers();
        }

        const content = formatTooltip(type, object);
        tooltipData.set({ x: e.clientX, y: e.clientY, content });
        return { type, object };
    }

    if (hoveredEntity) {
        renderLayers();
    }
    tooltipData.set(null);
    return null;
}

/** Format a number with K/M suffix. */
function shortNum(n: number): string {
    if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
    if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
    return `${Math.round(n)}`;
}

/** Build a multi-line tooltip string from an entity's properties. */
function formatTooltip(type: string, object: any): string {
    if (type === 'city') {
        const pop = `Pop: ${shortNum(object.population)}`;
        const demand = object.telecom_demand !== undefined
            ? ` | Demand: ${Math.round(object.telecom_demand)}`
            : '';
        const sat = object.infrastructure_satisfaction !== undefined
            ? `\nService: ${Math.round(object.infrastructure_satisfaction * 100)}%`
            : '';
        const emp = object.employment_rate !== undefined
            ? ` | Jobs: ${Math.round(object.employment_rate * 100)}%`
            : '';
        const dev = object.development !== undefined
            ? `\nDev: ${Math.round(object.development * 100)}%`
            : '';
        return `${object.name}\n${pop}${demand}${sat}${emp}${dev}`;
    }

    if (type === 'node') {
        const building = object.under_construction ? ' (building...)' : '';
        const util = `Util: ${Math.round((object.utilization || 0) * 100)}%`;
        const load = object.current_load !== undefined && object.max_throughput
            ? ` (${Math.round(object.current_load)}/${Math.round(object.max_throughput)})`
            : '';
        const owner = object.owner_name ? `\nOwner: ${object.owner_name}` : '';
        return `${object.node_type}${building}\n${util}${load}${owner}`;
    }

    if (type === 'edge') {
        const deploy = object.deployment ? `\nDeployment: ${object.deployment}` : '';
        const wps = Array.isArray(object.waypoints) && object.waypoints.length > 2
            ? `\nWaypoints: ${object.waypoints.length}`
            : '';
        const util = object.bandwidth > 0
            ? `\nUtil: ${Math.round((object.current_load / object.bandwidth) * 100)}%`
            : '';
        const bw = object.bandwidth ? ` | BW: ${shortNum(object.bandwidth)}` : '';
        const load = object.current_load !== undefined ? `\nLoad: ${Math.round(object.current_load)}` : '';
        const owner = object.owner_name ? `\nOwner: ${object.owner_name}` : '';
        return `${object.edge_type}${deploy}${wps}\nLength: ${Math.round(object.length_km || 0)}km${bw}${util}${load}${owner}`;
    }

    if (type === 'building') {
        const ZONE_LABELS: Record<string, string> = {
            downtown: 'Downtown',
            commercial: 'Commercial',
            residential_inner: 'Residential (Inner)',
            residential_outer: 'Residential (Outer)',
            suburban: 'Suburban',
        };
        const STATUS_LABELS: Record<string, string> = {
            unserved: 'Unserved',
            covered: 'Covered',
            connected: 'Connected',
            competitor: 'Competitor Served',
        };
        const zoneLabel = ZONE_LABELS[object.zone] ?? object.zone ?? 'Unknown';
        const buildingType = object.zone === 'downtown' || object.zone === 'commercial'
            ? 'Commercial Building'
            : 'Residential Building';
        const demand = object.demandValue !== undefined ? `\nDemand: ${object.demandValue}` : '';
        const status = object.connectionStatus
            ? `\nStatus: ${STATUS_LABELS[object.connectionStatus] ?? object.connectionStatus}`
            : '';
        return `${buildingType}\nZone: ${zoneLabel}${demand}${status}`;
    }

    return '';
}
