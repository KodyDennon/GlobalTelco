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
    // Original 7
    FiberLocal:    { color: [34, 211, 160], width: 2 },
    FiberRegional: { color: [96, 165, 250], width: 3 },
    FiberNational: { color: [129, 140, 248], width: 4 },
    Copper:        { color: [217, 119, 6], width: 2 },
    Microwave:     { color: [34, 211, 238], width: 3 },
    Satellite:     { color: [251, 191, 36], width: 3 },
    Submarine:     { color: [59, 130, 246], width: 5 },

    // Era 1: Telegraph
    TelegraphWire:         { color: [139, 115, 85], width: 2 },   // old rope brown
    SubseaTelegraphCable:  { color: [139, 115, 85], width: 5 },   // submarine class

    // Era 2: Telephone
    CopperTrunkLine:       { color: [184, 115, 51], width: 2 },   // copper, local/access
    LongDistanceCopper:    { color: [184, 115, 51], width: 3 },   // metro/regional

    // Era 3: Early Digital
    CoaxialCable:          { color: [105, 105, 105], width: 2 },   // grey, local/access
    MicrowaveLink:         { color: [34, 211, 238], width: 3 },    // metro/regional
    EarlySatelliteLink:    { color: [251, 191, 36], width: 3 },    // satellite class

    // Era 4: Internet
    SubseaFiberCable:      { color: [59, 130, 246], width: 5 },    // submarine class

    // Era 5: Modern
    FiberMetro:            { color: [129, 140, 248], width: 3 },    // metro/regional
    FiberLongHaul:         { color: [99, 102, 241], width: 4 },     // national/long-haul
    DWDM_Backbone:         { color: [79, 70, 229], width: 5 },      // backbone
    SatelliteLEOLink:      { color: [251, 191, 36], width: 3 },     // satellite class
    FeederFiber:           { color: [34, 211, 160], width: 3 },     // feeder
    DistributionFiber:     { color: [52, 211, 153], width: 2 },     // distribution
    DropCable:             { color: [110, 231, 183], width: 1 },    // drop cable, thinnest

    // Era 6: Near Future
    QuantumFiberLink:      { color: [192, 132, 252], width: 6 },    // near-future
    TerahertzBeam:         { color: [244, 114, 182], width: 6 },    // near-future
    LaserInterSatelliteLink: { color: [251, 191, 36], width: 6 },   // near-future

    // Satellite dynamic edges
    SatelliteDownlink:   { color: [56, 189, 248], width: 2 },     // sky blue — sat to ground
    IntraplaneISL:       { color: [96, 165, 250], width: 2 },     // blue — in-plane laser link
    CrossplaneISL:       { color: [129, 140, 248], width: 2 },    // indigo — cross-plane laser link
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
    OceanTrench:  [3, 8, 22],
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
    OceanTrench:  [4, 10, 28],
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
 * Handles acronyms (ISPGateway -> isp-gateway), underscores (DWDM_Terminal -> dwdm-terminal),
 * and standard camelCase (CellTower -> cell-tower).
 */
export function toIconKey(camelCase: string): string {
    return camelCase
        .replace(/([A-Z]+)([A-Z][a-z])/g, '$1-$2')  // ISPGateway -> ISP-Gateway
        .replace(/([a-z])([A-Z])/g, '$1-$2')          // CellTower -> Cell-Tower
        .replace(/_/g, '-')                             // DWDM_Terminal -> DWDM-Terminal
        .toLowerCase();
}
