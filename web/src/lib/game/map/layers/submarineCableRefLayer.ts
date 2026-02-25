// ── Submarine Cable Reference Layer ──────────────────────────────────────────
// TeleGeography-style reference overlay showing ~20 major real-world submarine
// cable routes. Rendered as thin, semi-transparent PathLayer lines with landing
// station markers. Only visible in Real Earth mode when the overlay is active.

import { PathLayer, ScatterplotLayer, TextLayer } from '@deck.gl/layers';
import type { Layer } from '@deck.gl/core';
import { tooltipData } from '$lib/stores/uiState';

// ── Cable route data ────────────────────────────────────────────────────────

export interface SubmarineCableRoute {
    name: string;
    year: number;
    capacity: string;
    waypoints: [number, number][];
    color: [number, number, number];
}

/**
 * Approximate waypoints for ~20 major submarine cable systems.
 * Coordinates are [longitude, latitude]. Waypoints trace start, major bends,
 * and landing points — sufficient for a game reference, not GIS precision.
 */
export const SUBMARINE_CABLE_ROUTES: SubmarineCableRoute[] = [
    {
        name: 'TAT-14',
        year: 2001,
        capacity: '3.2 Tbps',
        waypoints: [
            [-73.95, 40.65],   // Tuckerton, NJ (US)
            [-55, 46],         // Mid-Atlantic bend
            [-30, 50],         // Mid-ocean
            [-10, 51],         // Approach to Europe
            [-5.5, 50.1],      // Bude, UK
            [-1.8, 48.6],      // Saint-Valery, France
        ],
        color: [100, 180, 255],
    },
    {
        name: 'AC-1',
        year: 2000,
        capacity: '2.56 Tbps',
        waypoints: [
            [-74.0, 40.5],     // Brookhaven, NY (US)
            [-50, 44],         // Mid-Atlantic
            [-30, 48],         // Mid-ocean
            [-10, 51],         // Approach UK
            [-4.8, 50.3],      // Whitesand Bay, UK
        ],
        color: [120, 200, 240],
    },
    {
        name: 'SEA-ME-WE 5',
        year: 2017,
        capacity: '24 Tbps',
        waypoints: [
            [103.8, 1.3],      // Singapore
            [101.7, 3.1],      // Malaysia
            [100.5, 7.0],      // Strait of Malacca exit
            [80.2, 6.0],       // Sri Lanka
            [72, 10],          // Arabian Sea
            [57, 15],          // Gulf of Aden approach
            [43.1, 11.6],      // Djibouti
            [42.5, 14],        // Red Sea
            [38, 22],          // Mid Red Sea
            [33.8, 28.5],      // Suez approach
            [32.3, 31.3],      // Port Said, Egypt
            [28, 35],          // Eastern Med
            [16.5, 38.2],      // Italy (Catania)
            [5.4, 43.3],       // Toulon, France
        ],
        color: [255, 140, 60],
    },
    {
        name: 'FLAG',
        year: 1997,
        capacity: '10 Gbps',
        waypoints: [
            [-5.5, 50.1],      // Porthcurno, UK
            [-9, 37],          // Portugal coast
            [5, 36],           // Western Med
            [11, 37],          // Sicily approach
            [25, 35],          // Crete
            [32.3, 31.3],      // Egypt
            [38, 22],          // Red Sea
            [43.1, 11.6],      // Djibouti
            [57, 15],          // Arabian Sea
            [72.8, 18.9],      // Mumbai, India
            [80.2, 6.0],       // Sri Lanka
            [98, 5],           // Strait of Malacca
            [103.8, 1.3],      // Singapore
            [114, 4.5],        // Borneo approach
            [120.6, 14.6],     // Philippines
            [121, 22],         // South China Sea
            [139.7, 35.6],     // Tokyo, Japan
        ],
        color: [200, 100, 255],
    },
    {
        name: 'PEACE',
        year: 2022,
        capacity: '96 Tbps',
        waypoints: [
            [66.9, 24.8],      // Karachi, Pakistan
            [57, 18],          // Arabian Sea
            [43.1, 11.6],      // Djibouti
            [42, 15],          // Red Sea
            [38, 22],          // Mid Red Sea
            [32.3, 31.3],      // Egypt
            [25, 35],          // Eastern Med
            [16.5, 38.2],      // Sicily
            [5.4, 43.3],       // Marseille, France
        ],
        color: [60, 220, 180],
    },
    {
        name: 'Equiano',
        year: 2023,
        capacity: '144 Tbps',
        waypoints: [
            [-9.1, 38.7],      // Sesimbra, Portugal
            [-12, 33],         // Off Morocco
            [-17, 20],         // Off Mauritania
            [-17.5, 14.7],     // Senegal coast
            [-3, 5.5],         // Gulf of Guinea
            [3.4, 6.4],        // Lagos, Nigeria
            [8.5, 4.0],        // Cameroon coast
            [9, -1],           // Equatorial region
            [12, -8],          // Angola coast
            [13, -15],         // Mid-Angola
            [14.5, -22.5],     // Namibia coast
            [18.4, -33.9],     // Cape Town, South Africa
        ],
        color: [80, 200, 120],
    },
    {
        name: '2Africa',
        year: 2024,
        capacity: '180 Tbps',
        waypoints: [
            [-5.5, 50.1],      // Bude, UK
            [-10, 43],         // Bay of Biscay
            [-9.1, 38.7],      // Portugal
            [-14, 28],         // Canary Islands
            [-17, 14.7],       // Senegal
            [-14, 9.5],        // Guinea
            [-5, 5],           // Ivory Coast
            [3.4, 6.4],        // Nigeria
            [9, -1],           // Equatorial
            [12, -8],          // Angola
            [14.5, -22.5],     // Namibia
            [18.4, -33.9],     // Cape Town
            [28, -33],         // South coast SA
            [32.5, -29],       // Durban
            [40.5, -11],       // Mozambique
            [43.2, -11.7],     // Comoros
            [47, -12],         // Mayotte
            [49.3, -18.9],     // Madagascar
            [55.5, -4.6],      // Seychelles
            [57, -20.2],       // Mauritius
            [56, -10],         // Mid Indian Ocean
            [51.5, 8],         // Somalia coast
            [43.1, 11.6],      // Djibouti
            [42, 15],          // Red Sea
            [38, 22],          // Red Sea north
            [32.3, 31.3],      // Egypt
            [25, 35],          // Eastern Med
            [16.5, 38.2],      // Italy
            [5.4, 43.3],       // France
            [-1, 39.5],        // Spain east coast
            [-9.1, 38.7],      // Portugal (ring closure toward UK)
        ],
        color: [255, 80, 80],
    },
    {
        name: 'MAREA',
        year: 2018,
        capacity: '200 Tbps',
        waypoints: [
            [-73.95, 40.65],   // Virginia Beach, US
            [-55, 40],         // Mid-Atlantic
            [-30, 39],         // Mid-ocean
            [-10, 38],         // Approach Iberia
            [-2.8, 43.3],      // Bilbao, Spain
        ],
        color: [255, 200, 60],
    },
    {
        name: 'Dunant',
        year: 2020,
        capacity: '250 Tbps',
        waypoints: [
            [-73.95, 40.65],   // Virginia Beach, US
            [-50, 42],         // Mid-Atlantic
            [-25, 44],         // Mid-ocean
            [-8, 46],          // Bay of Biscay
            [-1.8, 48.6],      // Saint-Hilaire, France
        ],
        color: [60, 180, 255],
    },
    {
        name: 'Japan-US',
        year: 2001,
        capacity: '640 Gbps',
        waypoints: [
            [-118.2, 33.7],    // LA area, US
            [-140, 35],        // Mid-Pacific
            [-160, 30],        // Central Pacific
            [-180, 28],        // Date line area
            [175, 30],         // West of date line
            [155, 32],         // Approach Japan
            [139.7, 35.6],     // Chikura, Japan
        ],
        color: [180, 140, 255],
    },
    {
        name: 'Pacific Crossing',
        year: 2000,
        capacity: '1.28 Tbps',
        waypoints: [
            [-122.4, 37.6],    // Harbour Pointe, US
            [-140, 38],        // Mid-Pacific
            [-160, 33],        // Central Pacific
            [-180, 30],        // Date line
            [170, 28],         // Marshall Islands area
            [150, 30],         // Approach Japan
            [140, 35],         // Shima, Japan
        ],
        color: [160, 200, 140],
    },
    {
        name: 'Southern Cross',
        year: 2000,
        capacity: '6.4 Tbps',
        waypoints: [
            [151.2, -33.8],    // Sydney, Australia
            [165, -38],        // Tasman Sea
            [174.8, -41.3],    // Wellington, NZ
            [178, -18],        // Fiji
            [-170, -14],       // Samoa area
            [-155, -5],        // Central Pacific
            [-140, 10],        // Equatorial Pacific
            [-122.4, 37.6],    // San Jose, US
        ],
        color: [255, 160, 200],
    },
    {
        name: 'AAE-1',
        year: 2017,
        capacity: '40 Tbps',
        waypoints: [
            [114.2, 22.3],     // Hong Kong
            [109, 16],         // South China Sea
            [104, 10.3],       // Vietnam
            [103.8, 1.3],      // Singapore
            [80.2, 6.0],       // Sri Lanka
            [72.8, 18.9],      // Mumbai
            [57, 15],          // Gulf of Aden
            [43.1, 11.6],      // Djibouti
            [42, 15],          // Red Sea
            [38, 22],          // Red Sea north
            [32.3, 31.3],      // Egypt
            [25, 35],          // Crete
            [16.5, 38.2],      // Italy
            [5.4, 43.3],       // Marseille, France
        ],
        color: [220, 180, 80],
    },
    {
        name: 'PLCN',
        year: 2020,
        capacity: '144 Tbps',
        waypoints: [
            [-118.2, 33.7],    // LA area, US
            [-150, 25],        // Mid-Pacific
            [-180, 20],        // Date line
            [170, 18],         // West Pacific
            [150, 15],         // Mariana area
            [121.5, 14.6],     // Luzon, Philippines
        ],
        color: [100, 255, 200],
    },
    {
        name: 'Havfrue',
        year: 2020,
        capacity: '108 Tbps',
        waypoints: [
            [-73.95, 40.65],   // Wall Township, NJ (US)
            [-50, 48],         // Mid-Atlantic
            [-25, 54],         // Mid-ocean north
            [-12, 57],         // Approach Ireland
            [-10, 51.8],       // Old Head of Kinsale, Ireland
            [-2, 57],          // North Sea approach
            [5, 58],           // North Sea
            [8.6, 58.0],       // Stavanger, Norway
            [11.5, 55.7],      // Blaabjerg, Denmark
        ],
        color: [140, 220, 255],
    },
    {
        name: 'EllaLink',
        year: 2021,
        capacity: '100 Tbps',
        waypoints: [
            [-34.9, -8.0],     // Fortaleza, Brazil
            [-30, -2],         // Off northeast Brazil
            [-25, 10],         // Mid-Atlantic equatorial
            [-20, 20],         // Mid-ocean
            [-17, 28],         // Off Canary Islands
            [-9.5, 38.7],      // Sines, Portugal
        ],
        color: [80, 255, 160],
    },
    {
        name: 'SAEx1',
        year: 2024,
        capacity: '36 Tbps',
        waypoints: [
            [18.4, -33.9],     // Cape Town, South Africa
            [30, -30],         // Off Durban
            [40, -20],         // Mozambique Channel
            [50, -10],         // Madagascar area
            [58, 0],           // Mid Indian Ocean
            [65, 8],           // Arabian Sea
            [72.8, 18.9],      // Mumbai, India
        ],
        color: [255, 120, 160],
    },
    {
        name: 'WACS',
        year: 2012,
        capacity: '14.5 Tbps',
        waypoints: [
            [18.4, -33.9],     // Cape Town, South Africa
            [14.5, -22.5],     // Namibia
            [12, -8],          // Angola
            [9, -1],           // Gabon
            [3.4, 6.4],        // Nigeria
            [-2, 5.5],         // Ghana
            [-5, 5],           // Ivory Coast
            [-14, 9.5],        // Guinea
            [-17, 14.7],       // Senegal
            [-17, 21],         // Off Mauritania
            [-14, 28],         // Canary Islands
            [-9.1, 38.7],      // Portugal
            [-5.5, 50.1],      // UK
        ],
        color: [200, 160, 100],
    },
    {
        name: 'Asia-America Gateway',
        year: 2009,
        capacity: '2.88 Tbps',
        waypoints: [
            [-118.2, 33.7],    // LA area, US
            [-150, 22],        // Hawaii area
            [-170, 15],        // Mid-Pacific
            [175, 8],          // Micronesia
            [155, 5],          // Guam area
            [121.5, 14.6],     // Philippines
            [109, 12],         // Vietnam approach
            [106.7, 10.8],     // Ho Chi Minh City area
            [103.8, 1.3],      // Singapore
            [114.2, 22.3],     // Hong Kong
        ],
        color: [180, 100, 160],
    },
    {
        name: 'Unity',
        year: 2010,
        capacity: '7.68 Tbps',
        waypoints: [
            [-118.2, 33.7],    // LA area, US
            [-140, 33],        // Mid-Pacific
            [-160, 28],        // Central Pacific
            [-180, 25],        // Date line
            [170, 25],         // West Pacific
            [155, 30],         // Approach Japan
            [139.7, 35.6],     // Chikura, Japan
        ],
        color: [140, 180, 220],
    },
];

// ── Landing station data ────────────────────────────────────────────────────

interface LandingStation {
    position: [number, number];
    cableNames: string[];
}

/** Extract unique landing stations (first and last waypoint of each cable). */
function buildLandingStations(): LandingStation[] {
    const stationMap = new Map<string, LandingStation>();

    for (const cable of SUBMARINE_CABLE_ROUTES) {
        const endpoints = [
            cable.waypoints[0],
            cable.waypoints[cable.waypoints.length - 1],
        ];

        for (const pt of endpoints) {
            // Quantize to ~0.5 degree for dedup of nearby landings
            const key = `${(Math.round(pt[0] * 2) / 2).toFixed(1)},${(Math.round(pt[1] * 2) / 2).toFixed(1)}`;
            const existing = stationMap.get(key);
            if (existing) {
                if (!existing.cableNames.includes(cable.name)) {
                    existing.cableNames.push(cable.name);
                }
            } else {
                stationMap.set(key, {
                    position: [pt[0], pt[1]],
                    cableNames: [cable.name],
                });
            }
        }
    }

    return Array.from(stationMap.values());
}

const LANDING_STATIONS = buildLandingStations();

// ── Hover state ─────────────────────────────────────────────────────────────

let hoveredCableIndex: number | null = null;

// ── Layer factory ───────────────────────────────────────────────────────────

/**
 * Create deck.gl layers for the submarine cable reference overlay.
 *
 * @param visible - Whether the submarine_reference overlay is active.
 * @param isRealEarth - Only show on real earth maps.
 * @param currentZoom - Current map zoom level (visible at zoom 0-6).
 * @returns Array of deck.gl layers (empty if not visible).
 */
export function createSubmarineCableRefLayers(
    visible: boolean,
    isRealEarth: boolean,
    currentZoom: number,
): Layer[] {
    if (!visible || !isRealEarth || currentZoom > 7) return [];

    const layers: Layer[] = [];

    // Opacity ramps down as zoom approaches 7
    const baseOpacity = currentZoom > 5 ? Math.max(0.05, 0.3 - (currentZoom - 5) * 0.125) : 0.3;

    // ── 1. Cable route paths ────────────────────────────────────────────
    // Default: thin, muted, semi-transparent lines. Hovered cable is highlighted.

    layers.push(new PathLayer({
        id: 'submarine-ref-cables',
        data: SUBMARINE_CABLE_ROUTES,
        getPath: (d: SubmarineCableRoute) => d.waypoints,
        getColor: (d: SubmarineCableRoute, { index }: { index: number }) => {
            if (index === hoveredCableIndex) {
                return [...d.color, 220] as [number, number, number, number];
            }
            return [180, 190, 210, Math.round(baseOpacity * 255)] as [number, number, number, number];
        },
        getWidth: (d: SubmarineCableRoute, { index }: { index: number }) => {
            return index === hoveredCableIndex ? 3 : 2;
        },
        widthUnits: 'pixels',
        widthMinPixels: 1,
        widthMaxPixels: 4,
        jointRounded: true,
        capRounded: true,
        getDashArray: [8, 4],
        dashJustified: true,
        pickable: true,
        autoHighlight: false,
        onHover: (info: any) => {
            if (info.object) {
                const cable = info.object as SubmarineCableRoute;
                const idx = SUBMARINE_CABLE_ROUTES.indexOf(cable);
                if (hoveredCableIndex !== idx) {
                    hoveredCableIndex = idx;
                }
                tooltipData.set({
                    x: info.x,
                    y: info.y,
                    content: `${cable.name}\nYear: ${cable.year}\nCapacity: ${cable.capacity}`,
                });
            } else {
                if (hoveredCableIndex !== null) {
                    hoveredCableIndex = null;
                }
                tooltipData.set(null);
            }
        },
        parameters: { depthTest: false },
        updateTriggers: {
            getColor: [hoveredCableIndex, baseOpacity],
            getWidth: [hoveredCableIndex],
        },
    }));

    // ── 2. Cable name labels at midpoint ────────────────────────────────
    const labelData = SUBMARINE_CABLE_ROUTES.map((cable, index) => {
        const mid = Math.floor(cable.waypoints.length / 2);
        return {
            position: cable.waypoints[mid],
            text: cable.name,
            index,
            color: cable.color,
        };
    });

    layers.push(new TextLayer({
        id: 'submarine-ref-labels',
        data: labelData,
        getPosition: (d: (typeof labelData)[number]) => d.position,
        getText: (d: (typeof labelData)[number]) => d.text,
        getColor: (d: (typeof labelData)[number]) => {
            if (d.index === hoveredCableIndex) {
                return [...d.color, 255] as [number, number, number, number];
            }
            return [180, 190, 210, Math.round(baseOpacity * 1.5 * 255)] as [number, number, number, number];
        },
        getSize: 11,
        getTextAnchor: 'middle',
        getAlignmentBaseline: 'center',
        fontFamily: 'monospace',
        fontWeight: 'bold',
        outlineWidth: 2,
        outlineColor: [10, 15, 25, 200],
        sizeUnits: 'pixels',
        pickable: false,
        parameters: { depthTest: false },
        updateTriggers: {
            getColor: [hoveredCableIndex, baseOpacity],
        },
    }));

    // ── 3. Landing station markers ──────────────────────────────────────
    // Small semi-transparent circles at cable endpoints.

    layers.push(new ScatterplotLayer({
        id: 'submarine-ref-landings',
        data: LANDING_STATIONS,
        getPosition: (d: LandingStation) => d.position,
        getFillColor: [140, 200, 255, Math.round(baseOpacity * 0.8 * 255)],
        getLineColor: [180, 220, 255, Math.round(baseOpacity * 1.2 * 255)],
        getRadius: 20000,
        radiusMinPixels: 3,
        radiusMaxPixels: 8,
        lineWidthUnits: 'pixels',
        getLineWidth: 1,
        stroked: true,
        filled: true,
        pickable: true,
        onHover: (info: any) => {
            if (info.object) {
                const station = info.object as LandingStation;
                const cables = station.cableNames.join(', ');
                tooltipData.set({
                    x: info.x,
                    y: info.y,
                    content: `Suggested Landing\nCables: ${cables}`,
                });
            } else {
                tooltipData.set(null);
            }
        },
        parameters: { depthTest: false },
        updateTriggers: {
            getFillColor: [baseOpacity],
            getLineColor: [baseOpacity],
        },
    }));

    return layers;
}
