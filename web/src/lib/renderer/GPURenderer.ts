/**
 * GPURenderer — Main rendering orchestrator.
 * Manages WebGPU context, render pipelines, camera, and the render loop.
 * Falls back to deck.gl (WebGL) when WebGPU is unavailable.
 *
 * Architecture:
 *   MapLibre renders base map → own canvas (below)
 *   GPURenderer draws game layers → WebGPU canvas (overlay, on top)
 *   Camera synced from MapLibre's view state
 */

import { GPUContext } from './GPUContext';
import { Camera2D, type ViewState } from './camera/Camera2D';
import { InfraPipeline, type InfraRenderData } from './pipelines/infraPipeline';
import { OverlayPipeline, type OverlayCellData } from './pipelines/overlayPipeline';
import { CoverageCompute, type CoverageComputeInput } from './compute/coverageCompute';

export type RenderBackend = 'webgpu' | 'webgl-fallback';

export interface GPURendererOptions {
	canvas: HTMLCanvasElement;
	onBackendReady?: (backend: RenderBackend) => void;
	onDeviceLost?: () => void;
}

export class GPURenderer {
	private ctx: GPUContext;
	private camera: Camera2D;
	private infraPipeline: InfraPipeline | null = null;
	private overlayPipeline: OverlayPipeline | null = null;
	private coverageCompute: CoverageCompute | null = null;

	private canvas: HTMLCanvasElement;
	private backend: RenderBackend = 'webgl-fallback';
	private time = 0;
	private frameId: number | null = null;
	private running = false;

	// Render state
	private selectedId = 0;
	private hoveredId = 0;
	private activeOverlayType: string = 'none';
	private overlayDirty = false;

	// Latest data references (avoid re-upload if unchanged)
	private lastInfraData: InfraRenderData | null = null;
	private lastOverlayData: OverlayCellData | null = null;

	constructor() {
		this.ctx = new GPUContext();
		this.camera = new Camera2D();
		this.canvas = null!;
	}

	/** Initialize the renderer. Returns the backend type used. */
	async init(opts: GPURendererOptions): Promise<RenderBackend> {
		this.canvas = opts.canvas;

		if (GPUContext.isSupported()) {
			const ok = await this.ctx.init(opts.canvas, {
				onDeviceLost: () => {
					console.warn('[GPURenderer] Device lost, falling back');
					this.backend = 'webgl-fallback';
					opts.onDeviceLost?.();
				},
			});

			if (ok && this.ctx.device) {
				this.backend = 'webgpu';
				this.infraPipeline = new InfraPipeline(this.ctx.device, this.ctx.format);
				this.overlayPipeline = new OverlayPipeline(this.ctx.device, this.ctx.format);
				this.coverageCompute = new CoverageCompute(this.ctx.device);
			}
		}

		opts.onBackendReady?.(this.backend);
		return this.backend;
	}

	getBackend(): RenderBackend {
		return this.backend;
	}

	isWebGPU(): boolean {
		return this.backend === 'webgpu';
	}

	/** Sync camera from MapLibre's view state (called every frame or on map move). */
	syncCamera(vs: Partial<ViewState>): void {
		this.camera.setViewState(vs);
	}

	/** Update infrastructure data for rendering. */
	updateInfrastructure(data: InfraRenderData): void {
		if (this.backend !== 'webgpu' || !this.infraPipeline) return;
		this.lastInfraData = data;
		const zoom = this.camera.getViewState().zoom;
		this.infraPipeline.updateData(data, zoom);
	}

	/** Update overlay cell data. */
	updateOverlay(data: OverlayCellData): void {
		if (this.backend !== 'webgpu' || !this.overlayPipeline) return;
		this.lastOverlayData = data;
		this.overlayPipeline.updateData(data);
		this.overlayDirty = false;
	}

	/** Set overlay type (triggers re-compute on next frame if needed). */
	setOverlayType(type: string): void {
		if (type !== this.activeOverlayType) {
			this.activeOverlayType = type;
			this.overlayDirty = true;
		}
	}

	/** Run GPU coverage compute and return results. */
	async computeCoverage(input: CoverageComputeInput): Promise<Float32Array | null> {
		if (!this.coverageCompute) return null;
		return this.coverageCompute.compute(input);
	}

	setSelected(id: number | null): void {
		this.selectedId = id ?? 0;
	}

	setHovered(id: number | null): void {
		this.hoveredId = id ?? 0;
	}

	/** Start the WebGPU render loop. */
	start(): void {
		if (this.running || this.backend !== 'webgpu') return;
		this.running = true;
		this.time = 0;
		this.renderFrame(0);
	}

	/** Stop the render loop. */
	stop(): void {
		this.running = false;
		if (this.frameId !== null) {
			cancelAnimationFrame(this.frameId);
			this.frameId = null;
		}
	}

	private renderFrame(timestamp: number): void {
		if (!this.running) return;

		this.time = timestamp / 1000;

		if (this.ctx.device && this.ctx.context) {
			this.renderWebGPU();
		}

		this.frameId = requestAnimationFrame((t) => this.renderFrame(t));
	}

	private renderWebGPU(): void {
		const device = this.ctx.device!;
		const texture = this.ctx.getCurrentTexture();
		if (!texture) return;

		const view = texture.createView();

		// Update uniforms
		this.infraPipeline?.updateUniforms(this.camera, this.time, this.selectedId, this.hoveredId);

		// Cell size in Mercator units (approximate — 120km at equator ≈ 0.003 Mercator units)
		this.overlayPipeline?.updateUniforms(this.camera, 0.003);

		// Create render pass
		const encoder = device.createCommandEncoder();
		const pass = encoder.beginRenderPass({
			colorAttachments: [{
				view,
				loadOp: 'clear',
				storeOp: 'store',
				clearValue: { r: 0, g: 0, b: 0, a: 0 }, // transparent — base map shows through
			}],
		});

		// Render overlay (behind infrastructure)
		if (this.activeOverlayType !== 'none' && this.lastOverlayData) {
			this.overlayPipeline?.render(pass);
		}

		// Render infrastructure
		this.infraPipeline?.render(pass);

		pass.end();
		device.queue.submit([encoder.finish()]);
	}

	/** Resize the canvas and reconfigure the GPU context. */
	resize(width: number, height: number): void {
		this.canvas.width = width;
		this.canvas.height = height;
		this.camera.setViewState({ width, height });

		if (this.ctx.device && this.ctx.context) {
			this.ctx.context.configure({
				device: this.ctx.device,
				format: this.ctx.format,
				alphaMode: 'premultiplied',
			});
		}
	}

	dispose(): void {
		this.stop();
		this.infraPipeline?.dispose();
		this.overlayPipeline?.dispose();
		this.coverageCompute?.dispose();
		this.ctx.dispose();
	}
}

/**
 * Feature-detect and create the appropriate renderer.
 * Returns a GPURenderer for WebGPU or signals fallback for deck.gl.
 */
export async function createRenderer(opts: GPURendererOptions): Promise<GPURenderer> {
	const renderer = new GPURenderer();
	await renderer.init(opts);
	return renderer;
}
