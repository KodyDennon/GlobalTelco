// ── Map rendering constants ─────────────────────────────────────────────────
// Extracted from the monolithic MapRenderer.ts for modular reuse.

/** Corporation brand colors — indexed by corp order. */
export const CORP_COLORS: [number, number, number][] = [
    [16, 185, 129],  // Emerald
    [59, 130, 246],  // Blue
    [245, 158, 11],  // Amber
    [239, 68, 68],   // Red
    [139, 92, 246],  // Violet
    [236, 72, 153],  // Pink
    [20, 184, 166],  // Teal
    [249, 115, 22],  // Orange
];

/** Edge type visual styles — keyed by Rust EdgeType Debug names. */
export const EDGE_STYLES: Record<string, { color: [number, number, number]; width: number }> = {
    FiberLocal:    { color: [34, 211, 160], width: 2 },
    FiberRegional: { color: [96, 165, 250], width: 3 },
    FiberNational: { color: [129, 140, 248], width: 5 },
    Copper:        { color: [217, 119, 6], width: 1 },
    Microwave:     { color: [34, 211, 238], width: 2 },
    Satellite:     { color: [251, 191, 36], width: 3 },
    Submarine:     { color: [59, 130, 246], width: 5 },
};

/** Night-earth satellite palette for terrain — dark, muted tones. */
export const SATELLITE_COLORS: Record<string, [number, number, number]> = {
    Urban:        [55, 55, 72],
    Suburban:     [42, 45, 52],
    Rural:        [24, 42, 26],
    Mountainous:  [48, 42, 35],
    Desert:       [62, 52, 32],
    Coastal:      [28, 48, 58],
    Tundra:       [48, 56, 64],
    Frozen:       [62, 70, 78],
    OceanShallow: [12, 24, 52],
    OceanDeep:    [6, 12, 32],
    Ocean:        [6, 12, 32],
};

/** Brighter terrain colors for the terrain overlay toggle. */
export const TERRAIN_OVERLAY_COLORS: Record<string, [number, number, number]> = {
    Urban:        [110, 110, 135],
    Suburban:     [85, 95, 85],
    Rural:        [50, 95, 50],
    Mountainous:  [95, 85, 72],
    Desert:       [125, 108, 68],
    Coastal:      [55, 100, 115],
    OceanShallow: [22, 55, 100],
    OceanDeep:    [8, 18, 50],
    Ocean:        [8, 18, 50],
    Tundra:       [85, 100, 115],
    Frozen:       [110, 120, 130],
};

/** Node icon size by network tier — keyed by Rust NetworkLevel Debug names. */
export const NODE_TIER_SIZE: Record<string, number> = {
    Local: 24,
    Regional: 32,
    National: 40,
    Continental: 48,
    GlobalBackbone: 56,
};

/** Short tier badge labels for map display. */
export const NETWORK_TIER_LABEL: Record<string, string> = {
    Local: 'T1',
    Regional: 'T2',
    National: 'T3',
    Continental: 'T4',
    GlobalBackbone: 'T5',
};

/**
 * Convert a Rust CamelCase enum variant name to a kebab-case icon key.
 * e.g. "CellTower" -> "cell-tower", "DataCenter" -> "data-center"
 */
export function toIconKey(camelCase: string): string {
    return camelCase.replace(/([a-z])([A-Z])/g, '$1-$2').toLowerCase();
}
