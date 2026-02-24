import type { AudioManager, AmbienceZone } from './AudioManager';

interface ViewportState {
	centerLon: number;
	centerLat: number;
	zoom: number;
	nearestCityPop?: number;
	nearestInfraDistance?: number;
	isOverOcean?: boolean;
}

/**
 * Positional audio controller that determines the ambient soundscape
 * based on camera position and zoom level. Crossfades smoothly between
 * ambience zones as the player pans and zooms around the map.
 */
export class SpatialAudioController {
	private audioManager: AudioManager;
	private currentZone: AmbienceZone = 'silence';
	private updateInterval: number | null = null;
	private lastViewport: ViewportState | null = null;
	private targetAmbienceVolume = 0;

	constructor(audioManager: AudioManager) {
		this.audioManager = audioManager;
	}

	/**
	 * Call when the camera/viewport changes to re-evaluate which ambience
	 * zone the player is in and adjust volume based on zoom level.
	 */
	updateFromViewport(opts: ViewportState): void {
		this.lastViewport = opts;

		const newZone = this.determineZone(opts);
		const volume = this.calculateVolume(opts.zoom);

		this.targetAmbienceVolume = volume;
		this.audioManager.setAmbienceVolume(volume);

		if (newZone !== this.currentZone) {
			this.currentZone = newZone;
			this.audioManager.crossfadeAmbience(newZone);
		}
	}

	/**
	 * Start periodic viewport-based ambience updates.
	 * The caller should still invoke updateFromViewport when the camera moves;
	 * the periodic check serves as a safety net for missed updates and allows
	 * ambient variation over time even when the camera is stationary.
	 */
	start(): void {
		if (this.updateInterval !== null) return;
		this.updateInterval = window.setInterval(() => {
			if (this.lastViewport) {
				this.updateFromViewport(this.lastViewport);
			}
		}, 2000);
	}

	/**
	 * Stop periodic updates and silence ambience.
	 */
	stop(): void {
		if (this.updateInterval !== null) {
			clearInterval(this.updateInterval);
			this.updateInterval = null;
		}
		this.audioManager.stopAmbience();
		this.currentZone = 'silence';
	}

	/**
	 * Clean up all resources.
	 */
	dispose(): void {
		this.stop();
		this.lastViewport = null;
	}

	/**
	 * Get the currently active ambience zone.
	 */
	getCurrentZone(): AmbienceZone {
		return this.currentZone;
	}

	// ── Private helpers ───────────────────────────────────────────────

	/**
	 * Determine which ambience zone to play based on viewport state.
	 *
	 * Priority order:
	 * 1. zoom < 3  => silence (too zoomed out)
	 * 2. isOverOcean => ocean
	 * 3. nearestCityPop > 100k and zoom >= 5 => urban
	 * 4. nearestInfraDistance < 50km and zoom >= 5 => infrastructure
	 * 5. default land => rural
	 */
	private determineZone(opts: ViewportState): AmbienceZone {
		if (opts.zoom < 3) {
			return 'silence';
		}

		if (opts.isOverOcean) {
			return 'ocean';
		}

		if (opts.zoom >= 5) {
			if (opts.nearestCityPop !== undefined && opts.nearestCityPop > 100000) {
				return 'urban';
			}

			if (opts.nearestInfraDistance !== undefined && opts.nearestInfraDistance < 50) {
				return 'infrastructure';
			}
		}

		return 'rural';
	}

	/**
	 * Calculate ambience volume based on zoom level.
	 *
	 * At zoom 3 (the threshold where spatial audio activates): 20% volume.
	 * At zoom 8 and above: 100% volume.
	 * Linear interpolation between those points.
	 * Below zoom 3: 0% (silence zone handles this, but volume also zeroed).
	 */
	private calculateVolume(zoom: number): number {
		if (zoom < 3) return 0;
		if (zoom >= 8) return 1.0;

		// Linear interpolation from 0.2 at zoom=3 to 1.0 at zoom=8
		const t = (zoom - 3) / (8 - 3);
		return 0.2 + t * 0.8;
	}
}
