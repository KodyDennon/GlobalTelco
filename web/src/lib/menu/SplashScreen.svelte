<script lang="ts">
	import { tr } from '$lib/i18n/index';

	let { onComplete, duration = 2000 }: { onComplete: () => void; duration?: number } = $props();

	const version = typeof __APP_VERSION__ !== 'undefined' ? __APP_VERSION__ : '0.1.0';

	// Trigger fade-out after the visible phase, then call onComplete when done
	let fadingOut = $state(false);

	$effect(() => {
		const fadeOutDelay = setTimeout(() => {
			fadingOut = true;
		}, duration * 0.65);

		const completeDelay = setTimeout(() => {
			onComplete();
		}, duration);

		return () => {
			clearTimeout(fadeOutDelay);
			clearTimeout(completeDelay);
		};
	});
</script>

<div class="splash" class:fading-out={fadingOut} role="presentation" aria-label="Loading">
	<div class="splash-content">
		<h1 class="splash-title">{$tr('splash.title')}</h1>
		<div class="splash-divider"></div>
		<p class="splash-tagline">{$tr('splash.tagline')}</p>
	</div>
	<span class="splash-version">{$tr('splash.version', { version })}</span>
</div>

<style>
	.splash {
		width: 100vw;
		height: 100vh;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		background: #0a0e17;
		animation: splashFadeIn 0.6s ease-out forwards;
		transition: opacity 0.5s ease-in;
	}

	.splash.fading-out {
		opacity: 0;
	}

	.splash-content {
		text-align: center;
		animation: splashSlideUp 0.8s cubic-bezier(0.16, 1, 0.3, 1) forwards;
		opacity: 0;
	}

	.splash-title {
		font-size: 64px;
		font-weight: 800;
		letter-spacing: -1.5px;
		margin: 0;
		background: linear-gradient(90deg, #10b981, #3b82f6);
		-webkit-background-clip: text;
		-webkit-text-fill-color: transparent;
		background-clip: text;
		animation: splashGlow 2s ease-in-out infinite alternate;
		font-family: system-ui, sans-serif;
	}

	.splash-divider {
		width: 64px;
		height: 2px;
		margin: 16px auto;
		background: linear-gradient(90deg, transparent, rgba(16, 185, 129, 0.5), transparent);
		animation: splashDividerExpand 1s ease-out 0.4s forwards;
		transform: scaleX(0);
	}

	.splash-tagline {
		color: #4b5563;
		font-size: 16px;
		margin: 0;
		font-family: system-ui, sans-serif;
		letter-spacing: 2px;
		text-transform: uppercase;
		font-weight: 400;
		animation: splashTaglineFade 0.6s ease-out 0.6s forwards;
		opacity: 0;
	}

	.splash-version {
		position: absolute;
		bottom: 24px;
		color: #374151;
		font-size: 12px;
		font-family: 'SF Mono', 'Fira Code', 'Cascadia Code', monospace;
		animation: splashTaglineFade 0.5s ease-out 0.8s forwards;
		opacity: 0;
	}

	@keyframes splashFadeIn {
		from { opacity: 0; }
		to { opacity: 1; }
	}

	@keyframes splashSlideUp {
		from {
			opacity: 0;
			transform: translateY(20px);
		}
		to {
			opacity: 1;
			transform: translateY(0);
		}
	}

	@keyframes splashGlow {
		from { filter: brightness(0.85); }
		to { filter: brightness(1.15); }
	}

	@keyframes splashDividerExpand {
		from { transform: scaleX(0); }
		to { transform: scaleX(1); }
	}

	@keyframes splashTaglineFade {
		from { opacity: 0; }
		to { opacity: 1; }
	}
</style>
