<script lang="ts">
	import {
		hotbarSlots,
		selectedBuildItem,
		buildCategory,
		buildMode,
		enterPlacementMode,
		exitPlacementMode,
	} from '$lib/stores/uiState';
	import type { HotbarSlot } from '$lib/stores/uiState';
	import { tooltip } from '$lib/ui/tooltip';

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
		// Edges
		Copper: 'Copper',
		FiberLocal: 'Fiber Local',
		FiberRegional: 'Fiber Reg.',
		FiberNational: 'Fiber Nat.',
		Microwave: 'Microwave',
		Satellite: 'Satellite',
		Submarine: 'Submarine',
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
		Copper: 'Cu',
		FiberLocal: 'F.Loc',
		FiberRegional: 'F.Reg',
		FiberNational: 'F.Nat',
		Microwave: 'uWave',
		Satellite: 'Sat',
		Submarine: 'SubC',
	};

	// Category badge color
	const CATEGORY_COLOR: Record<string, string> = {
		node: '#10b981',
		edge: '#fbbf24',
	};

	function activateSlot(index: number) {
		const slot = $hotbarSlots[index];
		if (!slot || !slot.itemType || !slot.category) return;

		// Toggle off if already active
		if ($selectedBuildItem === slot.itemType && $buildCategory === slot.category) {
			exitPlacementMode();
			return;
		}

		enterPlacementMode(slot.itemType, slot.category);
	}

	function getTooltipText(slot: HotbarSlot, index: number): string {
		if (!slot.itemType) return `Slot ${index + 1} (empty)\nRight-click map to build`;
		const name = ITEM_NAMES[slot.itemType] ?? slot.itemType;
		const cat = slot.category === 'node' ? 'Node' : 'Link';
		return `[${index + 1}] ${name} (${cat})\nPress ${index + 1} or click to activate`;
	}
</script>

<div class="hotbar" role="toolbar" aria-label="Build hotbar">
	{#each $hotbarSlots as slot, i}
		{@const isActive = slot.itemType !== null && $selectedBuildItem === slot.itemType && $buildCategory === slot.category}
		<button
			class="hotbar-slot"
			class:active={isActive}
			class:filled={slot.itemType !== null}
			onclick={() => activateSlot(i)}
			use:tooltip={getTooltipText(slot, i)}
			aria-pressed={isActive}
		>
			<span class="slot-key">{i + 1}</span>
			{#if slot.itemType}
				<span class="slot-name">{ITEM_SHORT[slot.itemType] ?? slot.itemType}</span>
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

	.slot-name {
		font-size: 10px;
		font-family: var(--font-mono, monospace);
		color: #d1d5db;
		font-weight: 500;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		max-width: 48px;
		text-align: center;
		margin-top: 4px;
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
