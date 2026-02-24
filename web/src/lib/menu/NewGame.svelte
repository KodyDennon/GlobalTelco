<script lang="ts">
	import { tr } from '$lib/i18n/index';
	import { createWorldPreview } from '$lib/wasm/bridge';
	import type { WorldPreset, WorldPreviewData } from '$lib/wasm/types';
	import WorldPreview from './WorldPreview.svelte';

	let {
		onStart,
		onBack,
	}: { onStart: (config: object) => void; onBack: () => void } = $props();

	// ── Basic settings ──────────────────────────────────────────────────────
	let corpName = $state('Player Corp');
	let mapSize = $state('Small');
	let era = $state('Internet');
	let difficulty = $state('Normal');
	let aiCount = $state(4);
	let seed = $state(Math.floor(Math.random() * 999999));

	// ── Procgen settings ────────────────────────────────────────────────────
	let preset = $state<WorldPreset>('continents');
	let continentCount = $state(4);
	let oceanPercentage = $state(70);
	let terrainRoughness = $state(50);
	let climateVariation = $state(50);
	let cityDensity = $state(50);

	// ── UI state ────────────────────────────────────────────────────────────
	let advancedOpen = $state(false);
	let previewData: WorldPreviewData | null = $state(null);
	let previewLoading = $state(false);

	let isRealEarth = $derived(preset === 'real_earth');

	// ── Preset definitions ──────────────────────────────────────────────────
	const PRESETS: Record<
		WorldPreset,
		{
			labelKey: string;
			descKey: string;
			continents: number;
			ocean: number;
			roughness: number;
			climate: number;
			density: number;
		}
	> = {
		real_earth: {
			labelKey: 'menu.preset_real_earth',
			descKey: 'menu.preset_desc_real_earth',
			continents: 6,
			ocean: 71,
			roughness: 50,
			climate: 50,
			density: 50,
		},
		pangaea: {
			labelKey: 'menu.preset_pangaea',
			descKey: 'menu.preset_desc_pangaea',
			continents: 1,
			ocean: 40,
			roughness: 60,
			climate: 40,
			density: 40,
		},
		archipelago: {
			labelKey: 'menu.preset_archipelago',
			descKey: 'menu.preset_desc_archipelago',
			continents: 8,
			ocean: 80,
			roughness: 70,
			climate: 60,
			density: 30,
		},
		continents: {
			labelKey: 'menu.preset_continents',
			descKey: 'menu.preset_desc_continents',
			continents: 4,
			ocean: 65,
			roughness: 50,
			climate: 50,
			density: 50,
		},
		random: {
			labelKey: 'menu.preset_random',
			descKey: 'menu.preset_desc_random',
			continents: 4,
			ocean: 65,
			roughness: 50,
			climate: 50,
			density: 50,
		},
	};

	function applyPreset(p: WorldPreset) {
		preset = p;
		if (p === 'random') {
			continentCount = Math.floor(Math.random() * 8) + 1;
			oceanPercentage = Math.floor(Math.random() * 61) + 30;
			terrainRoughness = Math.floor(Math.random() * 101);
			climateVariation = Math.floor(Math.random() * 101);
			cityDensity = Math.floor(Math.random() * 101);
			seed = Math.floor(Math.random() * 999999);
		} else {
			const def = PRESETS[p];
			continentCount = def.continents;
			oceanPercentage = def.ocean;
			terrainRoughness = def.roughness;
			climateVariation = def.climate;
			cityDensity = def.density;
		}
		previewData = null;
	}

	function randomizeSeed() {
		seed = Math.floor(Math.random() * 999999);
		previewData = null;
	}

	function buildConfig(): object {
		return {
			seed,
			starting_era: era,
			difficulty,
			map_size: mapSize,
			ai_corporations: aiCount,
			use_real_earth: isRealEarth,
			corp_name: corpName || 'Player Corp',
			continent_count: continentCount,
			ocean_percentage: oceanPercentage / 100,
			terrain_roughness: terrainRoughness / 100,
			climate_variation: climateVariation / 100,
			city_density: cityDensity / 100,
		};
	}

	function handleStart() {
		onStart(buildConfig());
	}

	function handlePreview() {
		previewLoading = true;
		const config = buildConfig();
		const result = createWorldPreview(config);
		previewData = result;
		previewLoading = false;
	}

	function handleGenerateNew() {
		randomizeSeed();
		handlePreview();
	}
</script>

<div class="new-game">
	<div class="layout">
		<!-- LEFT COLUMN: Configuration -->
		<div class="config-panel">
			<h2>{$tr('menu.new_game')}</h2>

			<!-- Preset buttons -->
			<div class="section-label">{$tr('menu.world_preset')}</div>
			<div class="preset-grid">
				{#each Object.entries(PRESETS) as [key, def]}
					<button
						class="preset-btn"
						class:active={preset === key}
						onclick={() => applyPreset(key as WorldPreset)}
						title={$tr(def.descKey)}
					>
						<span class="preset-icon">
							{#if key === 'real_earth'}
								&#127758;
							{:else if key === 'pangaea'}
								&#127757;
							{:else if key === 'archipelago'}
								&#127965;
							{:else if key === 'continents'}
								&#127759;
							{:else}
								&#127922;
							{/if}
						</span>
						<span class="preset-label">{$tr(def.labelKey)}</span>
					</button>
				{/each}
			</div>
			<div class="preset-desc">{$tr(PRESETS[preset].descKey)}</div>

			<!-- Basic settings -->
			<div class="form-group">
				<label for="corp-name">{$tr('menu.corporation_name')}</label>
				<input id="corp-name" type="text" bind:value={corpName} />
			</div>

			<div class="form-row">
				<div class="form-group half">
					<label for="era">{$tr('menu.starting_era')}</label>
					<select id="era" bind:value={era}>
						<option value="Telegraph">{$tr('menu.era_telegraph')}</option>
						<option value="Telephone">{$tr('menu.era_telephone')}</option>
						<option value="EarlyDigital">{$tr('menu.era_early_digital')}</option>
						<option value="Internet">{$tr('menu.era_internet')}</option>
						<option value="Modern">{$tr('menu.era_modern')}</option>
						<option value="NearFuture">{$tr('menu.era_near_future')}</option>
					</select>
				</div>
				<div class="form-group half">
					<label for="difficulty">{$tr('menu.difficulty')}</label>
					<select id="difficulty" bind:value={difficulty}>
						<option value="Easy">{$tr('menu.difficulty_easy')}</option>
						<option value="Normal">{$tr('menu.difficulty_normal')}</option>
						<option value="Hard">{$tr('menu.difficulty_hard')}</option>
						<option value="Expert">{$tr('menu.difficulty_expert')}</option>
					</select>
				</div>
			</div>

			<div class="form-row">
				<div class="form-group half">
					<label for="map-size">{$tr('menu.map_size')}</label>
					<select id="map-size" bind:value={mapSize}>
						<option value="Small">{$tr('menu.size_small')}</option>
						<option value="Medium">{$tr('menu.size_medium')}</option>
						<option value="Large">{$tr('menu.size_large')}</option>
						<option value="Huge">{$tr('menu.size_huge')}</option>
					</select>
				</div>
				<div class="form-group half">
					<label for="ai-count">{$tr('menu.ai_corporations')}</label>
					<div class="slider-row">
						<input
							id="ai-count"
							type="range"
							min="0"
							max="20"
							bind:value={aiCount}
						/>
						<span class="slider-value">{aiCount}</span>
					</div>
				</div>
			</div>

			<div class="form-group">
				<label for="seed">{$tr('menu.world_seed')}</label>
				<div class="seed-row">
					<input
						id="seed"
						type="number"
						bind:value={seed}
						class="seed-input"
					/>
					<button class="btn compact" onclick={randomizeSeed}>
						{$tr('menu.randomize_seed')}
					</button>
				</div>
			</div>

			<!-- Advanced settings (collapsible) -->
			<button
				class="advanced-toggle"
				onclick={() => (advancedOpen = !advancedOpen)}
			>
				<span class="toggle-arrow" class:open={advancedOpen}>&#9654;</span>
				{$tr('menu.advanced_settings')}
			</button>

			{#if advancedOpen}
				<div class="advanced-section">
					<div class="form-group">
						<label for="continents">
							{$tr('menu.continent_count')}: {continentCount}
						</label>
						<input
							id="continents"
							type="range"
							min="1"
							max="8"
							bind:value={continentCount}
							disabled={isRealEarth}
						/>
					</div>

					<div class="form-group">
						<label for="ocean">
							{$tr('menu.ocean_percentage')}: {oceanPercentage}%
						</label>
						<input
							id="ocean"
							type="range"
							min="30"
							max="90"
							bind:value={oceanPercentage}
							disabled={isRealEarth}
						/>
					</div>

					<div class="form-group">
						<label for="roughness">
							{$tr('menu.terrain_roughness')}: {terrainRoughness}%
						</label>
						<input
							id="roughness"
							type="range"
							min="0"
							max="100"
							bind:value={terrainRoughness}
							disabled={isRealEarth}
						/>
					</div>

					<div class="form-group">
						<label for="climate">
							{$tr('menu.climate_variation')}: {climateVariation}%
						</label>
						<input
							id="climate"
							type="range"
							min="0"
							max="100"
							bind:value={climateVariation}
							disabled={isRealEarth}
						/>
					</div>

					<div class="form-group">
						<label for="density">
							{$tr('menu.city_density')}: {cityDensity}%
						</label>
						<input
							id="density"
							type="range"
							min="0"
							max="100"
							bind:value={cityDensity}
							disabled={isRealEarth}
						/>
					</div>
				</div>
			{/if}

			<!-- Action buttons -->
			<div class="form-actions">
				<button class="btn secondary" onclick={onBack}>
					{$tr('menu.back')}
				</button>
				<button class="btn primary" onclick={handleStart}>
					{$tr('menu.start_game')}
				</button>
			</div>
		</div>

		<!-- RIGHT COLUMN: World Preview -->
		<div class="preview-panel">
			<div class="preview-header">
				<h3>{$tr('menu.world_preview')}</h3>
				<div class="preview-buttons">
					<button class="btn compact" onclick={handlePreview}>
						{$tr('menu.preview')}
					</button>
					<button class="btn compact" onclick={handleGenerateNew}>
						{$tr('menu.generate_new')}
					</button>
				</div>
			</div>

			<div class="preview-canvas-container">
				{#if previewLoading}
					<div class="preview-placeholder">
						<span class="placeholder-icon">&#8987;</span>
						<span>{$tr('common.loading')}</span>
					</div>
				{:else if previewData && previewData.cells.length > 0}
					<WorldPreview cells={previewData.cells} width={400} height={250} />
				{:else}
					<div class="preview-placeholder">
						<span class="placeholder-icon">&#127760;</span>
						<span class="placeholder-text">
							{#if previewData !== null}
								{$tr('menu.preview_unavailable')}
							{:else}
								{$tr('menu.preview_placeholder')}
							{/if}
						</span>
					</div>
				{/if}
			</div>

			{#if previewData}
				<div class="preview-stats">
					<div class="stat">
						<span class="stat-label">{$tr('menu.preview_continents')}</span>
						<span class="stat-value">{previewData.continentCount}</span>
					</div>
					<div class="stat">
						<span class="stat-label">{$tr('menu.preview_cities')}</span>
						<span class="stat-value">{previewData.cityCount}</span>
					</div>
					<div class="stat">
						<span class="stat-label">{$tr('menu.preview_regions')}</span>
						<span class="stat-value">{previewData.regionCount}</span>
					</div>
					<div class="stat">
						<span class="stat-label">{$tr('menu.preview_ocean')}</span>
						<span class="stat-value">{previewData.oceanPercent}%</span>
					</div>
				</div>
			{/if}

			<!-- Config summary card -->
			<div class="config-summary">
				<div class="summary-row">
					<span class="summary-label">{$tr('menu.world_preset')}</span>
					<span class="summary-value">{$tr(PRESETS[preset].labelKey)}</span>
				</div>
				<div class="summary-row">
					<span class="summary-label">{$tr('menu.starting_era')}</span>
					<span class="summary-value">{era}</span>
				</div>
				<div class="summary-row">
					<span class="summary-label">{$tr('menu.difficulty')}</span>
					<span class="summary-value">{difficulty}</span>
				</div>
				<div class="summary-row">
					<span class="summary-label">{$tr('menu.map_size')}</span>
					<span class="summary-value">{mapSize}</span>
				</div>
				<div class="summary-row">
					<span class="summary-label">{$tr('menu.ai_corporations')}</span>
					<span class="summary-value">{aiCount}</span>
				</div>
				<div class="summary-row">
					<span class="summary-label">{$tr('menu.world_seed')}</span>
					<span class="summary-value mono">{seed}</span>
				</div>
			</div>
		</div>
	</div>
</div>

<style>
	.new-game {
		width: 100vw;
		height: 100vh;
		display: flex;
		align-items: center;
		justify-content: center;
		background: #0f1724;
		font-family: system-ui, sans-serif;
		color: #f3f4f6;
		overflow: auto;
	}

	.layout {
		display: flex;
		gap: 24px;
		max-width: 920px;
		width: 100%;
		padding: 24px;
		align-items: flex-start;
	}

	/* ── Left column: config ────────────────────────────────────────────── */

	.config-panel {
		flex: 1;
		min-width: 0;
		background: rgba(17, 24, 39, 0.95);
		border: 1px solid rgba(55, 65, 81, 0.5);
		border-radius: 12px;
		padding: 28px;
	}

	h2 {
		margin: 0 0 20px;
		font-size: 22px;
		font-weight: 700;
	}

	.section-label {
		font-size: 12px;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		color: #6b7280;
		margin-bottom: 8px;
		font-weight: 600;
	}

	/* Preset buttons */

	.preset-grid {
		display: flex;
		gap: 6px;
		flex-wrap: wrap;
		margin-bottom: 4px;
	}

	.preset-btn {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 2px;
		padding: 8px 10px;
		background: rgba(31, 41, 55, 0.6);
		border: 1px solid rgba(55, 65, 81, 0.5);
		border-radius: 8px;
		color: #9ca3af;
		cursor: pointer;
		transition: all 0.15s;
		font-family: system-ui, sans-serif;
		font-size: 11px;
		min-width: 72px;
	}

	.preset-btn:hover {
		background: rgba(31, 41, 55, 0.9);
		border-color: rgba(75, 85, 99, 0.7);
		color: #d1d5db;
	}

	.preset-btn.active {
		background: rgba(16, 185, 129, 0.12);
		border-color: rgba(16, 185, 129, 0.4);
		color: #10b981;
	}

	.preset-icon {
		font-size: 20px;
		line-height: 1;
	}

	.preset-label {
		font-weight: 500;
	}

	.preset-desc {
		font-size: 12px;
		color: #6b7280;
		margin-bottom: 16px;
		min-height: 18px;
	}

	/* Form fields */

	.form-group {
		margin-bottom: 12px;
	}

	.form-row {
		display: flex;
		gap: 12px;
	}

	.form-group.half {
		flex: 1;
		min-width: 0;
	}

	label {
		display: block;
		font-size: 12px;
		color: #9ca3af;
		margin-bottom: 4px;
	}

	input[type='text'],
	input[type='number'],
	select {
		width: 100%;
		padding: 8px 10px;
		background: rgba(31, 41, 55, 0.8);
		border: 1px solid rgba(55, 65, 81, 0.5);
		border-radius: 6px;
		color: #f3f4f6;
		font-size: 13px;
		font-family: system-ui, sans-serif;
		box-sizing: border-box;
	}

	input:focus,
	select:focus {
		outline: none;
		border-color: rgba(16, 185, 129, 0.5);
	}

	input[type='range'] {
		width: 100%;
		accent-color: #10b981;
		cursor: pointer;
	}

	input[type='range']:disabled {
		opacity: 0.4;
		cursor: not-allowed;
	}

	.slider-row {
		display: flex;
		align-items: center;
		gap: 8px;
	}

	.slider-row input[type='range'] {
		flex: 1;
	}

	.slider-value {
		font-size: 13px;
		font-family: 'SF Mono', 'Fira Code', monospace;
		color: #d1d5db;
		min-width: 24px;
		text-align: right;
	}

	.seed-row {
		display: flex;
		gap: 8px;
	}

	.seed-input {
		flex: 1;
	}

	/* Advanced toggle */

	.advanced-toggle {
		display: flex;
		align-items: center;
		gap: 6px;
		width: 100%;
		padding: 8px 0;
		background: none;
		border: none;
		color: #6b7280;
		font-size: 12px;
		font-family: system-ui, sans-serif;
		cursor: pointer;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		font-weight: 600;
		transition: color 0.15s;
	}

	.advanced-toggle:hover {
		color: #9ca3af;
	}

	.toggle-arrow {
		font-size: 9px;
		transition: transform 0.15s;
		display: inline-block;
	}

	.toggle-arrow.open {
		transform: rotate(90deg);
	}

	.advanced-section {
		padding: 12px 0 4px;
		border-top: 1px solid rgba(55, 65, 81, 0.3);
	}

	/* Buttons */

	.form-actions {
		display: flex;
		gap: 10px;
		margin-top: 20px;
	}

	.btn {
		padding: 10px 16px;
		border: 1px solid rgba(55, 65, 81, 0.5);
		border-radius: 6px;
		font-size: 13px;
		font-family: system-ui, sans-serif;
		cursor: pointer;
		transition: all 0.15s;
		white-space: nowrap;
	}

	.btn.secondary {
		flex: 0;
		background: rgba(31, 41, 55, 0.8);
		color: #d1d5db;
	}

	.btn.primary {
		flex: 1;
		background: rgba(16, 185, 129, 0.2);
		border-color: rgba(16, 185, 129, 0.3);
		color: #10b981;
		font-weight: 600;
	}

	.btn.compact {
		padding: 6px 12px;
		font-size: 12px;
		background: rgba(31, 41, 55, 0.8);
		color: #d1d5db;
	}

	.btn:hover {
		filter: brightness(1.15);
	}

	.btn.primary:hover {
		background: rgba(16, 185, 129, 0.3);
	}

	/* ── Right column: preview ──────────────────────────────────────────── */

	.preview-panel {
		width: 420px;
		flex-shrink: 0;
		background: rgba(17, 24, 39, 0.95);
		border: 1px solid rgba(55, 65, 81, 0.5);
		border-radius: 12px;
		padding: 20px;
	}

	.preview-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 12px;
	}

	.preview-header h3 {
		margin: 0;
		font-size: 15px;
		font-weight: 600;
		color: #d1d5db;
	}

	.preview-buttons {
		display: flex;
		gap: 6px;
	}

	.preview-canvas-container {
		margin-bottom: 12px;
		border-radius: 6px;
		overflow: hidden;
		min-height: 250px;
		display: flex;
		align-items: center;
		justify-content: center;
		background: #0a1020;
	}

	.preview-placeholder {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 8px;
		padding: 40px 20px;
		color: #4b5563;
		text-align: center;
		width: 100%;
	}

	.placeholder-icon {
		font-size: 40px;
		opacity: 0.4;
	}

	.placeholder-text {
		font-size: 12px;
		max-width: 240px;
		line-height: 1.4;
	}

	.preview-stats {
		display: flex;
		gap: 8px;
		margin-bottom: 16px;
	}

	.stat {
		flex: 1;
		text-align: center;
		padding: 8px 4px;
		background: rgba(31, 41, 55, 0.5);
		border-radius: 6px;
		border: 1px solid rgba(55, 65, 81, 0.3);
	}

	.stat-label {
		display: block;
		font-size: 10px;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		color: #6b7280;
		margin-bottom: 2px;
	}

	.stat-value {
		display: block;
		font-size: 16px;
		font-weight: 700;
		color: #d1d5db;
		font-family: 'SF Mono', 'Fira Code', monospace;
	}

	/* Config summary */

	.config-summary {
		border-top: 1px solid rgba(55, 65, 81, 0.3);
		padding-top: 12px;
	}

	.summary-row {
		display: flex;
		justify-content: space-between;
		padding: 4px 0;
		font-size: 12px;
	}

	.summary-label {
		color: #6b7280;
	}

	.summary-value {
		color: #d1d5db;
		font-weight: 500;
	}

	.summary-value.mono {
		font-family: 'SF Mono', 'Fira Code', monospace;
	}

	/* ── Responsive ─────────────────────────────────────────────────────── */

	@media (max-width: 860px) {
		.layout {
			flex-direction: column;
			align-items: stretch;
		}

		.preview-panel {
			width: auto;
		}
	}
</style>
