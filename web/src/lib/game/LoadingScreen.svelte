<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { tr } from '$lib/i18n/index';

	let { stage = 0 }: { stage?: number } = $props();

	const stages = [
		{ key: 'game.loading_engine', icon: 'cog' },
		{ key: 'game.loading_world', icon: 'globe' },
		{ key: 'game.loading_systems', icon: 'circuit' },
		{ key: 'game.loading_ready', icon: 'check' },
	];

	const tips = Array.from({ length: 10 }, (_, i) => `tips.tip_${i + 1}`);
	const tipIndex = Math.floor(Math.random() * tips.length);

	// Elapsed time counter
	let elapsed = $state(0);
	let intervalId: ReturnType<typeof setInterval> | null = null;

	onMount(() => {
		const start = Date.now();
		intervalId = setInterval(() => {
			elapsed = Math.floor((Date.now() - start) / 1000);
		}, 1000);
	});

	onDestroy(() => {
		if (intervalId) clearInterval(intervalId);
	});

	const progressPct = $derived(Math.min(100, ((stage + 1) / stages.length) * 100));
</script>

<div class="loading-screen" role="status" aria-label="Loading game" aria-live="polite">
	<!-- Animated background grid -->
	<div class="bg-grid" aria-hidden="true"></div>
	<div class="bg-glow" aria-hidden="true"></div>

	<div class="loading-content">
		<!-- Logo + tagline -->
		<div class="logo-section">
			<h1 class="title">GlobalTelco</h1>
			<p class="subtitle">Building the world's network</p>
		</div>

		<!-- Signal wave animation -->
		<div class="signal-wave">
			{#each Array(5) as _, i}
				<div class="wave-bar" style="animation-delay: {i * 0.15}s"></div>
			{/each}
		</div>

		<!-- Progress bar -->
		<div class="progress-section">
			<div class="progress-bar" role="progressbar" aria-valuenow={Math.round(progressPct)} aria-valuemin={0} aria-valuemax={100} aria-label="Loading progress">
				<div class="progress-fill" style="width: {progressPct}%">
					<div class="progress-shine"></div>
				</div>
			</div>
			<div class="progress-label">
				<span class="progress-pct">{Math.round(progressPct)}%</span>
				{#if elapsed > 0}
					<span class="elapsed">{elapsed}s</span>
				{/if}
			</div>
		</div>

		<!-- Stage checklist -->
		<div class="stages">
			{#each stages as stageInfo, i}
				<div class="stage" class:active={i === stage} class:done={i < stage} class:pending={i > stage}>
					<div class="stage-indicator">
						{#if i < stage}
							<svg viewBox="0 0 16 16" class="stage-icon check"><path d="M13.78 4.22a.75.75 0 010 1.06l-7.25 7.25a.75.75 0 01-1.06 0L2.22 9.28a.75.75 0 011.06-1.06L6 10.94l6.72-6.72a.75.75 0 011.06 0z"/></svg>
						{:else if i === stage}
							<div class="stage-spinner"></div>
						{:else}
							<div class="stage-dot"></div>
						{/if}
					</div>
					<span class="stage-text">{$tr(stageInfo.key)}</span>
				</div>
			{/each}
		</div>

		<!-- Tip -->
		<div class="tip-section">
			<span class="tip-label">TIP</span>
			<p class="tip">{$tr(tips[tipIndex])}</p>
		</div>
	</div>
</div>

<style>
	.loading-screen {
		width: 100vw;
		height: 100vh;
		display: flex;
		align-items: center;
		justify-content: center;
		background: #060a12;
		color: #f3f4f6;
		font-family: 'Inter', system-ui, sans-serif;
		position: relative;
		overflow: hidden;
	}

	/* Subtle animated grid background */
	.bg-grid {
		position: absolute;
		inset: 0;
		background-image:
			linear-gradient(rgba(59, 130, 246, 0.03) 1px, transparent 1px),
			linear-gradient(90deg, rgba(59, 130, 246, 0.03) 1px, transparent 1px);
		background-size: 40px 40px;
		animation: grid-drift 20s linear infinite;
	}

	@keyframes grid-drift {
		to { transform: translate(40px, 40px); }
	}

	/* Central radial glow */
	.bg-glow {
		position: absolute;
		width: 600px;
		height: 600px;
		top: 50%;
		left: 50%;
		transform: translate(-50%, -50%);
		background: radial-gradient(circle, rgba(59, 130, 246, 0.06) 0%, transparent 70%);
		animation: glow-pulse 4s ease-in-out infinite;
	}

	@keyframes glow-pulse {
		0%, 100% { opacity: 0.6; transform: translate(-50%, -50%) scale(1); }
		50% { opacity: 1; transform: translate(-50%, -50%) scale(1.1); }
	}

	.loading-content {
		text-align: center;
		max-width: 420px;
		width: 100%;
		padding: 0 24px;
		position: relative;
		z-index: 1;
	}

	/* Logo section */
	.logo-section {
		margin-bottom: 36px;
	}

	.title {
		font-size: 40px;
		font-weight: 800;
		letter-spacing: -0.02em;
		background: linear-gradient(135deg, #10b981, #3b82f6, #8b5cf6);
		background-size: 200% 200%;
		-webkit-background-clip: text;
		-webkit-text-fill-color: transparent;
		background-clip: text;
		animation: gradient-shift 3s ease infinite;
		margin: 0 0 8px;
	}

	@keyframes gradient-shift {
		0%, 100% { background-position: 0% 50%; }
		50% { background-position: 100% 50%; }
	}

	.subtitle {
		font-size: 13px;
		color: #4b5563;
		letter-spacing: 0.15em;
		text-transform: uppercase;
		margin: 0;
	}

	/* Signal wave animation */
	.signal-wave {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 4px;
		height: 32px;
		margin-bottom: 32px;
	}

	.wave-bar {
		width: 4px;
		height: 8px;
		background: linear-gradient(180deg, #3b82f6, #10b981);
		border-radius: 2px;
		animation: wave 1.2s ease-in-out infinite;
	}

	@keyframes wave {
		0%, 100% { height: 8px; opacity: 0.4; }
		50% { height: 28px; opacity: 1; }
	}

	/* Progress section */
	.progress-section {
		margin-bottom: 28px;
	}

	.progress-bar {
		width: 100%;
		height: 6px;
		background: rgba(55, 65, 81, 0.3);
		border-radius: 3px;
		overflow: hidden;
		position: relative;
	}

	.progress-fill {
		height: 100%;
		background: linear-gradient(90deg, #10b981, #3b82f6);
		border-radius: 3px;
		transition: width 0.6s cubic-bezier(0.4, 0, 0.2, 1);
		position: relative;
		overflow: hidden;
	}

	.progress-shine {
		position: absolute;
		top: 0;
		left: -100%;
		width: 100%;
		height: 100%;
		background: linear-gradient(90deg, transparent, rgba(255, 255, 255, 0.2), transparent);
		animation: shine 2s ease-in-out infinite;
	}

	@keyframes shine {
		0% { left: -100%; }
		100% { left: 200%; }
	}

	.progress-label {
		display: flex;
		justify-content: space-between;
		margin-top: 8px;
		font-size: 12px;
	}

	.progress-pct {
		color: #9ca3af;
		font-variant-numeric: tabular-nums;
		font-family: 'JetBrains Mono', 'Fira Code', monospace;
	}

	.elapsed {
		color: #4b5563;
		font-variant-numeric: tabular-nums;
		font-family: 'JetBrains Mono', 'Fira Code', monospace;
	}

	/* Stage checklist */
	.stages {
		display: flex;
		flex-direction: column;
		gap: 12px;
		margin-bottom: 32px;
		text-align: left;
	}

	.stage {
		display: flex;
		align-items: center;
		gap: 12px;
		font-size: 13px;
		color: #374151;
		transition: color 0.3s, opacity 0.3s;
	}

	.stage.active {
		color: #e5e7eb;
	}

	.stage.done {
		color: #10b981;
	}

	.stage.pending {
		opacity: 0.4;
	}

	.stage-indicator {
		width: 20px;
		height: 20px;
		display: flex;
		align-items: center;
		justify-content: center;
		flex-shrink: 0;
	}

	.stage-icon.check {
		width: 16px;
		height: 16px;
		fill: #10b981;
	}

	.stage-spinner {
		width: 16px;
		height: 16px;
		border: 2px solid rgba(59, 130, 246, 0.2);
		border-top: 2px solid #3b82f6;
		border-radius: 50%;
		animation: spin 0.8s linear infinite;
	}

	@keyframes spin {
		to { transform: rotate(360deg); }
	}

	.stage-dot {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		background: #1f2937;
		border: 1px solid #374151;
	}

	.stage-text {
		font-weight: 500;
	}

	.stage.active .stage-text {
		font-weight: 600;
	}

	/* Tip section */
	.tip-section {
		border-top: 1px solid rgba(55, 65, 81, 0.2);
		padding-top: 20px;
		text-align: left;
	}

	.tip-label {
		font-size: 9px;
		font-weight: 700;
		letter-spacing: 0.15em;
		color: #374151;
		text-transform: uppercase;
		display: block;
		margin-bottom: 6px;
	}

	.tip {
		color: #6b7280;
		font-size: 12px;
		font-style: italic;
		line-height: 1.5;
		margin: 0;
	}
</style>
