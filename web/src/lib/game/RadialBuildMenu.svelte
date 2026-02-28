<script lang="ts">
	import {
		radialMenuOpen,
		radialMenuPosition,
		radialMenuGeoPosition,
		enterPlacementMode,
		buildMode,
		buildEdgeSource,
		buildMenuLocation,
		getVisibleNodes,
		ftthBuilderActive,
		addToHotbar,
		isInHotbar,
		EDGE_FUNCTION_GROUPS,
		FTTH_EDGE_TYPES,
		FTTH_NODE_TYPES,
		EDGE_ICON_MAP,
	} from '$lib/stores/uiState';
	import { formatMoney } from '$lib/stores/gameState';
	import * as bridge from '$lib/wasm/bridge';
	import { gameCommand } from '$lib/game/commandRouter';
	import { icons } from '$lib/assets/icons/index';
	import { toIconKey } from '$lib/game/map/constants';
	import type { BuildOption } from '$lib/wasm/types';

	// ── Category definitions ──────────────────────────────────────────────────

	interface BuildItem {
		key: string;
		name: string;
		category: 'node' | 'edge';
		tier?: string;
	}

	interface BuildCategory {
		key: string;
		label: string;
		color: string;
		items: BuildItem[];
	}

	// All node categories — items will be filtered by supersession at render time
	const NODE_CATEGORIES: BuildCategory[] = [
		{
			key: 'access',
			label: 'Access',
			color: '#34d399',
			items: [
				{ key: 'CellTower', name: 'Cell Tower', category: 'node', tier: 'T1' },
				{ key: 'WirelessRelay', name: 'Wireless Relay', category: 'node', tier: 'T1' },
				{ key: 'SmallCell', name: 'Small Cell (5G)', category: 'node', tier: 'T1' },
				{ key: 'TelephonePole', name: 'Telephone Pole', category: 'node', tier: 'T1' },
				{ key: 'TelegraphOffice', name: 'Telegraph Office', category: 'node', tier: 'T1' },
				{ key: 'TelegraphRelay', name: 'Telegraph Relay', category: 'node', tier: 'T1' },
				{ key: 'MeshDroneRelay', name: 'Mesh Drone Relay', category: 'node', tier: 'T1' },
				{ key: 'TerahertzRelay', name: 'Terahertz Relay', category: 'node', tier: 'T1' },
			],
		},
		{
			key: 'metro',
			label: 'Metro',
			color: '#60a5fa',
			items: [
				{ key: 'CentralOffice', name: 'Central Office', category: 'node', tier: 'T2' },
				{ key: 'ExchangePoint', name: 'Exchange Point', category: 'node', tier: 'T2' },
				{ key: 'FiberPOP', name: 'Fiber POP', category: 'node', tier: 'T2' },
				{ key: 'ISPGateway', name: 'ISP Gateway', category: 'node', tier: 'T2' },
				{ key: 'MacroCell', name: 'Macro Cell (4G/5G)', category: 'node', tier: 'T2' },
				{ key: 'DigitalSwitch', name: 'Digital Switch', category: 'node', tier: 'T2' },
				{ key: 'CoaxHub', name: 'Coax Hub', category: 'node', tier: 'T2' },
				{ key: 'ManualExchange', name: 'Manual Exchange', category: 'node', tier: 'T2' },
				{ key: 'AutomaticExchange', name: 'Auto Exchange', category: 'node', tier: 'T2' },
				{ key: 'LongDistanceRelay', name: 'Long Distance Relay', category: 'node', tier: 'T2' },
				{ key: 'QuantumRepeater', name: 'Quantum Repeater', category: 'node', tier: 'T2' },
			],
		},
		{
			key: 'core',
			label: 'Core',
			color: '#818cf8',
			items: [
				{ key: 'DataCenter', name: 'Data Center', category: 'node', tier: 'T3' },
				{ key: 'InternetExchangePoint', name: 'Internet Exchange', category: 'node', tier: 'T3' },
				{ key: 'ColocationFacility', name: 'Colocation Facility', category: 'node', tier: 'T3' },
				{ key: 'EdgeDataCenter', name: 'Edge Data Center', category: 'node', tier: 'T3' },
				{ key: 'ContentDeliveryNode', name: 'CDN Node', category: 'node', tier: 'T3' },
				{ key: 'CloudOnRamp', name: 'Cloud On-Ramp', category: 'node', tier: 'T3' },
				{ key: 'DWDM_Terminal', name: 'DWDM Terminal', category: 'node', tier: 'T3' },
				{ key: 'MicrowaveTower', name: 'Microwave Tower', category: 'node', tier: 'T3' },
				{ key: 'EarlyDataCenter', name: 'Early Data Center', category: 'node', tier: 'T3' },
				{ key: 'NeuromorphicEdgeNode', name: 'Neuromorphic Edge', category: 'node', tier: 'T3' },
			],
		},
		{
			key: 'global',
			label: 'Global',
			color: '#a78bfa',
			items: [
				{ key: 'BackboneRouter', name: 'Backbone Router', category: 'node', tier: 'T4' },
				{ key: 'HyperscaleDataCenter', name: 'Hyperscale DC', category: 'node', tier: 'T4' },
				{ key: 'SatelliteGroundStation', name: 'Satellite Ground (GEO)', category: 'node', tier: 'T4' },
				{ key: 'SatelliteGround', name: 'Satellite Ground', category: 'node', tier: 'T5' },
				{ key: 'SubmarineLanding', name: 'Submarine Landing', category: 'node', tier: 'T5' },
				{ key: 'SubseaLandingStation', name: 'Subsea Landing', category: 'node', tier: 'T5' },
				{ key: 'CableHut', name: 'Cable Hut', category: 'node', tier: 'T5' },
				{ key: 'LEO_SatelliteGateway', name: 'LEO Satellite GW', category: 'node', tier: 'T5' },
				{ key: 'UnderwaterDataCenter', name: 'Underwater DC', category: 'node', tier: 'T5' },
			],
		},
	];

	// Edge category uses function groups (rendered differently)
	const EDGE_CATEGORY: BuildCategory = {
		key: 'edges',
		label: 'Edges',
		color: '#fbbf24',
		items: [], // populated dynamically via groups
	};

	// Satellite category — new 7th segment
	const SATELLITE_CATEGORY: BuildCategory = {
		key: 'satellite',
		label: 'Satellite',
		color: '#fbbf24',
		items: [
			// Ground Stations
			{ key: 'LEO_GroundStation', name: 'LEO Ground Station', category: 'node', tier: 'Sat' },
			{ key: 'MEO_GroundStation', name: 'MEO Ground Station', category: 'node', tier: 'Sat' },
			// Manufacturing
			{ key: 'SatelliteFactory', name: 'Satellite Factory', category: 'node', tier: 'Sat' },
			{ key: 'TerminalFactory', name: 'Terminal Factory', category: 'node', tier: 'Sat' },
			// Logistics
			{ key: 'SatelliteWarehouse', name: 'Satellite Warehouse', category: 'node', tier: 'Sat' },
			{ key: 'LaunchPad', name: 'Launch Pad', category: 'node', tier: 'Sat' },
			// Gateways
			{ key: 'LEO_SatelliteGateway', name: 'LEO Satellite GW', category: 'node', tier: 'Sat' },
		],
	};

	// Satellite sub-function headers for grouping in the flyout
	const SAT_SUBGROUPS: Array<{ label: string; keys: Set<string> }> = [
		{ label: 'Ground Stations', keys: new Set(['LEO_GroundStation', 'MEO_GroundStation']) },
		{ label: 'Manufacturing', keys: new Set(['SatelliteFactory', 'TerminalFactory']) },
		{ label: 'Logistics', keys: new Set(['SatelliteWarehouse', 'LaunchPad']) },
		{ label: 'Gateways', keys: new Set(['LEO_SatelliteGateway']) },
	];

	// All 7 categories in radial order
	const CATEGORIES: BuildCategory[] = [
		...NODE_CATEGORIES,
		EDGE_CATEGORY,
		{ key: 'wireless', label: 'Wireless', color: '#22d3ee', items: [] }, // placeholder — edges rendered via groups
		SATELLITE_CATEGORY,
	];

	let hoveredCategory: string | null = $state(null);
	let buildOptions: BuildOption[] = $state([]);
	let closeTimer: ReturnType<typeof setTimeout> | null = null;
	let expandedGroups: Set<string> = $state(new Set());
	let pinFlash: string | null = $state(null);

	/** Delay clearing hoveredCategory so the flyout has time to receive mouseenter */
	function scheduleClose() {
		closeTimer = setTimeout(() => {
			hoveredCategory = null;
			closeTimer = null;
		}, 150);
	}

	/** Cancel any pending close (mouse entered flyout or another segment) */
	function cancelClose() {
		if (closeTimer !== null) {
			clearTimeout(closeTimer);
			closeTimer = null;
		}
	}

	// Load build options when radial menu opens at a geo position
	$effect(() => {
		const geo = $radialMenuGeoPosition;
		if ($radialMenuOpen && geo) {
			buildOptions = bridge.getBuildableNodes(geo.lon, geo.lat);
		} else {
			buildOptions = [];
		}
	});

	// Compute visible nodes (supersession-filtered)
	let visibleNodes = $derived.by(() => {
		const buildable = new Set(buildOptions.map(o => o.node_type));
		return getVisibleNodes(buildable);
	});

	function getCostForNode(nodeType: string): number | null {
		const opt = buildOptions.find(o => o.node_type === nodeType);
		return opt ? opt.cost : null;
	}

	function isAffordable(nodeType: string): boolean {
		const opt = buildOptions.find(o => o.node_type === nodeType);
		return opt?.affordable ?? true;
	}

	function selectItem(item: BuildItem) {
		if (item.category === 'node') {
			const geo = $radialMenuGeoPosition;
			if (geo) {
				const opt = buildOptions.find(o => o.node_type === item.key);
				if (opt && opt.affordable) {
					gameCommand({
						BuildNode: { node_type: item.key, lon: geo.lon, lat: geo.lat }
					});
				}
			}
			radialMenuOpen.set(false);
		} else {
			enterPlacementMode(item.key, 'edge');
		}
	}

	function close() {
		cancelClose();
		radialMenuOpen.set(false);
		hoveredCategory = null;
		expandedGroups = new Set();
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') {
			close();
		}
	}

	function handleBackdropClick(e: MouseEvent) {
		if ((e.target as HTMLElement).classList.contains('radial-backdrop')) {
			close();
		}
	}

	function toggleEdgeGroup(groupName: string) {
		const next = new Set(expandedGroups);
		if (next.has(groupName)) {
			next.delete(groupName);
		} else {
			next.add(groupName);
		}
		expandedGroups = next;
	}

	function handlePinToHotbar(e: MouseEvent, item: BuildItem) {
		e.stopPropagation();
		if (isInHotbar(item.key)) return;
		const success = addToHotbar(item.key, item.category);
		if (success) {
			pinFlash = item.key;
			setTimeout(() => { pinFlash = null; }, 600);
		}
	}

	/** Get icon SVG string for a node type */
	function getNodeIcon(nodeType: string): string | null {
		const key = toIconKey(nodeType);
		return (icons as Record<string, string>)[key] ?? null;
	}

	/** Get icon SVG string for an edge type */
	function getEdgeIcon(edgeType: string): string | null {
		const iconKey = EDGE_ICON_MAP[edgeType];
		if (!iconKey) return null;
		return (icons as Record<string, string>)[iconKey] ?? null;
	}

	/** Filter node items: remove superseded and FTTH-only nodes */
	function filterNodeItems(items: BuildItem[]): BuildItem[] {
		return items.filter(item => {
			if (FTTH_NODE_TYPES.has(item.key)) return false;
			return visibleNodes.has(item.key);
		});
	}

	// Edge display names for groups
	const EDGE_DISPLAY_NAMES: Record<string, string> = {
		TelegraphWire: 'Telegraph Wire',
		CopperTrunkLine: 'Copper Trunk',
		CoaxialCable: 'Coaxial Cable',
		Copper: 'Copper',
		FiberLocal: 'Fiber Local',
		LongDistanceCopper: 'Long Distance Cu',
		FiberRegional: 'Fiber Regional',
		FiberMetro: 'Fiber Metro',
		FiberNational: 'Fiber National',
		FiberLongHaul: 'Fiber Long Haul',
		DWDM_Backbone: 'DWDM Backbone',
		QuantumFiberLink: 'Quantum Fiber',
		SubseaTelegraphCable: 'Subsea Telegraph',
		Submarine: 'Submarine Cable',
		SubseaFiberCable: 'Subsea Fiber',
		Microwave: 'Microwave',
		MicrowaveLink: 'Microwave Link',
		TerahertzBeam: 'Terahertz Beam',
		EarlySatelliteLink: 'Early Satellite',
		Satellite: 'Satellite Link',
		SatelliteLEOLink: 'Satellite LEO',
		LaserInterSatelliteLink: 'Laser ISL',
	};

	// Geometry calculations for radial segments
	const RADIUS_INNER = 50;
	const RADIUS_OUTER = 140;
	const SEGMENT_COUNT = CATEGORIES.length; // 7

	function getSegmentPath(index: number): string {
		const angleStep = (2 * Math.PI) / SEGMENT_COUNT;
		const startAngle = index * angleStep - Math.PI / 2 - angleStep / 2;
		const endAngle = startAngle + angleStep;
		const gap = 0.03;

		const x1Inner = Math.cos(startAngle + gap) * RADIUS_INNER;
		const y1Inner = Math.sin(startAngle + gap) * RADIUS_INNER;
		const x2Inner = Math.cos(endAngle - gap) * RADIUS_INNER;
		const y2Inner = Math.sin(endAngle - gap) * RADIUS_INNER;
		const x1Outer = Math.cos(startAngle + gap) * RADIUS_OUTER;
		const y1Outer = Math.sin(startAngle + gap) * RADIUS_OUTER;
		const x2Outer = Math.cos(endAngle - gap) * RADIUS_OUTER;
		const y2Outer = Math.sin(endAngle - gap) * RADIUS_OUTER;

		const largeArc = angleStep > Math.PI ? 1 : 0;

		return [
			`M ${x1Inner} ${y1Inner}`,
			`L ${x1Outer} ${y1Outer}`,
			`A ${RADIUS_OUTER} ${RADIUS_OUTER} 0 ${largeArc} 1 ${x2Outer} ${y2Outer}`,
			`L ${x2Inner} ${y2Inner}`,
			`A ${RADIUS_INNER} ${RADIUS_INNER} 0 ${largeArc} 0 ${x1Inner} ${y1Inner}`,
			'Z',
		].join(' ');
	}

	function getLabelPosition(index: number): { x: number; y: number } {
		const angleStep = (2 * Math.PI) / SEGMENT_COUNT;
		const midAngle = index * angleStep - Math.PI / 2;
		const r = (RADIUS_INNER + RADIUS_OUTER) / 2;
		return {
			x: Math.cos(midAngle) * r,
			y: Math.sin(midAngle) * r,
		};
	}

	function getFlyoutPosition(index: number): { x: number; y: number } {
		const angleStep = (2 * Math.PI) / SEGMENT_COUNT;
		const midAngle = index * angleStep - Math.PI / 2;
		const r = RADIUS_OUTER + 20;
		return {
			x: Math.cos(midAngle) * r,
			y: Math.sin(midAngle) * r,
		};
	}

	let hoveredCategoryData = $derived(
		hoveredCategory ? CATEGORIES.find(c => c.key === hoveredCategory) : null
	);
	let hoveredCategoryIndex = $derived(
		hoveredCategory ? CATEGORIES.findIndex(c => c.key === hoveredCategory) : -1
	);
	let flyoutPos = $derived(
		hoveredCategoryIndex >= 0 ? getFlyoutPosition(hoveredCategoryIndex) : { x: 0, y: 0 }
	);

	// Determine flyout type: 'nodes' | 'edges' | 'wireless' | 'satellite'
	let flyoutType = $derived.by(() => {
		if (!hoveredCategory) return 'nodes';
		if (hoveredCategory === 'edges') return 'edges';
		if (hoveredCategory === 'wireless') return 'wireless';
		if (hoveredCategory === 'satellite') return 'satellite';
		return 'nodes';
	});

	// Wired edge groups (for 'edges' flyout)
	const WIRED_GROUPS = ['Local Access', 'Metro/Regional', 'National/Backbone', 'Submarine'];
	// Wireless edge groups (for 'wireless' flyout)
	const WIRELESS_GROUPS = ['Terrestrial Wireless', 'Satellite Link'];

	let menuStyle = $derived.by(() => {
		const pos = $radialMenuPosition;
		const size = RADIUS_OUTER * 2 + 40;
		const halfSize = size / 2;
		const x = Math.max(halfSize, Math.min(window.innerWidth - halfSize - 200, pos.x));
		const y = Math.max(halfSize, Math.min(window.innerHeight - halfSize, pos.y));
		return `left: ${x}px; top: ${y}px;`;
	});
</script>

<svelte:window onkeydown={handleKeydown} />

{#if $radialMenuOpen && !$ftthBuilderActive}
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div class="radial-backdrop" onclick={handleBackdropClick} role="presentation">
		<div class="radial-menu" style={menuStyle} role="menu" aria-label="Build menu">
			<svg
				viewBox="-160 -160 320 320"
				width="320"
				height="320"
				class="radial-svg"
			>
				{#each CATEGORIES as cat, i}
					{@const label = getLabelPosition(i)}
					<!-- svelte-ignore a11y_click_events_have_key_events -->
					<!-- svelte-ignore a11y_no_static_element_interactions -->
					<g
						class="segment"
						class:hovered={hoveredCategory === cat.key}
						role="menuitem"
						tabindex="0"
						aria-label="{cat.label} build category"
						onmouseenter={() => { cancelClose(); hoveredCategory = cat.key; }}
						onmouseleave={() => scheduleClose()}
						onclick={() => { /* handled by sub-menu items */ }}
					>
						<path
							d={getSegmentPath(i)}
							class="segment-path"
							style="--cat-color: {cat.color}"
						/>
						<text
							x={label.x}
							y={label.y}
							class="segment-label"
							text-anchor="middle"
							dominant-baseline="central"
						>
							{cat.label}
						</text>
					</g>
				{/each}

				<!-- Center circle -->
				<circle cx="0" cy="0" r={RADIUS_INNER - 4} class="center-circle" />
				<text x="0" y="-6" class="center-text" text-anchor="middle" dominant-baseline="central">BUILD</text>
				<text x="0" y="10" class="center-hint" text-anchor="middle" dominant-baseline="central">Right-click</text>
			</svg>

			<!-- Flyout sub-menu when hovering a category -->
			{#if hoveredCategoryData}
				{@const screenFlyout = { x: flyoutPos.x, y: flyoutPos.y }}
				<!-- svelte-ignore a11y_no_static_element_interactions -->
				<!-- svelte-ignore a11y_click_events_have_key_events -->
				<div
					class="flyout"
					role="menu"
					tabindex="0"
					aria-label="{hoveredCategoryData.label} items"
					style="transform: translate({screenFlyout.x}px, {screenFlyout.y}px);"
					onmouseenter={() => { cancelClose(); hoveredCategory = hoveredCategoryData?.key ?? null; }}
					onmouseleave={() => scheduleClose()}
				>
					<div class="flyout-header" style="border-color: {hoveredCategoryData.color}">
						{hoveredCategoryData.label}
					</div>

					{#if flyoutType === 'nodes'}
						<!-- Node items with icons, supersession filtering, and pin button -->
						{#each filterNodeItems(hoveredCategoryData.items) as item}
							{@const cost = getCostForNode(item.key)}
							{@const affordable = isAffordable(item.key)}
							{@const icon = getNodeIcon(item.key)}
							{@const inHotbar = isInHotbar(item.key)}
							<!-- svelte-ignore a11y_no_static_element_interactions -->
							<!-- svelte-ignore a11y_click_events_have_key_events -->
							<div
								class="flyout-item"
								class:unaffordable={!affordable}
								onclick={() => { if (affordable) selectItem(item); }}
								role="menuitem"
								tabindex="0"
							>
								<div class="item-left">
									{#if icon}
										<span class="item-icon">{@html icon}</span>
									{/if}
									<span class="item-name">{item.name}</span>
									{#if item.tier}
										<span class="item-tier">{item.tier}</span>
									{/if}
								</div>
								<div class="item-right">
									{#if cost !== null}
										<span class="item-cost" class:red={!affordable}>{formatMoney(cost)}</span>
									{/if}
									<button
										class="pin-btn"
										class:pinned={inHotbar}
										class:flash={pinFlash === item.key}
										onclick={(e) => handlePinToHotbar(e, item)}
										title={inHotbar ? 'Already in hotbar' : 'Pin to hotbar'}
										aria-label={inHotbar ? 'Already in hotbar' : 'Pin to hotbar'}
									>
										{inHotbar ? '\u2713' : '+'}
									</button>
								</div>
							</div>
						{/each}

					{:else if flyoutType === 'edges' || flyoutType === 'wireless'}
						<!-- Edge items grouped by function -->
						{@const groups = flyoutType === 'edges' ? WIRED_GROUPS : WIRELESS_GROUPS}
						{#each groups as groupName}
							{@const edgeTypes = EDGE_FUNCTION_GROUPS[groupName] ?? []}
							{@const isExpanded = expandedGroups.has(groupName)}
							{@const bestEdge = edgeTypes[edgeTypes.length - 1]}
							{#if edgeTypes.length > 0}
								<div class="edge-group">
									<button
										class="edge-group-header"
										onclick={() => toggleEdgeGroup(groupName)}
										aria-expanded={isExpanded}
									>
										<span class="group-label">{groupName}</span>
										<span class="group-chevron" class:expanded={isExpanded}>{isExpanded ? '\u25BC' : '\u25B6'}</span>
									</button>
									{#if isExpanded}
										<!-- Show all edges in group -->
										{#each edgeTypes as edgeType}
											{@const edgeIcon = getEdgeIcon(edgeType)}
											{@const eName = EDGE_DISPLAY_NAMES[edgeType] ?? edgeType}
											{@const edgeItem = { key: edgeType, name: eName, category: 'edge' as const }}
											{@const inHotbar = isInHotbar(edgeType)}
											<!-- svelte-ignore a11y_no_static_element_interactions -->
											<!-- svelte-ignore a11y_click_events_have_key_events -->
											<div
												class="flyout-item edge-item"
												onclick={() => selectItem(edgeItem)}
												role="menuitem"
												tabindex="0"
											>
												<div class="item-left">
													{#if edgeIcon}
														<span class="item-icon">{@html edgeIcon}</span>
													{/if}
													<span class="item-name">{eName}</span>
												</div>
												<div class="item-right">
													<span class="item-hint">Link</span>
													<button
														class="pin-btn"
														class:pinned={inHotbar}
														class:flash={pinFlash === edgeType}
														onclick={(e) => handlePinToHotbar(e, edgeItem)}
														title={inHotbar ? 'Already in hotbar' : 'Pin to hotbar'}
														aria-label={inHotbar ? 'Already in hotbar' : 'Pin to hotbar'}
													>
														{inHotbar ? '\u2713' : '+'}
													</button>
												</div>
											</div>
										{/each}
									{:else}
										<!-- Show only best available -->
										{@const edgeIcon = getEdgeIcon(bestEdge)}
										{@const eName = EDGE_DISPLAY_NAMES[bestEdge] ?? bestEdge}
										{@const edgeItem = { key: bestEdge, name: eName, category: 'edge' as const }}
										{@const inHotbar = isInHotbar(bestEdge)}
										<!-- svelte-ignore a11y_no_static_element_interactions -->
										<!-- svelte-ignore a11y_click_events_have_key_events -->
										<div
											class="flyout-item edge-item"
											onclick={() => selectItem(edgeItem)}
											role="menuitem"
											tabindex="0"
										>
											<div class="item-left">
												{#if edgeIcon}
													<span class="item-icon">{@html edgeIcon}</span>
												{/if}
												<span class="item-name">{eName}</span>
											</div>
											<div class="item-right">
												<span class="item-hint">Link</span>
												<button
													class="pin-btn"
													class:pinned={inHotbar}
													class:flash={pinFlash === bestEdge}
													onclick={(e) => handlePinToHotbar(e, edgeItem)}
													title={inHotbar ? 'Already in hotbar' : 'Pin to hotbar'}
													aria-label={inHotbar ? 'Already in hotbar' : 'Pin to hotbar'}
												>
													{inHotbar ? '\u2713' : '+'}
												</button>
											</div>
										</div>
									{/if}
								</div>
							{/if}
						{/each}

					{:else if flyoutType === 'satellite'}
						<!-- Satellite items grouped by sub-function -->
						{#each SAT_SUBGROUPS as subgroup}
							<div class="sat-subgroup-header">{subgroup.label}</div>
							{#each SATELLITE_CATEGORY.items.filter(it => subgroup.keys.has(it.key)) as item}
								{@const cost = getCostForNode(item.key)}
								{@const affordable = isAffordable(item.key)}
								{@const icon = getNodeIcon(item.key)}
								{@const inHotbar = isInHotbar(item.key)}
								<!-- svelte-ignore a11y_no_static_element_interactions -->
								<!-- svelte-ignore a11y_click_events_have_key_events -->
								<div
									class="flyout-item"
									class:unaffordable={!affordable}
									onclick={() => { if (affordable) selectItem(item); }}
									role="menuitem"
									tabindex="0"
								>
									<div class="item-left">
										{#if icon}
											<span class="item-icon">{@html icon}</span>
										{/if}
										<span class="item-name">{item.name}</span>
									</div>
									<div class="item-right">
										{#if cost !== null}
											<span class="item-cost" class:red={!affordable}>{formatMoney(cost)}</span>
										{/if}
										<button
											class="pin-btn"
											class:pinned={inHotbar}
											class:flash={pinFlash === item.key}
											onclick={(e) => handlePinToHotbar(e, item)}
											title={inHotbar ? 'Already in hotbar' : 'Pin to hotbar'}
											aria-label={inHotbar ? 'Already in hotbar' : 'Pin to hotbar'}
										>
											{inHotbar ? '\u2713' : '+'}
										</button>
									</div>
								</div>
							{/each}
						{/each}
					{/if}
				</div>
			{/if}
		</div>
	</div>
{/if}

<style>
	.radial-backdrop {
		position: fixed;
		inset: 0;
		z-index: 50;
		cursor: default;
	}

	.radial-menu {
		position: absolute;
		transform: translate(-50%, -50%);
		z-index: 51;
		pointer-events: auto;
	}

	.radial-svg {
		filter: drop-shadow(0 8px 32px rgba(0, 0, 0, 0.6));
		overflow: visible;
	}

	.segment-path {
		fill: rgba(19, 26, 43, 0.92);
		stroke: rgba(55, 65, 81, 0.5);
		stroke-width: 1;
		cursor: pointer;
		transition: fill 0.15s, stroke 0.15s;
	}

	.segment:hover .segment-path,
	.segment.hovered .segment-path {
		fill: rgba(30, 41, 62, 0.95);
		stroke: var(--cat-color, rgba(96, 165, 250, 0.7));
		stroke-width: 2;
	}

	.segment-label {
		fill: #9ca3af;
		font-size: 11px;
		font-family: var(--font-sans, system-ui, sans-serif);
		font-weight: 600;
		pointer-events: none;
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}

	.segment:hover .segment-label,
	.segment.hovered .segment-label {
		fill: #f3f4f6;
	}

	.center-circle {
		fill: rgba(10, 15, 30, 0.95);
		stroke: rgba(55, 65, 81, 0.4);
		stroke-width: 1.5;
	}

	.center-text {
		fill: #6b7280;
		font-size: 11px;
		font-family: var(--font-mono, monospace);
		font-weight: 700;
		letter-spacing: 0.15em;
	}

	.center-hint {
		fill: #4b5563;
		font-size: 8px;
		font-family: var(--font-sans, system-ui, sans-serif);
	}

	/* ── Flyout sub-menu ────────────────────────────────────────────────────── */

	.flyout {
		position: absolute;
		top: 50%;
		left: 50%;
		min-width: 220px;
		max-height: 440px;
		overflow-y: auto;
		background: rgba(17, 24, 39, 0.97);
		border: 1px solid rgba(55, 65, 81, 0.6);
		border-radius: 8px;
		box-shadow: 0 12px 40px rgba(0, 0, 0, 0.5);
		pointer-events: auto;
		z-index: 52;
	}

	.flyout-header {
		padding: 8px 14px;
		font-size: 11px;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.06em;
		color: #d1d5db;
		border-bottom: 2px solid;
		background: rgba(31, 41, 55, 0.4);
	}

	.flyout-item {
		display: flex;
		align-items: center;
		justify-content: space-between;
		width: 100%;
		padding: 6px 14px;
		background: none;
		border: none;
		color: #d1d5db;
		font-size: 12px;
		font-family: var(--font-sans, system-ui, sans-serif);
		cursor: pointer;
		transition: background 0.1s;
		text-align: left;
	}

	.flyout-item:hover:not(.unaffordable) {
		background: rgba(55, 65, 81, 0.4);
		color: #f3f4f6;
	}

	.flyout-item.unaffordable {
		opacity: 0.4;
		cursor: not-allowed;
	}

	.item-left {
		display: inline-flex;
		align-items: center;
		gap: 6px;
	}

	.item-right {
		display: inline-flex;
		align-items: center;
		gap: 6px;
	}

	.item-icon {
		width: 20px;
		height: 20px;
		flex-shrink: 0;
		display: inline-flex;
		align-items: center;
		justify-content: center;
	}

	.item-icon :global(svg) {
		width: 20px;
		height: 20px;
	}

	.item-name {
		font-weight: 500;
	}

	.item-tier {
		font-size: 10px;
		font-family: var(--font-mono, monospace);
		padding: 1px 5px;
		background: rgba(55, 65, 81, 0.5);
		border-radius: 3px;
		color: #9ca3af;
	}

	.item-cost {
		font-family: var(--font-mono, monospace);
		font-size: 11px;
		color: #10b981;
		font-weight: 600;
	}

	.item-cost.red {
		color: #ef4444;
	}

	.item-hint {
		font-size: 10px;
		color: #6b7280;
		font-style: italic;
	}

	/* ── Pin to hotbar button ──────────────────────────────────────────────── */

	.pin-btn {
		width: 20px;
		height: 20px;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		background: rgba(55, 65, 81, 0.4);
		border: 1px solid rgba(75, 85, 99, 0.5);
		border-radius: 4px;
		color: #9ca3af;
		font-size: 13px;
		font-weight: 700;
		cursor: pointer;
		transition: all 0.15s;
		padding: 0;
		line-height: 1;
	}

	.pin-btn:hover:not(.pinned) {
		background: rgba(16, 185, 129, 0.2);
		border-color: rgba(16, 185, 129, 0.5);
		color: #10b981;
	}

	.pin-btn.pinned {
		background: rgba(16, 185, 129, 0.15);
		border-color: rgba(16, 185, 129, 0.4);
		color: #10b981;
		cursor: default;
	}

	.pin-btn.flash {
		animation: pin-flash 0.6s ease-out;
	}

	@keyframes pin-flash {
		0% { background: rgba(16, 185, 129, 0.5); }
		100% { background: rgba(16, 185, 129, 0.15); }
	}

	/* ── Edge group headers ────────────────────────────────────────────────── */

	.edge-group {
		border-bottom: 1px solid rgba(55, 65, 81, 0.2);
	}

	.edge-group:last-child {
		border-bottom: none;
	}

	.edge-group-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		width: 100%;
		padding: 6px 14px;
		background: rgba(31, 41, 55, 0.3);
		border: none;
		color: #9ca3af;
		font-size: 10px;
		font-weight: 700;
		font-family: var(--font-sans, system-ui, sans-serif);
		text-transform: uppercase;
		letter-spacing: 0.04em;
		cursor: pointer;
		transition: background 0.12s;
	}

	.edge-group-header:hover {
		background: rgba(55, 65, 81, 0.3);
		color: #d1d5db;
	}

	.group-chevron {
		font-size: 8px;
		transition: transform 0.15s;
	}

	.edge-item {
		padding-left: 20px;
	}

	/* ── Satellite sub-group headers ───────────────────────────────────────── */

	.sat-subgroup-header {
		padding: 5px 14px 3px;
		font-size: 9px;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.04em;
		color: #fbbf24;
		background: rgba(251, 191, 36, 0.05);
		border-top: 1px solid rgba(251, 191, 36, 0.1);
	}

	.sat-subgroup-header:first-child {
		border-top: none;
	}
</style>
