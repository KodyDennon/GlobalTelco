<script lang="ts">
	import { tr } from '$lib/i18n/index';

	let { onBack }: { onBack: () => void } = $props();
</script>

<div class="credits">
	<div class="credits-content">
		<h1 class="title">{$tr('credits.game_title')}</h1>
		<p class="subtitle">{$tr('credits.game_subtitle')}</p>

		<section>
			<h3>{$tr('credits.developed_by')}</h3>
			<ul class="dev-list">
				<li class="ai-credit">Claude Opus 4.5 & 4.6</li>
				<li class="ai-credit">Claude Sonnet 4.5 & 4.6</li>
				<li class="ai-credit">Gemini Pro 3.0</li>
				<li class="ai-credit">Gemini Flash</li>
			</ul>
		</section>

		<section>
			<h3>{$tr('credits.orchestrated_by')}</h3>
			<p class="credit-entry orchestrator">Kody Dennon</p>
		</section>

		<section>
			<h3>{$tr('credits.tech_stack')}</h3>
			<ul class="tech-list">
				<li>{$tr('credits.simulation')}</li>
				<li>{$tr('credits.frontend')}</li>
				<li>{$tr('credits.desktop')}</li>
				<li>{$tr('credits.server')}</li>
			</ul>
		</section>

		<p class="built-with">{$tr('credits.built_with')}</p>

		<button class="btn" onclick={onBack}>{$tr('menu.back')}</button>
	</div>
</div>

<style>
	.credits {
		width: 100vw;
		height: 100vh;
		display: flex;
		align-items: center;
		justify-content: center;
		background: linear-gradient(135deg, #0a0e17 0%, #111827 50%, #0a0e17 100%);
		font-family: system-ui, sans-serif;
		color: #f3f4f6;
	}

	.credits-content {
		text-align: center;
		max-width: 440px;
		padding: 48px;
		background: rgba(17, 24, 39, 0.95);
		border: 1px solid rgba(55, 65, 81, 0.5);
		border-radius: 12px;
	}

	.title {
		font-size: 36px;
		font-weight: 800;
		margin: 0;
		background: linear-gradient(90deg, #10b981, #3b82f6);
		-webkit-background-clip: text;
		-webkit-text-fill-color: transparent;
		background-clip: text;
	}

	.subtitle {
		color: #6b7280;
		font-size: 14px;
		margin: 8px 0 32px;
	}

	section {
		margin-bottom: 24px;
	}

	h3 {
		font-size: 11px;
		font-weight: 600;
		color: #6b7280;
		text-transform: uppercase;
		letter-spacing: 0.5px;
		margin-bottom: 8px;
	}

	.dev-list {
		list-style: none;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 6px;
	}

	.ai-credit {
		font-size: 14px;
		color: #d1d5db;
		font-weight: 500;
	}

	.credit-entry {
		font-size: 14px;
		color: #d1d5db;
	}

	.orchestrator {
		font-size: 18px;
		font-weight: 700;
		background: linear-gradient(90deg, #10b981, #3b82f6);
		-webkit-background-clip: text;
		-webkit-text-fill-color: transparent;
		background-clip: text;
	}

	.tech-list {
		list-style: none;
		font-size: 13px;
		color: #9ca3af;
		display: flex;
		flex-direction: column;
		gap: 4px;
	}

	.built-with {
		color: #4b5563;
		font-size: 12px;
		margin-bottom: 24px;
	}

	.btn {
		width: 100%;
		padding: 12px;
		background: rgba(31, 41, 55, 0.8);
		border: 1px solid rgba(55, 65, 81, 0.5);
		border-radius: 6px;
		color: #d1d5db;
		font-size: 14px;
		cursor: pointer;
	}

	.btn:hover {
		background: rgba(55, 65, 81, 0.6);
	}
</style>
