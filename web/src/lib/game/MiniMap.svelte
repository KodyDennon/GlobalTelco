<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { viewport, zoomLevel } from '$lib/stores/uiState';
	import { regions, cities } from '$lib/stores/gameState';
	import { tooltip } from '$lib/ui/tooltip';
	import * as bridge from '$lib/wasm/bridge';
	import { buildTerrainBitmapAsync } from './map/terrainBitmap';
	import { SATELLITE_COLORS } from './map/constants';

	const CANVAS_W = 200;
	const CANVAS_H = 150;

	const LON_MIN = -180;
	const LON_MAX = 180;
	const LAT_MIN = -80;
	const LAT_MAX = 80;

	const STORAGE_KEY = 'gt-minimap-visible';

	// Muted pastel region fills — semi-transparent for layered look
	const REGION_FILLS = [
		'rgba(124, 158, 178, 0.35)',
		'rgba(143, 170, 124, 0.35)',
		'rgba(184, 156, 126, 0.35)',
		'rgba(154, 140, 170, 0.35)',
		'rgba(126, 184, 164, 0.35)',
		'rgba(184, 136, 124, 0.35)',
		'rgba(138, 164, 199, 0.35)',
		'rgba(164, 184, 124, 0.35)',
		'rgba(184, 124, 164, 0.35)',
		'rgba(124, 196, 184, 0.35)',
		'rgba(196, 168, 124, 0.35)',
		'rgba(124, 138, 184, 0.35)',
		'rgba(184, 164, 124, 0.35)',
	];

	// Border stroke colors (slightly brighter version of fills)
	const REGION_BORDERS = [
		'rgba(144, 178, 198, 0.5)',
		'rgba(163, 190, 144, 0.5)',
		'rgba(204, 176, 146, 0.5)',
		'rgba(174, 160, 190, 0.5)',
		'rgba(146, 204, 184, 0.5)',
		'rgba(204, 156, 144, 0.5)',
		'rgba(158, 184, 219, 0.5)',
		'rgba(184, 204, 144, 0.5)',
		'rgba(204, 144, 184, 0.5)',
		'rgba(144, 216, 204, 0.5)',
		'rgba(216, 188, 144, 0.5)',
		'rgba(144, 158, 204, 0.5)',
		'rgba(204, 184, 144, 0.5)',
	];

	let canvas: HTMLCanvasElement | undefined = $state();
	let ctx: CanvasRenderingContext2D | null = $state(null);

	// Visibility toggle (persisted in localStorage)
	let visible = $state(true);

	// Pre-rendered terrain background (scaled down from full terrain bitmap)
	let terrainImage: HTMLCanvasElement | null = $state(null);
	let terrainReady = $state(false);

	// Precomputed Mercator constants (avoid recalculating every frame)
	const radMin = (LAT_MIN * Math.PI) / 180;
	const radMax = (LAT_MAX * Math.PI) / 180;
	const mercMin = Math.log(Math.tan(Math.PI / 4 + radMin / 2));
	const mercMax = Math.log(Math.tan(Math.PI / 4 + radMax / 2));
	const mercRange = mercMax - mercMin;

	/** Convert latitude to Mercator Y canvas coordinate. */
	function mercatorY(lat: number): number {
		const clamped = Math.max(LAT_MIN, Math.min(LAT_MAX, lat));
		const radLat = (clamped * Math.PI) / 180;
		const mercY = Math.log(Math.tan(Math.PI / 4 + radLat / 2));
		return CANVAS_H - ((mercY - mercMin) / mercRange) * CANVAS_H;
	}

	/** Convert longitude to canvas X coordinate. */
	function lonToX(lon: number): number {
		return ((lon - LON_MIN) / (LON_MAX - LON_MIN)) * CANVAS_W;
	}

	/** Build terrain bitmap at minimap resolution. */
	async function buildTerrainBackground() {
		if (!bridge.isInitialized()) return;

		try {
			const cells = bridge.getGridCells();
			if (cells.length === 0) return;

			const worldInfo = bridge.getWorldInfo();
			const cellSpacingKm = worldInfo.cell_spacing_km > 0 ? worldInfo.cell_spacing_km : 120;

			// Build terrain bitmap at low quality (minimap is only 200x150)
			const fullBitmap = await buildTerrainBitmapAsync(cells, cellSpacingKm, SATELLITE_COLORS, 'low');

			// Scale down to minimap size using an offscreen canvas
			// The terrain bitmap uses equirectangular projection (lat -85 to 85)
			// We need to re-project to Mercator for our minimap
			const miniCanvas = document.createElement('canvas');
			miniCanvas.width = CANVAS_W;
			miniCanvas.height = CANVAS_H;
			const miniCtx = miniCanvas.getContext('2d')!;

			// Sample the equirectangular bitmap and re-project to Mercator
			const srcW = fullBitmap.width;
			const srcH = fullBitmap.height;
			const srcCtx = fullBitmap.getContext('2d')!;
			const srcData = srcCtx.getImageData(0, 0, srcW, srcH);

			const dstData = miniCtx.createImageData(CANVAS_W, CANVAS_H);

			for (let dy = 0; dy < CANVAS_H; dy++) {
				// Reverse Mercator: canvas Y -> latitude
				const normalizedY = (CANVAS_H - dy) / CANVAS_H;
				const mercY = mercMin + normalizedY * mercRange;
				const lat = (2 * Math.atan(Math.exp(mercY)) - Math.PI / 2) * (180 / Math.PI);

				// Source Y in equirectangular bitmap (lat -85 to 85, top=85, bottom=-85)
				const srcY = Math.round(((85 - lat) / 170) * srcH);
				if (srcY < 0 || srcY >= srcH) continue;

				for (let dx = 0; dx < CANVAS_W; dx++) {
					// Source X: direct linear mapping (lon -180 to 180)
					const srcX = Math.round((dx / CANVAS_W) * srcW);
					if (srcX < 0 || srcX >= srcW) continue;

					const srcIdx = (srcY * srcW + srcX) * 4;
					const dstIdx = (dy * CANVAS_W + dx) * 4;
					dstData.data[dstIdx] = srcData.data[srcIdx];
					dstData.data[dstIdx + 1] = srcData.data[srcIdx + 1];
					dstData.data[dstIdx + 2] = srcData.data[srcIdx + 2];
					dstData.data[dstIdx + 3] = srcData.data[srcIdx + 3];
				}
			}

			miniCtx.putImageData(dstData, 0, 0);
			terrainImage = miniCanvas;
			terrainReady = true;
		} catch {
			// Terrain building failed; minimap will still show regions/cities
			terrainReady = false;
		}
	}

	function render() {
		if (!ctx) return;

		// Clear canvas
		ctx.clearRect(0, 0, CANVAS_W, CANVAS_H);

		// ── Layer 1: Dark ocean base ────────────────────────────────────────
		ctx.fillStyle = '#060c1f';
		ctx.fillRect(0, 0, CANVAS_W, CANVAS_H);

		// ── Layer 2: Terrain background (Mercator-projected) ────────────────
		if (terrainReady && terrainImage) {
			ctx.drawImage(terrainImage, 0, 0, CANVAS_W, CANVAS_H);
		}

		// ── Layer 3: Region boundary polygons (filled + stroked) ────────────
		const regionList = $regions;
		for (let i = 0; i < regionList.length; i++) {
			const region = regionList[i];
			const poly = region.boundary_polygon;
			if (!poly || poly.length < 3) continue;

			ctx.beginPath();
			const firstX = lonToX(poly[0][0]);
			const firstY = mercatorY(poly[0][1]);
			ctx.moveTo(firstX, firstY);

			for (let j = 1; j < poly.length; j++) {
				ctx.lineTo(lonToX(poly[j][0]), mercatorY(poly[j][1]));
			}
			ctx.closePath();

			// Fill with translucent pastel
			ctx.fillStyle = REGION_FILLS[i % REGION_FILLS.length];
			ctx.fill();

			// Stroke border
			ctx.strokeStyle = REGION_BORDERS[i % REGION_BORDERS.length];
			ctx.lineWidth = 0.5;
			ctx.stroke();
		}

		// ── Layer 4: City dots (sized by population) ────────────────────────
		const cityList = $cities;
		for (const city of cityList) {
			const cx = lonToX(city.x);
			const cy = mercatorY(city.y);

			// Clamp to minimap bounds
			if (cx < 0 || cx > CANVAS_W || cy < 0 || cy > CANVAS_H) continue;

			// Radius and color by population tier
			let radius: number;
			let color: string;
			if (city.population > 5_000_000) {
				radius = 2.5;
				color = 'rgba(255, 160, 60, 1.0)';  // Bright orange (Megalopolis)
			} else if (city.population > 1_000_000) {
				radius = 2.0;
				color = 'rgba(255, 180, 80, 0.9)';  // Orange (Metropolis)
			} else if (city.population > 250_000) {
				radius = 1.5;
				color = 'rgba(255, 220, 100, 0.8)';  // Yellow (City)
			} else if (city.population > 50_000) {
				radius = 1.0;
				color = 'rgba(255, 220, 120, 0.6)';  // Dim yellow (Town)
			} else {
				radius = 0.7;
				color = 'rgba(255, 220, 120, 0.4)';  // Dim yellow (Hamlet)
			}

			ctx.fillStyle = color;
			ctx.beginPath();
			ctx.arc(cx, cy, radius, 0, Math.PI * 2);
			ctx.fill();
		}

		// ── Layer 5: Competitor infrastructure (grey dots) ───────────────────
		if (bridge.isInitialized()) {
			try {
				const allInfra = bridge.getAllInfrastructure();
				const worldInfo = bridge.getWorldInfo();
				const playerId = worldInfo.player_corp_id;

				// Draw competitor nodes as small grey dots
				ctx.fillStyle = 'rgba(120, 130, 150, 0.5)';
				for (const node of allInfra.nodes) {
					if (node.owner === playerId) continue;
					const nx = lonToX(node.x);
					const ny = mercatorY(node.y);
					if (nx < 0 || nx > CANVAS_W || ny < 0 || ny > CANVAS_H) continue;
					ctx.beginPath();
					ctx.arc(nx, ny, 1, 0, Math.PI * 2);
					ctx.fill();
				}
			} catch {
				// All infrastructure query failed; skip competitor layer
			}
		}

		// ── Layer 6: Player infrastructure nodes (emerald dots) ──────────────
		if (bridge.isInitialized()) {
			try {
				const worldInfo = bridge.getWorldInfo();
				const infra = bridge.getInfrastructureList(worldInfo.player_corp_id);
				if (infra.nodes.length > 0) {
					ctx.fillStyle = 'rgba(16, 185, 129, 0.9)'; // Emerald (player corp color)
					for (const node of infra.nodes) {
						const nx = lonToX(node.x);
						const ny = mercatorY(node.y);
						if (nx < 0 || nx > CANVAS_W || ny < 0 || ny > CANVAS_H) continue;
						ctx.beginPath();
						ctx.arc(nx, ny, 1.5, 0, Math.PI * 2);
						ctx.fill();
					}

					// Draw edges as thin lines
					ctx.strokeStyle = 'rgba(16, 185, 129, 0.5)';
					ctx.lineWidth = 0.5;
					for (const edge of infra.edges) {
						const sx = lonToX(edge.src_x);
						const sy = mercatorY(edge.src_y);
						const dx = lonToX(edge.dst_x);
						const dy = mercatorY(edge.dst_y);
						ctx.beginPath();
						ctx.moveTo(sx, sy);
						ctx.lineTo(dx, dy);
						ctx.stroke();
					}
				}
			} catch {
				// Infrastructure query failed; skip layer
			}
		}

		// ── Layer 7: Viewport rectangle ─────────────────────────────────────
		const vp = $viewport;
		const vpLeft = lonToX(vp.minX);
		const vpRight = lonToX(vp.maxX);
		const vpTop = mercatorY(vp.maxY);
		const vpBottom = mercatorY(vp.minY);

		let vpW = vpRight - vpLeft;
		let vpH = vpBottom - vpTop;

		// Enforce minimum visible size so the rect is always visible
		const minSize = 6;
		if (vpW < minSize) {
			const mid = (vpLeft + vpRight) / 2;
			vpW = minSize;
			drawViewportRect(mid - minSize / 2, vpTop, vpW, Math.max(vpH, minSize));
		} else {
			drawViewportRect(vpLeft, vpTop, vpW, Math.max(vpH, minSize));
		}
	}

	function drawViewportRect(x: number, y: number, w: number, h: number) {
		if (!ctx) return;

		// Semi-transparent fill
		ctx.fillStyle = 'rgba(255, 255, 255, 0.06)';
		ctx.fillRect(x, y, w, h);

		// White border
		ctx.strokeStyle = 'rgba(255, 255, 255, 0.8)';
		ctx.lineWidth = 1.5;
		ctx.strokeRect(x, y, w, h);

		// Corner accents for visibility
		const cornerLen = Math.min(4, w / 3, h / 3);
		ctx.strokeStyle = 'rgba(255, 255, 255, 1.0)';
		ctx.lineWidth = 2;

		// Top-left
		ctx.beginPath();
		ctx.moveTo(x, y + cornerLen);
		ctx.lineTo(x, y);
		ctx.lineTo(x + cornerLen, y);
		ctx.stroke();

		// Top-right
		ctx.beginPath();
		ctx.moveTo(x + w - cornerLen, y);
		ctx.lineTo(x + w, y);
		ctx.lineTo(x + w, y + cornerLen);
		ctx.stroke();

		// Bottom-left
		ctx.beginPath();
		ctx.moveTo(x, y + h - cornerLen);
		ctx.lineTo(x, y + h);
		ctx.lineTo(x + cornerLen, y + h);
		ctx.stroke();

		// Bottom-right
		ctx.beginPath();
		ctx.moveTo(x + w - cornerLen, y + h);
		ctx.lineTo(x + w, y + h);
		ctx.lineTo(x + w, y + h - cornerLen);
		ctx.stroke();
	}

	function handleClick(e: MouseEvent) {
		if (!canvas) return;
		const rect = canvas.getBoundingClientRect();
		const canvasX = ((e.clientX - rect.left) / rect.width) * CANVAS_W;
		const canvasY = ((e.clientY - rect.top) / rect.height) * CANVAS_H;

		// Convert canvas coords back to lon/lat (reverse Mercator)
		const lon = LON_MIN + (canvasX / CANVAS_W) * (LON_MAX - LON_MIN);

		const normalizedY = (CANVAS_H - canvasY) / CANVAS_H;
		const mercY = mercMin + normalizedY * mercRange;
		const lat = (2 * Math.atan(Math.exp(mercY)) - Math.PI / 2) * (180 / Math.PI);

		// Dispatch on window so MapView can handle navigation
		window.dispatchEvent(
			new CustomEvent('minimap-navigate', {
				detail: { lon, lat }
			})
		);
	}

	// Also handle drag for continuous navigation
	let isDragging = $state(false);

	function handleMouseDown(e: MouseEvent) {
		isDragging = true;
		handleClick(e);
	}

	function handleMouseMove(e: MouseEvent) {
		if (!isDragging) return;
		handleClick(e);
	}

	function handleMouseUp() {
		isDragging = false;
	}

	function toggleVisibility() {
		visible = !visible;
		try {
			localStorage.setItem(STORAGE_KEY, visible ? '1' : '0');
		} catch {
			// localStorage unavailable — ignore
		}
	}

	// Build terrain once data is available
	let terrainBuilt = false;

	onMount(() => {
		// Restore visibility from localStorage
		try {
			const stored = localStorage.getItem(STORAGE_KEY);
			if (stored === '0') visible = false;
		} catch {
			// localStorage unavailable — default to visible
		}

		if (canvas) {
			ctx = canvas.getContext('2d');
		}

		// Listen for mouseup on window to catch drag release outside the minimap
		window.addEventListener('mouseup', handleMouseUp);
	});

	onDestroy(() => {
		window.removeEventListener('mouseup', handleMouseUp);
	});

	// Reactively rebuild terrain when regions become available
	$effect(() => {
		const regionList = $regions;
		if (regionList.length > 0 && !terrainBuilt && bridge.isInitialized()) {
			terrainBuilt = true;
			// Defer to next tick so the DOM is ready
			requestAnimationFrame(() => buildTerrainBackground());
		}
	});

	// Re-render whenever reactive state changes (throttled to ~2s via effect debouncing)
	$effect(() => {
		void $viewport;
		void $zoomLevel;
		void $regions;
		void $cities;
		void terrainReady;
		render();
	});
</script>

{#if visible}
	<div class="minimap-container" role="navigation" aria-label="Mini map" use:tooltip={'Click to navigate the map. Drag to pan.'}>
		<canvas
			bind:this={canvas}
			width={CANVAS_W}
			height={CANVAS_H}
			class="minimap-canvas"
			onmousedown={handleMouseDown}
			onmousemove={handleMouseMove}
			onmouseup={handleMouseUp}
		></canvas>
		<div class="minimap-label">MAP</div>
		<button
			class="minimap-toggle-close"
			onclick={toggleVisibility}
			aria-label="Hide minimap"
			use:tooltip={'Hide minimap (M)'}
		>
			<svg width="8" height="8" viewBox="0 0 8 8" fill="none">
				<path d="M1 1L7 7M7 1L1 7" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
			</svg>
		</button>
	</div>
{:else}
	<button
		class="minimap-toggle-show"
		onclick={toggleVisibility}
		aria-label="Show minimap"
		use:tooltip={'Show minimap (M)'}
	>
		<svg width="14" height="14" viewBox="0 0 14 14" fill="none">
			<rect x="1" y="1" width="12" height="8" rx="1" stroke="currentColor" stroke-width="1.2"/>
			<rect x="3" y="3" width="4" height="3" rx="0.5" stroke="currentColor" stroke-width="0.8" stroke-dasharray="1.5 0.8"/>
			<circle cx="10" cy="5" r="1" fill="currentColor"/>
		</svg>
	</button>
{/if}

<style>
	.minimap-container {
		position: absolute;
		bottom: 70px;
		right: 16px;
		z-index: 12;
		width: 200px;
		height: 150px;
		background: #060c1f;
		border: 1px solid rgba(55, 65, 81, 0.6);
		border-radius: 6px;
		overflow: hidden;
		cursor: crosshair;
		opacity: 0.9;
		box-shadow: 0 4px 16px rgba(0, 0, 0, 0.5), inset 0 0 0 1px rgba(255, 255, 255, 0.04);
	}

	.minimap-canvas {
		display: block;
		width: 200px;
		height: 150px;
	}

	.minimap-label {
		position: absolute;
		top: 4px;
		left: 6px;
		font-size: 8px;
		font-weight: 600;
		letter-spacing: 0.1em;
		color: rgba(255, 255, 255, 0.3);
		pointer-events: none;
		font-family: 'Inter', sans-serif;
		text-transform: uppercase;
	}

	.minimap-toggle-close {
		position: absolute;
		top: 3px;
		right: 3px;
		width: 16px;
		height: 16px;
		border: none;
		background: rgba(0, 0, 0, 0.5);
		color: rgba(255, 255, 255, 0.4);
		border-radius: 3px;
		cursor: pointer;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 0;
		transition: all 0.15s;
		line-height: 1;
	}

	.minimap-toggle-close:hover {
		background: rgba(239, 68, 68, 0.4);
		color: rgba(255, 255, 255, 0.9);
	}

	.minimap-toggle-show {
		position: absolute;
		bottom: 70px;
		right: 16px;
		z-index: 12;
		width: 32px;
		height: 28px;
		border: 1px solid rgba(55, 65, 81, 0.5);
		background: rgba(17, 24, 39, 0.9);
		color: rgba(255, 255, 255, 0.5);
		border-radius: 6px;
		cursor: pointer;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 0;
		transition: all 0.15s;
		box-shadow: 0 2px 8px rgba(0, 0, 0, 0.3);
	}

	.minimap-toggle-show:hover {
		background: rgba(31, 41, 55, 0.95);
		color: rgba(255, 255, 255, 0.8);
		border-color: rgba(75, 85, 99, 0.6);
	}
</style>
