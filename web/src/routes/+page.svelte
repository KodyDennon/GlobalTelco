<script lang="ts">
	import { onMount } from 'svelte';
	import { SITE_URL } from '$lib/config';

	onMount(() => {
		window.location.replace(SITE_URL);
	});
</script>

<svelte:head>
	<title>GlobalTelco moved to GitHub</title>
	<meta name="robots" content="noindex" />
</svelte:head>

<main class="retired">
	<h1>GlobalTelco has moved</h1>
	<p>The hosted game is retired. The project is available in the public repository.</p>
	<a href={SITE_URL}>Open public repo</a>
</main>

<style>
	.retired {
		width: 100vw;
		height: 100vh;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		background: #0a0e17;
		color: #f3f4f6;
		font-family: system-ui, sans-serif;
		text-align: center;
		padding: 24px;
		box-sizing: border-box;
	}

	h1 {
		font-size: 28px;
		font-weight: 700;
		margin: 0 0 12px;
	}

	p {
		max-width: 520px;
		margin: 0 0 24px;
		color: #9ca3af;
		line-height: 1.5;
	}

	a {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		min-height: 44px;
		padding: 0 18px;
		border-radius: 8px;
		background: #10b981;
		color: #04110d;
		font-weight: 700;
		text-decoration: none;
	}
</style>
