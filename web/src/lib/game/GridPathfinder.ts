import type { GridCell } from '$lib/wasm/types';

/**
 * Terrain categories for pathfinding cost calculation.
 * Land edges (Fiber, Copper) strongly prefer land cells and avoid ocean.
 * Submarine edges strongly prefer ocean cells and avoid land.
 * Wireless edges (Microwave, Satellite) don't use pathfinding — rendered as straight/curved lines.
 */

const LAND_TERRAINS = new Set([
	'Urban', 'Suburban', 'Rural', 'Mountainous', 'Desert', 'Coastal', 'Tundra', 'Frozen'
]);
const OCEAN_TERRAINS = new Set(['OceanShallow', 'OceanDeep', 'Ocean']);
const COASTAL_TERRAINS = new Set(['Coastal']);

/** Cost multipliers for land-based edges (Fiber, Copper) traversing different terrain */
function landTraversalCost(terrain: string): number {
	switch (terrain) {
		case 'Urban': return 1.0;        // Easy — existing infrastructure corridors
		case 'Suburban': return 1.1;
		case 'Rural': return 1.2;
		case 'Coastal': return 1.3;       // Possible but harder
		case 'Desert': return 1.8;        // Expensive but doable
		case 'Tundra': return 2.0;
		case 'Frozen': return 2.5;
		case 'Mountainous': return 3.0;   // Very hard terrain
		case 'OceanShallow': return 15.0; // Strongly discouraged — need submarine for this
		case 'OceanDeep': return 50.0;    // Essentially impassable for land cables
		case 'Ocean': return 50.0;
		default: return 2.0;
	}
}

/** Cost multipliers for submarine edges traversing different terrain */
function submarineTraversalCost(terrain: string): number {
	switch (terrain) {
		case 'OceanDeep': return 1.0;     // Ideal for submarine cables
		case 'OceanShallow': return 1.2;
		case 'Ocean': return 1.0;
		case 'Coastal': return 1.5;       // Transition zone — landing point
		case 'Urban': return 8.0;         // Can cross land briefly at landing
		case 'Suburban': return 10.0;
		case 'Rural': return 15.0;
		default: return 20.0;             // Strongly discouraged on land
	}
}

/** Whether an edge type should use terrain-aware routing */
export function needsTerrainRouting(edgeType: string): boolean {
	return edgeType === 'FiberLocal' || edgeType === 'FiberRegional' ||
		edgeType === 'FiberNational' || edgeType === 'Copper' || edgeType === 'Submarine';
}

/** Haversine distance between two lat/lon points in degrees (for heuristic) */
function haversineDeg(lat1: number, lon1: number, lat2: number, lon2: number): number {
	const dlat = lat1 - lat2;
	const dlon = lon1 - lon2;
	// Approximate degree distance (faster than full haversine for heuristic)
	const cosLat = Math.cos(((lat1 + lat2) / 2) * Math.PI / 180);
	return Math.sqrt(dlat * dlat + dlon * dlon * cosLat * cosLat);
}

interface PathNode {
	cell: number;
	g: number;  // Cost from start
	f: number;  // g + heuristic
	parent: number; // Parent cell index, -1 for start
}

/**
 * Grid-based A* pathfinder for routing infrastructure edges through terrain cells.
 * Computes terrain-aware shortest paths and caches results.
 */
export class GridPathfinder {
	private cells: GridCell[] = [];
	private pathCache = new Map<string, number[]>(); // "srcCell-dstCell-type" -> cell indices

	/** Initialize with grid cell data (call once after getGridCells()) */
	init(cells: GridCell[]) {
		this.cells = cells;
		this.pathCache.clear();
	}

	/** Clear the path cache (call when infrastructure changes) */
	clearCache() {
		this.pathCache.clear();
	}

	/**
	 * Find a terrain-aware path between two cells.
	 * Returns an array of (lon, lat) waypoints for rendering.
	 * Falls back to straight line if no path found or cells are adjacent.
	 */
	findPath(srcCell: number, dstCell: number, edgeType: string): Array<[number, number]> {
		if (srcCell === dstCell || this.cells.length === 0) {
			const c = this.cells[srcCell];
			return c ? [[c.lon, c.lat]] : [];
		}

		// Check cache
		const cacheKey = `${srcCell}-${dstCell}-${edgeType}`;
		const cached = this.pathCache.get(cacheKey);
		if (cached) {
			return cached.map(i => {
				const c = this.cells[i];
				return [c.lon, c.lat] as [number, number];
			});
		}

		// Select cost function based on edge type
		const costFn = edgeType === 'Submarine' ? submarineTraversalCost : landTraversalCost;

		const src = this.cells[srcCell];
		const dst = this.cells[dstCell];
		if (!src || !dst) return [[src?.lon ?? 0, src?.lat ?? 0], [dst?.lon ?? 0, dst?.lat ?? 0]];

		// A* pathfinding
		const cellCount = this.cells.length;
		const gScore = new Float64Array(cellCount).fill(Infinity);
		const fScore = new Float64Array(cellCount).fill(Infinity);
		const cameFrom = new Int32Array(cellCount).fill(-1);
		const closed = new Uint8Array(cellCount);

		// Min-heap using array (simple binary heap)
		const open: PathNode[] = [];
		gScore[srcCell] = 0;
		fScore[srcCell] = haversineDeg(src.lat, src.lon, dst.lat, dst.lon);
		open.push({ cell: srcCell, g: 0, f: fScore[srcCell], parent: -1 });

		let found = false;
		let iterations = 0;
		const maxIterations = Math.min(cellCount * 2, 20000); // Safety limit

		while (open.length > 0 && iterations < maxIterations) {
			iterations++;

			// Find min-f node (simple linear scan — fine for our grid sizes)
			let minIdx = 0;
			for (let i = 1; i < open.length; i++) {
				if (open[i].f < open[minIdx].f) minIdx = i;
			}
			const current = open[minIdx];
			open[minIdx] = open[open.length - 1];
			open.pop();

			if (current.cell === dstCell) {
				found = true;
				break;
			}

			if (closed[current.cell]) continue;
			closed[current.cell] = 1;

			const currentData = this.cells[current.cell];
			if (!currentData) continue;

			// Explore neighbors
			for (const neighborIdx of currentData.neighbors) {
				if (closed[neighborIdx]) continue;
				const neighbor = this.cells[neighborIdx];
				if (!neighbor) continue;

				// Base movement cost = geographic distance between cell centers
				const dist = haversineDeg(currentData.lat, currentData.lon, neighbor.lat, neighbor.lon);
				// Apply terrain cost multiplier
				const terrainCost = costFn(neighbor.terrain);
				const moveCost = dist * terrainCost;

				const tentativeG = gScore[current.cell] + moveCost;
				if (tentativeG < gScore[neighborIdx]) {
					gScore[neighborIdx] = tentativeG;
					cameFrom[neighborIdx] = current.cell;
					const h = haversineDeg(neighbor.lat, neighbor.lon, dst.lat, dst.lon);
					fScore[neighborIdx] = tentativeG + h;
					open.push({ cell: neighborIdx, g: tentativeG, f: fScore[neighborIdx], parent: current.cell });
				}
			}
		}

		// Reconstruct path
		let path: number[];
		if (found) {
			path = [];
			let current = dstCell;
			while (current !== -1) {
				path.push(current);
				current = cameFrom[current];
			}
			path.reverse();
		} else {
			// No path found — fall back to straight line
			path = [srcCell, dstCell];
		}

		// Simplify path: remove intermediate cells that are nearly collinear
		// This reduces the number of line segments while keeping the terrain-following shape
		const simplified = this.simplifyPath(path, 0.3);

		// Cache both directions
		this.pathCache.set(cacheKey, simplified);
		this.pathCache.set(`${dstCell}-${srcCell}-${edgeType}`, [...simplified].reverse());

		return simplified.map(i => {
			const c = this.cells[i];
			return [c.lon, c.lat] as [number, number];
		});
	}

	/**
	 * Douglas-Peucker simplification on the cell path.
	 * Removes intermediate points that are within `tolerance` degrees of the line
	 * between their neighbors, while keeping the overall shape.
	 */
	private simplifyPath(path: number[], tolerance: number): number[] {
		if (path.length <= 3) return path;

		const points = path.map(i => {
			const c = this.cells[i];
			return { idx: i, lon: c.lon, lat: c.lat };
		});

		const keep = new Uint8Array(points.length);
		keep[0] = 1;
		keep[points.length - 1] = 1;

		this.dpSimplify(points, 0, points.length - 1, tolerance, keep);

		const result: number[] = [];
		for (let i = 0; i < points.length; i++) {
			if (keep[i]) result.push(points[i].idx);
		}
		return result;
	}

	private dpSimplify(
		points: Array<{ idx: number; lon: number; lat: number }>,
		start: number, end: number, tolerance: number,
		keep: Uint8Array
	) {
		if (end - start < 2) return;

		let maxDist = 0;
		let maxIdx = start;

		const sx = points[start].lon, sy = points[start].lat;
		const ex = points[end].lon, ey = points[end].lat;
		const dx = ex - sx, dy = ey - sy;
		const lenSq = dx * dx + dy * dy;

		for (let i = start + 1; i < end; i++) {
			const px = points[i].lon - sx, py = points[i].lat - sy;
			let dist: number;
			if (lenSq === 0) {
				dist = Math.sqrt(px * px + py * py);
			} else {
				const t = Math.max(0, Math.min(1, (px * dx + py * dy) / lenSq));
				const projX = px - t * dx, projY = py - t * dy;
				dist = Math.sqrt(projX * projX + projY * projY);
			}
			if (dist > maxDist) {
				maxDist = dist;
				maxIdx = i;
			}
		}

		if (maxDist > tolerance) {
			keep[maxIdx] = 1;
			this.dpSimplify(points, start, maxIdx, tolerance, keep);
			this.dpSimplify(points, maxIdx, end, tolerance, keep);
		}
	}
}
