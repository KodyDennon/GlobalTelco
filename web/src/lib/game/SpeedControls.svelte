<script lang="ts">
	import { setSpeed, togglePause } from './GameLoop';
	import { worldInfo } from '$lib/stores/gameState';

	let currentSpeed = $derived($worldInfo.speed);

	const speeds = [
		{ label: '||', value: 0, key: 'Paused' },
		{ label: '1x', value: 1, key: 'Normal' },
		{ label: '2x', value: 2, key: 'Fast' },
		{ label: '4x', value: 4, key: 'VeryFast' },
		{ label: '8x', value: 8, key: 'Ultra' }
	];
</script>

<div class="speed-controls">
	{#each speeds as s}
		<button
			class:active={currentSpeed === s.key}
			onclick={() => setSpeed(s.value)}
		>
			{s.label}
		</button>
	{/each}
</div>

<style>
	.speed-controls {
		display: flex;
		gap: 2px;
		background: rgba(31, 41, 55, 0.8);
		border-radius: 4px;
		padding: 2px;
	}

	button {
		background: transparent;
		border: none;
		color: #9ca3af;
		padding: 4px 10px;
		font-size: 12px;
		font-family: 'SF Mono', 'Fira Code', monospace;
		cursor: pointer;
		border-radius: 3px;
		transition: all 0.15s;
	}

	button:hover {
		background: rgba(55, 65, 81, 0.5);
		color: #f3f4f6;
	}

	button.active {
		background: rgba(16, 185, 129, 0.2);
		color: #10b981;
	}
</style>
