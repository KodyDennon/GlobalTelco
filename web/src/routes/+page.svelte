<script lang="ts">
	import MainMenu from '$lib/menu/MainMenu.svelte';
	import NewGame from '$lib/menu/NewGame.svelte';
	import LoadGame from '$lib/menu/LoadGame.svelte';
	import Settings from '$lib/menu/Settings.svelte';
	import { goto } from '$app/navigation';
	import { initGame, start, loadFromSave } from '$lib/game/GameLoop';
	import { initWasm } from '$lib/wasm/bridge';

	type Screen = 'main' | 'newGame' | 'loadGame' | 'settings';
	let screen: Screen = $state('main');

	async function handleStart(config: object) {
		await initGame(config);
		start();
		goto('/game');
	}

	async function handleLoad(data: string) {
		await initWasm();
		await loadFromSave(data);
		start();
		goto('/game');
	}
</script>

<svelte:head>
	<title>GlobalTelco</title>
</svelte:head>

{#if screen === 'main'}
	<MainMenu onNewGame={() => (screen = 'newGame')} onLoadGame={() => (screen = 'loadGame')} onSettings={() => (screen = 'settings')} />
{:else if screen === 'newGame'}
	<NewGame onStart={handleStart} onBack={() => (screen = 'main')} />
{:else if screen === 'loadGame'}
	<LoadGame onLoad={handleLoad} onBack={() => (screen = 'main')} />
{:else if screen === 'settings'}
	<Settings onBack={() => (screen = 'main')} />
{/if}
