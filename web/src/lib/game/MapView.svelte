<script lang="ts">
	import 'maplibre-gl/dist/maplibre-gl.css';
	import { onMount, onDestroy } from "svelte";
	import { MapRenderer } from "./map";
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
		edgeTargets,
		canEdgeConnect,
		getCompatibleEdgeTypes,
		viewport,
		zoomLevel,
	} from "$lib/stores/uiState";
	import * as bridge from "$lib/wasm/bridge";
	import { gameCommand } from '$lib/game/commandRouter';
	import { injectEventEffectStyles } from './EventEffects';
	import { mapReady } from './GameLoop';

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
				// First click: set source, load valid targets
				buildEdgeSource.set(id);
				loadEdgeTargets(id);
			} else {
				// Second click: build the edge
				const edgeType = get(selectedEdgeType);
				gameCommand({
					BuildEdge: { edge_type: edgeType, from: source, to: id },
				});
				buildEdgeSource.set(null);
				edgeTargets.set([]);
			}
			return;
		}

		selectedEntityId.set(id);
		selectedEntityType.set(type);
	}

	function loadEdgeTargets(sourceId: number) {
		const targets = bridge.getBuildableEdges(sourceId);
		edgeTargets.set(targets);

		// Find source node type from infrastructure data
		const allInfra = bridge.getAllInfrastructure();
		const sourceNode = allInfra.nodes.find(n => n.id === sourceId);
		if (!sourceNode) return;

		// Auto-select best edge type for the source node
		if (targets.length > 0) {
			const currentEdge = get(selectedEdgeType);
			// If current edge type can't connect to ANY target, auto-switch
			const hasValidTarget = targets.some(t =>
				canEdgeConnect(currentEdge, sourceNode.node_type, t.target_type)
			);
			if (!hasValidTarget) {
				// Find the first compatible edge type
				const compatTypes = targets.reduce((types: Set<string>, t) => {
					for (const et of getCompatibleEdgeTypes(sourceNode.node_type, t.target_type)) {
						types.add(et);
					}
					return types;
				}, new Set<string>());
				if (compatTypes.size > 0) {
					selectedEdgeType.set([...compatTypes][0]);
				}
			}
		}

		// Filter targets to only those compatible with selected edge type
		updateMapTargets(sourceNode.node_type, targets);
	}

	function updateMapTargets(sourceType: string, targets: typeof $edgeTargets) {
		const edgeType = get(selectedEdgeType);
		const validTargets = targets.filter(t => canEdgeConnect(edgeType, sourceType, t.target_type));
		renderer?.setEdgeTargets(validTargets);
	}

	function handleMapClicked(e: CustomEvent) {
		const currentBuildMode = get(buildMode);

		if (currentBuildMode === "node") {
			const { lon, lat } = e.detail;
			buildMenuLocation.set({ lon, lat });
		}
	}

	onMount(() => {
		injectEventEffectStyles();

		const unsub = initialized.subscribe(async (init) => {
			if (init && container && !renderer) {
				renderer = new MapRenderer(container, get(mapQuality));
				await renderer.buildMap();
				mapReady.set(true);
				renderer.updateInfrastructure();

				// Track viewport bounds for minimap
				const updateViewportBounds = () => {
					const map = (renderer as any)?.map;
					if (!map) return;
					const bounds = map.getBounds();
					viewport.set({
						minX: bounds.getWest(),
						minY: bounds.getSouth(),
						maxX: bounds.getEast(),
						maxY: bounds.getNorth(),
					});
					zoomLevel.set(map.getZoom());
				};
				const mapRef = (renderer as any)?.map;
				if (mapRef) {
					mapRef.on('moveend', updateViewportBounds);
					mapRef.on('zoomend', updateViewportBounds);
					// Set initial viewport
					updateViewportBounds();
				}

				// Map navigation from keyboard
				const handleMapPan = (e: Event) => {
					const { direction } = (e as CustomEvent).detail;
					const map = (renderer as any)?.map;
					if (!map) return;
					const offset = 100; // pixels
					const offsets: Record<string, [number, number]> = {
						up: [0, -offset],
						down: [0, offset],
						left: [-offset, 0],
						right: [offset, 0],
					};
					const [dx, dy] = offsets[direction] ?? [0, 0];
					map.panBy([dx, dy], { duration: 200 });
				};

				const handleMapZoom = (e: Event) => {
					const { direction } = (e as CustomEvent).detail;
					const map = (renderer as any)?.map;
					if (!map) return;
					if (direction === 'in') map.zoomIn({ duration: 200 });
					else map.zoomOut({ duration: 200 });
				};

				const handleMapResetView = () => {
					const map = (renderer as any)?.map;
					if (!map) return;
					map.flyTo({ center: [0, 20], zoom: 2, pitch: 0, duration: 1000 });
				};

				const handleMapTogglePitch = () => {
					const map = (renderer as any)?.map;
					if (!map) return;
					const currentPitch = map.getPitch();
					map.easeTo({ pitch: currentPitch > 10 ? 0 : 45, duration: 500 });
				};

				// Minimap click navigation
				const handleMinimapNavigate = (e: Event) => {
					const { lon, lat } = (e as CustomEvent).detail;
					const map = (renderer as any)?.map;
					if (!map) return;
					map.flyTo({ center: [lon, lat], duration: 400 });
				};

				window.addEventListener('map-pan', handleMapPan);
				window.addEventListener('map-zoom', handleMapZoom);
				window.addEventListener('map-reset-view', handleMapResetView);
				window.addEventListener('map-toggle-pitch', handleMapTogglePitch);
				window.addEventListener('minimap-navigate', handleMinimapNavigate);

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
					if (sourceId === null) {
						edgeTargets.set([]);
						renderer?.setEdgeTargets([]);
					}
				});

				// Subscribe to edge type changes to update target highlighting
				const edgeTypeSub = selectedEdgeType.subscribe(() => {
					const source = get(buildEdgeSource);
					if (source !== null) {
						const targets = get(edgeTargets);
						if (targets.length > 0) {
							const allInfra = bridge.getAllInfrastructure();
							const sourceNode = allInfra.nodes.find(n => n.id === source);
							if (sourceNode) {
								updateMapTargets(sourceNode.node_type, targets);
							}
						}
					}
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
					edgeTypeSub();
					selectionSub();
					window.removeEventListener(
						"entity-selected",
						handleEntitySelected as EventListener,
					);
					window.removeEventListener(
						"map-clicked",
						handleMapClicked as EventListener,
					);
					window.removeEventListener('map-pan', handleMapPan);
					window.removeEventListener('map-zoom', handleMapZoom);
					window.removeEventListener('map-reset-view', handleMapResetView);
					window.removeEventListener('map-toggle-pitch', handleMapTogglePitch);
					window.removeEventListener('minimap-navigate', handleMinimapNavigate);
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
		background: #030810;
	}

	.map-container :global(canvas) {
		display: block;
	}
</style>
