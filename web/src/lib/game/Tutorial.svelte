<script lang="ts">
	import { tr } from '$lib/i18n/index';
	import {
		tutorialActive,
		tutorialStep,
		tutorialSteps,
		nextStep,
		prevStep,
		skipTutorial
	} from '$lib/stores/tutorialState';

	let currentStep = $derived(tutorialSteps[$tutorialStep]);
	let isFirst = $derived($tutorialStep === 0);
	let isLast = $derived($tutorialStep === tutorialSteps.length - 1);
	let progress = $derived(Math.round((($tutorialStep + 1) / tutorialSteps.length) * 100));

	function getPositionClass(pos: string): string {
		switch (pos) {
			case 'top-right':
				return 'pos-top-right';
			case 'bottom-left':
				return 'pos-bottom-left';
			case 'bottom-right':
				return 'pos-bottom-right';
			default:
				return 'pos-center';
		}
	}

	function handleKeyDown(e: KeyboardEvent) {
		if (!$tutorialActive) return;
		switch (e.key) {
			case 'Enter':
				e.preventDefault();
				nextStep();
				break;
			case 'ArrowLeft':
				e.preventDefault();
				if (!isFirst) prevStep();
				break;
			case 'Escape':
				e.preventDefault();
				skipTutorial();
				break;
		}
	}
</script>

<svelte:window onkeydown={handleKeyDown} />

{#if $tutorialActive}
	<div class="tutorial-overlay" role="dialog" aria-modal="true" aria-label={$tr('tutorial.title')}>
		<div class="tutorial-card {getPositionClass(currentStep.position)}">
			<div class="tutorial-header">
				<span class="step-counter">{$tutorialStep + 1}/{tutorialSteps.length}</span>
				<button class="skip-btn" onclick={skipTutorial} aria-label={$tr('tutorial.skip')}>{$tr('tutorial.skip')}</button>
			</div>
			<h3 class="tutorial-title">{$tr(`tutorial.${currentStep.id}_title`)}</h3>
			<p class="tutorial-text">{$tr(`tutorial.${currentStep.id}_text`)}</p>
			<div class="tutorial-progress" role="progressbar" aria-valuenow={progress} aria-valuemin={0} aria-valuemax={100}>
				<div class="progress-bar" style:width="{progress}%"></div>
			</div>
			<div class="tutorial-actions">
				{#if !isFirst}
					<button class="btn btn-secondary" onclick={prevStep} aria-label={$tr('tutorial.back')}>{$tr('tutorial.back')}</button>
				{:else}
					<div></div>
				{/if}
				<button class="btn btn-primary" onclick={nextStep} aria-label={isLast ? $tr('tutorial.finish') : $tr('tutorial.next')}>
					{isLast ? $tr('tutorial.finish') : $tr('tutorial.next')}
				</button>
			</div>
		</div>
	</div>
{/if}

<style>
	.tutorial-overlay {
		position: absolute;
		inset: 0;
		background: rgba(0, 0, 0, 0.4);
		z-index: 50;
		display: flex;
		align-items: center;
		justify-content: center;
		pointer-events: auto;
	}

	.tutorial-card {
		background: rgba(17, 24, 39, 0.97);
		border: 1px solid rgba(96, 165, 250, 0.4);
		border-radius: 12px;
		padding: 24px;
		width: 420px;
		max-width: 90vw;
		box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5), 0 0 48px rgba(96, 165, 250, 0.1);
	}

	.pos-center {
		position: relative;
	}

	.pos-top-right {
		position: absolute;
		top: 80px;
		right: 24px;
	}

	.pos-bottom-left {
		position: absolute;
		bottom: 80px;
		left: 24px;
	}

	.pos-bottom-right {
		position: absolute;
		bottom: 80px;
		right: 24px;
	}

	.tutorial-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 12px;
	}

	.step-counter {
		font-family: monospace;
		font-size: 12px;
		color: #6b7280;
		background: rgba(31, 41, 55, 0.8);
		padding: 2px 8px;
		border-radius: 4px;
	}

	.skip-btn {
		background: none;
		border: none;
		color: #6b7280;
		font-size: 12px;
		cursor: pointer;
		padding: 2px 6px;
	}

	.skip-btn:hover {
		color: #9ca3af;
	}

	.tutorial-title {
		font-size: 18px;
		font-weight: 700;
		color: #f3f4f6;
		margin: 0 0 8px;
	}

	.tutorial-text {
		font-size: 14px;
		color: #9ca3af;
		line-height: 1.5;
		margin: 0 0 16px;
	}

	.tutorial-progress {
		height: 3px;
		background: #1f2937;
		border-radius: 2px;
		margin-bottom: 16px;
		overflow: hidden;
	}

	.progress-bar {
		height: 100%;
		background: #3b82f6;
		border-radius: 2px;
		transition: width 0.3s ease;
	}

	.tutorial-actions {
		display: flex;
		justify-content: space-between;
		align-items: center;
	}

	.btn {
		padding: 8px 20px;
		border: none;
		border-radius: 6px;
		font-size: 13px;
		font-weight: 600;
		cursor: pointer;
		transition: background 0.15s;
	}

	.btn-primary {
		background: #3b82f6;
		color: #fff;
	}

	.btn-primary:hover {
		background: #2563eb;
	}

	.btn-secondary {
		background: rgba(31, 41, 55, 0.8);
		color: #9ca3af;
		border: 1px solid rgba(55, 65, 81, 0.5);
	}

	.btn-secondary:hover {
		background: rgba(55, 65, 81, 0.6);
	}
</style>
