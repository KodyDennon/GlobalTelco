import { get } from 'svelte/store';
import { musicVolume, sfxVolume } from '$lib/stores/settings';

export type SfxName =
	| 'build'
	| 'demolish'
	| 'research_complete'
	| 'contract_signed'
	| 'error'
	| 'click'
	| 'earthquake'
	| 'storm'
	| 'flood'
	| 'cyber_glitch'
	| 'cash_register'
	| 'ui_open'
	| 'ui_close'
	| 'ui_tab'
	| 'ui_hover'
	| 'ui_slider'
	| 'notification'
	| 'disaster_alert'
	| 'achievement'
	| 'victory';

export type AmbienceZone = 'urban' | 'rural' | 'ocean' | 'infrastructure' | 'silence';

export type EraName =
	| 'telegraph'
	| 'telephone'
	| 'early_digital'
	| 'internet'
	| 'modern'
	| 'near_future';

/**
 * Central audio management using the Web Audio API.
 * All sounds are procedurally generated — no external audio files.
 * AudioContext is lazily initialized on the first user gesture (browser requirement).
 *
 * Phase 8.1 features:
 * - Era-specific ambient music (procedurally generated drone/pad per era)
 * - Era-specific sound palettes (telegraph clicks, digital tones, etc.)
 * - UI interaction sounds (panel open/close, tab switch, button hover, slider drag)
 * - Additional disaster SFX (flood, cyber glitch)
 * - Victory/achievement fanfare with escalating intensity
 * - Audio ducking during important notifications
 * - Respects prefers-reduced-motion media query
 */
export class AudioManager {
	private ctx: AudioContext | null = null;
	private masterGain: GainNode | null = null;
	private musicGain: GainNode | null = null;
	private sfxGain: GainNode | null = null;
	private ambienceGain: GainNode | null = null;
	private initialized = false;
	private unsubscribers: (() => void)[] = [];

	// Active ambience state
	private currentAmbienceZone: AmbienceZone = 'silence';
	private ambienceNodes: AudioNode[] = [];
	private ambienceTimers: ReturnType<typeof setTimeout>[] = [];
	private ambienceRunning = false;

	// Era music state
	private currentEra: EraName = 'internet';
	private musicNodes: AudioNode[] = [];
	private musicTimers: ReturnType<typeof setTimeout>[] = [];
	private musicRunning = false;

	// Audio ducking state
	private duckingActive = false;
	private preDuckMusicVolume = 0;

	// Reduced motion preference
	private prefersReducedMotion = false;

	async init(): Promise<void> {
		if (this.initialized) return;
		if (typeof AudioContext === 'undefined') return;

		// Check reduced motion preference
		if (typeof window !== 'undefined' && window.matchMedia) {
			const mql = window.matchMedia('(prefers-reduced-motion: reduce)');
			this.prefersReducedMotion = mql.matches;
			mql.addEventListener('change', (e) => {
				this.prefersReducedMotion = e.matches;
				// If user enables reduced motion, stop non-essential sounds
				if (e.matches) {
					this.stopMusic();
					this.stopAmbience();
				}
			});
		}

		try {
			this.ctx = new AudioContext();

			this.masterGain = this.ctx.createGain();
			this.masterGain.gain.value = 1.0;
			this.masterGain.connect(this.ctx.destination);

			this.musicGain = this.ctx.createGain();
			this.musicGain.connect(this.masterGain);

			this.sfxGain = this.ctx.createGain();
			this.sfxGain.connect(this.masterGain);

			this.ambienceGain = this.ctx.createGain();
			this.ambienceGain.connect(this.masterGain);

			this.setMusicVolume(get(musicVolume));
			this.setSfxVolume(get(sfxVolume));
			this.setAmbienceVolume(0.5);

			this.unsubscribers.push(
				musicVolume.subscribe((v) => this.setMusicVolume(v)),
				sfxVolume.subscribe((v) => this.setSfxVolume(v))
			);

			this.initialized = true;
		} catch {
			// Web Audio not supported in this browser
		}
	}

	private ensureContext(): boolean {
		if (!this.ctx || !this.initialized) return false;
		if (this.ctx.state === 'suspended') {
			this.ctx.resume();
		}
		return true;
	}

	/** Whether non-essential audio should be suppressed (prefers-reduced-motion). */
	private shouldSuppressNonEssential(): boolean {
		return this.prefersReducedMotion;
	}

	setMasterVolume(v: number): void {
		if (this.masterGain) {
			this.masterGain.gain.value = Math.max(0, Math.min(1, v));
		}
	}

	setMusicVolume(v: number): void {
		if (this.musicGain) {
			this.musicGain.gain.value = Math.max(0, Math.min(1, v));
		}
	}

	setSfxVolume(v: number): void {
		if (this.sfxGain) {
			this.sfxGain.gain.value = Math.max(0, Math.min(1, v));
		}
	}

	setAmbienceVolume(v: number): void {
		if (this.ambienceGain) {
			this.ambienceGain.gain.value = Math.max(0, Math.min(1, v));
		}
	}

	// ── SFX (one-shot procedural sounds) ──────────────────────────────

	playSfx(name: SfxName): void {
		if (!this.ensureContext() || !this.ctx || !this.sfxGain) return;

		// Suppress non-essential UI sounds when user prefers reduced motion
		if (this.shouldSuppressNonEssential()) {
			const essentialSounds: SfxName[] = [
				'error', 'disaster_alert', 'notification',
			];
			if (!essentialSounds.includes(name)) return;
		}

		const now = this.ctx.currentTime;

		switch (name) {
			case 'build':
				this.playTone(200, 'sine', 0.3, now, 0.075);
				this.playTone(400, 'sine', 0.3, now + 0.075, 0.075);
				break;

			case 'demolish':
				this.playFreqSweep(400, 100, 'sawtooth', 0.3, now, 0.2);
				break;

			case 'research_complete':
				this.playTone(523.25, 'sine', 0.25, now, 0.1);          // C5
				this.playTone(659.25, 'sine', 0.25, now + 0.1, 0.1);   // E5
				this.playTone(783.99, 'sine', 0.25, now + 0.2, 0.1);   // G5
				this.playTone(1046.5, 'sine', 0.3, now + 0.3, 0.1);    // C6
				break;

			case 'contract_signed':
				this.playTone(500, 'sine', 0.3, now, 0.05);
				this.playTone(500, 'sine', 0.3, now + 0.1, 0.05);
				break;

			case 'error':
				this.playTone(100, 'square', 0.25, now, 0.2);
				break;

			case 'click':
				this.playTone(1000, 'square', 0.15, now, 0.02);
				break;

			case 'earthquake': {
				const noise = this.createNoiseSource('brown');
				const gain = this.ctx.createGain();
				gain.gain.setValueAtTime(0, now);
				gain.gain.linearRampToValueAtTime(0.4, now + 0.3);
				gain.gain.linearRampToValueAtTime(0.5, now + 0.6);
				gain.gain.exponentialRampToValueAtTime(0.001, now + 1.0);
				const lp = this.ctx.createBiquadFilter();
				lp.type = 'lowpass';
				lp.frequency.value = 80;
				noise.connect(lp);
				lp.connect(gain);
				gain.connect(this.sfxGain);
				noise.start(now);
				noise.stop(now + 1.05);
				break;
			}

			case 'storm': {
				const noise = this.createNoiseSource('white');
				const bp = this.ctx.createBiquadFilter();
				bp.type = 'bandpass';
				bp.frequency.value = 2000;
				bp.Q.value = 0.5;
				const gain = this.ctx.createGain();
				gain.gain.setValueAtTime(0.3, now);
				gain.gain.exponentialRampToValueAtTime(0.001, now + 0.5);
				noise.connect(bp);
				bp.connect(gain);
				gain.connect(this.sfxGain);
				noise.start(now);
				noise.stop(now + 0.55);
				break;
			}

			case 'flood': {
				// Low rumbling water sound: brown noise through bandpass
				const noise = this.createNoiseSource('brown');
				const bp = this.ctx.createBiquadFilter();
				bp.type = 'bandpass';
				bp.frequency.value = 250;
				bp.Q.value = 0.8;
				const gain = this.ctx.createGain();
				gain.gain.setValueAtTime(0, now);
				gain.gain.linearRampToValueAtTime(0.35, now + 0.2);
				gain.gain.linearRampToValueAtTime(0.4, now + 0.5);
				gain.gain.exponentialRampToValueAtTime(0.001, now + 1.2);
				// Modulate filter for rushing water effect
				const lfo = this.ctx.createOscillator();
				lfo.type = 'sine';
				lfo.frequency.value = 3;
				const lfoGain = this.ctx.createGain();
				lfoGain.gain.value = 100;
				lfo.connect(lfoGain);
				lfoGain.connect(bp.frequency);
				lfo.start(now);
				lfo.stop(now + 1.25);
				noise.connect(bp);
				bp.connect(gain);
				gain.connect(this.sfxGain);
				noise.start(now);
				noise.stop(now + 1.25);
				break;
			}

			case 'cyber_glitch': {
				// Digital glitch: rapid random frequency changes + noise burst
				for (let i = 0; i < 6; i++) {
					const freq = 200 + Math.random() * 3000;
					const t = now + i * 0.04;
					this.playTone(freq, 'square', 0.2, t, 0.03);
				}
				// White noise burst
				const noise = this.createNoiseSource('white');
				const hp = this.ctx.createBiquadFilter();
				hp.type = 'highpass';
				hp.frequency.value = 4000;
				const gain = this.ctx.createGain();
				gain.gain.setValueAtTime(0.25, now);
				gain.gain.exponentialRampToValueAtTime(0.001, now + 0.3);
				noise.connect(hp);
				hp.connect(gain);
				gain.connect(this.sfxGain);
				noise.start(now);
				noise.stop(now + 0.35);
				break;
			}

			case 'cash_register':
				this.playTone(2000, 'sine', 0.25, now, 0.05);
				this.playTone(3000, 'sine', 0.2, now + 0.02, 0.08);
				this.playTone(2500, 'triangle', 0.15, now + 0.06, 0.09);
				break;

			// ── UI interaction sounds ────────────────────────────────

			case 'ui_open':
				// Soft ascending two-note chime
				this.playTone(600, 'sine', 0.12, now, 0.06);
				this.playTone(900, 'sine', 0.1, now + 0.06, 0.06);
				break;

			case 'ui_close':
				// Soft descending two-note chime
				this.playTone(900, 'sine', 0.1, now, 0.06);
				this.playTone(600, 'sine', 0.08, now + 0.06, 0.06);
				break;

			case 'ui_tab':
				// Quick single tick
				this.playTone(1200, 'sine', 0.08, now, 0.025);
				break;

			case 'ui_hover':
				// Very subtle high tick (barely audible)
				this.playTone(2000, 'sine', 0.04, now, 0.015);
				break;

			case 'ui_slider':
				// Quick blip — frequency varies with repeated calls for variety
				this.playTone(800 + Math.random() * 400, 'sine', 0.06, now, 0.02);
				break;

			case 'notification':
				// Gentle attention chime
				this.playTone(880, 'sine', 0.2, now, 0.1);
				this.playTone(1100, 'sine', 0.15, now + 0.12, 0.08);
				break;

			case 'disaster_alert': {
				// Urgent two-tone alert siren
				this.duckMusic(2.0);
				this.playTone(800, 'sine', 0.35, now, 0.15);
				this.playTone(600, 'sine', 0.35, now + 0.15, 0.15);
				this.playTone(800, 'sine', 0.35, now + 0.3, 0.15);
				this.playTone(600, 'sine', 0.35, now + 0.45, 0.15);
				break;
			}

			case 'achievement': {
				// Triumphant ascending fanfare — C major arpeggio + octave
				this.duckMusic(1.5);
				this.playTone(523.25, 'sine', 0.3, now, 0.12);        // C5
				this.playTone(659.25, 'sine', 0.3, now + 0.1, 0.12);  // E5
				this.playTone(783.99, 'sine', 0.3, now + 0.2, 0.12);  // G5
				this.playTone(1046.5, 'sine', 0.35, now + 0.3, 0.15); // C6
				// Sustain the top note with gentle decay
				this.playTone(1046.5, 'triangle', 0.2, now + 0.45, 0.3);
				break;
			}

			case 'victory': {
				// Grand victory fanfare — escalating intensity
				this.duckMusic(3.0);
				// First phrase: C major
				this.playTone(523.25, 'sine', 0.3, now, 0.15);        // C5
				this.playTone(659.25, 'sine', 0.3, now + 0.15, 0.15); // E5
				this.playTone(783.99, 'sine', 0.3, now + 0.3, 0.15);  // G5
				this.playTone(1046.5, 'sine', 0.35, now + 0.45, 0.2); // C6
				// Second phrase: higher, louder
				this.playTone(1174.7, 'sine', 0.35, now + 0.7, 0.15);  // D6
				this.playTone(1318.5, 'sine', 0.35, now + 0.85, 0.15); // E6
				this.playTone(1568.0, 'sine', 0.35, now + 1.0, 0.15);  // G6
				this.playTone(2093.0, 'sine', 0.4, now + 1.15, 0.25);  // C7
				// Final sustain chord
				this.playTone(1046.5, 'triangle', 0.2, now + 1.4, 0.5); // C6
				this.playTone(1318.5, 'triangle', 0.15, now + 1.4, 0.5); // E6
				this.playTone(1568.0, 'triangle', 0.15, now + 1.4, 0.5); // G6
				break;
			}
		}
	}

	// ── Era-specific sounds ──────────────────────────────────────────

	/**
	 * Play an era-appropriate variant of a generic action.
	 * Falls back to the default SFX if no era variant is defined.
	 */
	playEraSfx(action: 'build' | 'click' | 'notification', era?: EraName): void {
		if (!this.ensureContext() || !this.ctx || !this.sfxGain) return;
		if (this.shouldSuppressNonEssential()) return;

		const activeEra = era ?? this.currentEra;
		const now = this.ctx.currentTime;

		switch (action) {
			case 'build':
				switch (activeEra) {
					case 'telegraph':
						// Mechanical clicking/clacking (telegraph key sound)
						this.playTone(300, 'square', 0.2, now, 0.02);
						this.playTone(250, 'square', 0.15, now + 0.05, 0.02);
						this.playTone(350, 'square', 0.2, now + 0.1, 0.03);
						break;
					case 'telephone':
						// Rotary dial click + bell ding
						this.playTone(400, 'square', 0.15, now, 0.03);
						this.playTone(2000, 'sine', 0.2, now + 0.08, 0.1);
						break;
					case 'early_digital':
						// 8-bit style beep sequence
						this.playTone(440, 'square', 0.2, now, 0.05);
						this.playTone(660, 'square', 0.2, now + 0.06, 0.05);
						this.playTone(880, 'square', 0.25, now + 0.12, 0.06);
						break;
					case 'internet':
						// Modem-style ascending tones
						this.playFreqSweep(300, 800, 'sine', 0.2, now, 0.15);
						this.playTone(1200, 'sine', 0.15, now + 0.15, 0.05);
						break;
					case 'modern':
						// Clean digital confirmation
						this.playTone(600, 'sine', 0.25, now, 0.06);
						this.playTone(800, 'sine', 0.25, now + 0.06, 0.06);
						break;
					case 'near_future':
						// Futuristic shimmer
						this.playTone(800, 'sine', 0.15, now, 0.1);
						this.playTone(1200, 'triangle', 0.12, now + 0.03, 0.1);
						this.playTone(1600, 'sine', 0.1, now + 0.06, 0.12);
						break;
				}
				break;

			case 'click':
				switch (activeEra) {
					case 'telegraph':
					case 'telephone':
						// Mechanical click
						this.playTone(300, 'square', 0.12, now, 0.015);
						break;
					case 'early_digital':
						// 8-bit blip
						this.playTone(1500, 'square', 0.1, now, 0.02);
						break;
					default:
						// Standard click
						this.playTone(1000, 'square', 0.15, now, 0.02);
						break;
				}
				break;

			case 'notification':
				switch (activeEra) {
					case 'telegraph':
						// Telegraph ticker notification
						this.playTone(600, 'square', 0.15, now, 0.03);
						this.playTone(600, 'square', 0.15, now + 0.06, 0.03);
						this.playTone(800, 'square', 0.15, now + 0.12, 0.04);
						break;
					case 'telephone':
						// Telephone bell ring
						this.playTone(2000, 'sine', 0.2, now, 0.08);
						this.playTone(2500, 'sine', 0.15, now + 0.1, 0.08);
						break;
					case 'early_digital':
						// Retro computer beep
						this.playTone(1000, 'square', 0.2, now, 0.1);
						this.playTone(1200, 'square', 0.15, now + 0.12, 0.05);
						break;
					default:
						this.playSfx('notification');
						break;
				}
				break;
		}
	}

	// ── Era Music (procedural ambient drone/pad) ─────────────────────

	/**
	 * Start playing era-appropriate ambient music.
	 * Each era has a distinct procedurally generated soundscape.
	 */
	playMusic(era: EraName): void {
		if (!this.ensureContext() || !this.ctx || !this.musicGain) return;
		if (this.shouldSuppressNonEssential()) return;

		this.stopMusicInternal();
		this.currentEra = era;
		this.musicRunning = true;

		switch (era) {
			case 'telegraph':
				this.startTelegraphMusic();
				break;
			case 'telephone':
				this.startTelephoneMusic();
				break;
			case 'early_digital':
				this.startEarlyDigitalMusic();
				break;
			case 'internet':
				this.startInternetMusic();
				break;
			case 'modern':
				this.startModernMusic();
				break;
			case 'near_future':
				this.startNearFutureMusic();
				break;
		}
	}

	stopMusic(): void {
		this.stopMusicInternal();
	}

	/**
	 * Crossfade to a new era's music over the given duration (seconds).
	 */
	crossfadeMusic(newEra: EraName, duration: number = 2.0): void {
		if (!this.ctx || !this.musicGain) return;
		if (newEra === this.currentEra && this.musicRunning) return;
		if (this.shouldSuppressNonEssential()) return;

		const now = this.ctx.currentTime;
		const savedVolume = get(musicVolume);

		// Fade out current music
		this.musicGain.gain.linearRampToValueAtTime(0, now + duration);

		setTimeout(() => {
			this.stopMusicInternal();
			this.playMusic(newEra);
			if (this.ctx && this.musicGain) {
				const t = this.ctx.currentTime;
				this.musicGain.gain.setValueAtTime(0, t);
				this.musicGain.gain.linearRampToValueAtTime(savedVolume, t + duration);
			}
		}, duration * 1000);
	}

	getCurrentEra(): EraName {
		return this.currentEra;
	}

	// ── Audio Ducking ────────────────────────────────────────────────

	/**
	 * Temporarily lower music volume for important notifications/alerts.
	 * Automatically restores volume after the given duration (seconds).
	 */
	private duckMusic(durationSec: number): void {
		if (!this.ctx || !this.musicGain || this.duckingActive) return;

		this.duckingActive = true;
		this.preDuckMusicVolume = this.musicGain.gain.value;

		const now = this.ctx.currentTime;
		const duckedVolume = this.preDuckMusicVolume * 0.25;

		// Quick duck down
		this.musicGain.gain.linearRampToValueAtTime(duckedVolume, now + 0.1);

		// Also duck ambience slightly
		if (this.ambienceGain) {
			const ambienceVol = this.ambienceGain.gain.value;
			this.ambienceGain.gain.linearRampToValueAtTime(ambienceVol * 0.4, now + 0.1);

			// Restore ambience
			setTimeout(() => {
				if (this.ctx && this.ambienceGain) {
					const t = this.ctx.currentTime;
					this.ambienceGain.gain.linearRampToValueAtTime(ambienceVol, t + 0.5);
				}
			}, durationSec * 1000);
		}

		// Restore music
		setTimeout(() => {
			if (this.ctx && this.musicGain) {
				const t = this.ctx.currentTime;
				this.musicGain.gain.linearRampToValueAtTime(this.preDuckMusicVolume, t + 0.5);
			}
			this.duckingActive = false;
		}, durationSec * 1000);
	}

	// ── Ambience (procedural noise loops) ─────────────────────────────

	startAmbience(zone: AmbienceZone): void {
		if (!this.ensureContext() || !this.ctx || !this.ambienceGain) return;
		if (this.shouldSuppressNonEssential()) return;
		this.stopAmbienceInternal();

		if (zone === 'silence') {
			this.currentAmbienceZone = 'silence';
			return;
		}

		this.currentAmbienceZone = zone;
		this.ambienceRunning = true;

		switch (zone) {
			case 'urban':
				this.startUrbanAmbience();
				break;
			case 'rural':
				this.startRuralAmbience();
				break;
			case 'ocean':
				this.startOceanAmbience();
				break;
			case 'infrastructure':
				this.startInfrastructureAmbience();
				break;
		}
	}

	stopAmbience(): void {
		this.stopAmbienceInternal();
		this.currentAmbienceZone = 'silence';
	}

	crossfadeAmbience(newZone: AmbienceZone): void {
		if (!this.ctx || !this.ambienceGain) return;
		if (newZone === this.currentAmbienceZone) return;

		const now = this.ctx.currentTime;
		const fadeDuration = 1.5;

		// Fade out current ambience
		this.ambienceGain.gain.linearRampToValueAtTime(0, now + fadeDuration);

		setTimeout(() => {
			this.stopAmbienceInternal();
			if (this.ambienceGain) {
				this.ambienceGain.gain.value = 0;
			}
			this.startAmbience(newZone);
			if (this.ctx && this.ambienceGain) {
				const t = this.ctx.currentTime;
				this.ambienceGain.gain.linearRampToValueAtTime(
					this.ambienceGain.gain.value || 0.5,
					t
				);
				this.ambienceGain.gain.linearRampToValueAtTime(0.5, t + fadeDuration);
			}
		}, fadeDuration * 1000);
	}

	getCurrentZone(): AmbienceZone {
		return this.currentAmbienceZone;
	}

	// ── Notification chimes ───────────────────────────────────────────

	playNotification(priority: 'critical' | 'important' | 'info'): void {
		if (!this.ensureContext() || !this.ctx || !this.sfxGain) return;
		const now = this.ctx.currentTime;

		switch (priority) {
			case 'critical':
				this.duckMusic(2.0);
				this.playTone(600, 'sine', 0.35, now, 0.08);
				this.playTone(800, 'sine', 0.35, now + 0.1, 0.08);
				this.playTone(1000, 'sine', 0.4, now + 0.2, 0.1);
				break;

			case 'important':
				this.duckMusic(1.0);
				this.playTone(500, 'sine', 0.25, now, 0.12);
				this.playTone(700, 'sine', 0.25, now + 0.15, 0.12);
				break;

			case 'info':
				this.playTone(800, 'sine', 0.15, now, 0.15);
				break;
		}
	}

	// ── Backward-compatible event-to-SFX mapping ──────────────────────

	private eventSoundMap: Record<string, SfxName> = {
		ConstructionStarted: 'build',
		ConstructionCompleted: 'build',
		RevenueEarned: 'cash_register',
		DisasterStruck: 'disaster_alert',
		InfrastructureDamaged: 'disaster_alert',
		BankruptcyDeclared: 'error',
		BailoutTaken: 'error',
		InsolvencyWarning: 'error',
		ResearchStarted: 'notification',
		ResearchCompleted: 'research_complete',
		AchievementUnlocked: 'achievement',
		ContractProposed: 'notification',
		ContractAccepted: 'contract_signed',
		AcquisitionProposed: 'notification',
		AcquisitionAccepted: 'contract_signed',
		AcquisitionRejected: 'error',
		AuctionWon: 'cash_register',
		AuctionStarted: 'notification',
		SabotageCompleted: 'cyber_glitch',
		SabotageDetected: 'disaster_alert',
		EspionageCompleted: 'notification',
		EspionageDetected: 'disaster_alert',
		VictoryAchieved: 'victory',
		ScandalOccurred: 'error',
		MergerCompleted: 'contract_signed',
		LobbyingSucceeded: 'notification',
		LobbyingFailed: 'error',
		InsurancePayout: 'cash_register',
		NodeBuilt: 'build',
		EdgeBuilt: 'build',
		NodeDestroyed: 'demolish',
		RepairCompleted: 'build',
		CorporationFounded: 'achievement',
		WeatherStarted: 'storm',
	};

	playEventSound(event: string | Record<string, unknown>): void {
		let eventName: string;
		if (typeof event === 'string') {
			const match = event.match(/^(\w+)/);
			eventName = match ? match[1] : event;
		} else {
			eventName = Object.keys(event)[0] ?? '';
		}
		const sfx = this.eventSoundMap[eventName];
		if (sfx) this.playSfx(sfx);
	}

	playUiSound(sound: SfxName): void {
		this.playSfx(sound);
	}

	// ── Intensity (music/ambience mood control) ──────────────────────

	/**
	 * Adjust overall audio intensity (0 = calm, 1 = tense).
	 * Scales ambience volume and can be driven by game state
	 * (e.g. low cash, disasters, competition pressure).
	 */
	setIntensity(level: number): void {
		const clamped = Math.max(0, Math.min(1, level));
		// Scale ambience volume: calm = 60% of current, tense = 120%
		if (this.ambienceGain && this.ctx) {
			const base = 0.3 + clamped * 0.5;
			this.ambienceGain.gain.linearRampToValueAtTime(
				base,
				this.ctx.currentTime + 0.5
			);
		}
		// Scale music filter brightness if music gain exists
		if (this.musicGain && this.ctx) {
			const vol = 0.1 + clamped * 0.15;
			this.musicGain.gain.linearRampToValueAtTime(
				Math.min(vol, get(musicVolume)),
				this.ctx.currentTime + 0.5
			);
		}
	}

	// ── Cleanup ───────────────────────────────────────────────────────

	dispose(): void {
		this.stopAmbienceInternal();
		this.stopMusicInternal();
		for (const unsub of this.unsubscribers) unsub();
		this.unsubscribers = [];
		if (this.ctx) {
			this.ctx.close();
			this.ctx = null;
		}
		this.initialized = false;
	}

	// ── Private helpers ───────────────────────────────────────────────

	private playTone(
		freq: number,
		type: OscillatorType,
		volume: number,
		startTime: number,
		duration: number
	): void {
		if (!this.ctx || !this.sfxGain) return;
		const osc = this.ctx.createOscillator();
		const gain = this.ctx.createGain();
		osc.type = type;
		osc.frequency.value = freq;
		gain.gain.setValueAtTime(volume, startTime);
		gain.gain.exponentialRampToValueAtTime(0.001, startTime + duration);
		osc.connect(gain);
		gain.connect(this.sfxGain);
		osc.start(startTime);
		osc.stop(startTime + duration + 0.05);
	}

	private playMusicTone(
		freq: number,
		type: OscillatorType,
		volume: number,
		startTime: number,
		duration: number
	): OscillatorNode | null {
		if (!this.ctx || !this.musicGain) return null;
		const osc = this.ctx.createOscillator();
		const gain = this.ctx.createGain();
		osc.type = type;
		osc.frequency.value = freq;
		gain.gain.setValueAtTime(volume, startTime);
		gain.gain.exponentialRampToValueAtTime(0.001, startTime + duration);
		osc.connect(gain);
		gain.connect(this.musicGain);
		osc.start(startTime);
		osc.stop(startTime + duration + 0.05);
		this.musicNodes.push(osc);
		this.musicNodes.push(gain);
		return osc;
	}

	private playFreqSweep(
		startFreq: number,
		endFreq: number,
		type: OscillatorType,
		volume: number,
		startTime: number,
		duration: number
	): void {
		if (!this.ctx || !this.sfxGain) return;
		const osc = this.ctx.createOscillator();
		const gain = this.ctx.createGain();
		osc.type = type;
		osc.frequency.setValueAtTime(startFreq, startTime);
		osc.frequency.exponentialRampToValueAtTime(endFreq, startTime + duration);
		gain.gain.setValueAtTime(volume, startTime);
		gain.gain.exponentialRampToValueAtTime(0.001, startTime + duration);
		osc.connect(gain);
		gain.connect(this.sfxGain);
		osc.start(startTime);
		osc.stop(startTime + duration + 0.05);
	}

	private createNoiseSource(color: 'white' | 'brown' | 'pink'): AudioBufferSourceNode {
		const ctx = this.ctx!;
		const sampleRate = ctx.sampleRate;
		const length = sampleRate * 2; // 2 seconds of noise
		const buffer = ctx.createBuffer(1, length, sampleRate);
		const data = buffer.getChannelData(0);

		if (color === 'white') {
			for (let i = 0; i < length; i++) {
				data[i] = Math.random() * 2 - 1;
			}
		} else if (color === 'pink') {
			// Pink noise approximation using Paul Kellet's algorithm
			let b0 = 0, b1 = 0, b2 = 0, b3 = 0, b4 = 0, b5 = 0, b6 = 0;
			for (let i = 0; i < length; i++) {
				const white = Math.random() * 2 - 1;
				b0 = 0.99886 * b0 + white * 0.0555179;
				b1 = 0.99332 * b1 + white * 0.0750759;
				b2 = 0.96900 * b2 + white * 0.1538520;
				b3 = 0.86650 * b3 + white * 0.3104856;
				b4 = 0.55000 * b4 + white * 0.5329522;
				b5 = -0.7616 * b5 - white * 0.0168980;
				data[i] = (b0 + b1 + b2 + b3 + b4 + b5 + b6 + white * 0.5362) * 0.11;
				b6 = white * 0.115926;
			}
		} else {
			// Brown noise: integrated white noise
			let last = 0;
			for (let i = 0; i < length; i++) {
				const white = Math.random() * 2 - 1;
				last = (last + 0.02 * white) / 1.02;
				data[i] = last * 3.5;
			}
		}

		const source = ctx.createBufferSource();
		source.buffer = buffer;
		source.loop = true;
		return source;
	}

	// ── Music zone implementations (per era) ─────────────────────────

	private stopMusicInternal(): void {
		this.musicRunning = false;
		for (const timer of this.musicTimers) clearTimeout(timer);
		this.musicTimers = [];
		for (const node of this.musicNodes) {
			try {
				if (node instanceof AudioBufferSourceNode) node.stop();
				else if (node instanceof OscillatorNode) node.stop();
				node.disconnect();
			} catch {
				// already stopped
			}
		}
		this.musicNodes = [];
	}

	private trackMusicNode(node: AudioNode): void {
		this.musicNodes.push(node);
	}

	private trackMusicTimer(timer: ReturnType<typeof setTimeout>): void {
		this.musicTimers.push(timer);
	}

	/**
	 * Telegraph era (~1850s): Sparse, mechanical.
	 * Low drone + occasional telegraph clicking patterns.
	 */
	private startTelegraphMusic(): void {
		if (!this.ctx || !this.musicGain) return;

		// Low sine drone at C2 (65Hz) — very quiet
		const drone = this.ctx.createOscillator();
		drone.type = 'sine';
		drone.frequency.value = 65.41;
		const droneGain = this.ctx.createGain();
		droneGain.gain.value = 0.06;
		drone.connect(droneGain);
		droneGain.connect(this.musicGain);
		drone.start();
		this.trackMusicNode(drone);
		this.trackMusicNode(droneGain);

		// Fifth harmonic for warmth (G2, 98Hz)
		const fifth = this.ctx.createOscillator();
		fifth.type = 'sine';
		fifth.frequency.value = 98.0;
		const fifthGain = this.ctx.createGain();
		fifthGain.gain.value = 0.03;
		fifth.connect(fifthGain);
		fifthGain.connect(this.musicGain);
		fifth.start();
		this.trackMusicNode(fifth);
		this.trackMusicNode(fifthGain);

		// Occasional telegraph key clicks (rhythmic Morse-code-like patterns)
		const scheduleClick = () => {
			if (!this.musicRunning || !this.ctx || !this.musicGain) return;
			const delay = 2000 + Math.random() * 5000;
			const timer = setTimeout(() => {
				if (!this.musicRunning || !this.ctx || !this.musicGain) return;
				const now = this.ctx.currentTime;
				// Random 3-6 click pattern (dot and dash)
				const clicks = 3 + Math.floor(Math.random() * 4);
				let t = now;
				for (let i = 0; i < clicks; i++) {
					const isDash = Math.random() > 0.6;
					const dur = isDash ? 0.08 : 0.03;
					this.playMusicTone(400, 'square', 0.04, t, dur);
					t += dur + 0.05 + Math.random() * 0.03;
				}
				scheduleClick();
			}, delay);
			this.trackMusicTimer(timer);
		};
		scheduleClick();
	}

	/**
	 * Telephone era (~1900s): Warm, slightly jazzy.
	 * Warm pad + occasional bell tones.
	 */
	private startTelephoneMusic(): void {
		if (!this.ctx || !this.musicGain) return;

		// Warm pad: low triangle waves forming a chord (C3, E3, G3)
		const notes = [130.81, 164.81, 196.0]; // C3, E3, G3
		for (const freq of notes) {
			const osc = this.ctx.createOscillator();
			osc.type = 'triangle';
			osc.frequency.value = freq;
			const gain = this.ctx.createGain();
			gain.gain.value = 0.035;
			osc.connect(gain);
			gain.connect(this.musicGain);
			osc.start();
			this.trackMusicNode(osc);
			this.trackMusicNode(gain);
		}

		// Volume modulation for breathing effect
		const lfo = this.ctx.createOscillator();
		lfo.type = 'sine';
		lfo.frequency.value = 0.08; // Very slow
		const lfoGain = this.ctx.createGain();
		lfoGain.gain.value = 0.015;
		// Connect LFO to first note's gain
		lfo.connect(lfoGain);
		// We just modulate the music gain slightly
		lfoGain.connect(this.musicGain.gain);
		lfo.start();
		this.trackMusicNode(lfo);
		this.trackMusicNode(lfoGain);

		// Occasional bell tones (telephone ring fragments)
		const scheduleBell = () => {
			if (!this.musicRunning || !this.ctx || !this.musicGain) return;
			const delay = 4000 + Math.random() * 8000;
			const timer = setTimeout(() => {
				if (!this.musicRunning || !this.ctx || !this.musicGain) return;
				const now = this.ctx.currentTime;
				const bellFreq = 1800 + Math.random() * 400;
				this.playMusicTone(bellFreq, 'sine', 0.03, now, 0.2);
				scheduleBell();
			}, delay);
			this.trackMusicTimer(timer);
		};
		scheduleBell();
	}

	/**
	 * Early Digital era (~1970s): Synthetic, retro computing.
	 * Square wave arpeggios + filtered noise.
	 */
	private startEarlyDigitalMusic(): void {
		if (!this.ctx || !this.musicGain) return;

		// Base pad: filtered square wave at A2
		const pad = this.ctx.createOscillator();
		pad.type = 'square';
		pad.frequency.value = 110;
		const padFilter = this.ctx.createBiquadFilter();
		padFilter.type = 'lowpass';
		padFilter.frequency.value = 400;
		padFilter.Q.value = 2;
		const padGain = this.ctx.createGain();
		padGain.gain.value = 0.04;
		pad.connect(padFilter);
		padFilter.connect(padGain);
		padGain.connect(this.musicGain);
		pad.start();
		this.trackMusicNode(pad);
		this.trackMusicNode(padFilter);
		this.trackMusicNode(padGain);

		// Subtle arpeggio pattern
		const arpNotes = [220, 277.18, 329.63, 440]; // A3, C#4, E4, A4
		let arpIndex = 0;
		const scheduleArp = () => {
			if (!this.musicRunning || !this.ctx || !this.musicGain) return;
			const delay = 400 + Math.random() * 200;
			const timer = setTimeout(() => {
				if (!this.musicRunning || !this.ctx || !this.musicGain) return;
				const now = this.ctx.currentTime;
				this.playMusicTone(arpNotes[arpIndex], 'square', 0.025, now, 0.15);
				arpIndex = (arpIndex + 1) % arpNotes.length;
				scheduleArp();
			}, delay);
			this.trackMusicTimer(timer);
		};
		scheduleArp();
	}

	/**
	 * Internet era (~1990s): Ambient electronic, dial-up echoes.
	 * Sine pad chord + gentle digital textures.
	 */
	private startInternetMusic(): void {
		if (!this.ctx || !this.musicGain) return;

		// Ambient pad: D minor chord (D3, F3, A3)
		const notes = [146.83, 174.61, 220.0];
		for (const freq of notes) {
			const osc = this.ctx.createOscillator();
			osc.type = 'sine';
			osc.frequency.value = freq;
			const gain = this.ctx.createGain();
			gain.gain.value = 0.04;
			osc.connect(gain);
			gain.connect(this.musicGain);
			osc.start();
			this.trackMusicNode(osc);
			this.trackMusicNode(gain);
		}

		// Very subtle pink noise layer (digital texture)
		const noise = this.createNoiseSource('pink');
		const bp = this.ctx.createBiquadFilter();
		bp.type = 'bandpass';
		bp.frequency.value = 1000;
		bp.Q.value = 0.5;
		const noiseGain = this.ctx.createGain();
		noiseGain.gain.value = 0.01;
		noise.connect(bp);
		bp.connect(noiseGain);
		noiseGain.connect(this.musicGain);
		noise.start();
		this.trackMusicNode(noise);
		this.trackMusicNode(bp);
		this.trackMusicNode(noiseGain);

		// Breathing LFO
		const lfo = this.ctx.createOscillator();
		lfo.type = 'sine';
		lfo.frequency.value = 0.06;
		const lfoGain = this.ctx.createGain();
		lfoGain.gain.value = 0.02;
		lfo.connect(lfoGain);
		lfoGain.connect(this.musicGain.gain);
		lfo.start();
		this.trackMusicNode(lfo);
		this.trackMusicNode(lfoGain);
	}

	/**
	 * Modern era (~2010s): Clean, minimal electronic.
	 * Wide stereo sine pads with gentle filtering.
	 */
	private startModernMusic(): void {
		if (!this.ctx || !this.musicGain) return;

		// E minor 7 pad (E3, G3, B3, D4)
		const notes = [164.81, 196.0, 246.94, 293.66];
		for (const freq of notes) {
			const osc = this.ctx.createOscillator();
			osc.type = 'sine';
			osc.frequency.value = freq;

			// Gentle low-pass for warmth
			const filter = this.ctx.createBiquadFilter();
			filter.type = 'lowpass';
			filter.frequency.value = 800;

			const gain = this.ctx.createGain();
			gain.gain.value = 0.03;
			osc.connect(filter);
			filter.connect(gain);
			gain.connect(this.musicGain);
			osc.start();
			this.trackMusicNode(osc);
			this.trackMusicNode(filter);
			this.trackMusicNode(gain);
		}

		// Slow LFO on filter cutoff for movement
		const filterLfo = this.ctx.createOscillator();
		filterLfo.type = 'sine';
		filterLfo.frequency.value = 0.04; // Very slow sweep
		const filterLfoGain = this.ctx.createGain();
		filterLfoGain.gain.value = 300;
		filterLfo.connect(filterLfoGain);
		// We connect it to the overall music gain for subtle motion
		filterLfo.start();
		this.trackMusicNode(filterLfo);
		this.trackMusicNode(filterLfoGain);
	}

	/**
	 * Near Future era (~2030s): Ethereal, shimmering, sci-fi.
	 * Detuned harmonics, high overtones, subtle modulation.
	 */
	private startNearFutureMusic(): void {
		if (!this.ctx || !this.musicGain) return;

		// Shimmering pad: slightly detuned pairs
		const pairs = [
			[220, 220.5],     // A3 pair
			[329.63, 330.1],  // E4 pair
			[440, 441],       // A4 pair (1Hz beat)
		];

		for (const [f1, f2] of pairs) {
			for (const freq of [f1, f2]) {
				const osc = this.ctx.createOscillator();
				osc.type = 'sine';
				osc.frequency.value = freq;
				const gain = this.ctx.createGain();
				gain.gain.value = 0.025;
				osc.connect(gain);
				gain.connect(this.musicGain);
				osc.start();
				this.trackMusicNode(osc);
				this.trackMusicNode(gain);
			}
		}

		// High shimmer overtone
		const shimmer = this.ctx.createOscillator();
		shimmer.type = 'sine';
		shimmer.frequency.value = 1760; // A6
		const shimmerGain = this.ctx.createGain();
		shimmerGain.gain.value = 0.008;
		shimmer.connect(shimmerGain);
		shimmerGain.connect(this.musicGain);
		shimmer.start();
		this.trackMusicNode(shimmer);
		this.trackMusicNode(shimmerGain);

		// Shimmer volume LFO
		const lfo = this.ctx.createOscillator();
		lfo.type = 'sine';
		lfo.frequency.value = 0.2;
		const lfoGain = this.ctx.createGain();
		lfoGain.gain.value = 0.006;
		lfo.connect(lfoGain);
		lfoGain.connect(shimmerGain.gain);
		lfo.start();
		this.trackMusicNode(lfo);
		this.trackMusicNode(lfoGain);
	}

	// ── Ambience zone implementations ─────────────────────────────────

	private stopAmbienceInternal(): void {
		this.ambienceRunning = false;
		for (const timer of this.ambienceTimers) clearTimeout(timer);
		this.ambienceTimers = [];
		for (const node of this.ambienceNodes) {
			try {
				if (node instanceof AudioBufferSourceNode) node.stop();
				else if (node instanceof OscillatorNode) node.stop();
				node.disconnect();
			} catch {
				// already stopped
			}
		}
		this.ambienceNodes = [];
	}

	private trackNode(node: AudioNode): void {
		this.ambienceNodes.push(node);
	}

	private trackTimer(timer: ReturnType<typeof setTimeout>): void {
		this.ambienceTimers.push(timer);
	}

	private startUrbanAmbience(): void {
		if (!this.ctx || !this.ambienceGain) return;

		// Brown noise base (traffic rumble)
		const noise = this.createNoiseSource('brown');
		const lp = this.ctx.createBiquadFilter();
		lp.type = 'lowpass';
		lp.frequency.value = 300;
		const noiseGain = this.ctx.createGain();
		noiseGain.gain.value = 0.15;
		noise.connect(lp);
		lp.connect(noiseGain);
		noiseGain.connect(this.ambienceGain);
		noise.start();
		this.trackNode(noise);
		this.trackNode(lp);
		this.trackNode(noiseGain);

		// Random car horn beeps at intervals
		const scheduleHorn = () => {
			if (!this.ambienceRunning || !this.ctx || !this.ambienceGain) return;
			const delay = 3000 + Math.random() * 8000;
			const timer = setTimeout(() => {
				if (!this.ambienceRunning || !this.ctx || !this.ambienceGain) return;
				const now = this.ctx.currentTime;
				const freq = 350 + Math.random() * 150;
				const osc = this.ctx.createOscillator();
				osc.type = 'square';
				osc.frequency.value = freq;
				const g = this.ctx.createGain();
				g.gain.setValueAtTime(0.04, now);
				g.gain.exponentialRampToValueAtTime(0.001, now + 0.15);
				osc.connect(g);
				g.connect(this.ambienceGain);
				osc.start(now);
				osc.stop(now + 0.2);
				scheduleHorn();
			}, delay);
			this.trackTimer(timer);
		};
		scheduleHorn();
	}

	private startRuralAmbience(): void {
		if (!this.ctx || !this.ambienceGain) return;

		// Gentle wind (filtered white noise, very low)
		const wind = this.createNoiseSource('white');
		const bp = this.ctx.createBiquadFilter();
		bp.type = 'bandpass';
		bp.frequency.value = 500;
		bp.Q.value = 0.3;
		const windGain = this.ctx.createGain();
		windGain.gain.value = 0.04;
		wind.connect(bp);
		bp.connect(windGain);
		windGain.connect(this.ambienceGain);
		wind.start();
		this.trackNode(wind);
		this.trackNode(bp);
		this.trackNode(windGain);

		// Wind volume modulation via LFO
		const lfo = this.ctx.createOscillator();
		lfo.type = 'sine';
		lfo.frequency.value = 0.15;
		const lfoGain = this.ctx.createGain();
		lfoGain.gain.value = 0.02;
		lfo.connect(lfoGain);
		lfoGain.connect(windGain.gain);
		lfo.start();
		this.trackNode(lfo);
		this.trackNode(lfoGain);

		// Occasional bird chirps
		const scheduleBird = () => {
			if (!this.ambienceRunning || !this.ctx || !this.ambienceGain) return;
			const delay = 2000 + Math.random() * 6000;
			const timer = setTimeout(() => {
				if (!this.ambienceRunning || !this.ctx || !this.ambienceGain) return;
				const now = this.ctx.currentTime;
				const baseFreq = 2500 + Math.random() * 1500;
				// A quick two-note chirp
				const osc = this.ctx.createOscillator();
				osc.type = 'sine';
				osc.frequency.setValueAtTime(baseFreq, now);
				osc.frequency.setValueAtTime(baseFreq * 1.2, now + 0.05);
				const g = this.ctx.createGain();
				g.gain.setValueAtTime(0.05, now);
				g.gain.exponentialRampToValueAtTime(0.001, now + 0.1);
				osc.connect(g);
				g.connect(this.ambienceGain);
				osc.start(now);
				osc.stop(now + 0.15);
				scheduleBird();
			}, delay);
			this.trackTimer(timer);
		};
		scheduleBird();
	}

	private startOceanAmbience(): void {
		if (!this.ctx || !this.ambienceGain) return;

		// Brown noise through low-pass (waves)
		const noise = this.createNoiseSource('brown');
		const lp = this.ctx.createBiquadFilter();
		lp.type = 'lowpass';
		lp.frequency.value = 400;
		lp.Q.value = 1.0;
		const noiseGain = this.ctx.createGain();
		noiseGain.gain.value = 0.2;
		noise.connect(lp);
		lp.connect(noiseGain);
		noiseGain.connect(this.ambienceGain);
		noise.start();
		this.trackNode(noise);
		this.trackNode(lp);
		this.trackNode(noiseGain);

		// Wave-like volume modulation via LFO
		const lfo = this.ctx.createOscillator();
		lfo.type = 'sine';
		lfo.frequency.value = 0.12; // ~8 second wave cycle
		const lfoGain = this.ctx.createGain();
		lfoGain.gain.value = 0.1;
		lfo.connect(lfoGain);
		lfoGain.connect(noiseGain.gain);
		lfo.start();
		this.trackNode(lfo);
		this.trackNode(lfoGain);

		// Filter frequency modulation for wave shape
		const filterLfo = this.ctx.createOscillator();
		filterLfo.type = 'sine';
		filterLfo.frequency.value = 0.08;
		const filterLfoGain = this.ctx.createGain();
		filterLfoGain.gain.value = 200;
		filterLfo.connect(filterLfoGain);
		filterLfoGain.connect(lp.frequency);
		filterLfo.start();
		this.trackNode(filterLfo);
		this.trackNode(filterLfoGain);
	}

	private startInfrastructureAmbience(): void {
		if (!this.ctx || !this.ambienceGain) return;

		// 60Hz electrical hum
		const hum60 = this.ctx.createOscillator();
		hum60.type = 'sine';
		hum60.frequency.value = 60;
		const gain60 = this.ctx.createGain();
		gain60.gain.value = 0.06;
		hum60.connect(gain60);
		gain60.connect(this.ambienceGain);
		hum60.start();
		this.trackNode(hum60);
		this.trackNode(gain60);

		// 120Hz harmonic
		const hum120 = this.ctx.createOscillator();
		hum120.type = 'sine';
		hum120.frequency.value = 120;
		const gain120 = this.ctx.createGain();
		gain120.gain.value = 0.03;
		hum120.connect(gain120);
		gain120.connect(this.ambienceGain);
		hum120.start();
		this.trackNode(hum120);
		this.trackNode(gain120);

		// 180Hz harmonic (subtle)
		const hum180 = this.ctx.createOscillator();
		hum180.type = 'sine';
		hum180.frequency.value = 180;
		const gain180 = this.ctx.createGain();
		gain180.gain.value = 0.015;
		hum180.connect(gain180);
		gain180.connect(this.ambienceGain);
		hum180.start();
		this.trackNode(hum180);
		this.trackNode(gain180);

		// Slight volume flutter
		const lfo = this.ctx.createOscillator();
		lfo.type = 'sine';
		lfo.frequency.value = 0.3;
		const lfoGain = this.ctx.createGain();
		lfoGain.gain.value = 0.01;
		lfo.connect(lfoGain);
		lfoGain.connect(gain60.gain);
		lfo.start();
		this.trackNode(lfo);
		this.trackNode(lfoGain);
	}
}

// Singleton
export const audioManager = new AudioManager();
