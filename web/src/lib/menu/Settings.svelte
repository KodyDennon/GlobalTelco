<script lang="ts">
	import { musicVolume, sfxVolume, mapQuality } from '$lib/stores/settings';

	let { onBack }: { onBack: () => void } = $props();
</script>

<div class="settings">
	<div class="settings-container">
		<h2>Settings</h2>

		<div class="setting">
			<label for="music">Music Volume</label>
			<input id="music" type="range" min="0" max="1" step="0.1" bind:value={$musicVolume} />
			<span class="val">{Math.round($musicVolume * 100)}%</span>
		</div>

		<div class="setting">
			<label for="sfx">SFX Volume</label>
			<input id="sfx" type="range" min="0" max="1" step="0.1" bind:value={$sfxVolume} />
			<span class="val">{Math.round($sfxVolume * 100)}%</span>
		</div>

		<div class="setting">
			<label for="quality">Map Quality</label>
			<select id="quality" bind:value={$mapQuality}>
				<option value="low">Low</option>
				<option value="medium">Medium</option>
				<option value="high">High</option>
			</select>
		</div>

		<button class="btn" onclick={onBack}>Back</button>
	</div>
</div>

<style>
	.settings {
		width: 100vw;
		height: 100vh;
		display: flex;
		align-items: center;
		justify-content: center;
		background: #0a0e17;
		font-family: system-ui, sans-serif;
		color: #f3f4f6;
	}

	.settings-container {
		width: 400px;
		background: rgba(17, 24, 39, 0.95);
		border: 1px solid rgba(55, 65, 81, 0.5);
		border-radius: 12px;
		padding: 32px;
	}

	h2 {
		margin: 0 0 24px;
	}

	.setting {
		margin-bottom: 20px;
		display: flex;
		align-items: center;
		gap: 12px;
	}

	label {
		width: 120px;
		font-size: 13px;
		color: #9ca3af;
	}

	input[type='range'] {
		flex: 1;
		accent-color: #10b981;
	}

	select {
		flex: 1;
		padding: 8px;
		background: rgba(31, 41, 55, 0.8);
		border: 1px solid rgba(55, 65, 81, 0.5);
		border-radius: 6px;
		color: #f3f4f6;
		font-size: 14px;
	}

	.val {
		width: 40px;
		text-align: right;
		font-size: 12px;
		color: #6b7280;
		font-family: monospace;
	}

	.btn {
		margin-top: 16px;
		width: 100%;
		padding: 12px;
		background: rgba(31, 41, 55, 0.8);
		border: 1px solid rgba(55, 65, 81, 0.5);
		border-radius: 6px;
		color: #d1d5db;
		font-size: 14px;
		cursor: pointer;
	}
</style>
