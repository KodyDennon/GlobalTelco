<script lang="ts">
	import { tr, t } from '$lib/i18n/index';

	let { stage = 0 }: { stage?: number } = $props();

	const stages = [
		'game.loading_engine',
		'game.loading_world',
		'game.loading_systems',
		'game.loading_ready'
	];

	const tips = Array.from({ length: 10 }, (_, i) => `tips.tip_${i + 1}`);
	const tipIndex = Math.floor(Math.random() * tips.length);
</script>

<div class="loading-screen">
	<div class="loading-content">
		<h1 class="title">GlobalTelco</h1>
		<div class="spinner"></div>
		<div class="stages">
			{#each stages as stageKey, i}
				<span class="stage" class:active={i === stage} class:done={i < stage}>
					{i < stage ? '✓' : i === stage ? '●' : '○'} {$tr(stageKey)}
				</span>
			{/each}
		</div>
		<p class="tip">{$tr(tips[tipIndex])}</p>
	</div>
</div>

<style>
	.loading-screen {
		width: 100vw;
		height: 100vh;
		display: flex;
		align-items: center;
		justify-content: center;
		background: linear-gradient(135deg, #0a0e17 0%, #111827 50%, #0a0e17 100%);
		color: #f3f4f6;
		font-family: system-ui, sans-serif;
	}

	.loading-content {
		text-align: center;
		max-width: 400px;
	}

	.title {
		font-size: 36px;
		font-weight: 800;
		background: linear-gradient(90deg, #10b981, #3b82f6);
		-webkit-background-clip: text;
		-webkit-text-fill-color: transparent;
		background-clip: text;
		margin-bottom: 32px;
	}

	.spinner {
		width: 40px;
		height: 40px;
		margin: 0 auto 32px;
		border: 3px solid rgba(59, 130, 246, 0.2);
		border-top: 3px solid #3b82f6;
		border-radius: 50%;
		animation: spin 1s linear infinite;
	}

	@keyframes spin {
		to { transform: rotate(360deg); }
	}

	.stages {
		display: flex;
		flex-direction: column;
		gap: 8px;
		margin-bottom: 32px;
	}

	.stage {
		font-size: 13px;
		color: #4b5563;
		transition: color 0.3s;
	}

	.stage.active {
		color: #3b82f6;
		font-weight: 600;
	}

	.stage.done {
		color: #10b981;
	}

	.tip {
		color: #6b7280;
		font-size: 12px;
		font-style: italic;
		padding: 16px;
		border-top: 1px solid rgba(55, 65, 81, 0.3);
	}
</style>
