<script lang="ts">
	import GameView from '$lib/game/GameView.svelte';
	import { initialized } from '$lib/stores/gameState';
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';

	onMount(() => {
		if (!$initialized) {
			goto('/');
		}
	});
</script>

<svelte:head>
	<title>GlobalTelco - Playing</title>
</svelte:head>

{#if $initialized}
	<GameView />
{/if}
