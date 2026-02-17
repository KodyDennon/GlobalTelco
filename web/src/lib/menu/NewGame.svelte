<script lang="ts">
	let { onStart, onBack }: { onStart: (config: object) => void; onBack: () => void } = $props();

	let corpName = $state('Player Corp');
	let mapSize = $state('Small');
	let era = $state('Internet');
	let difficulty = $state('Normal');
	let aiCount = $state(4);
	let seed = $state(Math.floor(Math.random() * 999999));

	function handleStart() {
		onStart({
			seed,
			starting_era: era,
			difficulty,
			map_size: mapSize,
			ai_corporations: aiCount,
			use_real_earth: false
		});
	}
</script>

<div class="new-game">
	<div class="form-container">
		<h2>New Game</h2>

		<div class="form-group">
			<label for="corp-name">Corporation Name</label>
			<input id="corp-name" type="text" bind:value={corpName} />
		</div>

		<div class="form-group">
			<label for="map-size">Map Size</label>
			<select id="map-size" bind:value={mapSize}>
				<option>Small</option>
				<option>Medium</option>
				<option>Large</option>
				<option>Huge</option>
			</select>
		</div>

		<div class="form-group">
			<label for="era">Starting Era</label>
			<select id="era" bind:value={era}>
				<option>Telegraph</option>
				<option>Telephone</option>
				<option>EarlyDigital</option>
				<option>Internet</option>
				<option>Modern</option>
				<option>NearFuture</option>
			</select>
		</div>

		<div class="form-group">
			<label for="difficulty">Difficulty</label>
			<select id="difficulty" bind:value={difficulty}>
				<option>Easy</option>
				<option>Normal</option>
				<option>Hard</option>
				<option>Expert</option>
			</select>
		</div>

		<div class="form-group">
			<label for="ai-count">AI Corporations</label>
			<input id="ai-count" type="number" min="0" max="8" bind:value={aiCount} />
		</div>

		<div class="form-group">
			<label for="seed">World Seed</label>
			<input id="seed" type="number" bind:value={seed} />
		</div>

		<div class="form-actions">
			<button class="btn secondary" onclick={onBack}>Back</button>
			<button class="btn primary" onclick={handleStart}>Start Game</button>
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
		background: #0a0e17;
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

	input, select {
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

	input:focus, select:focus {
		outline: none;
		border-color: rgba(16, 185, 129, 0.5);
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
