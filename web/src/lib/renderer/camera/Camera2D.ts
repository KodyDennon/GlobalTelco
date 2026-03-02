/**
 * Camera2D — 2D Mercator projection camera for WebGPU map rendering.
 * Handles pan, zoom, and coordinate transformations between:
 *   - World (lon, lat)
 *   - Mercator (x, y in [0, 1])
 *   - NDC (-1..1 for vertex shaders)
 *   - Screen pixels
 */

export interface ViewState {
	centerLon: number;
	centerLat: number;
	zoom: number;
	bearing: number; // degrees
	width: number;
	height: number;
}

export class Camera2D {
	private state: ViewState = {
		centerLon: 0,
		centerLat: 30,
		zoom: 2,
		bearing: 0,
		width: 800,
		height: 600,
	};

	/** 4x4 projection matrix as Float32Array (column-major for GPU upload). */
	private matrixBuf = new Float32Array(16);
	private dirty = true;

	setViewState(vs: Partial<ViewState>): void {
		let changed = false;
		if (vs.centerLon !== undefined && vs.centerLon !== this.state.centerLon) { this.state.centerLon = vs.centerLon; changed = true; }
		if (vs.centerLat !== undefined && vs.centerLat !== this.state.centerLat) { this.state.centerLat = vs.centerLat; changed = true; }
		if (vs.zoom !== undefined && vs.zoom !== this.state.zoom) { this.state.zoom = vs.zoom; changed = true; }
		if (vs.bearing !== undefined && vs.bearing !== this.state.bearing) { this.state.bearing = vs.bearing; changed = true; }
		if (vs.width !== undefined && vs.width !== this.state.width) { this.state.width = vs.width; changed = true; }
		if (vs.height !== undefined && vs.height !== this.state.height) { this.state.height = vs.height; changed = true; }
		if (changed) this.dirty = true;
	}

	getViewState(): Readonly<ViewState> {
		return this.state;
	}

	/** Get the 4x4 view-projection matrix (column-major). Recomputed only when dirty. */
	getMatrix(): Float32Array {
		if (this.dirty) {
			this.recompute();
			this.dirty = false;
		}
		return this.matrixBuf;
	}

	/** Convert (lon, lat) → Mercator (x, y) in [0, 1] range. */
	static lonLatToMercator(lon: number, lat: number): [number, number] {
		const x = (lon + 180) / 360;
		const latRad = (lat * Math.PI) / 180;
		const y = (1 - Math.log(Math.tan(latRad) + 1 / Math.cos(latRad)) / Math.PI) / 2;
		return [x, y];
	}

	/** Convert Mercator (x, y) → (lon, lat). */
	static mercatorToLonLat(mx: number, my: number): [number, number] {
		const lon = mx * 360 - 180;
		const n = Math.PI - 2 * Math.PI * my;
		const lat = (180 / Math.PI) * Math.atan(0.5 * (Math.exp(n) - Math.exp(-n)));
		return [lon, lat];
	}

	/** World scale at current zoom — how many Mercator units fit on screen. */
	private getWorldScale(): number {
		// At zoom 0, the entire world (1 Mercator unit) fits in 512px
		return Math.pow(2, this.state.zoom) * 512;
	}

	/** Get visible Mercator bounds [minX, minY, maxX, maxY]. */
	getBounds(): [number, number, number, number] {
		const scale = this.getWorldScale();
		const [cx, cy] = Camera2D.lonLatToMercator(this.state.centerLon, this.state.centerLat);
		const halfW = this.state.width / scale / 2;
		const halfH = this.state.height / scale / 2;
		return [cx - halfW, cy - halfH, cx + halfW, cy + halfH];
	}

	/** Recompute the 4x4 projection matrix for the GPU uniform buffer. */
	private recompute(): void {
		const { width, height } = this.state;
		const scale = this.getWorldScale();
		const [cx, cy] = Camera2D.lonLatToMercator(this.state.centerLon, this.state.centerLat);

		// Orthographic projection: Mercator coords → NDC
		// Scale by (2 * scale / width) and (2 * scale / height), center at (cx, cy)
		const sx = (2 * scale) / width;
		const sy = -(2 * scale) / height; // flip Y (Mercator Y goes down, NDC Y goes up)
		const tx = -cx * sx;
		const ty = -cy * sy;

		// Optional bearing rotation (about screen center)
		const bearRad = (this.state.bearing * Math.PI) / 180;
		const cos = Math.cos(bearRad);
		const sin = Math.sin(bearRad);

		// Column-major 4x4 matrix: rotation × scale+translate
		const m = this.matrixBuf;
		m[0] = sx * cos;  m[1] = sx * sin;   m[2] = 0;  m[3] = 0;
		m[4] = sy * -sin; m[5] = sy * cos;    m[6] = 0;  m[7] = 0;
		m[8] = 0;         m[9] = 0;           m[10] = 1; m[11] = 0;
		m[12] = tx * cos + ty * -sin;
		m[13] = tx * sin + ty * cos;
		m[14] = 0;
		m[15] = 1;
	}
}
