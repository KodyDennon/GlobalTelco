<script lang="ts">
	import type { WorldConfig } from '$lib/api/types.js';

	type WorldPreset = 'real_earth' | 'pangaea' | 'archipelago' | 'continents' | 'random';

	let {
		value = $bindable<WorldConfig>({}),
		onchange
	}: {
		value: WorldConfig;
		onchange?: (config: WorldConfig) => void;
	} = $props();

	const ERAS = ['Telegraph', 'Telephone', 'EarlyDigital', 'Internet', 'Modern', 'NearFuture'];
	const DIFFICULTIES = ['Easy', 'Normal', 'Hard', 'Expert'];
	const MAP_SIZES = ['Tiny', 'Small', 'Medium', 'Large', 'Huge'];

	const PRESETS: Record<WorldPreset, { label: string; desc: string; continents: number; ocean: number; roughness: number; climate: number; density: number }> = {
		real_earth: { label: 'Real Earth', desc: 'Actual world geography', continents: 6, ocean: 71, roughness: 50, climate: 50, density: 50 },
		pangaea: { label: 'Pangaea', desc: 'Single supercontinent', continents: 1, ocean: 40, roughness: 60, climate: 40, density: 40 },
		archipelago: { label: 'Archipelago', desc: 'Many islands, lots of ocean', continents: 8, ocean: 80, roughness: 70, climate: 60, density: 30 },
		continents: { label: 'Continents', desc: 'Balanced landmasses', continents: 4, ocean: 65, roughness: 50, climate: 50, density: 50 },
		random: { label: 'Random', desc: 'Randomize everything', continents: 4, ocean: 65, roughness: 50, climate: 50, density: 50 }
	};

	let preset = $state<WorldPreset | null>(null);
	let seed = $state(value.seed ?? Math.floor(Math.random() * 999999));
	let era = $state(value.starting_era ?? 'Internet');
	let difficulty = $state(value.difficulty ?? 'Normal');
	let mapSize = $state(value.map_size ?? 'Medium');
	let aiCorps = $state(value.ai_corporations ?? 4);
	let useRealEarth = $state(value.use_real_earth ?? false);
	let continentCount = $state(value.continent_count ?? 4);
	let oceanPct = $state(Math.round((value.ocean_percentage ?? 0.65) * 100));
	let roughness = $state(Math.round((value.terrain_roughness ?? 0.5) * 100));
	let climate = $state(Math.round((value.climate_variation ?? 0.5) * 100));
	let density = $state(Math.round((value.city_density ?? 0.5) * 100));
	let sandbox = $state(value.sandbox ?? false);

	function emitChange() {
		const config: WorldConfig = {
			seed,
			starting_era: era,
			difficulty,
			map_size: mapSize,
			ai_corporations: aiCorps,
			use_real_earth: useRealEarth,
			continent_count: continentCount,
			ocean_percentage: oceanPct / 100,
			terrain_roughness: roughness / 100,
			climate_variation: climate / 100,
			city_density: density / 100,
			sandbox
		};
		value = config;
		onchange?.(config);
	}

	// Fire onchange whenever any value changes
	$effect(() => {
		void [seed, era, difficulty, mapSize, aiCorps, useRealEarth, continentCount, oceanPct, roughness, climate, density, sandbox];
		emitChange();
	});

	function applyPreset(p: WorldPreset) {
		preset = p;
		useRealEarth = p === 'real_earth';
		if (p === 'random') {
			continentCount = Math.floor(Math.random() * 8) + 1;
			oceanPct = Math.floor(Math.random() * 61) + 30;
			roughness = Math.floor(Math.random() * 101);
			climate = Math.floor(Math.random() * 101);
			density = Math.floor(Math.random() * 101);
			seed = Math.floor(Math.random() * 999999);
		} else {
			const def = PRESETS[p];
			continentCount = def.continents;
			oceanPct = def.ocean;
			roughness = def.roughness;
			climate = def.climate;
			density = def.density;
		}
	}

	function randomizeSeed() {
		seed = Math.floor(Math.random() * 999999);
	}

	// Initialize preset from defaults
	$effect(() => {
		if (preset === null) {
			applyPreset('continents');
		}
	});
</script>

<div class="wcf">
	<!-- Presets -->
	<div class="wcf-section">
		<span class="wcf-label">Map Preset</span>
		<div class="preset-row">
			{#each Object.entries(PRESETS) as [key, def]}
				<button
					class="preset-btn"
					class:active={preset === key}
					onclick={() => applyPreset(key as WorldPreset)}
					title={def.desc}
				>
					{def.label}
				</button>
			{/each}
		</div>
	</div>

	<!-- Core settings -->
	<div class="wcf-grid">
		<div class="wcf-field">
			<label class="wcf-label">
				Starting Era
				<select bind:value={era}>
					{#each ERAS as e}<option value={e}>{e}</option>{/each}
				</select>
			</label>
		</div>
		<div class="wcf-field">
			<label class="wcf-label">
				Difficulty
				<select bind:value={difficulty}>
					{#each DIFFICULTIES as d}<option value={d}>{d}</option>{/each}
				</select>
			</label>
		</div>
		<div class="wcf-field">
			<label class="wcf-label">
				Map Size
				<select bind:value={mapSize}>
					{#each MAP_SIZES as m}<option value={m}>{m}</option>{/each}
				</select>
			</label>
		</div>
		<div class="wcf-field">
			<label class="wcf-label">
				AI Corporations
				<div class="slider-row">
					<input type="range" min="0" max="20" bind:value={aiCorps} />
					<span class="slider-val">{aiCorps}</span>
				</div>
			</label>
		</div>
	</div>

	<!-- Seed -->
	<div class="wcf-field seed-field">
		<label class="wcf-label">
			Seed
			<div class="seed-row">
				<input type="number" bind:value={seed} min="0" />
				<button class="btn-sm" onclick={randomizeSeed} title="Randomize">Dice</button>
			</div>
		</label>
	</div>

	<!-- Terrain settings -->
	<div class="wcf-section">
		<span class="wcf-label">Terrain Generation</span>
		{#if useRealEarth}
			<div class="real-earth-notice">
				Using real-world geography — terrain sliders locked.
			</div>
		{/if}
		<div class="wcf-grid">
			<div class="wcf-field">
				<label class="wcf-label-sm">
					Continents
					<div class="slider-row">
						<input type="range" min="1" max="8" bind:value={continentCount} disabled={useRealEarth} />
						<span class="slider-val">{continentCount}</span>
					</div>
				</label>
			</div>
			<div class="wcf-field">
				<label class="wcf-label-sm">
					Ocean %
					<div class="slider-row">
						<input type="range" min="30" max="90" bind:value={oceanPct} disabled={useRealEarth} />
						<span class="slider-val">{oceanPct}%</span>
					</div>
				</label>
			</div>
			<div class="wcf-field">
				<label class="wcf-label-sm">
					Roughness
					<div class="slider-row">
						<input type="range" min="0" max="100" bind:value={roughness} disabled={useRealEarth} />
						<span class="slider-val">{roughness}%</span>
					</div>
				</label>
			</div>
			<div class="wcf-field">
				<label class="wcf-label-sm">
					Climate
					<div class="slider-row">
						<input type="range" min="0" max="100" bind:value={climate} disabled={useRealEarth} />
						<span class="slider-val">{climate}%</span>
					</div>
				</label>
			</div>
			<div class="wcf-field">
				<label class="wcf-label-sm">
					City Density
					<div class="slider-row">
						<input type="range" min="0" max="100" bind:value={density} disabled={useRealEarth} />
						<span class="slider-val">{density}%</span>
					</div>
				</label>
			</div>
		</div>
	</div>

	<!-- Sandbox toggle -->
	<div class="wcf-field">
		<label class="checkbox-label">
			<input type="checkbox" bind:checked={sandbox} />
			Sandbox Mode
			<span class="help-text">Infinite money, instant builds</span>
		</label>
	</div>
</div>

<style>
	.wcf { display: flex; flex-direction: column; gap: 14px; }
	.wcf-section { display: flex; flex-direction: column; gap: 8px; }
	.wcf-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 10px; }
	.wcf-field { display: flex; flex-direction: column; gap: 3px; }
	.wcf-label { font-size: 11px; color: var(--text-dim); font-weight: 600; text-transform: uppercase; letter-spacing: 0.04em; }
	.wcf-label-sm { font-size: 11px; color: var(--text-dim); font-weight: 500; }

	.preset-row { display: flex; gap: 6px; flex-wrap: wrap; }
	.preset-btn {
		padding: 4px 12px; font-size: 12px; background: var(--bg-surface);
		border: 1px solid var(--border); border-radius: var(--radius-sm);
		color: var(--text-muted); cursor: pointer; font-family: inherit;
	}
	.preset-btn:hover { background: var(--bg-hover); color: var(--text-primary); }
	.preset-btn.active { background: var(--blue); color: white; border-color: var(--blue); }

	select, input[type='number'] {
		padding: 6px 10px; background: var(--bg-surface); border: 1px solid var(--border);
		border-radius: var(--radius-sm); color: var(--text-primary); font-size: 13px; font-family: inherit;
	}

	.slider-row { display: flex; align-items: center; gap: 8px; }
	.slider-row input[type='range'] { flex: 1; accent-color: var(--blue); }
	.slider-row input[type='range']:disabled { opacity: 0.4; }
	.slider-val { font-size: 12px; min-width: 36px; text-align: right; font-family: monospace; color: var(--text-muted); }

	.seed-field { max-width: 300px; }
	.seed-row { display: flex; gap: 6px; }
	.seed-row input { flex: 1; }
	.btn-sm {
		padding: 4px 10px; font-size: 11px; background: var(--bg-surface);
		border: 1px solid var(--border); border-radius: var(--radius-sm);
		color: var(--text-muted); cursor: pointer; font-family: inherit;
	}
	.btn-sm:hover { background: var(--bg-hover); color: var(--text-primary); }

	.real-earth-notice {
		font-size: 12px; color: var(--amber, #f59e0b); background: rgba(245, 158, 11, 0.08);
		border: 1px solid rgba(245, 158, 11, 0.2); border-radius: var(--radius-sm);
		padding: 6px 10px;
	}

	.checkbox-label { display: flex; align-items: center; gap: 8px; font-size: 13px; cursor: pointer; }
	.help-text { font-size: 11px; color: var(--text-dim); }

	@media (max-width: 600px) {
		.wcf-grid { grid-template-columns: 1fr; }
		.preset-row { gap: 4px; }
		.preset-btn { padding: 4px 8px; font-size: 11px; }
	}
</style>
