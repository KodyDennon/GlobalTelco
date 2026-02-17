<script lang="ts">
	import MapView from './MapView.svelte';
	import HUD from './HUD.svelte';
	import InfoPanel from './InfoPanel.svelte';
	import { initialized } from '$lib/stores/gameState';
</script>

{#if $initialized}
	<div class="game-view">
		<MapView />
		<HUD />
		<InfoPanel />
	</div>
{:else}
	<div class="loading">
		<p>Initializing simulation...</p>
	</div>
{/if}

<style>
	.game-view {
		width: 100vw;
		height: 100vh;
		position: relative;
		overflow: hidden;
		background: #0a0e17;
	}

	.loading {
		width: 100vw;
		height: 100vh;
		display: flex;
		align-items: center;
		justify-content: center;
		background: #0a0e17;
		color: #9ca3af;
		font-family: system-ui, sans-serif;
		font-size: 16px;
	}
</style>
