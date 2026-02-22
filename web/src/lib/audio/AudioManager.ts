import { get } from 'svelte/store';
import { musicVolume, sfxVolume } from '$lib/stores/settings';

type SoundId =
	| 'build'
	| 'complete'
	| 'cash'
	| 'alarm'
	| 'crash'
	| 'discovery'
	| 'achievement'
	| 'click'
	| 'open'
	| 'close'
	| 'error';

interface AudioLayer {
	gain: GainNode;
	sources: Map<string, AudioBuffer>;
}

class AudioManager {
	private ctx: AudioContext | null = null;
	private musicGain: GainNode | null = null;
	private sfxGain: GainNode | null = null;
	private masterGain: GainNode | null = null;
	private initialized = false;
	private muted = false;
	private currentMusicSource: AudioBufferSourceNode | null = null;
	private ambient: AmbientMusicGenerator | null = null;
	private unsubscribers: (() => void)[] = [];

	// Map event types to sound categories
	private eventSoundMap: Record<string, SoundId> = {
		ConstructionStarted: 'build',
		ConstructionCompleted: 'complete',
		RevenueEarned: 'cash',
		DisasterStruck: 'alarm',
		BankruptcyDeclared: 'crash',
		BailoutTaken: 'alarm',
		ResearchCompleted: 'discovery',
		AchievementUnlocked: 'achievement',
		AcquisitionAccepted: 'complete',
		AuctionWon: 'cash',
		SabotageCompleted: 'alarm',
		EspionageDetected: 'alarm',
		VictoryAchieved: 'achievement',
		ScandalOccurred: 'error'
	};

	// Synthesized sound parameters (no audio files needed)
	private soundParams: Record<SoundId, { freq: number; duration: number; type: OscillatorType; volume: number }> = {
		build: { freq: 440, duration: 0.15, type: 'square', volume: 0.3 },
		complete: { freq: 880, duration: 0.2, type: 'sine', volume: 0.4 },
		cash: { freq: 523, duration: 0.1, type: 'sine', volume: 0.25 },
		alarm: { freq: 330, duration: 0.4, type: 'sawtooth', volume: 0.35 },
		crash: { freq: 165, duration: 0.6, type: 'sawtooth', volume: 0.5 },
		discovery: { freq: 660, duration: 0.3, type: 'sine', volume: 0.4 },
		achievement: { freq: 784, duration: 0.4, type: 'sine', volume: 0.5 },
		click: { freq: 1000, duration: 0.05, type: 'square', volume: 0.15 },
		open: { freq: 600, duration: 0.08, type: 'sine', volume: 0.2 },
		close: { freq: 400, duration: 0.08, type: 'sine', volume: 0.2 },
		error: { freq: 200, duration: 0.25, type: 'square', volume: 0.3 }
	};

	async init(): Promise<void> {
		if (this.initialized) return;

		try {
			this.ctx = new AudioContext();
			this.masterGain = this.ctx.createGain();
			this.masterGain.connect(this.ctx.destination);

			this.musicGain = this.ctx.createGain();
			this.musicGain.connect(this.masterGain);

			this.sfxGain = this.ctx.createGain();
			this.sfxGain.connect(this.masterGain);

			this.setMusicVolume(get(musicVolume));
			this.setSfxVolume(get(sfxVolume));

			// Subscribe to store changes
			this.unsubscribers.push(
				musicVolume.subscribe((v) => this.setMusicVolume(v)),
				sfxVolume.subscribe((v) => this.setSfxVolume(v))
			);

			this.initialized = true;
			this.startAmbientMusic();
		} catch {
			// Web Audio not available
		}
	}

	private ensureContext(): boolean {
		if (!this.ctx || !this.initialized) return false;
		if (this.ctx.state === 'suspended') {
			this.ctx.resume();
		}
		return true;
	}

	setMusicVolume(volume: number): void {
		if (this.musicGain) {
			this.musicGain.gain.value = volume;
		}
	}

	setSfxVolume(volume: number): void {
		if (this.sfxGain) {
			this.sfxGain.gain.value = volume;
		}
	}

	playEventSound(event: string | Record<string, unknown>): void {
		let eventName: string;
		if (typeof event === 'string') {
			// Legacy string format fallback
			const match = event.match(/^(\w+)/);
			eventName = match ? match[1] : event;
		} else {
			// Structured JSON event: { "VariantName": { ...fields } }
			eventName = Object.keys(event)[0] ?? '';
		}

		const soundId = this.eventSoundMap[eventName];
		if (soundId) {
			this.playSynthSound(soundId);
		}
	}

	playUiSound(sound: SoundId): void {
		this.playSynthSound(sound);
	}

	private playSynthSound(soundId: SoundId): void {
		if (!this.ensureContext() || !this.ctx || !this.sfxGain || this.muted) return;

		const params = this.soundParams[soundId];
		if (!params) return;

		const oscillator = this.ctx.createOscillator();
		const gainNode = this.ctx.createGain();

		oscillator.type = params.type;
		oscillator.frequency.value = params.freq;

		gainNode.gain.value = params.volume;
		gainNode.gain.exponentialRampToValueAtTime(
			0.001,
			this.ctx.currentTime + params.duration
		);

		oscillator.connect(gainNode);
		gainNode.connect(this.sfxGain);

		oscillator.start(this.ctx.currentTime);
		oscillator.stop(this.ctx.currentTime + params.duration + 0.05);

		// For multi-note sounds (achievement, complete)
		if (soundId === 'achievement' || soundId === 'complete') {
			const osc2 = this.ctx.createOscillator();
			const gain2 = this.ctx.createGain();
			osc2.type = 'sine';
			osc2.frequency.value = params.freq * 1.25; // Major third
			gain2.gain.value = params.volume * 0.7;
			gain2.gain.exponentialRampToValueAtTime(
				0.001,
				this.ctx.currentTime + params.duration
			);
			osc2.connect(gain2);
			gain2.connect(this.sfxGain);
			osc2.start(this.ctx.currentTime + 0.05);
			osc2.stop(this.ctx.currentTime + params.duration + 0.1);
		}
	}

	setIntensity(level: number): void {
		// Adjust music intensity (0 = calm, 1 = tense)
		if (this.musicGain) {
			const volume = 0.1 + level * 0.15;
			this.musicGain.gain.value = Math.min(volume, get(musicVolume));
		}
		if (this.ambient) {
			this.ambient.setIntensity(level);
		}
	}

	startAmbientMusic(): void {
		if (!this.ctx || !this.musicGain || this.ambient) return;
		this.ambient = new AmbientMusicGenerator(this.ctx, this.musicGain);
		this.ambient.start();
	}

	stopAmbientMusic(): void {
		if (this.ambient) {
			this.ambient.stop();
			this.ambient = null;
		}
	}

	mute(): void {
		this.muted = true;
		if (this.masterGain) {
			this.masterGain.gain.value = 0;
		}
	}

	unmute(): void {
		this.muted = false;
		if (this.masterGain) {
			this.masterGain.gain.value = 1;
		}
	}

	toggleMute(): void {
		if (this.muted) this.unmute();
		else this.mute();
	}

	dispose(): void {
		this.stopAmbientMusic();
		for (const unsub of this.unsubscribers) unsub();
		this.unsubscribers = [];
		if (this.currentMusicSource) {
			this.currentMusicSource.stop();
			this.currentMusicSource = null;
		}
		if (this.ctx) {
			this.ctx.close();
			this.ctx = null;
		}
		this.initialized = false;
	}
}

/**
 * Procedural ambient music generator using Web Audio API oscillators.
 * Creates a layered soundscape: bass drone + pad chords + filtered harmonics.
 * Intensity parameter (0-1) shifts from calm ambient to tense atmosphere.
 */
class AmbientMusicGenerator {
	private ctx: AudioContext;
	private output: GainNode;
	private masterGain: GainNode;
	private filter: BiquadFilterNode;
	private bassOsc: OscillatorNode | null = null;
	private padOscs: OscillatorNode[] = [];
	private lfo: OscillatorNode | null = null;
	private lfoGain: GainNode | null = null;
	private running = false;
	private intensity = 0;
	private chordInterval: ReturnType<typeof setInterval> | null = null;
	private currentChordIndex = 0;

	// Minor key chord progressions (frequencies in Hz)
	// Am -> Dm -> Em -> Am pattern at different octaves
	private chords = [
		[110.0, 164.81, 220.0],   // Am (A2, E3, A3)
		[146.83, 174.61, 220.0],  // Dm (D3, F3, A3)
		[164.81, 196.0, 246.94],  // Em (E3, G3, B3)
		[110.0, 130.81, 164.81],  // Am low (A2, C3, E3)
	];

	constructor(ctx: AudioContext, output: GainNode) {
		this.ctx = ctx;
		this.output = output;

		this.masterGain = ctx.createGain();
		this.masterGain.gain.value = 0.12;

		this.filter = ctx.createBiquadFilter();
		this.filter.type = 'lowpass';
		this.filter.frequency.value = 400;
		this.filter.Q.value = 1.5;

		this.filter.connect(this.masterGain);
		this.masterGain.connect(this.output);
	}

	start(): void {
		if (this.running) return;
		this.running = true;

		// Bass drone: low A (55 Hz)
		this.bassOsc = this.ctx.createOscillator();
		this.bassOsc.type = 'sine';
		this.bassOsc.frequency.value = 55;
		const bassGain = this.ctx.createGain();
		bassGain.gain.value = 0.3;
		this.bassOsc.connect(bassGain);
		bassGain.connect(this.filter);
		this.bassOsc.start();

		// LFO for subtle volume tremolo
		this.lfo = this.ctx.createOscillator();
		this.lfo.type = 'sine';
		this.lfo.frequency.value = 0.08; // Very slow wobble
		this.lfoGain = this.ctx.createGain();
		this.lfoGain.gain.value = 0.03;
		this.lfo.connect(this.lfoGain);
		this.lfoGain.connect(this.masterGain.gain);
		this.lfo.start();

		// Start first chord
		this.playChord(this.chords[0]);

		// Cycle chords every 8 seconds
		this.chordInterval = setInterval(() => {
			this.currentChordIndex = (this.currentChordIndex + 1) % this.chords.length;
			this.transitionToChord(this.chords[this.currentChordIndex]);
		}, 8000);
	}

	private playChord(freqs: number[]): void {
		this.clearPadOscs();
		for (const freq of freqs) {
			const osc = this.ctx.createOscillator();
			osc.type = 'triangle';
			osc.frequency.value = freq;
			const gain = this.ctx.createGain();
			gain.gain.value = 0.08;
			osc.connect(gain);
			gain.connect(this.filter);
			osc.start();
			this.padOscs.push(osc);
		}
	}

	private transitionToChord(freqs: number[]): void {
		// Crossfade: ramp down old, start new
		const now = this.ctx.currentTime;

		// Fade out existing pads
		for (const osc of this.padOscs) {
			try {
				osc.stop(now + 1.5);
			} catch {
				// already stopped
			}
		}

		// Create new pads with fade-in
		const newOscs: OscillatorNode[] = [];
		for (const freq of freqs) {
			const osc = this.ctx.createOscillator();
			osc.type = 'triangle';
			osc.frequency.value = freq;
			const gain = this.ctx.createGain();
			gain.gain.setValueAtTime(0, now);
			gain.gain.linearRampToValueAtTime(0.08, now + 1.5);
			osc.connect(gain);
			gain.connect(this.filter);
			osc.start();
			newOscs.push(osc);
		}

		this.padOscs = newOscs;
	}

	private clearPadOscs(): void {
		for (const osc of this.padOscs) {
			try { osc.stop(); } catch { /* already stopped */ }
		}
		this.padOscs = [];
	}

	setIntensity(level: number): void {
		this.intensity = Math.max(0, Math.min(1, level));

		// Higher intensity = higher filter cutoff (more bright/tense)
		const cutoff = 300 + this.intensity * 1200;
		this.filter.frequency.linearRampToValueAtTime(
			cutoff,
			this.ctx.currentTime + 0.5
		);

		// Adjust volume: slightly louder when tense
		const vol = 0.10 + this.intensity * 0.08;
		this.masterGain.gain.linearRampToValueAtTime(
			vol,
			this.ctx.currentTime + 0.5
		);

		// Speed up LFO wobble when tense
		if (this.lfo) {
			this.lfo.frequency.linearRampToValueAtTime(
				0.06 + this.intensity * 0.15,
				this.ctx.currentTime + 0.5
			);
		}
	}

	stop(): void {
		if (!this.running) return;
		this.running = false;

		if (this.chordInterval) {
			clearInterval(this.chordInterval);
			this.chordInterval = null;
		}

		if (this.bassOsc) {
			try { this.bassOsc.stop(); } catch { /* ok */ }
			this.bassOsc = null;
		}

		if (this.lfo) {
			try { this.lfo.stop(); } catch { /* ok */ }
			this.lfo = null;
		}

		this.clearPadOscs();
	}
}

export const audioManager = new AudioManager();
