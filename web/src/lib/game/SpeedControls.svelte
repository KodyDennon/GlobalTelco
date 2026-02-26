<script lang="ts">
	import { setSpeed, togglePause } from './GameLoop';
	import { worldInfo } from '$lib/stores/gameState';
	import { tr } from '$lib/i18n/index';
	import { tooltip } from '$lib/ui/tooltip';

	let currentSpeed = $derived($worldInfo.speed);
	let isSandbox = $derived($worldInfo.sandbox);

	const baseSpeedList = [
		{ labelKey: 'speed.label_paused', value: 0, key: 'Paused', ariaKey: 'speed.paused', tip: 'Pause — game time stops. Use this to plan your next move.' },
		{ labelKey: 'speed.label_1x', value: 1, key: 'Normal', ariaKey: 'speed.normal', tip: 'Normal speed (1x) — one tick per second' },
		{ labelKey: 'speed.label_2x', value: 2, key: 'Fast', ariaKey: 'speed.fast', tip: 'Fast speed (2x) — two ticks per second' },
		{ labelKey: 'speed.label_4x', value: 4, key: 'VeryFast', ariaKey: 'speed.very_fast', tip: 'Very fast (4x) — four ticks per second' },
		{ labelKey: 'speed.label_8x', value: 8, key: 'Ultra', ariaKey: 'speed.ultra', tip: 'Ultra speed (8x) — eight ticks per second.\nUseful for skipping ahead.' }
	];

	const ludicrousSpeed = { labelKey: 'speed.label_32x', value: 32, key: 'Ludicrous', ariaKey: 'speed.ludicrous', tip: 'Ludicrous speed (32x) — sandbox only.\n32 ticks per second!' };

	let speeds = $derived(isSandbox ? [...baseSpeedList, ludicrousSpeed] : baseSpeedList);
</script>

<div class="speed-controls" role="radiogroup" aria-label={$tr('speed.normal')}>
	{#each speeds as s}
		<button
			class:active={currentSpeed === s.key}
			onclick={() => setSpeed(s.value)}
			role="radio"
			aria-checked={currentSpeed === s.key}
			aria-label={$tr(s.ariaKey)}
			use:tooltip={s.tip}
		>
			{$tr(s.labelKey)}
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
		font-family: var(--font-mono);
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
