<script lang="ts">
	import * as bridge from '$lib/wasm/bridge';
	import type { OrbitalShellStatus } from '$lib/wasm/bridge';
	import { worldInfo } from '$lib/stores/gameState';

	let shells: OrbitalShellStatus[] = $state([]);

	$effect(() => {
		const tick = $worldInfo.tick;
		if (tick % 5 !== 0) return;
		shells = bridge.getDebrisStatus();
	});

	let totalDebris = $derived(shells.reduce((s, sh) => s + sh.debris_count, 0));
	let cascadeCount = $derived(shells.filter((sh) => sh.cascade_active).length);

	function shellLabel(shell: OrbitalShellStatus): string {
		if (shell.min_altitude_km < 600) return 'LEO Low';
		if (shell.min_altitude_km < 1200) return 'LEO Mid';
		if (shell.min_altitude_km < 2000) return 'LEO High';
		if (shell.min_altitude_km < 10000) return 'MEO Low';
		if (shell.min_altitude_km < 35786) return 'MEO High';
		if (shell.min_altitude_km < 36000) return 'GEO';
		return 'HEO';
	}

	function debrisPct(shell: OrbitalShellStatus): number {
		return shell.kessler_threshold > 0 ? (shell.debris_count / shell.kessler_threshold) * 100 : 0;
	}

	function riskClass(shell: OrbitalShellStatus): string {
		if (shell.cascade_active) return 'critical';
		const pct = debrisPct(shell);
		if (pct > 80) return 'high';
		if (pct > 50) return 'medium';
		return 'low';
	}
</script>

<div class="panel">
	<div class="section">
		<h3>Orbital Environment</h3>
		<div class="stat-row">
			<span class="muted">Total debris objects</span>
			<span class="mono" class:warn={totalDebris > 1000}>{totalDebris.toLocaleString()}</span>
		</div>
		{#if cascadeCount > 0}
			<div class="stat-row">
				<span class="muted">Kessler cascades</span>
				<span class="mono red">{cascadeCount} shell{cascadeCount > 1 ? 's' : ''}</span>
			</div>
			<div class="cascade-warning">
				Kessler syndrome active! Debris is self-replicating in {cascadeCount} orbital shell{cascadeCount > 1 ? 's' : ''}. Collision risk is exponentially increasing.
			</div>
		{/if}
	</div>

	<div class="section">
		<h3>Orbital Shells</h3>
		{#each shells as shell}
			<div class="shell-card" class:cascade={shell.cascade_active}>
				<div class="shell-header">
					<span class="shell-name">{shellLabel(shell)}</span>
					<span class="shell-alt">{shell.min_altitude_km.toFixed(0)}-{shell.max_altitude_km.toFixed(0)} km</span>
				</div>
				<div class="debris-bar-container">
					<div class="debris-bar">
						<div class="debris-fill {riskClass(shell)}" style="width: {Math.min(debrisPct(shell), 100)}%"></div>
					</div>
					<span class="debris-label mono">{shell.debris_count}/{shell.kessler_threshold}</span>
				</div>
				<div class="shell-stats">
					<span><span class="muted">Collision prob</span> <span class="mono">{(shell.collision_probability * 100).toFixed(3)}%</span></span>
				</div>
			</div>
		{:else}
			<div class="empty">No orbital data available.</div>
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
	.red { color: var(--red); }
	.warn { color: var(--red); }

	.cascade-warning { background: rgba(239, 68, 68, 0.1); border: 1px solid rgba(239, 68, 68, 0.3); border-radius: var(--radius-sm); padding: 8px 10px; margin-top: 8px; color: var(--red); font-size: 11px; line-height: 1.4; }

	.shell-card { background: var(--bg-surface); border: 1px solid var(--border); border-radius: var(--radius-sm); padding: 10px; margin-bottom: 6px; }
	.shell-card.cascade { border-color: rgba(239, 68, 68, 0.5); background: rgba(239, 68, 68, 0.05); }
	.shell-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 6px; }
	.shell-name { font-weight: 600; color: var(--text-primary); }
	.shell-alt { font-family: var(--font-mono); font-size: 11px; color: var(--text-muted); }
	.debris-bar-container { display: flex; align-items: center; gap: 8px; margin-bottom: 4px; }
	.debris-bar { flex: 1; height: 6px; background: var(--bg-hover); border-radius: 3px; overflow: hidden; }
	.debris-fill { height: 100%; border-radius: 3px; transition: width 0.3s; }
	.debris-fill.low { background: var(--green); }
	.debris-fill.medium { background: var(--amber, #f59e0b); }
	.debris-fill.high { background: var(--red); }
	.debris-fill.critical { background: var(--red); animation: pulse 1s infinite; }
	.debris-label { font-size: 10px; white-space: nowrap; }
	.shell-stats { font-size: 11px; }
	.empty { color: var(--text-dim); text-align: center; padding: 16px; font-size: 12px; }

	@keyframes pulse {
		0%, 100% { opacity: 1; }
		50% { opacity: 0.5; }
	}
</style>
