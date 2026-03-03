<script lang="ts">
	import type { WorldTemplate, WorldConfig } from '$lib/multiplayer/lobbyApi';
	import { createWorldFromTemplate, createWorldDirect } from '$lib/multiplayer/lobbyApi';

	let {
		templates,
		onCreated
	}: {
		templates: WorldTemplate[];
		onCreated: (worldId: string) => void;
	} = $props();

	type Mode = 'grid' | 'template' | 'custom';
	type WorldPreset = 'real_earth' | 'pangaea' | 'archipelago' | 'continents' | 'random';

	let mode = $state<Mode>('grid');
	let selectedTemplate = $state<WorldTemplate | null>(null);
	let worldName = $state('');
	let maxPlayers = $state(8);
	let overrides = $state<Record<string, unknown>>({});
	let creating = $state(false);
	let error = $state('');

	// Custom world config
	let preset = $state<WorldPreset>('continents');
	let seed = $state(Math.floor(Math.random() * 999999));
	let era = $state('Internet');
	let difficulty = $state('Normal');
	let mapSize = $state('Medium');
	let aiCorps = $state(4);
	let useRealEarth = $state(false);
	let continentCount = $state(4);
	let oceanPct = $state(65);
	let roughness = $state(50);
	let climate = $state(50);
	let density = $state(50);
	let disasterSeverity = $state(5);
	let sandbox = $state(false);
	let showAdvanced = $state(false);

	const ERAS = ['Telegraph', 'Telephone', 'EarlyDigital', 'Internet', 'Modern', 'NearFuture'];
	const DIFFICULTIES = ['Easy', 'Normal', 'Hard', 'Expert'];
	const MAP_SIZES = ['Tiny', 'Small', 'Medium', 'Large', 'Huge'];
	const DIFFICULTY_DISASTER: Record<string, number> = { Easy: 3, Normal: 5, Hard: 7, Expert: 9 };

	const PRESETS: Record<WorldPreset, { label: string; continents: number; ocean: number; roughness: number; climate: number; density: number }> = {
		real_earth: { label: 'Real Earth', continents: 6, ocean: 71, roughness: 50, climate: 50, density: 50 },
		pangaea: { label: 'Pangaea', continents: 1, ocean: 40, roughness: 60, climate: 40, density: 40 },
		archipelago: { label: 'Archipelago', continents: 8, ocean: 80, roughness: 70, climate: 60, density: 30 },
		continents: { label: 'Continents', continents: 4, ocean: 65, roughness: 50, climate: 50, density: 50 },
		random: { label: 'Random', continents: 4, ocean: 65, roughness: 50, climate: 50, density: 50 }
	};

	let disasterFrequency = $derived(
		+(0.1 * Math.pow(10, (disasterSeverity - 1) / 9 * Math.log10(30))).toFixed(2)
	);

	function selectTemplate(t: WorldTemplate) {
		selectedTemplate = t;
		mode = 'template';
		worldName = '';
		overrides = {};
		error = '';
		const defaults = t.config_defaults as Record<string, unknown>;
		for (const key of Object.keys(t.config_bounds)) {
			if (key in defaults) {
				overrides[key] = defaults[key];
			}
		}
		const bounds = t.config_bounds as Record<string, { min?: number; max?: number }>;
		if (bounds.max_players) {
			maxPlayers = (defaults.max_players as number) || 8;
		}
	}

	function selectCustom() {
		mode = 'custom';
		worldName = '';
		error = '';
		applyPreset('continents');
	}

	function backToGrid() {
		mode = 'grid';
		selectedTemplate = null;
		error = '';
	}

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

	$effect(() => {
		disasterSeverity = DIFFICULTY_DISASTER[difficulty] ?? 5;
	});

	async function handleCreateFromTemplate() {
		if (!selectedTemplate || !worldName.trim()) return;
		creating = true;
		error = '';
		try {
			const result = await createWorldFromTemplate(
				selectedTemplate.id,
				worldName.trim(),
				maxPlayers,
				overrides
			);
			onCreated(result.world_id);
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to create world';
		} finally {
			creating = false;
		}
	}

	async function handleCreateCustom() {
		if (!worldName.trim()) return;
		creating = true;
		error = '';
		try {
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
				disaster_frequency: disasterFrequency,
				sandbox
			};
			const result = await createWorldDirect(worldName.trim(), maxPlayers, config);
			onCreated(result.world_id);
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to create world';
		} finally {
			creating = false;
		}
	}
</script>

<div class="creator-container">
	{#if mode === 'grid'}
		<div class="template-grid">
			{#each templates as template}
				<button
					class="template-card"
					onclick={() => selectTemplate(template)}
					disabled={template.current_instances >= template.max_instances}
				>
					<div class="template-icon">{template.icon}</div>
					<div class="template-name">{template.name}</div>
					<div class="template-desc">{template.description}</div>
					<div class="template-instances">
						{template.current_instances}/{template.max_instances} active
					</div>
				</button>
			{/each}

			<!-- Custom World card — always shown -->
			<button class="template-card custom-card" onclick={selectCustom}>
				<div class="template-icon">+</div>
				<div class="template-name">Custom World</div>
				<div class="template-desc">Create a world with your own settings</div>
			</button>
		</div>

	{:else if mode === 'template' && selectedTemplate}
		<div class="customize-form">
			<button class="btn-back-template" onclick={backToGrid}>
				Back to Templates
			</button>

			<h3>Create: {selectedTemplate.name}</h3>
			<p class="template-description">{selectedTemplate.description}</p>

			<div class="form-field">
				<label for="world-name">World Name</label>
				<input
					id="world-name"
					type="text"
					bind:value={worldName}
					placeholder="Enter world name..."
					maxlength={128}
				/>
			</div>

			{#each Object.entries(selectedTemplate.config_bounds) as [key, rawBounds]}
				{@const bounds = rawBounds as { min?: number; max?: number; allowed?: string[] }}
				<div class="form-field">
					<label for="config-{key}">{key.replace(/_/g, ' ')}</label>
					{#if bounds.allowed}
						<select id="config-{key}" bind:value={overrides[key]}>
							{#each bounds.allowed as opt}
								<option value={opt}>{opt}</option>
							{/each}
						</select>
					{:else if bounds.min !== undefined && bounds.max !== undefined}
						<div class="slider-row">
							<input
								id="config-{key}"
								type="range"
								min={bounds.min}
								max={bounds.max}
								step={Number.isInteger(bounds.min) && Number.isInteger(bounds.max) ? 1 : 0.1}
								bind:value={overrides[key]}
							/>
							<span class="slider-value">{overrides[key] ?? bounds.min}</span>
						</div>
					{/if}
				</div>
			{/each}

			{#if error}
				<div class="error">{error}</div>
			{/if}

			<button
				class="btn-create"
				onclick={handleCreateFromTemplate}
				disabled={creating || !worldName.trim()}
			>
				{creating ? 'Creating...' : 'Create World'}
			</button>
		</div>

	{:else if mode === 'custom'}
		<div class="customize-form">
			<button class="btn-back-template" onclick={backToGrid}>
				Back
			</button>

			<h3>Custom World</h3>

			<div class="form-field">
				<label for="cw-name">World Name</label>
				<input id="cw-name" type="text" bind:value={worldName} placeholder="Enter world name..." maxlength={128} />
			</div>

			<div class="form-field">
				<label for="cw-max">Max Players</label>
				<input id="cw-max" type="number" bind:value={maxPlayers} min="1" max="100" />
			</div>

			<!-- Presets -->
			<div class="form-field">
				<span class="field-label">Map Preset</span>
				<div class="preset-row">
					{#each Object.entries(PRESETS) as [key, def]}
						<button
							class="preset-btn"
							class:active={preset === key}
							onclick={() => applyPreset(key as WorldPreset)}
						>
							{def.label}
						</button>
					{/each}
				</div>
			</div>

			<!-- Core -->
			<div class="form-row">
				<div class="form-field half">
					<label for="cw-era">Era</label>
					<select id="cw-era" bind:value={era}>
						{#each ERAS as e}<option value={e}>{e}</option>{/each}
					</select>
				</div>
				<div class="form-field half">
					<label for="cw-diff">Difficulty</label>
					<select id="cw-diff" bind:value={difficulty}>
						{#each DIFFICULTIES as d}<option value={d}>{d}</option>{/each}
					</select>
				</div>
			</div>

			<div class="form-row">
				<div class="form-field half">
					<label for="cw-size">Map Size</label>
					<select id="cw-size" bind:value={mapSize}>
						{#each MAP_SIZES as m}<option value={m}>{m}</option>{/each}
					</select>
				</div>
				<div class="form-field half">
					<label for="cw-ai">AI Corporations: {aiCorps}</label>
					<input id="cw-ai" type="range" min="0" max="20" bind:value={aiCorps} />
				</div>
			</div>

			<div class="form-field">
				<label for="cw-disaster">Disaster Severity: {disasterSeverity}</label>
				<input id="cw-disaster" type="range" min="1" max="10" bind:value={disasterSeverity} />
			</div>


			<div class="form-field">
				<label class="checkbox-label">
					<input type="checkbox" bind:checked={sandbox} /> Sandbox Mode
				</label>
			</div>

			<!-- Seed -->
			<div class="form-field">
				<label for="cw-seed">Seed</label>
				<div class="seed-row">
					<input id="cw-seed" type="number" bind:value={seed} min="0" />
					<button class="preset-btn" onclick={() => (seed = Math.floor(Math.random() * 999999))}>Dice</button>
				</div>
			</div>

			<!-- Advanced terrain sliders -->
			<button class="btn-toggle-advanced" onclick={() => (showAdvanced = !showAdvanced)}>
				{showAdvanced ? 'Hide Advanced' : 'Show Advanced Terrain'}
			</button>

			{#if showAdvanced}
				<div class="advanced-section">
					{#if useRealEarth}
						<div class="real-earth-notice">Using real-world geography — terrain sliders locked.</div>
					{/if}
					<div class="form-field">
						<label for="cw-continents">Continents: {continentCount}</label>
						<input id="cw-continents" type="range" min="1" max="8" bind:value={continentCount} disabled={useRealEarth} />
					</div>
					<div class="form-field">
						<label for="cw-ocean">Ocean: {oceanPct}%</label>
						<input id="cw-ocean" type="range" min="30" max="90" bind:value={oceanPct} disabled={useRealEarth} />
					</div>
					<div class="form-field">
						<label for="cw-roughness">Roughness: {roughness}%</label>
						<input id="cw-roughness" type="range" min="0" max="100" bind:value={roughness} disabled={useRealEarth} />
					</div>
					<div class="form-field">
						<label for="cw-climate">Climate: {climate}%</label>
						<input id="cw-climate" type="range" min="0" max="100" bind:value={climate} disabled={useRealEarth} />
					</div>
					<div class="form-field">
						<label for="cw-density">City Density: {density}%</label>
						<input id="cw-density" type="range" min="0" max="100" bind:value={density} disabled={useRealEarth} />
					</div>
				</div>
			{/if}

			{#if error}
				<div class="error">{error}</div>
			{/if}

			<button
				class="btn-create"
				onclick={handleCreateCustom}
				disabled={creating || !worldName.trim()}
			>
				{creating ? 'Creating...' : 'Create World'}
			</button>
		</div>
	{/if}
</div>

<style>
	.creator-container {
		flex: 1;
		overflow-y: auto;
	}

	.template-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
		gap: 12px;
	}

	.template-card {
		background: rgba(31, 41, 55, 0.6);
		border: 1px solid rgba(55, 65, 81, 0.3);
		border-radius: 8px;
		padding: 16px;
		text-align: left;
		cursor: pointer;
		color: #d1d5db;
		font-family: system-ui, sans-serif;
		transition: border-color 0.15s;
	}

	.template-card:hover:not(:disabled) {
		border-color: rgba(59, 130, 246, 0.5);
	}

	.template-card:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.custom-card {
		border-style: dashed;
	}

	.custom-card .template-icon {
		font-size: 28px;
		color: #60a5fa;
	}

	.template-icon {
		font-size: 24px;
		margin-bottom: 8px;
	}

	.template-name {
		font-size: 15px;
		font-weight: 600;
		margin-bottom: 4px;
	}

	.template-desc {
		font-size: 12px;
		color: #9ca3af;
		line-height: 1.4;
		margin-bottom: 8px;
	}

	.template-instances {
		font-size: 11px;
		color: #6b7280;
	}

	.customize-form {
		max-width: 480px;
	}

	.btn-back-template {
		background: rgba(31, 41, 55, 0.8);
		border: 1px solid rgba(55, 65, 81, 0.5);
		color: #9ca3af;
		padding: 6px 14px;
		border-radius: 6px;
		cursor: pointer;
		font-size: 13px;
		font-family: system-ui, sans-serif;
		margin-bottom: 16px;
	}

	.btn-back-template:hover {
		color: #d1d5db;
	}

	.customize-form h3 {
		font-size: 18px;
		margin: 0 0 4px;
	}

	.template-description {
		font-size: 13px;
		color: #9ca3af;
		margin-bottom: 20px;
	}

	.form-field {
		margin-bottom: 14px;
	}

	.form-field label {
		display: block;
		font-size: 12px;
		color: #6b7280;
		text-transform: capitalize;
		margin-bottom: 4px;
	}

	.form-field input[type='text'],
	.form-field input[type='number'] {
		width: 100%;
		padding: 8px 12px;
		background: rgba(31, 41, 55, 0.8);
		border: 1px solid rgba(55, 65, 81, 0.5);
		border-radius: 6px;
		color: #f3f4f6;
		font-size: 14px;
		font-family: system-ui, sans-serif;
		box-sizing: border-box;
	}

	.form-field input[type='text']:focus,
	.form-field input[type='number']:focus {
		outline: none;
		border-color: #10b981;
	}

	.form-field select {
		width: 100%;
		padding: 8px 12px;
		background: rgba(31, 41, 55, 0.8);
		border: 1px solid rgba(55, 65, 81, 0.5);
		border-radius: 6px;
		color: #d1d5db;
		font-size: 13px;
		font-family: system-ui, sans-serif;
	}

	.form-field input[type='range'] {
		width: 100%;
		accent-color: #10b981;
	}

	.form-field input[type='range']:disabled {
		opacity: 0.4;
	}

	.form-row {
		display: flex;
		gap: 12px;
	}

	.half {
		flex: 1;
	}

	.slider-row {
		display: flex;
		align-items: center;
		gap: 12px;
	}

	.slider-row input[type='range'] {
		flex: 1;
		accent-color: #10b981;
	}

	.slider-value {
		font-size: 13px;
		color: #d1d5db;
		min-width: 40px;
		text-align: right;
		font-family: monospace;
	}

	.preset-row {
		display: flex;
		gap: 6px;
		flex-wrap: wrap;
	}

	.preset-btn {
		padding: 5px 12px;
		font-size: 12px;
		background: rgba(31, 41, 55, 0.8);
		border: 1px solid rgba(55, 65, 81, 0.5);
		border-radius: 6px;
		color: #9ca3af;
		cursor: pointer;
		font-family: system-ui, sans-serif;
	}

	.preset-btn:hover {
		color: #d1d5db;
		border-color: rgba(59, 130, 246, 0.4);
	}

	.preset-btn.active {
		background: rgba(59, 130, 246, 0.2);
		border-color: rgba(59, 130, 246, 0.5);
		color: #60a5fa;
	}

	.checkbox-label {
		display: flex;
		align-items: center;
		gap: 8px;
		cursor: pointer;
	}

	.seed-row {
		display: flex;
		gap: 8px;
	}

	.seed-row input {
		flex: 1;
	}

	.btn-toggle-advanced {
		background: none;
		border: none;
		color: #6b7280;
		font-size: 12px;
		cursor: pointer;
		padding: 4px 0;
		font-family: system-ui, sans-serif;
		text-decoration: underline;
		margin-bottom: 8px;
	}

	.btn-toggle-advanced:hover {
		color: #9ca3af;
	}

	.advanced-section {
		background: rgba(31, 41, 55, 0.4);
		border: 1px solid rgba(55, 65, 81, 0.3);
		border-radius: 6px;
		padding: 12px;
		margin-bottom: 8px;
	}

	.real-earth-notice {
		font-size: 12px;
		color: #f59e0b;
		background: rgba(245, 158, 11, 0.08);
		border: 1px solid rgba(245, 158, 11, 0.2);
		border-radius: 6px;
		padding: 6px 10px;
		margin-bottom: 10px;
	}

	.error {
		color: #ef4444;
		font-size: 13px;
		margin-bottom: 12px;
		padding: 8px 12px;
		background: rgba(239, 68, 68, 0.1);
		border: 1px solid rgba(239, 68, 68, 0.2);
		border-radius: 6px;
	}

	.btn-create {
		width: 100%;
		background: rgba(16, 185, 129, 0.2);
		border: 1px solid rgba(16, 185, 129, 0.3);
		color: #10b981;
		padding: 12px;
		border-radius: 6px;
		font-size: 14px;
		cursor: pointer;
		font-family: system-ui, sans-serif;
		margin-top: 8px;
	}

	.btn-create:hover:not(:disabled) {
		background: rgba(16, 185, 129, 0.3);
	}

	.btn-create:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

</style>
