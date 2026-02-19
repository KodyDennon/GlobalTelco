<script lang="ts">
	import { tr } from "$lib/i18n/index";

	let {
		onStart,
		onBack,
	}: { onStart: (config: object) => void; onBack: () => void } = $props();

	let corpName = $state("Player Corp");
	let mapSize = $state("Small");
	let era = $state("Internet");
	let difficulty = $state("Normal");
	let aiCount = $state(4);
	let seed = $state(Math.floor(Math.random() * 999999));
	let realEarth = $state(false);

	function handleStart() {
		onStart({
			seed,
			starting_era: era,
			difficulty,
			map_size: mapSize,
			ai_corporations: aiCount,
			use_real_earth: realEarth,
			corp_name: corpName || "Player Corp",
		});
	}
</script>

<div class="new-game">
	<div class="form-container">
		<h2>{$tr("menu.new_game")}</h2>

		<div class="form-group">
			<label for="corp-name">{$tr("menu.corporation_name")}</label>
			<input id="corp-name" type="text" bind:value={corpName} />
		</div>

		<div class="form-group">
			<label for="map-size">{$tr("menu.map_size")}</label>
			<select id="map-size" bind:value={mapSize}>
				<option value="Small">{$tr("menu.size_small")}</option>
				<option value="Medium">{$tr("menu.size_medium")}</option>
				<option value="Large">{$tr("menu.size_large")}</option>
				<option value="Huge">{$tr("menu.size_huge")}</option>
			</select>
		</div>

		<div class="form-group">
			<label for="era">{$tr("menu.starting_era")}</label>
			<select id="era" bind:value={era}>
				<option value="Telegraph">{$tr("menu.era_telegraph")}</option>
				<option value="Telephone">{$tr("menu.era_telephone")}</option>
				<option value="EarlyDigital"
					>{$tr("menu.era_early_digital")}</option
				>
				<option value="Internet">{$tr("menu.era_internet")}</option>
				<option value="Modern">{$tr("menu.era_modern")}</option>
				<option value="NearFuture">{$tr("menu.era_near_future")}</option
				>
			</select>
		</div>

		<div class="form-group">
			<label for="difficulty">{$tr("menu.difficulty")}</label>
			<select id="difficulty" bind:value={difficulty}>
				<option value="Easy">{$tr("menu.difficulty_easy")}</option>
				<option value="Normal">{$tr("menu.difficulty_normal")}</option>
				<option value="Hard">{$tr("menu.difficulty_hard")}</option>
				<option value="Expert">{$tr("menu.difficulty_expert")}</option>
			</select>
		</div>

		<div class="form-group">
			<label for="ai-count">{$tr("menu.ai_corporations")}</label>
			<input
				id="ai-count"
				type="number"
				min="0"
				max="8"
				bind:value={aiCount}
			/>
		</div>

		<div class="form-group">
			<label for="seed">{$tr("menu.world_seed")}</label>
			<input id="seed" type="number" bind:value={seed} />
		</div>

		<div class="form-group flex-row">
			<label for="real-earth">{$tr("menu.use_real_earth")}</label>
			<input id="real-earth" type="checkbox" bind:checked={realEarth} />
		</div>

		<div class="form-actions">
			<button class="btn secondary" onclick={onBack}
				>{$tr("menu.back")}</button
			>
			<button class="btn primary" onclick={handleStart}
				>{$tr("menu.start_game")}</button
			>
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
		background: #06101f;
		font-family: system-ui, sans-serif;
		color: #f3f4f6;
	}

	.form-container {
		width: 400px;
		background: rgba(17, 24, 39, 0.95);
		border: 1px solid rgba(55, 65, 81, 0.5);
		border-radius: 12px;
		padding: 32px;
	}

	h2 {
		margin: 0 0 24px;
		font-size: 24px;
		font-weight: 700;
	}

	.form-group {
		margin-bottom: 16px;
	}

	label {
		display: block;
		font-size: 13px;
		color: #9ca3af;
		margin-bottom: 4px;
	}

	input,
	select {
		width: 100%;
		padding: 10px 12px;
		background: rgba(31, 41, 55, 0.8);
		border: 1px solid rgba(55, 65, 81, 0.5);
		border-radius: 6px;
		color: #f3f4f6;
		font-size: 14px;
		font-family: system-ui, sans-serif;
		box-sizing: border-box;
	}

	input:focus,
	select:focus {
		outline: none;
		border-color: rgba(16, 185, 129, 0.5);
	}

	.flex-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 12px;
	}

	input[type="checkbox"] {
		width: 20px;
		height: 20px;
		cursor: pointer;
	}

	.form-actions {
		display: flex;
		gap: 12px;
		margin-top: 24px;
	}

	.btn {
		flex: 1;
		padding: 12px;
		border: 1px solid rgba(55, 65, 81, 0.5);
		border-radius: 6px;
		font-size: 14px;
		font-family: system-ui, sans-serif;
		cursor: pointer;
		transition: all 0.2s;
	}

	.btn.secondary {
		background: rgba(31, 41, 55, 0.8);
		color: #d1d5db;
	}

	.btn.primary {
		background: rgba(16, 185, 129, 0.2);
		border-color: rgba(16, 185, 129, 0.3);
		color: #10b981;
	}

	.btn:hover {
		opacity: 0.9;
	}
</style>
