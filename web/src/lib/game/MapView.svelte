<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { MapRenderer } from './MapRenderer';
	import { initialized } from '$lib/stores/gameState';
	import {
		selectedEntityId,
		selectedEntityType,
		buildMode,
		buildMenuParcel,
		buildEdgeSource,
		activeOverlay,
		tooltipData
	} from '$lib/stores/uiState';
	import * as bridge from '$lib/wasm/bridge';

	let container: HTMLElement;
	let renderer: MapRenderer | null = null;
	let frameId: number | null = null;

	function animate() {
		renderer?.render();
		frameId = requestAnimationFrame(animate);
	}

	function handleEntitySelected(e: CustomEvent) {
		const { id, type } = e.detail;
		let currentBuildMode: string | null = null;
		buildMode.subscribe((m) => (currentBuildMode = m))();

		if (currentBuildMode === 'edge' && type === 'node') {
			let source: number | null = null;
			buildEdgeSource.subscribe((s) => (source = s))();

			if (source === null) {
				buildEdgeSource.set(id);
			} else {
				bridge.processCommand({
					BuildEdge: { edge_type: 'Fiber', from: source, to: id }
				});
				buildEdgeSource.set(null);
			}
			return;
		}

		selectedEntityId.set(id);
		selectedEntityType.set(type);
	}

	function handleParcelClicked(e: CustomEvent) {
		let currentBuildMode: string | null = null;
		buildMode.subscribe((m) => (currentBuildMode = m))();

		if (currentBuildMode === 'node') {
			buildMenuParcel.set(e.detail);
		}
	}

	onMount(() => {
		const unsub = initialized.subscribe((init) => {
			if (init && container && !renderer) {
				renderer = new MapRenderer(container);
				renderer.buildMap();
				renderer.updateInfrastructure();
				animate();

				const interval = setInterval(() => {
					renderer?.updateInfrastructure();
				}, 2000);

				window.addEventListener('entity-selected', handleEntitySelected as EventListener);
				window.addEventListener('parcel-clicked', handleParcelClicked as EventListener);

				// Subscribe to overlay changes
				const overlaySub = activeOverlay.subscribe((overlay) => {
					renderer?.setOverlay(overlay);
				});

				// Mouse move for tooltips
				container.addEventListener('mousemove', handleMouseMove);
				container.addEventListener('mouseleave', handleMouseLeave);

				return () => {
					clearInterval(interval);
					overlaySub();
					window.removeEventListener('entity-selected', handleEntitySelected as EventListener);
					window.removeEventListener('parcel-clicked', handleParcelClicked as EventListener);
					container.removeEventListener('mousemove', handleMouseMove);
					container.removeEventListener('mouseleave', handleMouseLeave);
				};
			}
		});

		return () => unsub();
	});

	function handleMouseMove(e: MouseEvent) {
		// Simple tooltip on hover — could be extended with raycasting for entity info
		tooltipData.set(null); // Clear by default; entity hover sets it via raycaster
	}

	function handleMouseLeave() {
		tooltipData.set(null);
	}

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
