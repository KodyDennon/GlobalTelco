<script lang="ts">
	import { onMount } from 'svelte';
	import MainMenu from '$lib/menu/MainMenu.svelte';
	import NewGame from '$lib/menu/NewGame.svelte';
	import LoadGame from '$lib/menu/LoadGame.svelte';
	import Settings from '$lib/menu/Settings.svelte';
	import Credits from '$lib/menu/Credits.svelte';
	import WorldBrowser from '$lib/menu/WorldBrowser.svelte';
	import { goto } from '$app/navigation';
	import { initGame, start, loadFromSave, setSpeed } from '$lib/game/GameLoop';
	import { initWasm } from '$lib/wasm/bridge';

	type Screen = 'splash' | 'main' | 'newGame' | 'loadGame' | 'settings' | 'multiplayer' | 'credits';
	let screen: Screen = $state('splash');

	onMount(() => {
		setTimeout(() => {
			if (screen === 'splash') screen = 'main';
		}, 1500);
	});

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

	async function handleMultiplayerJoin(_worldId: string) {
		await initGame();
		setSpeed(0); // Server drives ticks in multiplayer — don't tick locally
		start();
		goto('/game');
	}
</script>

<svelte:head>
	<title>GlobalTelco</title>
</svelte:head>

{#if screen === 'splash'}
	<div class="splash">
		<h1 class="splash-title">GlobalTelco</h1>
		<p class="splash-sub">Build your telecom empire</p>
	</div>
{:else if screen === 'main'}
	<MainMenu onNewGame={() => (screen = 'newGame')} onLoadGame={() => (screen = 'loadGame')} onSettings={() => (screen = 'settings')} onMultiplayer={() => (screen = 'multiplayer')} onCredits={() => (screen = 'credits')} />
{:else if screen === 'newGame'}
	<NewGame onStart={handleStart} onBack={() => (screen = 'main')} />
{:else if screen === 'loadGame'}
	<LoadGame onLoad={handleLoad} onBack={() => (screen = 'main')} />
{:else if screen === 'settings'}
	<Settings onBack={() => (screen = 'main')} />
{:else if screen === 'credits'}
	<Credits onBack={() => (screen = 'main')} />
{:else if screen === 'multiplayer'}
	<WorldBrowser onBack={() => (screen = 'main')} onJoin={handleMultiplayerJoin} />
{/if}

<style>
	.splash {
		width: 100vw;
		height: 100vh;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		background: #0a0e17;
		animation: fadeIn 0.5s ease-out;
	}

	.splash-title {
		font-size: 56px;
		font-weight: 800;
		letter-spacing: -1px;
		background: linear-gradient(90deg, #10b981, #3b82f6);
		-webkit-background-clip: text;
		-webkit-text-fill-color: transparent;
		background-clip: text;
		animation: pulseGlow 1.5s ease-in-out infinite alternate;
	}

	.splash-sub {
		color: #4b5563;
		font-size: 16px;
		margin-top: 8px;
		font-family: system-ui, sans-serif;
	}

	@keyframes fadeIn {
		from { opacity: 0; }
		to { opacity: 1; }
	}

	@keyframes pulseGlow {
		from { filter: brightness(0.8); }
		to { filter: brightness(1.2); }
	}
</style>
