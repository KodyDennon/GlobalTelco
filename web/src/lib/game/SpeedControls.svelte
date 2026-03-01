<script lang="ts">
	import { setSpeed, togglePause } from './GameLoop';
	import { worldInfo } from '$lib/stores/gameState';
	import { isMultiplayer, speedVotes } from '$lib/stores/multiplayerState';
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

	// Count votes per speed key for multiplayer badge display
	let votesBySpeed = $derived.by(() => {
		const counts: Record<string, number> = {};
		for (const vote of $speedVotes) {
			counts[vote.speed] = (counts[vote.speed] ?? 0) + 1;
		}
		return counts;
	});
</script>

<div class="speed-controls" role="radiogroup" aria-label="Game speed">
	{#each speeds as s}
		<button
			class:active={currentSpeed === s.key}
			onclick={() => setSpeed(s.value)}
			role="radio"
			aria-checked={currentSpeed === s.key}
			aria-label={$tr(s.ariaKey)}
			use:tooltip={$isMultiplayer ? `${s.tip}\n(Vote for this speed)` : s.tip}
		>
			{$tr(s.labelKey)}
			{#if $isMultiplayer && votesBySpeed[s.key]}
				<span class="vote-badge">{votesBySpeed[s.key]}</span>
			{/if}
		</button>
	{/each}
	{#if $isMultiplayer}
		<span class="vote-hint">(vote)</span>
	{/if}
</div>

<style>
	.speed-controls {
		display: flex;
		gap: 2px;
		background: rgba(31, 41, 55, 0.8);
		border-radius: 4px;
		padding: 2px;
		align-items: center;
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
		position: relative;
	}

	button:hover {
		background: rgba(55, 65, 81, 0.5);
		color: #f3f4f6;
	}

	button.active {
		background: rgba(16, 185, 129, 0.2);
		color: #10b981;
	}

	.vote-badge {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		min-width: 14px;
		height: 14px;
		font-size: 9px;
		font-weight: 700;
		color: #fff;
		background: #3b82f6;
		border-radius: 7px;
		padding: 0 3px;
		margin-left: 3px;
		vertical-align: top;
	}

	.vote-hint {
		font-size: 9px;
		color: #6b7280;
		margin-left: 4px;
		white-space: nowrap;
	}
</style>
