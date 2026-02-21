<script lang="ts">
	import { onMount } from 'svelte';
	import { viewport, zoomLevel } from '$lib/stores/uiState';
	import { regions } from '$lib/stores/gameState';

	const CANVAS_W = 200;
	const CANVAS_H = 150;

	const LON_MIN = -180;
	const LON_MAX = 180;
	const LAT_MIN = -80;
	const LAT_MAX = 80;

	const PASTELS = [
		'#7c9eb2', '#8faa7c', '#b89c7e', '#9a8caa', '#7eb8a4',
		'#b5887c', '#8aa4c7', '#a4b87c', '#b87ca4', '#7cc4b8',
		'#c4a87c', '#7c8ab8', '#b8a47c'
	];

	let canvas: HTMLCanvasElement | undefined = $state();
	let ctx: CanvasRenderingContext2D | null = $state(null);

	/** Convert latitude to Mercator Y, clamped to LAT_MIN..LAT_MAX range. */
	function mercatorY(lat: number): number {
		const clamped = Math.max(LAT_MIN, Math.min(LAT_MAX, lat));
		const radLat = (clamped * Math.PI) / 180;
		const mercY = Math.log(Math.tan(Math.PI / 4 + radLat / 2));

		const radMin = (LAT_MIN * Math.PI) / 180;
		const radMax = (LAT_MAX * Math.PI) / 180;
		const mercMin = Math.log(Math.tan(Math.PI / 4 + radMin / 2));
		const mercMax = Math.log(Math.tan(Math.PI / 4 + radMax / 2));

		// Invert Y so north is at the top of the canvas
		return CANVAS_H - ((mercY - mercMin) / (mercMax - mercMin)) * CANVAS_H;
	}

	/** Convert longitude to canvas X coordinate. */
	function lonToX(lon: number): number {
		return ((lon - LON_MIN) / (LON_MAX - LON_MIN)) * CANVAS_W;
	}

	function render() {
		if (!ctx) return;

		// Clear
		ctx.clearRect(0, 0, CANVAS_W, CANVAS_H);

		// Background fill
		ctx.fillStyle = 'rgba(17, 24, 39, 0.9)';
		ctx.fillRect(0, 0, CANVAS_W, CANVAS_H);

		// Draw region dots/rects
		const regionList = $regions;
		for (let i = 0; i < regionList.length; i++) {
			const region = regionList[i];
			const cx = lonToX(region.center_lon);
			const cy = mercatorY(region.center_lat);
			const size = Math.max(3, Math.sqrt(region.cell_count) * 1.5);
			const color = PASTELS[i % PASTELS.length];

			ctx.fillStyle = color;
			ctx.fillRect(cx - size / 2, cy - size / 2, size, size);
		}

		// Draw viewport rectangle
		const vp = $viewport;
		const vpLeft = lonToX(vp.minX);
		const vpRight = lonToX(vp.maxX);
		const vpTop = mercatorY(vp.maxY); // maxY (north) maps to smaller canvas Y
		const vpBottom = mercatorY(vp.minY); // minY (south) maps to larger canvas Y

		const vpW = vpRight - vpLeft;
		const vpH = vpBottom - vpTop;

		ctx.strokeStyle = '#ffffff';
		ctx.lineWidth = 1.5;
		ctx.strokeRect(vpLeft, vpTop, vpW, vpH);
	}

	function handleClick(e: MouseEvent) {
		if (!canvas) return;
		const rect = canvas.getBoundingClientRect();
		const canvasX = ((e.clientX - rect.left) / rect.width) * CANVAS_W;
		const canvasY = ((e.clientY - rect.top) / rect.height) * CANVAS_H;

		// Convert canvas coords back to lon/lat
		const lon = LON_MIN + (canvasX / CANVAS_W) * (LON_MAX - LON_MIN);

		// Reverse Mercator Y to latitude
		const radMin = (LAT_MIN * Math.PI) / 180;
		const radMax = (LAT_MAX * Math.PI) / 180;
		const mercMin = Math.log(Math.tan(Math.PI / 4 + radMin / 2));
		const mercMax = Math.log(Math.tan(Math.PI / 4 + radMax / 2));

		// canvasY is inverted: 0 = top (north), CANVAS_H = bottom (south)
		const normalizedY = (CANVAS_H - canvasY) / CANVAS_H;
		const mercY = mercMin + normalizedY * (mercMax - mercMin);
		const lat = (2 * Math.atan(Math.exp(mercY)) - Math.PI / 2) * (180 / Math.PI);

		canvas.dispatchEvent(
			new CustomEvent('minimap-navigate', {
				detail: { x: lon, y: lat },
				bubbles: true
			})
		);
	}

	onMount(() => {
		if (canvas) {
			ctx = canvas.getContext('2d');
		}
	});

	$effect(() => {
		// Track reactive dependencies so we re-render on changes
		void $viewport;
		void $zoomLevel;
		void $regions;
		render();
	});
</script>

<div class="minimap-container" role="navigation" aria-label="Mini map">
	<canvas
		bind:this={canvas}
		width={CANVAS_W}
		height={CANVAS_H}
		class="minimap-canvas"
		onclick={handleClick}
	></canvas>
</div>

<style>
	.minimap-container {
		position: absolute;
		bottom: 70px;
		right: 16px;
		z-index: 12;
		width: 200px;
		height: 150px;
		background: rgba(17, 24, 39, 0.9);
		border: 1px solid rgba(55, 65, 81, 0.5);
		border-radius: 6px;
		overflow: hidden;
		cursor: crosshair;
	}

	.minimap-canvas {
		display: block;
		width: 200px;
		height: 150px;
	}
</style>
