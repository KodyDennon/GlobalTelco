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
	| 'cash_register';

export type AmbienceZone = 'urban' | 'rural' | 'ocean' | 'infrastructure' | 'silence';

/**
 * Central audio management using the Web Audio API.
 * All sounds are procedurally generated — no external audio files.
 * AudioContext is lazily initialized on the first user gesture (browser requirement).
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

	async init(): Promise<void> {
		if (this.initialized) return;
		if (typeof AudioContext === 'undefined') return;

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

			case 'cash_register':
				this.playTone(2000, 'sine', 0.25, now, 0.05);
				this.playTone(3000, 'sine', 0.2, now + 0.02, 0.08);
				this.playTone(2500, 'triangle', 0.15, now + 0.06, 0.09);
				break;
		}
	}

	// ── Ambience (procedural noise loops) ─────────────────────────────

	startAmbience(zone: AmbienceZone): void {
		if (!this.ensureContext() || !this.ctx || !this.ambienceGain) return;
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
				this.playTone(600, 'sine', 0.35, now, 0.08);
				this.playTone(800, 'sine', 0.35, now + 0.1, 0.08);
				this.playTone(1000, 'sine', 0.4, now + 0.2, 0.1);
				break;

			case 'important':
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
		DisasterStruck: 'earthquake',
		BankruptcyDeclared: 'error',
		BailoutTaken: 'error',
		ResearchCompleted: 'research_complete',
		AchievementUnlocked: 'research_complete',
		AcquisitionAccepted: 'contract_signed',
		AuctionWon: 'cash_register',
		SabotageCompleted: 'error',
		EspionageDetected: 'error',
		VictoryAchieved: 'research_complete',
		ScandalOccurred: 'error'
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

	private createNoiseSource(color: 'white' | 'brown'): AudioBufferSourceNode {
		const ctx = this.ctx!;
		const sampleRate = ctx.sampleRate;
		const length = sampleRate * 2; // 2 seconds of noise
		const buffer = ctx.createBuffer(1, length, sampleRate);
		const data = buffer.getChannelData(0);

		if (color === 'white') {
			for (let i = 0; i < length; i++) {
				data[i] = Math.random() * 2 - 1;
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
