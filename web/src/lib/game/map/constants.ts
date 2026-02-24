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

/** Night-earth satellite palette for terrain — high contrast against #030810 background. */
export const SATELLITE_COLORS: Record<string, [number, number, number]> = {
    Urban:        [130, 125, 148],
    Suburban:     [100, 110, 105],
    Rural:        [55, 105, 58],
    Mountainous:  [115, 100, 82],
    Desert:       [155, 130, 85],
    Coastal:      [65, 115, 130],
    Tundra:       [110, 125, 140],
    Frozen:       [145, 155, 170],
    OceanShallow: [18, 42, 82],
    OceanDeep:    [8, 18, 42],
    Ocean:        [8, 18, 42],
};

/** Brighter terrain colors for the terrain overlay toggle. */
export const TERRAIN_OVERLAY_COLORS: Record<string, [number, number, number]> = {
    Urban:        [160, 155, 180],
    Suburban:     [120, 135, 120],
    Rural:        [70, 140, 70],
    Mountainous:  [140, 120, 95],
    Desert:       [180, 155, 95],
    Coastal:      [80, 140, 155],
    OceanShallow: [25, 60, 110],
    OceanDeep:    [10, 22, 55],
    Ocean:        [10, 22, 55],
    Tundra:       [120, 140, 155],
    Frozen:       [155, 165, 180],
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
