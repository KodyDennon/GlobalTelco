/**
 * AnnotationLayer — strategic markers and route planning on the map.
 *
 * Provides Svelte stores for annotations and route plans, plus functions
 * to generate deck.gl layers for rendering them.
 */

import { writable } from 'svelte/store';
import { ScatterplotLayer, TextLayer, PathLayer } from '@deck.gl/layers';

// ── Types ────────────────────────────────────────────────────────────────

export interface Annotation {
	id: string;
	type: 'flag' | 'warning' | 'star' | 'text';
	lon: number;
	lat: number;
	label: string;
	color?: [number, number, number];
}

export interface RoutePlan {
	id: string;
	waypoints: Array<{ lon: number; lat: number }>;
	estimatedCost?: number;
}

// ── Color palette per annotation type ────────────────────────────────────

const ANNOTATION_COLORS: Record<Annotation['type'], [number, number, number]> = {
	flag: [34, 197, 94],     // Green
	warning: [239, 68, 68],  // Red
	star: [234, 179, 8],     // Gold
	text: [59, 130, 246],    // Blue
};

const ROUTE_COLOR: [number, number, number, number] = [234, 179, 8, 200]; // Yellow/gold

// ── Stores ───────────────────────────────────────────────────────────────

export const annotations = writable<Annotation[]>([]);
export const routePlans = writable<RoutePlan[]>([]);

// ── Layer factory ────────────────────────────────────────────────────────

/** Create deck.gl layers for annotations and route plans. */
export function createAnnotationLayers(
	currentAnnotations: Annotation[],
	currentRoutes: RoutePlan[]
): any[] {
	const layers: any[] = [];

	// ── Annotation markers (dots) ────────────────────────────────────
	if (currentAnnotations.length > 0) {
		layers.push(
			new ScatterplotLayer({
				id: 'annotation-markers',
				data: currentAnnotations,
				getPosition: (d: Annotation) => [d.lon, d.lat],
				getFillColor: (d: Annotation) => {
					const c = d.color ?? ANNOTATION_COLORS[d.type];
					return [...c, 220] as [number, number, number, number];
				},
				getLineColor: [255, 255, 255, 180],
				getLineWidth: 2,
				lineWidthUnits: 'pixels' as const,
				stroked: true,
				filled: true,
				getRadius: 15000,
				radiusMinPixels: 6,
				radiusMaxPixels: 18,
				pickable: true,
				autoHighlight: true,
				parameters: { depthTest: false },
				onClick: ({ object }: any) => {
					if (object) {
						window.dispatchEvent(
							new CustomEvent('annotation-clicked', { detail: { id: object.id } })
						);
					}
				},
			})
		);

		// ── Annotation labels ────────────────────────────────────────
		const labeled = currentAnnotations.filter((a) => a.label.length > 0);
		if (labeled.length > 0) {
			layers.push(
				new TextLayer({
					id: 'annotation-labels',
					data: labeled,
					getPosition: (d: Annotation) => [d.lon, d.lat],
					getText: (d: Annotation) => d.label,
					getSize: 13,
					getColor: (d: Annotation) => {
						const c = d.color ?? ANNOTATION_COLORS[d.type];
						return [...c, 240] as [number, number, number, number];
					},
					getPixelOffset: [0, -18],
					fontFamily: 'Inter, sans-serif',
					fontWeight: 'bold',
					outlineWidth: 2,
					outlineColor: [0, 0, 0, 200],
					parameters: { depthTest: false },
				})
			);
		}

		// ── Type icon glyph (Unicode symbols as text) ────────────────
		const GLYPH_MAP: Record<Annotation['type'], string> = {
			flag: '\u2691',    // Black flag
			warning: '\u26A0', // Warning sign
			star: '\u2605',    // Black star
			text: '\u270E',    // Pencil
		};

		layers.push(
			new TextLayer({
				id: 'annotation-glyphs',
				data: currentAnnotations,
				getPosition: (d: Annotation) => [d.lon, d.lat],
				getText: (d: Annotation) => GLYPH_MAP[d.type] ?? '',
				getSize: 16,
				getColor: [255, 255, 255, 255],
				getPixelOffset: [0, 0],
				getTextAnchor: 'middle',
				getAlignmentBaseline: 'center',
				parameters: { depthTest: false },
			})
		);
	}

	// ── Route plans (dashed path lines) ──────────────────────────────
	const validRoutes = currentRoutes.filter((r) => r.waypoints.length >= 2);
	if (validRoutes.length > 0) {
		layers.push(
			new PathLayer({
				id: 'route-plan-paths',
				data: validRoutes,
				getPath: (d: RoutePlan) =>
					d.waypoints.map((wp) => [wp.lon, wp.lat]) as any,
				getColor: ROUTE_COLOR,
				getWidth: 3,
				widthUnits: 'pixels' as const,
				getDashArray: [6, 3],
				dashJustified: true,
				pickable: true,
				parameters: { depthTest: false },
				onClick: ({ object }: any) => {
					if (object) {
						window.dispatchEvent(
							new CustomEvent('route-clicked', { detail: { id: object.id } })
						);
					}
				},
			})
		);

		// Waypoint dots on routes
		const waypointData: Array<{ lon: number; lat: number; routeId: string }> = [];
		for (const route of validRoutes) {
			for (const wp of route.waypoints) {
				waypointData.push({ lon: wp.lon, lat: wp.lat, routeId: route.id });
			}
		}

		layers.push(
			new ScatterplotLayer({
				id: 'route-waypoints',
				data: waypointData,
				getPosition: (d: any) => [d.lon, d.lat],
				getFillColor: [234, 179, 8, 180],
				getLineColor: [255, 255, 255, 200],
				getLineWidth: 1,
				lineWidthUnits: 'pixels' as const,
				stroked: true,
				filled: true,
				getRadius: 8000,
				radiusMinPixels: 4,
				radiusMaxPixels: 10,
				pickable: false,
				parameters: { depthTest: false },
			})
		);

		// Cost estimate labels at midpoint of each route
		const costLabels = validRoutes
			.filter((r) => r.estimatedCost !== undefined && r.estimatedCost > 0)
			.map((r) => {
				const midIdx = Math.floor(r.waypoints.length / 2);
				const mid = r.waypoints[midIdx];
				return {
					lon: mid.lon,
					lat: mid.lat,
					text: formatCost(r.estimatedCost!),
				};
			});

		if (costLabels.length > 0) {
			layers.push(
				new TextLayer({
					id: 'route-cost-labels',
					data: costLabels,
					getPosition: (d: any) => [d.lon, d.lat],
					getText: (d: any) => d.text,
					getSize: 12,
					getColor: [234, 179, 8, 255],
					getPixelOffset: [0, -14],
					fontFamily: '"JetBrains Mono", "Fira Code", monospace',
					outlineWidth: 2,
					outlineColor: [0, 0, 0, 200],
					parameters: { depthTest: false },
				})
			);
		}
	}

	return layers;
}

// ── Annotation helpers ───────────────────────────────────────────────────

let nextAnnotationId = 1;

/** Add a new annotation at the given position. */
export function addAnnotation(
	type: Annotation['type'],
	lon: number,
	lat: number,
	label?: string
): void {
	const id = `ann-${nextAnnotationId++}`;
	const annotation: Annotation = {
		id,
		type,
		lon,
		lat,
		label: label ?? '',
		color: ANNOTATION_COLORS[type],
	};
	annotations.update((list) => [...list, annotation]);
}

/** Remove an annotation by ID. */
export function removeAnnotation(id: string): void {
	annotations.update((list) => list.filter((a) => a.id !== id));
}

/** Clear all annotations. */
export function clearAnnotations(): void {
	annotations.set([]);
}

// ── Route planning helpers ───────────────────────────────────────────────

let nextRouteId = 1;

/** Start a new route plan, returning its ID. */
export function startRoute(): string {
	const id = `route-${nextRouteId++}`;
	const route: RoutePlan = { id, waypoints: [] };
	routePlans.update((list) => [...list, route]);
	return id;
}

/** Add a waypoint to an existing route. Creates the route if it does not exist. */
export function addRouteWaypoint(routeId: string, lon: number, lat: number): void {
	routePlans.update((list) => {
		const route = list.find((r) => r.id === routeId);
		if (route) {
			route.waypoints.push({ lon, lat });
			return [...list]; // trigger reactivity
		}
		// Route not found — create it with this waypoint
		return [...list, { id: routeId, waypoints: [{ lon, lat }] }];
	});
}

/** Finalize a route (mark as complete — could add cost estimation here). */
export function finalizeRoute(routeId: string): void {
	routePlans.update((list) => {
		const route = list.find((r) => r.id === routeId);
		if (route && route.waypoints.length >= 2) {
			// Estimate cost: rough distance-based calculation
			let totalKm = 0;
			for (let i = 1; i < route.waypoints.length; i++) {
				totalKm += haversineKm(
					route.waypoints[i - 1].lat,
					route.waypoints[i - 1].lon,
					route.waypoints[i].lat,
					route.waypoints[i].lon
				);
			}
			// Rough estimate: $10,000 per km (average infrastructure cost)
			route.estimatedCost = Math.round(totalKm * 10000);
		}
		return [...list];
	});
}

/** Remove all waypoints from a route. */
export function clearRoute(routeId: string): void {
	routePlans.update((list) => list.filter((r) => r.id !== routeId));
}

/** Remove all route plans. */
export function clearAllRoutes(): void {
	routePlans.set([]);
}

// ── Utility ──────────────────────────────────────────────────────────────

/** Haversine distance in km between two lat/lon points. */
function haversineKm(
	lat1: number,
	lon1: number,
	lat2: number,
	lon2: number
): number {
	const R = 6371; // Earth radius in km
	const dLat = ((lat2 - lat1) * Math.PI) / 180;
	const dLon = ((lon2 - lon1) * Math.PI) / 180;
	const a =
		Math.sin(dLat / 2) * Math.sin(dLat / 2) +
		Math.cos((lat1 * Math.PI) / 180) *
			Math.cos((lat2 * Math.PI) / 180) *
			Math.sin(dLon / 2) *
			Math.sin(dLon / 2);
	const c = 2 * Math.atan2(Math.sqrt(a), Math.sqrt(1 - a));
	return R * c;
}

/** Format a cost number into a human-readable string. */
function formatCost(amount: number): string {
	if (amount >= 1_000_000_000) return `$${(amount / 1_000_000_000).toFixed(1)}B`;
	if (amount >= 1_000_000) return `$${(amount / 1_000_000).toFixed(1)}M`;
	if (amount >= 1_000) return `$${(amount / 1_000).toFixed(1)}K`;
	return `$${amount}`;
}
