// ── Tooltip / mouse handling ────────────────────────────────────────────────
// Extracts entity pick + tooltip formatting from the map renderer.

import { tooltipData } from '$lib/stores/uiState';
import type { ActiveDisaster } from '../WeatherLayer';

// ── Active disaster state (set by MapRenderer) ──────────────────────────────
let _activeDisasters: ActiveDisaster[] = [];

/** Update the active disaster list used for vulnerability tooltip lines. */
export function setTooltipDisasters(disasters: ActiveDisaster[]): void {
    _activeDisasters = disasters;
}

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
        const health = object.health !== undefined ? `\nHealth: ${Math.round(object.health * 100)}%` : '';
        const owner = object.owner_name ? `\nOwner: ${object.owner_name}` : '';
        return `${object.node_type}${building}\n${util}${load}${health}${owner}`;
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
        const health = object.health !== undefined ? ` | HP: ${Math.round(object.health * 100)}%` : '';
        const owner = object.owner_name ? `\nOwner: ${object.owner_name}` : '';
        const vulnerability = getEdgeVulnerabilityWarning(object);
        return `${object.edge_type}${deploy}${wps}\nLength: ${Math.round(object.length_km || 0)}km${bw}${util}${load}${health}${owner}${vulnerability}`;
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

// ── Edge vulnerability warning ──────────────────────────────────────────────

/** Submarine edge types. */
const SUBMARINE_EDGE_TYPES = new Set([
    'Submarine', 'SubseaFiberCable', 'SubseaTelegraphCable',
]);

/**
 * Check if an edge is vulnerable to any active disaster, and return
 * a warning line for the tooltip. Returns empty string if no threat.
 */
function getEdgeVulnerabilityWarning(edge: any): string {
    if (_activeDisasters.length === 0) return '';

    const deployment = edge.deployment ?? 'Underground';
    const edgeType = edge.edge_type ?? '';
    const midLon = ((edge.src_x ?? 0) + (edge.dst_x ?? 0)) / 2;
    const midLat = ((edge.src_y ?? 0) + (edge.dst_y ?? 0)) / 2;
    const isSubmarine = SUBMARINE_EDGE_TYPES.has(edgeType);

    for (const disaster of _activeDisasters) {
        const dlat = midLat - disaster.lat;
        const dlon = midLon - disaster.lon;
        const dist = Math.sqrt(dlat * dlat + dlon * dlon);
        const effectRadius = 5 * disaster.severity;

        if (dist > effectRadius) continue;

        const lower = disaster.disasterType.toLowerCase();

        if (isSubmarine) {
            if (lower.includes('earthquake') || lower.includes('hurricane') ||
                lower.includes('typhoon') || lower.includes('storm')) {
                return `\n!! ${disaster.disasterType} Warning - Submarine cable at risk`;
            }
        } else if (deployment === 'Aerial') {
            if (lower.includes('hurricane') || lower.includes('typhoon') ||
                lower.includes('storm') || lower.includes('thunder') ||
                lower.includes('ice') || lower.includes('blizzard') ||
                lower.includes('landslide') || lower.includes('cyclone')) {
                return `\n!! ${disaster.disasterType} Warning - Aerial cable at risk`;
            }
        } else {
            // Underground
            if (lower.includes('earthquake') || lower.includes('flood')) {
                return `\n!! ${disaster.disasterType} Warning - Underground cable at risk`;
            }
        }
    }

    return '';
}
