<script lang="ts">
	import {
		hotbarSlots,
		selectedBuildItem,
		buildCategory,
		buildMode,
		enterPlacementMode,
		exitPlacementMode,
		EDGE_ICON_MAP,
	} from '$lib/stores/uiState';
	import type { HotbarSlot } from '$lib/stores/uiState';
	import { tooltip } from '$lib/ui/tooltip';
	import { playerCorp, notifications } from '$lib/stores/gameState';
	import { icons } from '$lib/assets/icons/index';
	import { toIconKey } from '$lib/game/map/constants';
	import * as bridge from '$lib/wasm/bridge';
	import { get } from 'svelte/store';

	// Display names for item types
	const ITEM_NAMES: Record<string, string> = {
		// Nodes
		CellTower: 'Cell Tower',
		WirelessRelay: 'Relay',
		CentralOffice: 'Central Office',
		ExchangePoint: 'Exchange',
		DataCenter: 'Data Center',
		BackboneRouter: 'Backbone',
		SatelliteGround: 'Satellite',
		SubmarineLanding: 'Sub Landing',
		SatelliteFactory: 'Sat Factory',
		TerminalFactory: 'Term Factory',
		SatelliteWarehouse: 'Sat Warehouse',
		LaunchPad: 'Launch Pad',
		LEO_GroundStation: 'LEO Ground',
		MEO_GroundStation: 'MEO Ground',
		SmallCell: 'Small Cell',
		MacroCell: 'Macro Cell',
		FiberPOP: 'Fiber POP',
		ColocationFacility: 'Colo Facility',
		// Edges
		Copper: 'Copper',
		FiberLocal: 'Fiber Local',
		FiberRegional: 'Fiber Reg.',
		FiberNational: 'Fiber Nat.',
		FiberMetro: 'Fiber Metro',
		FiberLongHaul: 'Fiber LH',
		DWDM_Backbone: 'DWDM',
		Microwave: 'Microwave',
		MicrowaveLink: 'uWave Link',
		Satellite: 'Satellite',
		Submarine: 'Submarine',
		SubseaFiberCable: 'Subsea Fiber',
		QuantumFiberLink: 'Quantum',
		TerahertzBeam: 'THz Beam',
		SatelliteLEOLink: 'LEO Link',
		LaserInterSatelliteLink: 'Laser ISL',
	};

	// Short names for the hotbar display
	const ITEM_SHORT: Record<string, string> = {
		CellTower: 'Tower',
		WirelessRelay: 'Relay',
		CentralOffice: 'Office',
		ExchangePoint: 'Exch.',
		DataCenter: 'DC',
		BackboneRouter: 'Bbone',
		SatelliteGround: 'Sat.',
		SubmarineLanding: 'SubL.',
		SatelliteFactory: 'SatF',
		TerminalFactory: 'TrmF',
		SatelliteWarehouse: 'SatW',
		LaunchPad: 'Pad',
		LEO_GroundStation: 'LEO',
		MEO_GroundStation: 'MEO',
		SmallCell: 'Cell',
		MacroCell: 'Macro',
		FiberPOP: 'POP',
		ColocationFacility: 'Colo',
		Copper: 'Cu',
		FiberLocal: 'F.Loc',
		FiberRegional: 'F.Reg',
		FiberNational: 'F.Nat',
		FiberMetro: 'F.Met',
		FiberLongHaul: 'F.LH',
		DWDM_Backbone: 'DWDM',
		Microwave: 'uWave',
		MicrowaveLink: 'uW.Lk',
		Satellite: 'Sat',
		Submarine: 'SubC',
		SubseaFiberCable: 'SubF',
		QuantumFiberLink: 'QFib',
		TerahertzBeam: 'THz',
		SatelliteLEOLink: 'LEOl',
		LaserInterSatelliteLink: 'LISL',
	};

	// Category badge color
	const CATEGORY_COLOR: Record<string, string> = {
		node: '#10b981',
		edge: '#fbbf24',
	};

	/** Get icon SVG for a hotbar slot item */
	function getSlotIcon(slot: HotbarSlot): string | null {
		if (!slot.itemType) return null;
		if (slot.category === 'node') {
			const key = toIconKey(slot.itemType);
			return (icons as Record<string, string>)[key] ?? null;
		}
		// Edge type: use EDGE_ICON_MAP
		const iconKey = EDGE_ICON_MAP[slot.itemType];
		if (!iconKey) return null;
		return (icons as Record<string, string>)[iconKey] ?? null;
	}

	// ── Drag-and-drop state ──────────────────────────────────────────────────
	let dragSourceIndex: number | null = $state(null);
	let dragOverIndex: number | null = $state(null);

	function handleDragStart(e: DragEvent, index: number) {
		const slot = $hotbarSlots[index];
		if (!slot || !slot.itemType) {
			e.preventDefault();
			return;
		}
		dragSourceIndex = index;
		if (e.dataTransfer) {
			e.dataTransfer.effectAllowed = 'move';
			e.dataTransfer.setData('text/plain', String(index));
		}
	}

	function handleDragOver(e: DragEvent, index: number) {
		e.preventDefault();
		if (e.dataTransfer) {
			e.dataTransfer.dropEffect = 'move';
		}
		dragOverIndex = index;
	}

	function handleDragLeave() {
		dragOverIndex = null;
	}

	function handleDrop(e: DragEvent, targetIndex: number) {
		e.preventDefault();
		dragOverIndex = null;

		if (dragSourceIndex === null || dragSourceIndex === targetIndex) {
			dragSourceIndex = null;
			return;
		}

		const slots = [...$hotbarSlots];
		const temp = slots[dragSourceIndex];
		slots[dragSourceIndex] = slots[targetIndex];
		slots[targetIndex] = temp;
		hotbarSlots.set(slots);

		dragSourceIndex = null;
	}

	function handleDragEnd() {
		dragSourceIndex = null;
		dragOverIndex = null;
	}

	/** Check if player can afford the cheapest build option for this node type. */
	function canAfford(nodeType: string): boolean {
		const corp = get(playerCorp);
		if (!corp) return false;
		const options = bridge.getBuildableNodes(0, 0);
		const opt = options.find(o => o.node_type === nodeType);
		if (opt) return opt.affordable;
		return corp.cash > 0;
	}

	function activateSlot(index: number) {
		const slot = $hotbarSlots[index];
		if (!slot || !slot.itemType || !slot.category) return;

		if ($selectedBuildItem === slot.itemType && $buildCategory === slot.category) {
			exitPlacementMode();
			return;
		}

		if (slot.category === 'node' && !canAfford(slot.itemType)) {
			const info = bridge.getWorldInfo();
			notifications.update((n) => [
				{ tick: info.tick, event: { GlobalNotification: { message: 'Insufficient funds', level: 'warning' } } },
				...n
			].slice(0, 50));
			return;
		}

		enterPlacementMode(slot.itemType, slot.category);
	}

	function getTooltipText(slot: HotbarSlot, index: number): string {
		if (!slot.itemType) return `Slot ${index + 1} (empty)\nRight-click map to build`;
		const name = ITEM_NAMES[slot.itemType] ?? slot.itemType;
		const cat = slot.category === 'node' ? 'Node' : 'Link';
		return `[${index + 1}] ${name} (${cat})\nPress ${index + 1} or click to activate\nDrag to reorder`;
	}
</script>

<div class="hotbar" role="toolbar" aria-label="Build hotbar">
	{#each $hotbarSlots as slot, i}
		{@const isActive = slot.itemType !== null && $selectedBuildItem === slot.itemType && $buildCategory === slot.category}
		{@const isDragging = dragSourceIndex === i}
		{@const isDragOver = dragOverIndex === i && dragSourceIndex !== i}
		{@const slotIcon = getSlotIcon(slot)}
		<button
			class="hotbar-slot"
			class:active={isActive}
			class:filled={slot.itemType !== null}
			class:dragging={isDragging}
			class:drag-over={isDragOver}
			draggable={slot.itemType !== null ? 'true' : 'false'}
			onclick={() => activateSlot(i)}
			ondragstart={(e) => handleDragStart(e, i)}
			ondragover={(e) => handleDragOver(e, i)}
			ondragleave={handleDragLeave}
			ondrop={(e) => handleDrop(e, i)}
			ondragend={handleDragEnd}
			use:tooltip={getTooltipText(slot, i)}
			aria-pressed={isActive}
			aria-label={slot.itemType ? `Slot ${i + 1}: ${ITEM_NAMES[slot.itemType] ?? slot.itemType} (${slot.category === 'node' ? 'Node' : 'Link'})` : `Slot ${i + 1}: empty`}
		>
			<span class="slot-key">{i + 1}</span>
			{#if slot.itemType}
				<div class="slot-content">
					{#if slotIcon}
						<span class="slot-icon">{@html slotIcon}</span>
					{/if}
					<span class="slot-name">{ITEM_SHORT[slot.itemType] ?? slot.itemType}</span>
				</div>
				{#if slot.category}
					<span class="slot-badge" style="background: {CATEGORY_COLOR[slot.category] ?? '#6b7280'}">
						{slot.category === 'node' ? 'N' : 'E'}
					</span>
				{/if}
			{/if}
		</button>
	{/each}
</div>

<style>
	.hotbar {
		position: fixed;
		bottom: 16px;
		left: 50%;
		transform: translateX(-50%);
		display: flex;
		gap: 3px;
		z-index: 15;
		background: rgba(10, 15, 30, 0.9);
		border: 1px solid rgba(55, 65, 81, 0.4);
		border-radius: 10px;
		padding: 4px 6px;
		box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
		backdrop-filter: blur(8px);
	}

	.hotbar-slot {
		position: relative;
		width: 56px;
		height: 48px;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		background: rgba(31, 41, 55, 0.5);
		border: 1px solid rgba(55, 65, 81, 0.3);
		border-radius: 6px;
		cursor: pointer;
		transition: all 0.12s;
		padding: 2px;
		gap: 1px;
	}

	.hotbar-slot:hover {
		background: rgba(55, 65, 81, 0.6);
		border-color: rgba(96, 165, 250, 0.4);
	}

	.hotbar-slot.active {
		background: rgba(16, 185, 129, 0.15);
		border-color: rgba(16, 185, 129, 0.6);
		box-shadow: 0 0 8px rgba(16, 185, 129, 0.2);
	}

	.hotbar-slot.dragging {
		opacity: 0.35;
		transform: scale(0.92);
		border-color: rgba(96, 165, 250, 0.5);
	}

	.hotbar-slot.drag-over {
		border-color: rgba(96, 165, 250, 0.8);
		background: rgba(59, 130, 246, 0.15);
		box-shadow: 0 0 8px rgba(59, 130, 246, 0.3);
	}

	.hotbar-slot:not(.filled) {
		opacity: 0.4;
	}

	.hotbar-slot:not(.filled):hover {
		opacity: 0.6;
	}

	.slot-key {
		position: absolute;
		top: 2px;
		left: 4px;
		font-size: 9px;
		font-family: var(--font-mono, monospace);
		color: #6b7280;
		font-weight: 700;
		line-height: 1;
	}

	.hotbar-slot.active .slot-key {
		color: #10b981;
	}

	.slot-content {
		display: flex;
		align-items: center;
		gap: 2px;
		margin-top: 4px;
	}

	.slot-icon {
		width: 16px;
		height: 16px;
		flex-shrink: 0;
		display: inline-flex;
		align-items: center;
		justify-content: center;
	}

	.slot-icon :global(svg) {
		width: 16px;
		height: 16px;
	}

	.slot-name {
		font-size: 9px;
		font-family: var(--font-mono, monospace);
		color: #d1d5db;
		font-weight: 500;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		max-width: 32px;
		text-align: center;
	}

	.hotbar-slot.active .slot-name {
		color: #10b981;
	}

	.slot-badge {
		font-size: 8px;
		font-weight: 700;
		color: #111827;
		padding: 0 3px;
		border-radius: 2px;
		line-height: 1.4;
	}
</style>
