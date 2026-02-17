<script lang="ts">
	import MainMenu from '$lib/menu/MainMenu.svelte';
	import NewGame from '$lib/menu/NewGame.svelte';
	import Settings from '$lib/menu/Settings.svelte';
	import { goto } from '$app/navigation';
	import { initGame, start } from '$lib/game/GameLoop';

	type Screen = 'main' | 'newGame' | 'settings';
	let screen: Screen = $state('main');

	async function handleStart(config: object) {
		await initGame(config);
		start();
		goto('/game');
	}
</script>

<svelte:head>
	<title>GlobalTelco</title>
</svelte:head>

{#if screen === 'main'}
	<MainMenu onNewGame={() => (screen = 'newGame')} onSettings={() => (screen = 'settings')} />
{:else if screen === 'newGame'}
	<NewGame onStart={handleStart} onBack={() => (screen = 'main')} />
{:else if screen === 'settings'}
	<Settings onBack={() => (screen = 'main')} />
{/if}
