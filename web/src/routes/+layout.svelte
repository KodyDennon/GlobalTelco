<script lang="ts">
	import '../app.css';
	import favicon from '$lib/assets/favicon.svg';
	import { onMount } from 'svelte';
	import { colorblindMode, uiScale } from '$lib/stores/settings';

	let { children } = $props();

	$effect(() => {
		document.body.dataset.colorblind = $colorblindMode;
		document.documentElement.style.setProperty('--ui-scale', String($uiScale));
	});

	onMount(() => {
		if ('serviceWorker' in navigator) {
			navigator.serviceWorker.register('/service-worker.js').catch(() => {
				// Service worker registration failed — offline mode unavailable
			});
		}
	});
</script>

<svelte:head>
	<link rel="icon" href={favicon} />
	<link rel="apple-touch-icon" href="/icons/icon-180.png" />
	<link rel="manifest" href="/manifest.json" />
	<meta name="theme-color" content="#0a0e17" />
</svelte:head>

{@render children()}
