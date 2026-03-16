<script lang="ts">
	import 'maplibre-gl/dist/maplibre-gl.css';
	import { onMount, onDestroy } from "svelte";
	import { MapRenderer } from "./map";
	import { initialized } from "$lib/stores/gameState";
	import { worldInfo } from "$lib/stores/gameState";
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
		radialMenuOpen,
		radialMenuPosition,
		radialMenuGeoPosition,
		selectedBuildItem,
		buildCategory,
		activePanelGroup,
		closePanelGroup,
		diploMenu,
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
		const { id, type, owner, owner_name, node_type, edge_type, screenX, screenY } = e.detail;
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

		// Show diplomatic context menu for non-player infrastructure
		if ((type === 'node' || type === 'edge') && id !== null && owner !== undefined) {
			const playerId = bridge.getPlayerCorpId();
			if (owner !== playerId) {
				diploMenu.set({
					visible: true,
					x: screenX ?? 300,
					y: screenY ?? 300,
					corpId: owner,
					corpName: owner_name ?? `Corp #${owner}`,
					nodeId: id,
					nodeType: node_type ?? edge_type ?? '',
					entityType: type,
				});
				return;
			}
		}
		// Close diplo menu when clicking own infrastructure
		diploMenu.set(null);
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
		// Close diplomatic context menu when clicking empty map
		diploMenu.set(null);

		// Close any open floating panel when clicking the map
		if (get(activePanelGroup) !== 'none') {
			closePanelGroup();
		}

		const currentBuildMode = get(buildMode);
		const currentBuildItem = get(selectedBuildItem);
		const currentBuildCat = get(buildCategory);

		if (currentBuildMode === "node" && currentBuildItem && currentBuildCat === 'node') {
			// Direct placement: build the selected node type at click position
			const { lon, lat } = e.detail;
			gameCommand({
				BuildNode: { node_type: currentBuildItem, lon, lat }
			});
			// Stay in build mode for rapid placement
			return;
		}

		if (currentBuildMode === "node") {
			// Legacy fallback: open build menu at location
			const { lon, lat } = e.detail;
			buildMenuLocation.set({ lon, lat });
		}
	}

	function handleMapContextMenu(e: Event) {
		const detail = (e as CustomEvent).detail;
		// Close radial menu if clicking while it's already open
		if (get(radialMenuOpen)) {
			radialMenuOpen.set(false);
			return;
		}
		radialMenuPosition.set({ x: detail.screenX, y: detail.screenY });
		radialMenuGeoPosition.set({ lon: detail.lon, lat: detail.lat });
		radialMenuOpen.set(true);
	}

	onMount(() => {
		injectEventEffectStyles();

		const unsub = initialized.subscribe(async (init) => {
			if (init && container && !renderer) {
				const info = get(worldInfo);
				renderer = new MapRenderer(container, get(mapQuality), info?.is_real_earth);
				try {
					await renderer.buildMap();
				} catch (e) {
					console.error('[MapView] buildMap() failed:', e);
				}
				mapReady.set(true);
				renderer.updateInfrastructure();

				// Track viewport bounds for minimap
				const updateViewportBounds = () => {
					const map = renderer?.getMap();
					if (!map) return;
					const bounds = map.getBounds();
					viewport.set({
						minX: bounds.getWest(),
						minY: bounds.getSouth(),
						maxX: bounds.getEast(),
						maxY: bounds.getNorth(),
						zoom: map.getZoom(),
					});
					zoomLevel.set(map.getZoom());
				};
				const mapRef = renderer?.getMap();
				if (mapRef) {
					mapRef.on('moveend', updateViewportBounds);
					mapRef.on('zoomend', updateViewportBounds);
					// Set initial viewport
					updateViewportBounds();
				}

				// Map navigation from keyboard
				const handleMapPan = (e: Event) => {
					const { direction } = (e as CustomEvent).detail;
					const map = renderer?.getMap();
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
					const map = renderer?.getMap();
					if (!map) return;
					if (direction === 'in') map.zoomIn({ duration: 200 });
					else map.zoomOut({ duration: 200 });
				};

				const handleMapResetView = () => {
					const map = renderer?.getMap();
					if (!map) return;
					map.flyTo({ center: [0, 20], zoom: 2, pitch: 0, duration: 1000 });
				};

				const handleMapTogglePitch = () => {
					const map = renderer?.getMap();
					if (!map) return;
					const currentPitch = map.getPitch();
					map.easeTo({ pitch: currentPitch > 10 ? 0 : 45, duration: 500 });
				};

				// Minimap click navigation
				const handleMinimapNavigate = (e: Event) => {
					const { lon, lat } = (e as CustomEvent).detail;
					const map = renderer?.getMap();
					if (!map) return;
					map.flyTo({ center: [lon, lat], duration: 400 });
				};

				// Search / fly-to navigation (with optional zoom)
				const handleMapFlyTo = (e: Event) => {
					const { lon, lat, zoom } = (e as CustomEvent).detail;
					renderer?.flyTo(lon, lat, zoom);
				};

				window.addEventListener('map-pan', handleMapPan);
				window.addEventListener('map-zoom', handleMapZoom);
				window.addEventListener('map-reset-view', handleMapResetView);
				window.addEventListener('map-toggle-pitch', handleMapTogglePitch);
				window.addEventListener('minimap-navigate', handleMinimapNavigate);
				window.addEventListener('map-fly-to', handleMapFlyTo);

				// Event-driven map updates: re-render on delta broadcasts + 2s fallback
				const handleMapDirty = () => {
					renderer?.updateInfrastructure();
				};
				window.addEventListener('map-dirty', handleMapDirty);
				// Combined fallback interval: ghost preview every 500ms, infra+cities every 2s
				let intervalTick = 0;
				const interval = setInterval(() => {
					renderer?.updateGhostBuildOptions();
					if (intervalTick % 4 === 0) {
						renderer?.updateInfrastructure();
						renderer?.updateCities();
					}
					intervalTick++;
				}, 500);

				window.addEventListener(
					"entity-selected",
					handleEntitySelected as EventListener,
				);
				window.addEventListener(
					"map-clicked",
					handleMapClicked as EventListener,
				);
				window.addEventListener(
					"map-contextmenu",
					handleMapContextMenu as EventListener,
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

				// Cable drawing preview state relay
				const handleCableDrawingUpdate = (e: Event) => {
					const state = (e as CustomEvent).detail;
					renderer?.setCableDrawingState(state);
				};
				window.addEventListener('cable-drawing-update', handleCableDrawingUpdate);

				// Mouse move for tooltips
				container.addEventListener("mousemove", handleMouseMove);
				container.addEventListener("mouseleave", handleMouseLeave);

				cleanup = () => {
					clearInterval(interval);
					overlaySub();
					edgeSrcSub();
					edgeTypeSub();
					selectionSub();
					window.removeEventListener('map-dirty', handleMapDirty);
					window.removeEventListener(
						"entity-selected",
						handleEntitySelected as EventListener,
					);
					window.removeEventListener(
						"map-clicked",
						handleMapClicked as EventListener,
					);
					window.removeEventListener(
						"map-contextmenu",
						handleMapContextMenu as EventListener,
					);
					window.removeEventListener('map-pan', handleMapPan);
					window.removeEventListener('map-zoom', handleMapZoom);
					window.removeEventListener('map-reset-view', handleMapResetView);
					window.removeEventListener('map-toggle-pitch', handleMapTogglePitch);
					window.removeEventListener('minimap-navigate', handleMinimapNavigate);
					window.removeEventListener('map-fly-to', handleMapFlyTo);
					window.removeEventListener('cable-drawing-update', handleCableDrawingUpdate);
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
		renderer?.clearCursorPosition();
	}

	onDestroy(() => {
		cleanup?.();
		if (frameId !== null) cancelAnimationFrame(frameId);
		renderer?.dispose();
	});
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="map-container" bind:this={container} oncontextmenu={(e) => e.preventDefault()}></div>

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
