<script lang="ts">
	import type { WorldTemplate } from '$lib/multiplayer/lobbyApi';
	import { createWorldFromTemplate } from '$lib/multiplayer/lobbyApi';

	let {
		templates,
		onCreated
	}: {
		templates: WorldTemplate[];
		onCreated: (worldId: string) => void;
	} = $props();

	let selectedTemplate = $state<WorldTemplate | null>(null);
	let worldName = $state('');
	let maxPlayers = $state(8);
	let overrides = $state<Record<string, unknown>>({});
	let creating = $state(false);
	let error = $state('');

	function selectTemplate(t: WorldTemplate) {
		selectedTemplate = t;
		worldName = '';
		overrides = {};
		error = '';
		// Initialize overrides from defaults
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

	function getBounds(key: string): { min?: number; max?: number; allowed?: string[] } {
		if (!selectedTemplate) return {};
		const b = selectedTemplate.config_bounds as Record<string, Record<string, unknown>>;
		return (b[key] as { min?: number; max?: number; allowed?: string[] }) || {};
	}

	async function handleCreate() {
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
</script>

<div class="creator-container">
	{#if !selectedTemplate}
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
			{#if templates.length === 0}
				<div class="empty">No templates available. An admin must create world templates first.</div>
			{/if}
		</div>
	{:else}
		<div class="customize-form">
			<button class="btn-back-template" onclick={() => (selectedTemplate = null)}>
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
				onclick={handleCreate}
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

	.form-field input[type='text'] {
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

	.form-field input[type='text']:focus {
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

	.empty {
		text-align: center;
		color: #6b7280;
		padding: 40px;
		grid-column: 1 / -1;
	}
</style>
