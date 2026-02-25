// ── Tile configuration for MapLibre GL ──────────────────────────────────────
// All tile sources are 100% free — no API keys required.
// Two styles: satellite (real earth) and procgen (blank dark canvas).

import type { StyleSpecification } from 'maplibre-gl';

// ESRI World Imagery — free satellite raster tiles, global coverage
const ESRI_SATELLITE_URL =
    'https://services.arcgisonline.com/arcgis/rest/services/World_Imagery/MapServer/tile/{z}/{y}/{x}';

// OpenFreeMap — free vector tiles for borders, labels, roads overlay
const VECTOR_TILES_URL = 'https://tiles.openfreemap.org/planet';

// Shared glyphs URL for text rendering in both styles
const GLYPHS_URL = 'https://tiles.openfreemap.org/fonts/{fontstack}/{range}.pbf';

/**
 * Real earth satellite style for GlobalTelco.
 * Base: ESRI satellite imagery (actual photos of earth).
 * Overlay: subtle vector borders and labels from OpenFreeMap.
 * Satellite brightness dimmed for game aesthetic.
 * Borders adaptive: bold at low zoom, subtle at high zoom.
 * Max zoom capped at 10 for strategy game scale.
 */
export const satelliteMapStyle: StyleSpecification = {
    version: 8,
    name: 'GlobalTelco Satellite',
    glyphs: GLYPHS_URL,
    sources: {
        'esri-satellite': {
            type: 'raster',
            tiles: [ESRI_SATELLITE_URL],
            tileSize: 256,
            attribution:
                '&copy; <a href="https://www.esri.com">Esri</a>, Maxar, Earthstar Geographics',
            maxzoom: 18,
        },
        openmaptiles: {
            type: 'vector',
            url: VECTOR_TILES_URL,
            attribution:
                '<a href="https://openfreemap.org">OpenFreeMap</a> | <a href="https://openstreetmap.org">OSM</a>',
        },
    },
    layers: [
        // Background — deep space (visible before tiles load)
        {
            id: 'background',
            type: 'background',
            paint: { 'background-color': '#030810' },
        },
        // Satellite imagery base layer — dimmed for night-earth aesthetic
        {
            id: 'satellite',
            type: 'raster',
            source: 'esri-satellite',
            paint: {
                'raster-brightness-max': 0.7,
                'raster-brightness-min': 0.0,
                'raster-contrast': 0.15,
                'raster-saturation': -0.3,
            },
        },
        // Country boundaries — bold at low zoom, subtle at high zoom
        {
            id: 'boundary-country',
            type: 'line',
            source: 'openmaptiles',
            'source-layer': 'boundary',
            filter: ['<=', ['get', 'admin_level'], 2],
            paint: {
                'line-color': 'rgba(200, 210, 230, 0.5)',
                'line-width': [
                    'interpolate', ['linear'], ['zoom'],
                    0, 0.6,
                    3, 1.5,
                    6, 0.8,
                    10, 0.3,
                ],
                'line-opacity': [
                    'interpolate', ['linear'], ['zoom'],
                    0, 0.6,
                    6, 0.35,
                    10, 0.15,
                ],
                'line-dasharray': [6, 3],
            },
        },
        // State/region boundaries — visible at mid-zoom
        {
            id: 'boundary-state',
            type: 'line',
            source: 'openmaptiles',
            'source-layer': 'boundary',
            filter: [
                'all',
                ['>=', ['get', 'admin_level'], 3],
                ['<=', ['get', 'admin_level'], 4],
            ],
            minzoom: 3,
            paint: {
                'line-color': 'rgba(180, 190, 210, 0.3)',
                'line-width': [
                    'interpolate', ['linear'], ['zoom'],
                    3, 0.3,
                    6, 0.6,
                    10, 0.2,
                ],
                'line-opacity': [
                    'interpolate', ['linear'], ['zoom'],
                    3, 0.3,
                    8, 0.12,
                ],
                'line-dasharray': [3, 2],
            },
        },
        // Country labels — readable over satellite, fade at high zoom
        {
            id: 'label-country',
            type: 'symbol',
            source: 'openmaptiles',
            'source-layer': 'place',
            filter: ['==', ['get', 'class'], 'country'],
            maxzoom: 6,
            layout: {
                'text-field': ['coalesce', ['get', 'name:en'], ['get', 'name']],
                'text-font': ['Noto Sans Bold'],
                'text-size': [
                    'interpolate', ['linear'], ['zoom'],
                    1, 10,
                    4, 16,
                ],
                'text-transform': 'uppercase',
                'text-letter-spacing': 0.15,
                'text-max-width': 8,
            },
            paint: {
                'text-color': 'rgba(220, 225, 240, 0.7)',
                'text-halo-color': 'rgba(0, 0, 0, 0.8)',
                'text-halo-width': 2,
                'text-opacity': [
                    'interpolate', ['linear'], ['zoom'],
                    1, 0.6,
                    5, 0.3,
                    6, 0.1,
                ],
            },
        },
        // ── Road layers from OpenFreeMap transportation source-layer ──────
        // Motorway/trunk casing (dark outline under the road fill)
        {
            id: 'road-motorway-casing',
            type: 'line',
            source: 'openmaptiles',
            'source-layer': 'transportation',
            filter: ['in', ['get', 'class'], ['literal', ['motorway', 'trunk']]],
            minzoom: 4,
            layout: {
                'line-cap': 'round',
                'line-join': 'round',
            },
            paint: {
                'line-color': 'rgba(30, 35, 45, 0.8)',
                'line-width': [
                    'interpolate', ['exponential', 1.5], ['zoom'],
                    4, 1.5,
                    6, 3,
                    8, 5,
                    10, 8,
                ],
                'line-opacity': [
                    'interpolate', ['linear'], ['zoom'],
                    4, 0.3,
                    6, 0.5,
                    10, 0.6,
                ],
            },
        },
        // Motorway/trunk fill (lighter line on top of casing)
        {
            id: 'road-motorway-fill',
            type: 'line',
            source: 'openmaptiles',
            'source-layer': 'transportation',
            filter: ['in', ['get', 'class'], ['literal', ['motorway', 'trunk']]],
            minzoom: 4,
            layout: {
                'line-cap': 'round',
                'line-join': 'round',
            },
            paint: {
                'line-color': 'rgba(74, 85, 104, 0.9)',
                'line-width': [
                    'interpolate', ['exponential', 1.5], ['zoom'],
                    4, 0.8,
                    6, 1.8,
                    8, 3,
                    10, 5,
                ],
                'line-opacity': [
                    'interpolate', ['linear'], ['zoom'],
                    4, 0.3,
                    6, 0.5,
                    10, 0.65,
                ],
            },
        },
        // Primary road casing
        {
            id: 'road-primary-casing',
            type: 'line',
            source: 'openmaptiles',
            'source-layer': 'transportation',
            filter: ['==', ['get', 'class'], 'primary'],
            minzoom: 6,
            layout: {
                'line-cap': 'round',
                'line-join': 'round',
            },
            paint: {
                'line-color': 'rgba(30, 35, 45, 0.7)',
                'line-width': [
                    'interpolate', ['exponential', 1.5], ['zoom'],
                    6, 1.2,
                    8, 2.5,
                    10, 5,
                ],
                'line-opacity': [
                    'interpolate', ['linear'], ['zoom'],
                    6, 0.25,
                    8, 0.4,
                    10, 0.5,
                ],
            },
        },
        // Primary road fill
        {
            id: 'road-primary-fill',
            type: 'line',
            source: 'openmaptiles',
            'source-layer': 'transportation',
            filter: ['==', ['get', 'class'], 'primary'],
            minzoom: 6,
            layout: {
                'line-cap': 'round',
                'line-join': 'round',
            },
            paint: {
                'line-color': 'rgba(113, 128, 150, 0.85)',
                'line-width': [
                    'interpolate', ['exponential', 1.5], ['zoom'],
                    6, 0.6,
                    8, 1.5,
                    10, 3,
                ],
                'line-opacity': [
                    'interpolate', ['linear'], ['zoom'],
                    6, 0.25,
                    8, 0.4,
                    10, 0.55,
                ],
            },
        },
        // Secondary road casing
        {
            id: 'road-secondary-casing',
            type: 'line',
            source: 'openmaptiles',
            'source-layer': 'transportation',
            filter: ['==', ['get', 'class'], 'secondary'],
            minzoom: 7,
            layout: {
                'line-cap': 'round',
                'line-join': 'round',
            },
            paint: {
                'line-color': 'rgba(30, 35, 45, 0.6)',
                'line-width': [
                    'interpolate', ['exponential', 1.5], ['zoom'],
                    7, 0.8,
                    9, 2,
                    10, 3.5,
                ],
                'line-opacity': [
                    'interpolate', ['linear'], ['zoom'],
                    7, 0.2,
                    9, 0.35,
                    10, 0.45,
                ],
            },
        },
        // Secondary road fill
        {
            id: 'road-secondary-fill',
            type: 'line',
            source: 'openmaptiles',
            'source-layer': 'transportation',
            filter: ['==', ['get', 'class'], 'secondary'],
            minzoom: 7,
            layout: {
                'line-cap': 'round',
                'line-join': 'round',
            },
            paint: {
                'line-color': 'rgba(160, 174, 192, 0.8)',
                'line-width': [
                    'interpolate', ['exponential', 1.5], ['zoom'],
                    7, 0.4,
                    9, 1.2,
                    10, 2,
                ],
                'line-opacity': [
                    'interpolate', ['linear'], ['zoom'],
                    7, 0.2,
                    9, 0.35,
                    10, 0.45,
                ],
            },
        },
        // Tertiary + residential road casing
        {
            id: 'road-minor-casing',
            type: 'line',
            source: 'openmaptiles',
            'source-layer': 'transportation',
            filter: ['in', ['get', 'class'], ['literal', ['tertiary', 'minor', 'service']]],
            minzoom: 9,
            layout: {
                'line-cap': 'round',
                'line-join': 'round',
            },
            paint: {
                'line-color': 'rgba(30, 35, 45, 0.5)',
                'line-width': [
                    'interpolate', ['exponential', 1.5], ['zoom'],
                    9, 0.5,
                    10, 1.5,
                ],
                'line-opacity': [
                    'interpolate', ['linear'], ['zoom'],
                    9, 0.15,
                    10, 0.3,
                ],
            },
        },
        // Tertiary + residential road fill
        {
            id: 'road-minor-fill',
            type: 'line',
            source: 'openmaptiles',
            'source-layer': 'transportation',
            filter: ['in', ['get', 'class'], ['literal', ['tertiary', 'minor', 'service']]],
            minzoom: 9,
            layout: {
                'line-cap': 'round',
                'line-join': 'round',
            },
            paint: {
                'line-color': 'rgba(203, 213, 224, 0.7)',
                'line-width': [
                    'interpolate', ['exponential', 1.5], ['zoom'],
                    9, 0.3,
                    10, 0.8,
                ],
                'line-opacity': [
                    'interpolate', ['linear'], ['zoom'],
                    9, 0.15,
                    10, 0.3,
                ],
            },
        },
        // City labels from tiles — subtle, high zoom only
        {
            id: 'label-city',
            type: 'symbol',
            source: 'openmaptiles',
            'source-layer': 'place',
            filter: ['in', ['get', 'class'], ['literal', ['city', 'town']]],
            minzoom: 7,
            layout: {
                'text-field': ['coalesce', ['get', 'name:en'], ['get', 'name']],
                'text-font': ['Noto Sans Regular'],
                'text-size': [
                    'interpolate', ['linear'], ['zoom'],
                    7, 9,
                    12, 12,
                ],
                'text-max-width': 8,
            },
            paint: {
                'text-color': 'rgba(200, 210, 230, 0.6)',
                'text-halo-color': 'rgba(0, 0, 0, 0.7)',
                'text-halo-width': 1.5,
                'text-opacity': [
                    'interpolate', ['linear'], ['zoom'],
                    7, 0.3,
                    10, 0.5,
                ],
            },
        },
    ],
};

/**
 * Style for procedurally generated worlds -- no external tiles.
 * Blank dark canvas -- all terrain, oceans, borders rendered via deck.gl overlay layers.
 */
export const procgenMapStyle: StyleSpecification = {
    version: 8,
    name: 'GlobalTelco Procgen',
    glyphs: GLYPHS_URL,
    sources: {},
    layers: [
        {
            id: 'background',
            type: 'background',
            paint: { 'background-color': '#030810' },
        },
    ],
};
