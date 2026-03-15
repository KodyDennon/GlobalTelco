<script lang="ts">
	import MainMenu from '$lib/menu/MainMenu.svelte';
	import NewGame from '$lib/menu/NewGame.svelte';
	import LoadGame from '$lib/menu/LoadGame.svelte';
	import Settings from '$lib/menu/Settings.svelte';
	import Credits from '$lib/menu/Credits.svelte';
	import WorldBrowser from '$lib/menu/WorldBrowser.svelte';
	import Lobby from '$lib/menu/Lobby.svelte';
	import ProfilePage from '$lib/menu/ProfilePage.svelte';
	import ForgotPassword from '$lib/menu/ForgotPassword.svelte';
	import SplashScreen from '$lib/menu/SplashScreen.svelte';
	import { goto } from '$app/navigation';
	import { initGame, initMultiplayer, start, loadFromSave, setSpeed } from '$lib/game/GameLoop';
	import { initWasm } from '$lib/wasm/bridge';
	import { githubCallback } from '$lib/multiplayer/accountApi';
	import { accessToken, refreshToken, playerId, playerUsername, isAuthenticated, connectionState, latestSnapshot } from '$lib/stores/multiplayerState';
	import { get } from 'svelte/store';
	import { onMount } from 'svelte';

	type Screen = 'splash' | 'main' | 'newGame' | 'loadGame' | 'settings' | 'multiplayer' | 'credits' | 'loading' | 'profile' | 'forgotPassword';

	// Handle GitHub OAuth callback
	onMount(() => {
		const params = new URLSearchParams(window.location.search);
		const code = params.get('code');
		if (code) {
			// Clean the URL
			window.history.replaceState({}, '', window.location.pathname);
			githubCallback(code).then((result) => {
				accessToken.set(result.access_token);
				refreshToken.set(result.refresh_token);
				playerId.set(result.player_id);
				playerUsername.set(result.username);
				isAuthenticated.set(true);
				screen = 'multiplayer';
			}).catch((e) => {
				console.error('GitHub OAuth failed:', e);
				screen = 'main';
			});
		}
	});
	let screen: Screen = $state('splash');
	let loadingTip = $state('');

	const LOADING_TIPS = [
		'Tip: Right-click the map to open the radial build menu',
		'Tip: Use hotbar keys 1-9 for quick building',
		'Tip: Check the Advisor panel for strategic suggestions',
		'Tip: Build backbone infrastructure first, then extend to cities',
		'Tip: Monitor your cash flow — bankruptcy means game over!',
		'Tip: Research new tech to unlock more powerful infrastructure',
		'Tip: Keep infrastructure health above 50% to avoid outages',
		'Tip: Contracts provide steady income — propose them to AI corps',
		'Tip: Use the Coverage overlay to find underserved areas',
		'Tip: Insure your expensive infrastructure against damage',
		'Tip: AI corporations will compete — watch their expansion patterns',
		'Tip: Overlays help you make strategic decisions about where to build',
	];

	async function handleStart(config: object) {
		loadingTip = LOADING_TIPS[Math.floor(Math.random() * LOADING_TIPS.length)];
		screen = 'loading';
		// Yield to let the loading screen render before blocking WASM init
		await new Promise(r => setTimeout(r, 50));
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
		try {
			console.log('[MP] Joining world, waiting for snapshot...', _worldId);
			// Wait for full world snapshot from server (auto-requested on WorldJoined)
			const snapshot = await new Promise<{ tick: number; state_json: string }>((resolve, reject) => {
				// 1. Check if snapshot is ALREADY here (race condition fix)
				const existing = get(latestSnapshot);
				if (existing) {
					console.log('[MP] Snapshot already available via store');
					resolve(existing);
					return;
				}

				let unsubscribe = () => {};

				const timeout = setTimeout(() => {
					window.removeEventListener('mp-snapshot', handler);
					unsubscribe();
					reject(new Error('Snapshot timeout (60s)'));
				}, 60000);

				const handler = (e: Event) => {
					clearTimeout(timeout);
					unsubscribe();
					resolve((e as CustomEvent).detail);
				};
				window.addEventListener('mp-snapshot', handler, { once: true });

				// Fail fast if connection drops
				unsubscribe = connectionState.subscribe(state => {
					if (state === 'disconnected') {
						clearTimeout(timeout);
						window.removeEventListener('mp-snapshot', handler);
						unsubscribe();
						reject(new Error('Connection lost while joining'));
					}
				});
				
				// Also subscribe to store changes in case the event was missed but store updated
				const storeUnsub = latestSnapshot.subscribe(snap => {
					if (snap) {
						clearTimeout(timeout);
						window.removeEventListener('mp-snapshot', handler);
						unsubscribe();
						resolve(snap);
					}
				});
				
				// Combine unsubscribes
				const prevUnsub = unsubscribe;
				unsubscribe = () => {
					prevUnsub();
					storeUnsub();
				};
			});
			console.log('[MP] Snapshot received, tick:', snapshot.tick, 'size:', snapshot.state_json.length);
			await initMultiplayer(snapshot.state_json);
			
			// Free up memory - we don't need the JSON snapshot in JS heap anymore
			latestSnapshot.set(null);
			
			console.log('[MP] Game initialized, starting loop...');
			start();
			goto('/game');
		} catch (e) {
			console.error('[MP] Failed to join multiplayer game:', e);
		}
	}
</script>

<svelte:head>
	<title>GlobalTelco</title>
</svelte:head>

{#if screen === 'loading'}
	<div class="loading-screen">
		<h1 class="loading-title">Generating World...</h1>
		<div class="loading-spinner"></div>
		<p class="loading-tip">{loadingTip}</p>
	</div>
{:else if screen === 'splash'}
	<SplashScreen onComplete={() => (screen = 'main')} />
{:else if screen === 'main'}
	<MainMenu onNewGame={() => (screen = 'newGame')} onLoadGame={() => (screen = 'loadGame')} onSettings={() => (screen = 'settings')} onMultiplayer={() => (screen = 'multiplayer')} onCredits={() => (screen = 'credits')} onProfile={() => (screen = 'profile')} />
{:else if screen === 'newGame'}
	<NewGame onStart={handleStart} onBack={() => (screen = 'main')} />
{:else if screen === 'loadGame'}
	<LoadGame onLoad={handleLoad} onBack={() => (screen = 'main')} />
{:else if screen === 'settings'}
	<Settings onBack={() => (screen = 'main')} />
{:else if screen === 'credits'}
	<Credits onBack={() => (screen = 'main')} />
{:else if screen === 'multiplayer'}
	<Lobby onBack={() => (screen = 'main')} onJoin={handleMultiplayerJoin} onForgotPassword={() => (screen = 'forgotPassword')} />
{:else if screen === 'profile'}
	<ProfilePage onBack={() => (screen = 'main')} />
{:else if screen === 'forgotPassword'}
	<ForgotPassword onBack={() => (screen = 'multiplayer')} />
{/if}

<style>
	.loading-screen {
		width: 100vw;
		height: 100vh;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		background: #0a0e17;
		gap: 24px;
		animation: fadeIn 0.3s ease-out;
	}

	.loading-title {
		font-size: 28px;
		font-weight: 700;
		color: #d1d5db;
		font-family: system-ui, sans-serif;
	}

	.loading-spinner {
		width: 48px;
		height: 48px;
		border: 3px solid rgba(55, 65, 81, 0.5);
		border-top-color: #10b981;
		border-radius: 50%;
		animation: spin 0.8s linear infinite;
	}

	@keyframes fadeIn {
		from { opacity: 0; }
		to { opacity: 1; }
	}

	@keyframes spin {
		to { transform: rotate(360deg); }
	}

	.loading-tip {
		color: #6b7280;
		font-size: 14px;
		font-family: system-ui, sans-serif;
		max-width: 400px;
		text-align: center;
		line-height: 1.5;
	}
</style>
