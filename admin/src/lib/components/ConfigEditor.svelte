<script lang="ts">
	interface Props {
		value: Record<string, unknown>;
		onchange: (value: Record<string, unknown>) => void;
		label?: string;
	}
	let { value, onchange, label = 'Configuration' }: Props = $props();

	let rawMode = $state(false);
	let rawJson = $state("");
	let parseError = $state('');

	import { onMount } from 'svelte';
	onMount(() => {
		rawJson = JSON.stringify($state.snapshot(value), null, 2);
	});

	$effect(() => {
		// Update rawJson when value changes from outside (or via form fields)
		// We only want to do this if we are NOT currently editing the raw JSON to avoid overwriting work
		// But detecting "currently editing" is hard.
		// For now, simple sync.
		rawJson = JSON.stringify(value, null, 2);
	});

	const knownFields: { key: string; label: string; type: 'string' | 'number' | 'boolean' | 'select'; options?: string[] }[] = [
		{ key: 'starting_era', label: 'Starting Era', type: 'select', options: ['Telegraph', 'Telephone', 'EarlyDigital', 'Internet', 'Modern', 'NearFuture'] },
		{ key: 'difficulty', label: 'Difficulty', type: 'select', options: ['Easy', 'Normal', 'Hard', 'Expert'] },
		{ key: 'map_size', label: 'Map Size', type: 'select', options: ['Small', 'Medium', 'Large', 'Huge'] },
		{ key: 'seed', label: 'Seed', type: 'number' },
		{ key: 'ai_corporations', label: 'AI Corporations', type: 'number' },
		{ key: 'max_ai_corporations', label: 'Max AI Corps', type: 'number' },
		{ key: 'continent_count', label: 'Continent Count', type: 'number' },
		{ key: 'ocean_percentage', label: 'Ocean %', type: 'number' },
		{ key: 'terrain_roughness', label: 'Terrain Roughness', type: 'number' },
		{ key: 'climate_variation', label: 'Climate Variation', type: 'number' },
		{ key: 'city_density', label: 'City Density', type: 'number' },
		{ key: 'disaster_frequency', label: 'Disaster Frequency', type: 'number' },
		{ key: 'use_real_earth', label: 'Real Earth Map', type: 'boolean' },
		{ key: 'sandbox', label: 'Sandbox Mode', type: 'boolean' },
	];

	function updateField(key: string, val: unknown) {
		const updated = { ...value, [key]: val };
		if (val === '' || val === undefined) delete updated[key];
		onchange(updated);
		rawJson = JSON.stringify(updated, null, 2);
	}

	function applyRaw() {
		try {
			const parsed = JSON.parse(rawJson);
			onchange(parsed);
			parseError = '';
		} catch (e) {
			parseError = String(e);
		}
	}
</script>

<div class="config-editor">
	<div class="config-header">
		<span class="config-label">{label}</span>
		<button class="mode-toggle" onclick={() => (rawMode = !rawMode)}>
			{rawMode ? 'Form View' : 'JSON View'}
		</button>
	</div>

	{#if rawMode}
		<textarea class="raw-json" bind:value={rawJson} rows="12" oninput={() => parseError = ''}></textarea>
		{#if parseError}
			<p class="parse-error">{parseError}</p>
		{/if}
		<button class="apply-btn" onclick={applyRaw}>Apply JSON</button>
	{:else}
		<div class="fields">
			{#each knownFields as field}
				<div class="field-row">
					<label class="field-label" for="cfg-{field.key}">{field.label}</label>
					{#if field.type === 'select' && field.options}
						<select
							id="cfg-{field.key}"
							value={value[field.key] ?? ''}
							onchange={(e) => updateField(field.key, (e.target as HTMLSelectElement).value || undefined)}
						>
							<option value="">-- Default --</option>
							{#each field.options as opt}
								<option value={opt}>{opt}</option>
							{/each}
						</select>
					{:else if field.type === 'number'}
						<input
							id="cfg-{field.key}"
							type="number"
							step="any"
							value={value[field.key] ?? ''}
							onchange={(e) => {
								const v = (e.target as HTMLInputElement).value;
								updateField(field.key, v ? Number(v) : undefined);
							}}
						/>
					{:else if field.type === 'boolean'}
						<input
							id="cfg-{field.key}"
							type="checkbox"
							checked={!!value[field.key]}
							onchange={(e) => updateField(field.key, (e.target as HTMLInputElement).checked)}
						/>
					{:else}
						<input
							id="cfg-{field.key}"
							type="text"
							value={value[field.key] ?? ''}
							onchange={(e) => updateField(field.key, (e.target as HTMLInputElement).value || undefined)}
						/>
					{/if}
				</div>
			{/each}
		</div>
	{/if}
</div>

<style>
	.config-editor {
		background: var(--bg-surface);
		border: 1px solid var(--border);
		border-radius: var(--radius-md);
		padding: 12px;
	}
	.config-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 10px;
	}
	.config-label {
		font-size: 12px;
		font-weight: 600;
		text-transform: uppercase;
		color: var(--text-muted);
	}
	.mode-toggle {
		font-size: 11px;
		background: var(--bg-panel);
		border: 1px solid var(--border);
		border-radius: var(--radius-sm);
		color: var(--text-muted);
		padding: 2px 8px;
		cursor: pointer;
	}
	.fields {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 8px;
	}
	.field-row {
		display: flex;
		flex-direction: column;
		gap: 2px;
	}
	.field-label {
		font-size: 11px;
		color: var(--text-dim);
	}
	input, select {
		padding: 4px 8px;
		background: var(--bg-panel);
		border: 1px solid var(--border);
		border-radius: var(--radius-sm);
		color: var(--text-primary);
		font-size: 13px;
		font-family: inherit;
	}
	input[type="checkbox"] {
		width: 16px;
		height: 16px;
	}
	.raw-json {
		width: 100%;
		padding: 8px;
		background: var(--bg-panel);
		border: 1px solid var(--border);
		border-radius: var(--radius-sm);
		color: var(--text-primary);
		font-family: var(--font-mono);
		font-size: 12px;
		resize: vertical;
	}
	.parse-error {
		font-size: 11px;
		color: var(--red);
		margin-top: 4px;
	}
	.apply-btn {
		margin-top: 8px;
		padding: 4px 12px;
		background: var(--blue);
		border: none;
		border-radius: var(--radius-sm);
		color: white;
		font-size: 12px;
		cursor: pointer;
	}
</style>
