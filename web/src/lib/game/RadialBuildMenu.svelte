<script lang="ts">
	import {
		radialMenuOpen,
		radialMenuPosition,
		radialMenuGeoPosition,
		enterPlacementMode,
		buildMode,
		buildEdgeSource,
		buildMenuLocation,
	} from '$lib/stores/uiState';
	import { formatMoney } from '$lib/stores/gameState';
	import * as bridge from '$lib/wasm/bridge';
	import { gameCommand } from '$lib/game/commandRouter';
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

	const CATEGORIES: BuildCategory[] = [
		{
			key: 'backbone',
			label: 'Backbone',
			color: '#818cf8',
			items: [
				{ key: 'BackboneRouter', name: 'Backbone Router', category: 'node', tier: 'T4' },
				{ key: 'SubmarineLanding', name: 'Submarine Landing', category: 'node', tier: 'T5' },
				{ key: 'SatelliteGround', name: 'Satellite Ground', category: 'node', tier: 'T5' },
			],
		},
		{
			key: 'distribution',
			label: 'Distribution',
			color: '#60a5fa',
			items: [
				{ key: 'CentralOffice', name: 'Central Office', category: 'node', tier: 'T2' },
				{ key: 'ExchangePoint', name: 'Exchange Point', category: 'node', tier: 'T2' },
				{ key: 'DataCenter', name: 'Data Center', category: 'node', tier: 'T3' },
			],
		},
		{
			key: 'access',
			label: 'Access',
			color: '#34d399',
			items: [
				{ key: 'CellTower', name: 'Cell Tower', category: 'node', tier: 'T1' },
				{ key: 'WirelessRelay', name: 'Wireless Relay', category: 'node', tier: 'T1' },
			],
		},
		{
			key: 'cables',
			label: 'Cables',
			color: '#fbbf24',
			items: [
				{ key: 'Copper', name: 'Copper', category: 'edge' },
				{ key: 'FiberLocal', name: 'Fiber Local', category: 'edge' },
				{ key: 'Microwave', name: 'Microwave', category: 'edge' },
				{ key: 'FiberRegional', name: 'Fiber Regional', category: 'edge' },
				{ key: 'FiberNational', name: 'Fiber National', category: 'edge' },
				{ key: 'Satellite', name: 'Satellite Link', category: 'edge' },
				{ key: 'Submarine', name: 'Submarine Cable', category: 'edge' },
			],
		},
		{
			key: 'wireless',
			label: 'Wireless',
			color: '#22d3ee',
			items: [
				{ key: 'CellTower', name: 'Cell Tower', category: 'node', tier: 'T1' },
				{ key: 'WirelessRelay', name: 'Wireless Relay', category: 'node', tier: 'T1' },
				{ key: 'SatelliteGround', name: 'Satellite Ground', category: 'node', tier: 'T5' },
			],
		},
		{
			key: 'infrastructure',
			label: 'Infra',
			color: '#a78bfa',
			items: [
				{ key: 'DataCenter', name: 'Data Center', category: 'node', tier: 'T3' },
				{ key: 'BackboneRouter', name: 'Backbone Router', category: 'node', tier: 'T4' },
			],
		},
	];

	let hoveredCategory: string | null = $state(null);
	let buildOptions: BuildOption[] = $state([]);

	// Load build options when radial menu opens at a geo position
	$effect(() => {
		const geo = $radialMenuGeoPosition;
		if ($radialMenuOpen && geo) {
			buildOptions = bridge.getBuildableNodes(geo.lon, geo.lat);
		} else {
			buildOptions = [];
		}
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
			// For nodes opened via right-click, build immediately at that location
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
			// For edges, enter edge placement mode
			enterPlacementMode(item.key, 'edge');
		}
	}

	function close() {
		radialMenuOpen.set(false);
		hoveredCategory = null;
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') {
			close();
		}
	}

	function handleBackdropClick(e: MouseEvent) {
		// Only close if clicking the backdrop itself
		if ((e.target as HTMLElement).classList.contains('radial-backdrop')) {
			close();
		}
	}

	// Geometry calculations for radial segments
	const RADIUS_INNER = 50;
	const RADIUS_OUTER = 140;
	const SEGMENT_COUNT = CATEGORIES.length;

	function getSegmentPath(index: number): string {
		const angleStep = (2 * Math.PI) / SEGMENT_COUNT;
		const startAngle = index * angleStep - Math.PI / 2 - angleStep / 2;
		const endAngle = startAngle + angleStep;
		const gap = 0.03; // small gap between segments

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

	// Sub-menu flyout position
	function getFlyoutPosition(index: number): { x: number; y: number } {
		const angleStep = (2 * Math.PI) / SEGMENT_COUNT;
		const midAngle = index * angleStep - Math.PI / 2;
		const r = RADIUS_OUTER + 20;
		return {
			x: Math.cos(midAngle) * r,
			y: Math.sin(midAngle) * r,
		};
	}

	// Get the hovered category data
	let hoveredCategoryData = $derived(
		hoveredCategory ? CATEGORIES.find(c => c.key === hoveredCategory) : null
	);
	let hoveredCategoryIndex = $derived(
		hoveredCategory ? CATEGORIES.findIndex(c => c.key === hoveredCategory) : -1
	);
	let flyoutPos = $derived(
		hoveredCategoryIndex >= 0 ? getFlyoutPosition(hoveredCategoryIndex) : { x: 0, y: 0 }
	);

	// Clamp menu position to keep it on screen
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

{#if $radialMenuOpen}
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div class="radial-backdrop" onclick={handleBackdropClick}>
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
						onmouseenter={() => hoveredCategory = cat.key}
						onmouseleave={() => hoveredCategory = null}
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
				{@const screenFlyout = {
					x: flyoutPos.x,
					y: flyoutPos.y
				}}
				<!-- svelte-ignore a11y_no_static_element_interactions -->
				<!-- svelte-ignore a11y_click_events_have_key_events -->
				<div
					class="flyout"
					style="transform: translate({screenFlyout.x}px, {screenFlyout.y}px);"
					onmouseenter={() => hoveredCategory = hoveredCategoryData?.key ?? null}
				>
					<div class="flyout-header" style="border-color: {hoveredCategoryData.color}">
						{hoveredCategoryData.label}
					</div>
					{#each hoveredCategoryData.items as item}
						{@const cost = item.category === 'node' ? getCostForNode(item.key) : null}
						{@const affordable = item.category === 'node' ? isAffordable(item.key) : true}
						<button
							class="flyout-item"
							class:unaffordable={!affordable}
							onclick={() => selectItem(item)}
							disabled={item.category === 'node' && !affordable}
							role="menuitem"
						>
							<div class="item-left">
								<span class="item-name">{item.name}</span>
								{#if item.tier}
									<span class="item-tier">{item.tier}</span>
								{/if}
							</div>
							{#if cost !== null}
								<span class="item-cost" class:red={!affordable}>{formatMoney(cost)}</span>
							{:else if item.category === 'edge'}
								<span class="item-hint">Link</span>
							{/if}
						</button>
					{/each}
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
		min-width: 200px;
		background: rgba(17, 24, 39, 0.97);
		border: 1px solid rgba(55, 65, 81, 0.6);
		border-radius: 8px;
		box-shadow: 0 12px 40px rgba(0, 0, 0, 0.5);
		overflow: hidden;
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
		padding: 8px 14px;
		background: none;
		border: none;
		color: #d1d5db;
		font-size: 12px;
		font-family: var(--font-sans, system-ui, sans-serif);
		cursor: pointer;
		transition: background 0.1s;
		text-align: left;
	}

	.flyout-item:hover:not(:disabled) {
		background: rgba(55, 65, 81, 0.4);
		color: #f3f4f6;
	}

	.flyout-item:disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}

	.flyout-item.unaffordable {
		opacity: 0.5;
	}

	.item-left {
		display: flex;
		align-items: center;
		gap: 8px;
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
</style>
