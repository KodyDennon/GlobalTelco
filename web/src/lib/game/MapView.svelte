<script lang="ts">
	import { onMount, onDestroy } from "svelte";
	import { MapRenderer } from "./MapRenderer";
	import { initialized } from "$lib/stores/gameState";
	import { mapQuality } from "$lib/stores/settings";
	import { get } from "svelte/store";
	import {
		selectedEntityId,
		selectedEntityType,
		buildMode,
		buildMenuLocation,
		buildEdgeSource,
		activeOverlay,
		tooltipData,
		selectedEdgeType,
	} from "$lib/stores/uiState";
	import * as bridge from "$lib/wasm/bridge";

	let container: HTMLElement;
	let renderer: MapRenderer | null = null;
	let frameId: number | null = null;
	let cleanup: (() => void) | null = null;

	function handleEntitySelected(e: CustomEvent) {
		const { id, type } = e.detail;
		const currentBuildMode = get(buildMode);

		if (currentBuildMode === "edge" && type === "node") {
			const source = get(buildEdgeSource);

			if (source === null) {
				buildEdgeSource.set(id);
			} else {
				const edgeType = get(selectedEdgeType);
				bridge.processCommand({
					BuildEdge: { edge_type: edgeType, from: source, to: id },
				});
				buildEdgeSource.set(null);
			}
			return;
		}

		selectedEntityId.set(id);
		selectedEntityType.set(type);
	}

	function handleMapClicked(e: CustomEvent) {
		const currentBuildMode = get(buildMode);

		if (currentBuildMode === "node") {
			const { lon, lat } = e.detail;
			buildMenuLocation.set({ lon, lat });
		}
	}

	onMount(() => {
		const unsub = initialized.subscribe(async (init) => {
			if (init && container && !renderer) {
				renderer = new MapRenderer(container, get(mapQuality));
				await renderer.buildMap();
				renderer.updateInfrastructure();

				// Update map every 500ms (roughly per-tick at 1x speed, responsive at higher speeds)
				const interval = setInterval(() => {
					renderer?.updateInfrastructure();
					renderer?.updateCities();
				}, 500);

				window.addEventListener(
					"entity-selected",
					handleEntitySelected as EventListener,
				);
				window.addEventListener(
					"map-clicked",
					handleMapClicked as EventListener,
				);

				// Subscribe to overlay changes
				const overlaySub = activeOverlay.subscribe((overlay) => {
					renderer?.setOverlay(overlay);
				});

				// Subscribe to edge source for highlight
				const edgeSrcSub = buildEdgeSource.subscribe((sourceId) => {
					renderer?.highlightEdgeSource(sourceId);
				});

				// Subscribe to selection changes to render selection ring on map
				const selectionSub = selectedEntityId.subscribe((id) => {
					renderer?.setSelected(id);
				});

				// Mouse move for tooltips
				container.addEventListener("mousemove", handleMouseMove);
				container.addEventListener("mouseleave", handleMouseLeave);

				cleanup = () => {
					clearInterval(interval);
					overlaySub();
					edgeSrcSub();
					selectionSub();
					window.removeEventListener(
						"entity-selected",
						handleEntitySelected as EventListener,
					);
					window.removeEventListener(
						"map-clicked",
						handleMapClicked as EventListener,
					);
					container?.removeEventListener("mousemove", handleMouseMove);
					container?.removeEventListener(
						"mouseleave",
						handleMouseLeave,
					);
				};
			}
		});

		return () => unsub();
	});

	function handleMouseMove(e: MouseEvent) {
		if (renderer) {
			renderer.handleMouseMove(e);
		}
	}

	function handleMouseLeave() {
		tooltipData.set(null);
	}

	onDestroy(() => {
		cleanup?.();
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
