<script lang="ts">
	interface Props {
		feature: string;
		phase?: string;
		description?: string;
	}

	let { feature, phase = 'a future update', description = '' }: Props = $props();
</script>

<div class="coming-soon">
	<div class="icon">&#128679;</div>
	<h3 class="title">{feature}</h3>
	<p class="message">Coming in {phase}</p>
	{#if description}
		<p class="description">{description}</p>
	{/if}
</div>

<style>
	.coming-soon {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		padding: 48px 24px;
		text-align: center;
	}

	.icon {
		font-size: 40px;
		margin-bottom: 12px;
		opacity: 0.6;
	}

	.title {
		font-family: var(--font-sans, system-ui, sans-serif);
		font-size: 16px;
		font-weight: 600;
		color: #d1d5db;
		margin: 0 0 8px;
	}

	.message {
		font-family: var(--font-sans, system-ui, sans-serif);
		font-size: 13px;
		color: #6b7280;
		margin: 0 0 8px;
	}

	.description {
		font-family: var(--font-sans, system-ui, sans-serif);
		font-size: 12px;
		color: #4b5563;
		margin: 0;
		max-width: 300px;
		line-height: 1.5;
	}
</style>
