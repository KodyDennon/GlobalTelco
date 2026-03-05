<script lang="ts">
	import { playerCorp, formatMoney, worldInfo } from '$lib/stores/gameState';
	import * as bridge from '$lib/wasm/bridge';
	import type { LaunchPadInfo } from '$lib/wasm/bridge';
	import type { SatelliteInventoryItem } from '$lib/wasm/types';
	import { gameCommand } from '$lib/game/commandRouter';
	import { tooltip } from '$lib/ui/tooltip';

	let launchPads: LaunchPadInfo[] = $state([]);
	let inventory: SatelliteInventoryItem[] = $state([]);

	// Selection state for new launch
	let selectedPad = $state(0);
	let selectedRocket = $state('Medium');
	let selectedSats: number[] = $state([]);

	$effect(() => {
		const corp = $playerCorp;
		const tick = $worldInfo.tick;
		if (tick % 5 !== 0) return;
		if (corp) {
			launchPads = bridge.getLaunchSchedule(corp.id);
			inventory = bridge.getSatelliteInventory(corp.id);
		}
	});

	const rocketSpecs: Record<string, { payload: number; cost: number; reliability: string; mass_cap: number }> = {
		Small: { payload: 5, cost: 15_000_000, reliability: '90%', mass_cap: 2000 },
		Medium: { payload: 15, cost: 40_000_000, reliability: '93%', mass_cap: 8000 },
		Heavy: { payload: 30, cost: 100_000_000, reliability: '95%', mass_cap: 20000 },
		SuperHeavy: { payload: 60, cost: 250_000_000, reliability: '97%', mass_cap: 50000 },
	};

	let totalQueued = $derived(
		launchPads.reduce((s, lp) => s + lp.queue.reduce((qs, q) => qs + q.satellite_count, 0), 0)
	);

	let currentRocketSpec = $derived(rocketSpecs[selectedRocket]);
	let totalSelectedMass = $derived(
		inventory.filter(s => selectedSats.includes(s.id)).reduce((sum, s) => sum + s.mass_kg, 0)
	);
	
	let canLaunch = $derived(
		selectedPad > 0 && 
		selectedSats.length > 0 && 
		selectedSats.length <= currentRocketSpec.payload &&
		totalSelectedMass <= currentRocketSpec.mass_cap &&
		($playerCorp?.cash ?? 0) >= currentRocketSpec.cost
	);

	function toggleSat(id: number) {
		if (selectedSats.includes(id)) {
			selectedSats = selectedSats.filter(s => s !== id);
		} else {
			if (selectedSats.length < currentRocketSpec.payload) {
				selectedSats = [...selectedSats, id];
			}
		}
	}

	function scheduleLaunch() {
		if (!canLaunch) return;
		gameCommand({
			ScheduleLaunch: {
				launch_pad: selectedPad,
				rocket_type: selectedRocket,
				satellites: selectedSats
			}
		});
		// Clear selection
		selectedSats = [];
		selectedPad = 0;
	}

	function contractLaunch() {
		if (selectedSats.length === 0) return;
		const cost = currentRocketSpec.cost * 1.5;
		if (($playerCorp?.cash ?? 0) < cost) return;

		gameCommand({
			ContractLaunch: {
				rocket_type: selectedRocket,
				satellites: selectedSats
			}
		});
		selectedSats = [];
	}
</script>

<div class="panel">
	<div class="section">
		<h3>Launch Overview</h3>
		<div class="stat-row">
			<span class="muted">Launch pads</span>
			<span class="mono">{launchPads.length}</span>
		</div>
		<div class="stat-row">
			<span class="muted">Queued launches</span>
			<span class="mono blue">{totalQueued} sats</span>
		</div>
		<div class="stat-row">
			<span class="muted">Inventory</span>
			<span class="mono green">{inventory.length} sats</span>
		</div>
	</div>

	<div class="section">
		<h3>Schedule Launch</h3>
		<div class="launch-form">
			<label class="form-field">
				<span class="label">1. Select Launch Pad</span>
				<select bind:value={selectedPad} aria-label="Select launch pad">
					<option value={0}>Select pad...</option>
					{#each launchPads as lp}
						<option value={lp.launch_pad_id}>Pad #{lp.launch_pad_id} ({lp.cooldown_remaining > 0 ? `Cooldown: ${lp.cooldown_remaining}` : 'Ready'})</option>
					{/each}
					<option value={-1}>Contract Launch (No pad, 1.5x cost)</option>
				</select>
			</label>

			<label class="form-field">
				<span class="label">2. Select Rocket</span>
				<div class="rocket-select">
					{#each Object.keys(rocketSpecs) as r}
						<button 
							class="rocket-btn" 
							class:active={selectedRocket === r}
							onclick={() => { selectedRocket = r; selectedSats = []; }}
						>
							{r}
						</button>
					{/each}
				</div>
			</label>

			<div class="form-field">
				<span class="label">3. Build Payload ({selectedSats.length} / {currentRocketSpec.payload} sats)</span>
				<div class="inventory-scroll">
					{#each inventory as sat}
						<button 
							class="sat-item" 
							class:selected={selectedSats.includes(sat.id)}
							onclick={() => toggleSat(sat.id)}
						>
							<div class="sat-info">
								<span class="sat-name">{sat.constellation_name}</span>
								<span class="sat-meta muted">{sat.orbit_type} • {sat.mass_kg}kg</span>
							</div>
							<div class="checkbox"></div>
						</button>
					{:else}
						<div class="empty-inventory muted">No satellites in inventory. Manufacture them in a Satellite Factory first.</div>
					{/each}
				</div>
			</div>

			<div class="payload-summary">
				<div class="stat-row">
					<span class="muted">Total Mass</span>
					<span class="mono" class:red={totalSelectedMass > currentRocketSpec.mass_cap}>{totalSelectedMass.toLocaleString()} / {currentRocketSpec.mass_cap.toLocaleString()} kg</span>
				</div>
				<div class="stat-row">
					<span class="muted">Launch Cost</span>
					<span class="mono green">{formatMoney(selectedPad === -1 ? currentRocketSpec.cost * 1.5 : currentRocketSpec.cost)}</span>
				</div>
			</div>

			{#if selectedPad === -1}
				<button class="launch-btn contract" onclick={contractLaunch} disabled={selectedSats.length === 0 || ($playerCorp?.cash ?? 0) < currentRocketSpec.cost * 1.5}>
					PURCHASE CONTRACT LAUNCH
				</button>
			{:else}
				<button class="launch-btn" onclick={scheduleLaunch} disabled={!canLaunch}>
					SCHEDULE LAUNCH
				</button>
			{/if}
		</div>
	</div>

	<div class="section">
		<h3>Launch Pads & Queue</h3>
		{#each launchPads as lp}
			<div class="pad-card">
				<div class="pad-header">
					<span class="pad-name">Pad #{lp.launch_pad_id}</span>
					{#if lp.reusable}
						<span class="reusable-badge">Reusable</span>
					{/if}
				</div>
				<div class="pad-stats">
					{#if lp.cooldown_remaining > 0}
						<span class="cooldown"><span class="muted">Cooldown</span> <span class="mono red">{lp.cooldown_remaining} ticks</span></span>
					{:else}
						<span class="ready"><span class="mono green">Ready</span></span>
					{/if}
				</div>
				{#if lp.queue.length > 0}
					<div class="queue-list">
						{#each lp.queue as launch, i}
							<div class="queue-item">
								<span class="muted">#{i + 1}</span>
								<span class="mono">{launch.rocket_type}</span>
								<span class="mono">{launch.satellite_count} sats</span>
							</div>
						{/each}
					</div>
				{:else}
					<div class="queue-empty">No launches queued</div>
				{/if}
			</div>
		{/each}
	</div>
</div>

<style>
	.panel { color: var(--text-secondary); font-family: var(--font-sans); font-size: 13px; }
	.section { padding: 12px 16px; border-bottom: 1px solid var(--border); }
	h3 { font-size: 12px; font-weight: 600; color: var(--text-muted); text-transform: uppercase; letter-spacing: 0.5px; margin-bottom: 8px; }
	.stat-row { display: flex; justify-content: space-between; padding: 3px 0; }
	.muted { color: var(--text-muted); }
	.mono { font-family: var(--font-mono); }
	.green { color: var(--green); }
	.blue { color: var(--blue); }
	.red { color: var(--red); }

	.launch-form { display: flex; flex-direction: column; gap: 12px; background: rgba(17, 24, 39, 0.4); padding: 12px; border-radius: var(--radius-md); border: 1px solid var(--border); }
	.form-field { display: flex; flex-direction: column; gap: 6px; }
	.label { font-size: 11px; font-weight: 700; color: var(--text-primary); text-transform: uppercase; }
	
	select { background: var(--bg-surface); border: 1px solid var(--border); color: var(--text-secondary); padding: 6px; border-radius: var(--radius-sm); }
	
	.rocket-select { display: grid; grid-template-columns: repeat(4, 1fr); gap: 4px; }
	.rocket-btn { padding: 6px 2px; font-size: 10px; font-weight: 700; border: 1px solid var(--border); background: var(--bg-surface); color: var(--text-dim); border-radius: 4px; cursor: pointer; }
	.rocket-btn.active { background: var(--blue); color: white; border-color: var(--blue); }

	.inventory-scroll { max-height: 120px; overflow-y: auto; border: 1px solid var(--border); border-radius: 4px; background: var(--bg-surface); }
	.sat-item { width: 100%; display: flex; justify-content: space-between; align-items: center; padding: 6px 10px; border-bottom: 1px solid var(--border); background: transparent; color: inherit; cursor: pointer; text-align: left; }
	.sat-item:hover { background: rgba(59, 130, 246, 0.1); }
	.sat-item.selected { background: rgba(59, 130, 246, 0.2); }
	.sat-info { display: flex; flex-direction: column; }
	.sat-name { font-weight: 600; font-size: 12px; }
	.sat-meta { font-size: 10px; }
	.checkbox { width: 14px; height: 14px; border: 1px solid var(--border); border-radius: 2px; }
	.selected .checkbox { background: var(--blue); border-color: var(--blue); position: relative; }
	.selected .checkbox::after { content: '✓'; color: white; font-size: 10px; position: absolute; top: 50%; left: 50%; transform: translate(-50%, -50%); }

	.payload-summary { border-top: 1px solid var(--border); padding-top: 8px; }
	.launch-btn { width: 100%; padding: 10px; background: var(--blue); color: white; border: none; border-radius: 4px; font-weight: 700; cursor: pointer; transition: opacity 0.2s; }
	.launch-btn:disabled { opacity: 0.3; cursor: not-allowed; }
	.launch-btn.contract { background: var(--purple); }

	.pad-card { background: var(--bg-surface); border: 1px solid var(--border); border-radius: var(--radius-sm); padding: 10px; margin-bottom: 8px; }
	.pad-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 6px; }
	.pad-name { font-weight: 600; color: var(--text-primary); }
	.reusable-badge { background: rgba(16, 185, 129, 0.1); color: var(--green); font-size: 10px; padding: 2px 6px; border-radius: 3px; }
	.pad-stats { margin-bottom: 6px; }
	.queue-list { border-top: 1px solid var(--border); padding-top: 6px; }
	.queue-item { display: flex; gap: 12px; padding: 2px 0; font-size: 11px; }
	.queue-empty { font-size: 11px; color: var(--text-dim); }
</style>
