<script lang="ts">
	import GameView from '$lib/game/GameView.svelte';
	import { initialized } from '$lib/stores/gameState';
	import { isInitialized } from '$lib/wasm/bridge';
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';

	// Check actual WASM bridge state, not just the store (which can reset on HMR)
	let wasmReady = $state(isInitialized());

	onMount(() => {
		wasmReady = isInitialized();
		if (!wasmReady && !$initialized) {
			goto('/');
		}
		// If WASM is ready but store was reset (HMR), restore it
		if (wasmReady && !$initialized) {
			initialized.set(true);
		}
	});
</script>

<svelte:head>
	<title>GlobalTelco - Playing</title>
</svelte:head>

{#if $initialized || wasmReady}
	<GameView />
{/if}
