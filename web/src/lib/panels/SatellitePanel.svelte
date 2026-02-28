<script lang="ts">
	import { playerCorp, formatMoney } from '$lib/stores/gameState';
	import * as bridge from '$lib/wasm/bridge';
	import { gameCommand } from '$lib/game/commandRouter';
	import type { ConstellationData, OrbitalSatellite } from '$lib/wasm/bridge';

	let constellations: ConstellationData[] = $state([]);
	let satellites: OrbitalSatellite[] = $state([]);

	$effect(() => {
		const corp = $playerCorp;
		if (corp) {
			constellations = bridge.getConstellationData(corp.id);
			satellites = bridge.getOrbitalView();
		}
	});

	let ownedSats = $derived(
		satellites.filter((s) => s.owner === $playerCorp?.id)
	);
	let operationalCount = $derived(ownedSats.filter((s) => s.status === 'Operational').length);
	let decayingCount = $derived(ownedSats.filter((s) => s.status === 'Decaying').length);
	let awaitingLaunch = $derived(ownedSats.filter((s) => s.status === 'AwaitingLaunch').length);
	let avgFuel = $derived(
		ownedSats.length > 0
			? ownedSats.reduce((s, sat) => s + (sat.fuel_capacity > 0 ? sat.fuel_remaining / sat.fuel_capacity : 0), 0) / ownedSats.length
			: 0
	);

	function deorbitSatellite(satId: number) {
		gameCommand({ DeorbitSatellite: { satellite: satId } });
	}

	function serviceSatellite(satId: number, serviceType: string) {
		gameCommand({ ServiceSatellite: { satellite: satId, service_type: serviceType } });
	}
</script>

<div class="panel">
	<div class="section">
		<h3>Fleet Overview</h3>
		<div class="stat-row">
			<span class="muted">Total satellites</span>
			<span class="mono">{ownedSats.length}</span>
		</div>
		<div class="stat-row">
			<span class="muted">Operational</span>
			<span class="mono green">{operationalCount}</span>
		</div>
		<div class="stat-row">
			<span class="muted">Awaiting launch</span>
			<span class="mono blue">{awaitingLaunch}</span>
		</div>
		{#if decayingCount > 0}
			<div class="stat-row">
				<span class="muted">Decaying</span>
				<span class="mono red">{decayingCount}</span>
			</div>
		{/if}
		<div class="stat-row">
			<span class="muted">Avg fuel level</span>
			<span class="mono" class:warn={avgFuel < 0.3}>{(avgFuel * 100).toFixed(0)}%</span>
		</div>
	</div>

	<div class="section">
		<h3>Constellations ({constellations.length})</h3>
		{#each constellations as c}
			<div class="constellation-card">
				<div class="constellation-header">
					<span class="constellation-name">{c.name}</span>
					<span class="orbit-badge">{c.orbit_type}</span>
				</div>
				<div class="constellation-stats">
					<span><span class="muted">Sats</span> <span class="mono">{c.operational_count}/{c.total_target}</span></span>
					<span><span class="muted">Alt</span> <span class="mono">{c.target_altitude_km.toFixed(0)}km</span></span>
					<span><span class="muted">Planes</span> <span class="mono">{c.num_planes}</span></span>
				</div>
				<div class="progress-bar">
					<div class="progress-fill" style="width: {(c.operational_count / Math.max(c.total_target, 1)) * 100}%"></div>
				</div>
			</div>
		{:else}
			<div class="empty">No constellations. Build a Satellite Factory and Launch Pad to get started.</div>
		{/each}
	</div>

	{#if decayingCount > 0}
		<div class="section">
			<h3>Satellites Needing Attention</h3>
			{#each ownedSats.filter((s) => s.status === 'Decaying' || (s.fuel_capacity > 0 && s.fuel_remaining / s.fuel_capacity < 0.2)) as sat}
				<div class="sat-row">
					<div class="sat-info">
						<span class="sat-id">SAT-{sat.id}</span>
						<span class="status-badge" class:decaying={sat.status === 'Decaying'}>{sat.status}</span>
					</div>
					<div class="sat-actions">
						<button class="action-btn" onclick={() => serviceSatellite(sat.id, 'Refuel')}>Refuel</button>
						<button class="action-btn danger" onclick={() => deorbitSatellite(sat.id)}>Deorbit</button>
					</div>
				</div>
			{/each}
		</div>
	{/if}
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
	.warn { color: var(--red); }

	.constellation-card { background: var(--bg-surface); border: 1px solid var(--border); border-radius: var(--radius-sm); padding: 10px; margin-bottom: 8px; }
	.constellation-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 6px; }
	.constellation-name { font-weight: 600; color: var(--text-primary); }
	.orbit-badge { background: var(--blue-bg, rgba(59, 130, 246, 0.1)); color: var(--blue); font-size: 10px; padding: 2px 6px; border-radius: 3px; font-family: var(--font-mono); }
	.constellation-stats { display: flex; gap: 16px; font-size: 11px; margin-bottom: 6px; }
	.progress-bar { height: 4px; background: var(--bg-hover); border-radius: 2px; overflow: hidden; }
	.progress-fill { height: 100%; background: var(--green); border-radius: 2px; transition: width 0.3s; }

	.sat-row { display: flex; justify-content: space-between; align-items: center; padding: 6px 0; border-bottom: 1px solid rgba(55, 65, 81, 0.2); }
	.sat-info { display: flex; gap: 8px; align-items: center; }
	.sat-id { font-family: var(--font-mono); font-size: 11px; color: var(--text-primary); }
	.status-badge { font-size: 10px; padding: 1px 5px; border-radius: 3px; background: var(--bg-hover); }
	.status-badge.decaying { background: rgba(239, 68, 68, 0.1); color: var(--red); }
	.sat-actions { display: flex; gap: 4px; }
	.action-btn { background: var(--bg-surface); border: 1px solid var(--border); color: var(--blue); padding: 3px 8px; border-radius: var(--radius-sm); cursor: pointer; font-size: 11px; font-family: var(--font-mono); }
	.action-btn:hover { background: var(--bg-hover); }
	.action-btn.danger { color: var(--red); }
	.empty { color: var(--text-dim); text-align: center; padding: 16px; font-size: 12px; }
</style>
