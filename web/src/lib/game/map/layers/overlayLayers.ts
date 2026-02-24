import { BitmapLayer, ScatterplotLayer } from '@deck.gl/layers';
import type { Layer } from '@deck.gl/core';
import type { City, Region, CellCoverage } from '$lib/wasm/types';
import { CORP_COLORS } from '../constants';
import * as bridge from '$lib/wasm/bridge';

/**
 * Creates overlay visualization layers: terrain, demand, disaster, coverage,
 * ownership, congestion, and traffic. Each uses ScatterplotLayer with color
 * gradients (except terrain which uses BitmapLayer).
 *
 * Note: congestion and traffic overlays are handled by infraLayer coloring.
 * This function handles terrain, demand, disaster, coverage, and ownership.
 */
export function createOverlayLayers(opts: {
    activeOverlay: string;
    terrainOverlayCanvas: HTMLCanvasElement | null;
    cities: City[];
    regions: Region[];
    cellRadiusM: number;
}): Layer[] {
    const { activeOverlay, terrainOverlayCanvas, cities, regions, cellRadiusM } = opts;

    const layers: Layer[] = [];
    const overlayRadius = cellRadiusM * 1.05;

    if (activeOverlay === 'terrain') {
        if (terrainOverlayCanvas) {
            layers.push(new BitmapLayer({
                id: 'overlay-terrain',
                image: terrainOverlayCanvas as any,
                bounds: [-180, -85, 180, 85] as [number, number, number, number],
                pickable: false
            }));
        }
        return layers;
    }

    if (activeOverlay === 'demand') {
        const demandCells: { position: [number, number]; color: [number, number, number, number] }[] = [];
        for (const city of cities) {
            const intensity = Math.min(1.0, city.telecom_demand / 500);
            const r = Math.floor(59 + intensity * 196);
            const g = Math.floor(130 * (1 - intensity));
            const b = Math.floor(246 * (1 - intensity));
            for (const cp of city.cell_positions) {
                demandCells.push({
                    position: [cp.lon, cp.lat],
                    color: [r, g, b, 150]
                });
            }
        }
        layers.push(new ScatterplotLayer({
            id: 'overlay-demand',
            data: demandCells,
            getPosition: (d: any) => d.position,
            getFillColor: (d: any) => d.color,
            getRadius: overlayRadius,
            radiusMinPixels: 6,
            pickable: false,
            parameters: { depthTest: false }
        }));
    }

    if (activeOverlay === 'disaster') {
        const regionRiskMap = new Map<number, number>();
        for (const r of regions) {
            regionRiskMap.set(r.id, r.disaster_risk);
        }
        const riskCells: { position: [number, number]; color: [number, number, number, number] }[] = [];
        for (const city of cities) {
            const risk = regionRiskMap.get(city.region_id) ?? 0;
            const intensity = Math.min(1.0, risk * 5);
            const r = Math.floor(intensity * 255);
            const g = Math.floor((1 - intensity) * 200);
            for (const cp of city.cell_positions) {
                riskCells.push({
                    position: [cp.lon, cp.lat],
                    color: [r, g, 50, 150]
                });
            }
        }
        layers.push(new ScatterplotLayer({
            id: 'overlay-disaster',
            data: riskCells,
            getPosition: (d: any) => d.position,
            getFillColor: (d: any) => d.color,
            getRadius: overlayRadius,
            radiusMinPixels: 6,
            pickable: false,
            parameters: { depthTest: false }
        }));
    }

    if (activeOverlay === 'coverage') {
        if (bridge.isInitialized()) {
            const coverageData = bridge.getCellCoverage();
            layers.push(new ScatterplotLayer({
                id: 'overlay-coverage',
                data: coverageData,
                getPosition: (d: CellCoverage) => [d.lon, d.lat],
                getFillColor: (d: CellCoverage) => {
                    const intensity = Math.min(1.0, d.signal_strength / 100);
                    return [Math.floor((1 - intensity) * 255), Math.floor(intensity * 200), 50, 150] as [number, number, number, number];
                },
                getRadius: overlayRadius,
                radiusMinPixels: 6,
                pickable: false,
                parameters: { depthTest: false }
            }));
        }
    }

    if (activeOverlay === 'ownership') {
        if (bridge.isInitialized()) {
            const coverageData = bridge.getCellCoverage();
            layers.push(new ScatterplotLayer({
                id: 'overlay-ownership',
                data: coverageData.filter(d => d.dominant_owner !== null),
                getPosition: (d: CellCoverage) => [d.lon, d.lat],
                getFillColor: (d: CellCoverage) => {
                    const corps = bridge.getAllCorporations();
                    const idx = corps.findIndex(c => c.id === d.dominant_owner);
                    const baseColor = CORP_COLORS[idx % CORP_COLORS.length];
                    return [...baseColor, 180] as [number, number, number, number];
                },
                getRadius: overlayRadius,
                radiusMinPixels: 6,
                pickable: false
            }));
        }
    }

    return layers;
}
