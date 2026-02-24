/**
 * ScreenshotManager — screenshot capture and timelapse recording.
 *
 * Uses canvas.toBlob() to capture the deck.gl / MapLibre map canvas.
 * Adds an optional watermark and triggers file download.
 */

export class ScreenshotManager {
	private recording: boolean = false;
	private frames: Blob[] = [];

	/** Take a single screenshot of the map, hide UI chrome, and download as PNG. */
	async captureScreenshot(
		mapContainer: HTMLElement,
		corpName?: string,
		tick?: number
	): Promise<void> {
		const blob = await this.captureMapBlob(mapContainer, corpName, tick);
		if (!blob) return;

		const timestamp = new Date().toISOString().replace(/[:.]/g, '-').slice(0, 19);
		const safeName = (corpName ?? 'GlobalTelco').replace(/\s+/g, '_');
		const filename = `GlobalTelco_${safeName}_${timestamp}.png`;

		this.downloadBlob(blob, filename);
	}

	/** Start timelapse recording. Frames are collected with captureFrame(). */
	startTimelapse(): void {
		this.frames = [];
		this.recording = true;
	}

	/** Capture one timelapse frame. Call this periodically (e.g., every N ticks). */
	async captureFrame(mapContainer: HTMLElement): Promise<void> {
		if (!this.recording) return;
		const blob = await this.captureMapBlob(mapContainer);
		if (blob) {
			this.frames.push(blob);
		}
	}

	/** Stop recording and download all frames as individual PNGs. */
	async stopTimelapse(): Promise<void> {
		this.recording = false;

		if (this.frames.length === 0) return;

		// Download each frame as a sequentially numbered PNG
		const padLength = String(this.frames.length).length;
		for (let i = 0; i < this.frames.length; i++) {
			const num = String(i + 1).padStart(padLength, '0');
			const filename = `GlobalTelco_timelapse_${num}.png`;
			this.downloadBlob(this.frames[i], filename);

			// Small delay between downloads to avoid browser blocking
			if (i < this.frames.length - 1) {
				await this.delay(100);
			}
		}

		this.frames = [];
	}

	/** Whether timelapse recording is currently active. */
	isRecording(): boolean {
		return this.recording;
	}

	/** Get the number of captured timelapse frames. */
	frameCount(): number {
		return this.frames.length;
	}

	// ── Internal ────────────────────────────────────────────────────────

	/** Find and capture the map canvas inside the container, returning a PNG Blob. */
	private async captureMapBlob(
		mapContainer: HTMLElement,
		corpName?: string,
		tick?: number
	): Promise<Blob | null> {
		// Find all canvas elements in the map container
		const canvases = mapContainer.querySelectorAll('canvas');
		if (canvases.length === 0) return null;

		// Use the largest canvas (usually the main deck.gl or MapLibre canvas)
		let sourceCanvas: HTMLCanvasElement | null = null;
		let maxArea = 0;
		for (const canvas of canvases) {
			const area = canvas.width * canvas.height;
			if (area > maxArea) {
				maxArea = area;
				sourceCanvas = canvas;
			}
		}
		if (!sourceCanvas) return null;

		// Create a compositing canvas to merge layers and add watermark
		const width = sourceCanvas.width;
		const height = sourceCanvas.height;
		const outputCanvas = document.createElement('canvas');
		outputCanvas.width = width;
		outputCanvas.height = height;
		const ctx = outputCanvas.getContext('2d');
		if (!ctx) return null;

		// Draw all canvases in DOM order (back to front) to composite them
		for (const canvas of canvases) {
			try {
				ctx.drawImage(canvas, 0, 0, width, height);
			} catch {
				// Cross-origin or tainted canvas — skip
			}
		}

		// Add watermark if corp name or tick is provided
		if (corpName || tick !== undefined) {
			this.drawWatermark(ctx, width, height, corpName, tick);
		}

		return new Promise<Blob | null>((resolve) => {
			outputCanvas.toBlob(
				(blob) => resolve(blob),
				'image/png'
			);
		});
	}

	/** Draw a subtle watermark in the bottom-right corner. */
	private drawWatermark(
		ctx: CanvasRenderingContext2D,
		width: number,
		height: number,
		corpName?: string,
		tick?: number
	): void {
		const parts: string[] = ['GlobalTelco'];
		if (corpName) parts.push(corpName);
		if (tick !== undefined) parts.push(`Tick ${tick}`);
		const text = parts.join(' \u2014 '); // em-dash separator

		const fontSize = Math.max(12, Math.round(height * 0.018));
		ctx.font = `${fontSize}px "Inter", "Segoe UI", sans-serif`;
		ctx.textAlign = 'right';
		ctx.textBaseline = 'bottom';

		const padding = Math.round(fontSize * 0.8);
		const x = width - padding;
		const y = height - padding;

		// Semi-transparent background pill
		const metrics = ctx.measureText(text);
		const bgPadH = fontSize * 0.5;
		const bgPadV = fontSize * 0.3;
		ctx.fillStyle = 'rgba(0, 0, 0, 0.5)';
		ctx.beginPath();
		const bgX = x - metrics.width - bgPadH;
		const bgY = y - fontSize - bgPadV;
		const bgW = metrics.width + bgPadH * 2;
		const bgH = fontSize + bgPadV * 2;
		const radius = bgH * 0.3;
		ctx.roundRect(bgX, bgY, bgW, bgH, radius);
		ctx.fill();

		// Text
		ctx.fillStyle = 'rgba(255, 255, 255, 0.8)';
		ctx.fillText(text, x, y);
	}

	/** Trigger a file download for a Blob. */
	private downloadBlob(blob: Blob, filename: string): void {
		const url = URL.createObjectURL(blob);
		const a = document.createElement('a');
		a.href = url;
		a.download = filename;
		a.style.display = 'none';
		document.body.appendChild(a);
		a.click();

		// Clean up after a short delay
		setTimeout(() => {
			URL.revokeObjectURL(url);
			a.remove();
		}, 1000);
	}

	/** Promise-based delay. */
	private delay(ms: number): Promise<void> {
		return new Promise((resolve) => setTimeout(resolve, ms));
	}
}
