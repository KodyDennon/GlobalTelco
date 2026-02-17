<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { MapRenderer } from './MapRenderer';
	import { initialized } from '$lib/stores/gameState';
	import { selectedEntityId, selectedEntityType } from '$lib/stores/uiState';

	let container: HTMLElement;
	let renderer: MapRenderer | null = null;
	let frameId: number | null = null;

	function animate() {
		renderer?.render();
		frameId = requestAnimationFrame(animate);
	}

	onMount(() => {
		const unsub = initialized.subscribe((init) => {
			if (init && container && !renderer) {
				renderer = new MapRenderer(container);
				renderer.buildMap();
				renderer.updateInfrastructure();
				animate();

				// Refresh infrastructure every 2 seconds
				const interval = setInterval(() => {
					renderer?.updateInfrastructure();
				}, 2000);

				window.addEventListener('entity-selected', ((e: CustomEvent) => {
					selectedEntityId.set(e.detail.id);
					selectedEntityType.set(e.detail.type);
				}) as EventListener);

				return () => clearInterval(interval);
			}
		});

		return () => unsub();
	});

	onDestroy(() => {
		if (frameId !== null) cancelAnimationFrame(frameId);
		renderer?.dispose();
	});
</script>

<div class="map-container" bind:this={container}></div>

<style>
	.map-container {
		width: 100%;
		height: 100%;
		position: absolute;
		inset: 0;
	}

	.map-container :global(canvas) {
		display: block;
	}
</style>
