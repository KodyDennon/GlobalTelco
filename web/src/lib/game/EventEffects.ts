/**
 * EventEffectManager — visual effects for game events.
 *
 * Effects are CSS-based (DOM elements with keyframe animations).
 * No WebGL — runs independently of the deck.gl renderer.
 */

interface Effect {
	element: HTMLElement;
	startTime: number;
	duration: number; // ms
}

const EFFECT_CLASS_PREFIX = 'gt-effect-';

export class EventEffectManager {
	private container: HTMLElement;
	private activeEffects: Effect[] = [];
	private animFrameId: number | null = null;

	constructor(container: HTMLElement) {
		this.container = container;
		this.startUpdateLoop();
	}

	/** Trigger a visual effect based on a game event type. */
	triggerEffect(event: {
		type: string;
		region_id?: number;
		x?: number;
		y?: number;
		severity?: number;
	}): void {
		switch (event.type) {
			case 'Earthquake':
			case 'EarthquakeStruck':
				this.shakeEffect(event.severity ?? 0.5);
				break;
			case 'Storm':
			case 'StormStruck':
			case 'Hurricane':
				this.stormEffect();
				break;
			case 'MarketCrash':
			case 'MarketCrashed':
				this.vignetteEffect();
				break;
			case 'ConstructionComplete':
			case 'ConstructionCompleted':
			case 'NodeBuilt':
				this.flashEffect(event.x, event.y, [34, 197, 94], 300);
				break;
			case 'DisasterStruck':
			case 'DisasterStrike':
				this.flashEffect(event.x, event.y, [239, 68, 68], 1000);
				break;
			case 'ResearchComplete':
			case 'ResearchCompleted':
			case 'TechResearched':
				this.sparkleEffect();
				break;
			default:
				break;
		}
	}

	/** Update loop — cleans up expired effects every frame. */
	update(): void {
		const now = performance.now();
		const expired: Effect[] = [];

		for (const effect of this.activeEffects) {
			if (now - effect.startTime >= effect.duration) {
				expired.push(effect);
			}
		}

		for (const effect of expired) {
			effect.element.remove();
			const idx = this.activeEffects.indexOf(effect);
			if (idx !== -1) this.activeEffects.splice(idx, 1);
		}
	}

	/** Clean up everything. */
	dispose(): void {
		if (this.animFrameId !== null) {
			cancelAnimationFrame(this.animFrameId);
			this.animFrameId = null;
		}
		for (const effect of this.activeEffects) {
			effect.element.remove();
		}
		this.activeEffects = [];
	}

	// ── Effect implementations ───────────────────────────────────────────

	/** Earthquake: CSS shake on the entire container. */
	private shakeEffect(severity: number): void {
		const intensity = Math.max(2, Math.round(severity * 8));
		const duration = 1000;

		const el = document.createElement('div');
		el.className = `${EFFECT_CLASS_PREFIX}shake`;
		// Apply the shake directly to the container via a CSS animation on a transparent overlay
		el.style.cssText = `
			position: absolute; inset: 0; pointer-events: none; z-index: 100;
		`;
		this.container.appendChild(el);

		// Animate the container itself
		this.container.style.setProperty('--shake-intensity', `${intensity}px`);
		this.container.classList.add('gt-shaking');
		setTimeout(() => {
			this.container.classList.remove('gt-shaking');
		}, duration);

		this.trackEffect(el, duration);
	}

	/** Storm: dark overlay flash + brief white lightning. */
	private stormEffect(): void {
		const duration = 500;

		// Dark overlay
		const dark = document.createElement('div');
		dark.className = `${EFFECT_CLASS_PREFIX}storm-dark`;
		dark.style.cssText = `
			position: absolute; inset: 0; pointer-events: none; z-index: 100;
			background: rgba(0, 0, 0, 0.4);
			animation: gt-storm-dark ${duration}ms ease-out forwards;
		`;
		this.container.appendChild(dark);
		this.trackEffect(dark, duration);

		// Lightning flash (delayed slightly)
		setTimeout(() => {
			const flash = document.createElement('div');
			flash.className = `${EFFECT_CLASS_PREFIX}storm-flash`;
			flash.style.cssText = `
				position: absolute; inset: 0; pointer-events: none; z-index: 101;
				background: rgba(255, 255, 255, 0.3);
				animation: gt-storm-flash 150ms ease-out forwards;
			`;
			this.container.appendChild(flash);
			this.trackEffect(flash, 150);
		}, 100);
	}

	/** Market crash: red vignette border flash. */
	private vignetteEffect(): void {
		const duration = 1000;

		const el = document.createElement('div');
		el.className = `${EFFECT_CLASS_PREFIX}vignette`;
		el.style.cssText = `
			position: absolute; inset: 0; pointer-events: none; z-index: 100;
			box-shadow: inset 0 0 120px 40px rgba(220, 38, 38, 0.5);
			animation: gt-vignette ${duration}ms ease-out forwards;
		`;
		this.container.appendChild(el);
		this.trackEffect(el, duration);
	}

	/** Localized flash at a world position (projected to screen via a centered approach). */
	private flashEffect(
		x: number | undefined,
		y: number | undefined,
		color: [number, number, number],
		duration: number
	): void {
		const el = document.createElement('div');
		el.className = `${EFFECT_CLASS_PREFIX}flash`;

		// If we have coordinates, position the flash; otherwise, full-screen subtle flash
		if (x !== undefined && y !== undefined) {
			// We cannot easily project lon/lat to screen pixels without the deck.gl viewport.
			// Use a full-screen tinted flash as a reliable fallback.
			el.style.cssText = `
				position: absolute; inset: 0; pointer-events: none; z-index: 100;
				background: radial-gradient(circle at center, rgba(${color[0]}, ${color[1]}, ${color[2]}, 0.25) 0%, transparent 70%);
				animation: gt-flash ${duration}ms ease-out forwards;
			`;
		} else {
			el.style.cssText = `
				position: absolute; inset: 0; pointer-events: none; z-index: 100;
				background: rgba(${color[0]}, ${color[1]}, ${color[2]}, 0.15);
				animation: gt-flash ${duration}ms ease-out forwards;
			`;
		}

		this.container.appendChild(el);
		this.trackEffect(el, duration);
	}

	/** Research complete: blue sparkle/shimmer flash. */
	private sparkleEffect(): void {
		const duration = 500;

		const el = document.createElement('div');
		el.className = `${EFFECT_CLASS_PREFIX}sparkle`;
		el.style.cssText = `
			position: absolute; inset: 0; pointer-events: none; z-index: 100;
			background: radial-gradient(ellipse at 50% 30%, rgba(59, 130, 246, 0.3) 0%, rgba(139, 92, 246, 0.15) 40%, transparent 70%);
			animation: gt-sparkle ${duration}ms ease-out forwards;
		`;
		this.container.appendChild(el);
		this.trackEffect(el, duration);
	}

	// ── Internal helpers ─────────────────────────────────────────────────

	private trackEffect(element: HTMLElement, duration: number): void {
		this.activeEffects.push({
			element,
			startTime: performance.now(),
			duration,
		});
	}

	private startUpdateLoop(): void {
		const tick = () => {
			this.update();
			this.animFrameId = requestAnimationFrame(tick);
		};
		this.animFrameId = requestAnimationFrame(tick);
	}
}

// ── CSS injection ────────────────────────────────────────────────────────

let stylesInjected = false;

/** Inject the keyframe animation styles into the document head. Safe to call multiple times. */
export function injectEventEffectStyles(): void {
	if (stylesInjected) return;
	stylesInjected = true;

	const style = document.createElement('style');
	style.id = 'gt-event-effects';
	style.textContent = `
		/* Earthquake shake — applied to the container */
		.gt-shaking {
			animation: gt-shake 0.1s linear infinite;
		}

		@keyframes gt-shake {
			0%   { transform: translate(0, 0); }
			25%  { transform: translate(var(--shake-intensity, 4px), calc(-1 * var(--shake-intensity, 4px))); }
			50%  { transform: translate(calc(-1 * var(--shake-intensity, 4px)), var(--shake-intensity, 4px)); }
			75%  { transform: translate(var(--shake-intensity, 4px), var(--shake-intensity, 4px)); }
			100% { transform: translate(0, 0); }
		}

		/* Storm dark overlay */
		@keyframes gt-storm-dark {
			0%   { opacity: 1; }
			100% { opacity: 0; }
		}

		/* Storm lightning flash */
		@keyframes gt-storm-flash {
			0%   { opacity: 1; }
			100% { opacity: 0; }
		}

		/* Market crash red vignette */
		@keyframes gt-vignette {
			0%   { opacity: 1; }
			30%  { opacity: 0.8; }
			100% { opacity: 0; }
		}

		/* Generic flash (construction complete, disaster strike) */
		@keyframes gt-flash {
			0%   { opacity: 1; }
			100% { opacity: 0; }
		}

		/* Research sparkle */
		@keyframes gt-sparkle {
			0%   { opacity: 0; transform: scale(0.8); }
			30%  { opacity: 1; transform: scale(1.05); }
			100% { opacity: 0; transform: scale(1.2); }
		}
	`;

	document.head.appendChild(style);
}
