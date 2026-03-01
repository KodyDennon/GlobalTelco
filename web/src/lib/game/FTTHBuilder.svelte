<script lang="ts">
	import {
		ftthBuilderActive,
		enterPlacementMode,
		exitPlacementMode,
		selectedBuildItem,
		buildCategory,
	} from '$lib/stores/uiState';
	// FTTH chain steps
	const STEPS = [
		{ key: 'co', label: 'CO', nodeType: 'CentralOffice', description: 'Place or select a Central Office' },
		{ key: 'feeder', label: 'Feeder', edgeType: 'FeederFiber', description: 'Draw Feeder Fiber from CO' },
		{ key: 'hub', label: 'Hub', nodeType: 'FiberDistributionHub', description: 'Place Distribution Hub at feeder end' },
		{ key: 'dist', label: 'Dist.', edgeType: 'DistributionFiber', description: 'Draw Distribution Fiber from hub' },
		{ key: 'nap', label: 'NAP', nodeType: 'NetworkAccessPoint', description: 'Place NAP — auto-covers nearby subscribers' },
	] as const;

	let currentStep = $state(0);

	// Track placement progress
	$effect(() => {
		// When a build completes (selectedBuildItem clears after placement), advance step
		if ($ftthBuilderActive && !$selectedBuildItem && !$buildCategory) {
			// The step's placement finished (user placed the item or cancelled)
			// Don't auto-advance — let user click next or the step button
		}
	});

	function activateStep(stepIndex: number) {
		if (stepIndex < 0 || stepIndex >= STEPS.length) return;
		currentStep = stepIndex;
		const step = STEPS[stepIndex];
		if ('nodeType' in step && step.nodeType) {
			enterPlacementMode(step.nodeType, 'node');
		} else if ('edgeType' in step && step.edgeType) {
			enterPlacementMode(step.edgeType, 'edge');
		}
	}

	function handleStepClick(index: number) {
		activateStep(index);
	}

	function handleBack() {
		if (currentStep > 0) {
			exitPlacementMode();
			currentStep--;
		}
	}

	function handleNext() {
		if (currentStep < STEPS.length - 1) {
			currentStep++;
			activateStep(currentStep);
		}
	}

	function handleDone() {
		exitPlacementMode();
		ftthBuilderActive.set(false);
		currentStep = 0;
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') {
			handleDone();
		}
	}
</script>

<svelte:window onkeydown={handleKeydown} />

{#if $ftthBuilderActive}
	<div class="ftth-builder" role="dialog" aria-label="FTTH Builder">
		<div class="ftth-header">
			<span class="ftth-title">FTTH Builder</span>
			<span class="ftth-desc">{STEPS[currentStep].description}</span>
		</div>

		<!-- Step indicator: CO -> Feeder -> Hub -> Distribution -> Drop -->
		<div class="ftth-steps" role="tablist" aria-label="FTTH build steps">
			{#each STEPS as step, i}
				{@const isCurrent = i === currentStep}
				{@const isComplete = i < currentStep}
				<button
					class="ftth-step"
					class:current={isCurrent}
					class:complete={isComplete}
					onclick={() => handleStepClick(i)}
					role="tab"
					aria-selected={isCurrent}
				>
					<span class="step-num">{i + 1}</span>
					<span class="step-label">{step.label}</span>
				</button>
				{#if i < STEPS.length - 1}
					<span class="step-arrow" class:active={i < currentStep}>&rarr;</span>
				{/if}
			{/each}
		</div>

		<!-- Coverage info on final step -->
		{#if currentStep === STEPS.length - 1}
			<div class="ftth-info">
				Active NAPs serve all buildings within range (500m urban, up to 2km rural). No manual wiring needed.
			</div>
		{/if}

		<!-- Controls -->
		<div class="ftth-controls">
			<button
				class="ftth-btn back"
				onclick={handleBack}
				disabled={currentStep === 0}
			>
				Back
			</button>
			{#if currentStep < STEPS.length - 1}
				<button
					class="ftth-btn next"
					onclick={handleNext}
				>
					Next
				</button>
			{/if}
			<button
				class="ftth-btn done"
				onclick={handleDone}
			>
				Done
			</button>
		</div>
	</div>
{/if}

<style>
	.ftth-builder {
		position: fixed;
		top: 90px;
		left: 50%;
		transform: translateX(-50%);
		z-index: 40;
		background: rgba(17, 24, 39, 0.97);
		border: 1px solid rgba(55, 65, 81, 0.6);
		border-radius: 10px;
		padding: 12px 20px;
		box-shadow: 0 12px 40px rgba(0, 0, 0, 0.5);
		min-width: 420px;
		backdrop-filter: blur(8px);
	}

	.ftth-header {
		display: flex;
		align-items: baseline;
		gap: 12px;
		margin-bottom: 10px;
	}

	.ftth-title {
		font-size: 12px;
		font-weight: 800;
		letter-spacing: 0.08em;
		text-transform: uppercase;
		color: #34d399;
		font-family: var(--font-mono, monospace);
	}

	.ftth-desc {
		font-size: 11px;
		color: #9ca3af;
	}

	/* ── Step indicator ────────────────────────────────────────────────────── */

	.ftth-steps {
		display: flex;
		align-items: center;
		gap: 4px;
		margin-bottom: 10px;
	}

	.ftth-step {
		display: flex;
		align-items: center;
		gap: 4px;
		padding: 4px 10px;
		background: rgba(31, 41, 55, 0.5);
		border: 1px solid rgba(55, 65, 81, 0.3);
		border-radius: 6px;
		color: #6b7280;
		font-size: 11px;
		font-family: var(--font-mono, monospace);
		cursor: pointer;
		transition: all 0.15s;
	}

	.ftth-step:hover {
		background: rgba(55, 65, 81, 0.5);
		color: #d1d5db;
	}

	.ftth-step.current {
		background: rgba(16, 185, 129, 0.15);
		border-color: rgba(16, 185, 129, 0.5);
		color: #10b981;
	}

	.ftth-step.complete {
		background: rgba(16, 185, 129, 0.08);
		border-color: rgba(16, 185, 129, 0.2);
		color: #34d399;
	}

	.step-num {
		font-size: 9px;
		font-weight: 800;
		width: 16px;
		height: 16px;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		border-radius: 50%;
		background: rgba(55, 65, 81, 0.5);
	}

	.ftth-step.current .step-num {
		background: rgba(16, 185, 129, 0.3);
		color: #fff;
	}

	.ftth-step.complete .step-num {
		background: rgba(16, 185, 129, 0.2);
	}

	.step-label {
		font-weight: 600;
	}

	.step-arrow {
		color: #4b5563;
		font-size: 12px;
	}

	.step-arrow.active {
		color: #34d399;
	}

	/* ── Controls ──────────────────────────────────────────────────────────── */

	.ftth-controls {
		display: flex;
		gap: 6px;
		justify-content: flex-end;
	}

	.ftth-btn {
		padding: 4px 14px;
		font-size: 11px;
		font-family: var(--font-mono, monospace);
		font-weight: 600;
		border-radius: 4px;
		cursor: pointer;
		border: 1px solid;
		transition: all 0.12s;
	}

	.ftth-btn.back {
		background: rgba(55, 65, 81, 0.3);
		border-color: rgba(75, 85, 99, 0.4);
		color: #9ca3af;
	}

	.ftth-btn.back:hover:not(:disabled) {
		background: rgba(55, 65, 81, 0.5);
		color: #d1d5db;
	}

	.ftth-btn.back:disabled {
		opacity: 0.3;
		cursor: not-allowed;
	}

	.ftth-btn.next {
		background: rgba(59, 130, 246, 0.15);
		border-color: rgba(59, 130, 246, 0.4);
		color: #60a5fa;
	}

	.ftth-btn.next:hover {
		background: rgba(59, 130, 246, 0.25);
	}

	.ftth-btn.done {
		background: rgba(239, 68, 68, 0.15);
		border-color: rgba(239, 68, 68, 0.3);
		color: #ef4444;
	}

	.ftth-btn.done:hover {
		background: rgba(239, 68, 68, 0.25);
	}

	.ftth-info {
		font-size: 10px;
		color: #6ee7b7;
		background: rgba(16, 185, 129, 0.08);
		border: 1px solid rgba(16, 185, 129, 0.2);
		border-radius: 4px;
		padding: 6px 10px;
		margin-bottom: 8px;
		line-height: 1.4;
	}
</style>
