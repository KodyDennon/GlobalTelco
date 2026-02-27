<script lang="ts">
	import { SATELLITE_COLORS } from '$lib/game/map/constants';

	let {
		cells,
		width = 400,
		height = 250,
	}: {
		cells: Array<{ lat: number; lon: number; terrain: string }>;
		width?: number;
		height?: number;
	} = $props();

	let canvasEl: HTMLCanvasElement | undefined = $state();

	function terrainToColor(terrain: string): string {
		const rgb = SATELLITE_COLORS[terrain];
		if (rgb) {
			// Brighten the satellite palette slightly for the small preview
			const r = Math.min(255, rgb[0] + 30);
			const g = Math.min(255, rgb[1] + 30);
			const b = Math.min(255, rgb[2] + 30);
			return `rgb(${r},${g},${b})`;
		}
		// Fallback for unknown terrain types
		if (terrain.startsWith('Ocean')) return 'rgb(36,42,62)';
		return 'rgb(72,72,72)';
	}

	function draw() {
		if (!canvasEl || cells.length === 0) return;
		const ctx = canvasEl.getContext('2d');
		if (!ctx) return;

		// Clear with dark ocean background
		ctx.fillStyle = '#0a1020';
		ctx.fillRect(0, 0, width, height);

		// Equirectangular projection
		const padding = 4;
		const drawW = width - padding * 2;
		const drawH = height - padding * 2;

		for (const cell of cells) {
			const x = padding + ((cell.lon + 180) / 360) * drawW;
			const y = padding + ((90 - cell.lat) / 180) * drawH;
			const isOcean =
				cell.terrain === 'OceanShallow' ||
				cell.terrain === 'OceanDeep' ||
				cell.terrain === 'OceanTrench' ||
				cell.terrain === 'Ocean';
			const radius = isOcean ? 1.5 : 2.5;

			ctx.fillStyle = terrainToColor(cell.terrain);
			ctx.beginPath();
			ctx.arc(x, y, radius, 0, Math.PI * 2);
			ctx.fill();
		}
	}

	$effect(() => {
		// Re-draw whenever cells or canvas dimensions change
		if (canvasEl && cells) {
			draw();
		}
	});
</script>

<canvas
	bind:this={canvasEl}
	{width}
	{height}
	class="world-preview-canvas"
></canvas>

<style>
	.world-preview-canvas {
		border-radius: 6px;
		border: 1px solid rgba(55, 65, 81, 0.4);
		background: #0a1020;
		display: block;
		width: 100%;
		height: auto;
	}
</style>
