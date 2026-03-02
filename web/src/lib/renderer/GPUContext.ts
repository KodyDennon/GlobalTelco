/**
 * GPUContext — WebGPU device/adapter/context setup with feature detection.
 * Handles initialization, device loss recovery, and capability reporting.
 */

export interface GPUCapabilities {
	maxBufferSize: number;
	maxStorageBufferBindingSize: number;
	maxComputeWorkgroupSizeX: number;
	maxComputeInvocationsPerWorkgroup: number;
	supportsFloat32Filterable: boolean;
	supportsTimestampQuery: boolean;
}

export class GPUContext {
	adapter: GPUAdapter | null = null;
	device: GPUDevice | null = null;
	context: GPUCanvasContext | null = null;
	format: GPUTextureFormat = 'bgra8unorm';
	capabilities: GPUCapabilities | null = null;

	private canvas: HTMLCanvasElement | OffscreenCanvas | null = null;
	private onDeviceLost: (() => void) | null = null;

	static isSupported(): boolean {
		return typeof navigator !== 'undefined' && 'gpu' in navigator;
	}

	async init(
		canvas: HTMLCanvasElement | OffscreenCanvas,
		opts?: { onDeviceLost?: () => void; powerPreference?: GPUPowerPreference }
	): Promise<boolean> {
		if (!GPUContext.isSupported()) return false;

		try {
			this.adapter = await navigator.gpu.requestAdapter({
				powerPreference: opts?.powerPreference ?? 'high-performance',
			});
			if (!this.adapter) return false;

			// Request device with compute + storage capabilities
			const features: GPUFeatureName[] = [];
			if (this.adapter.features.has('float32-filterable')) features.push('float32-filterable');
			if (this.adapter.features.has('timestamp-query')) features.push('timestamp-query');

			this.device = await this.adapter.requestDevice({
				requiredFeatures: features,
				requiredLimits: {
					maxStorageBufferBindingSize: Math.min(
						256 * 1024 * 1024,
						this.adapter.limits.maxStorageBufferBindingSize
					),
					maxBufferSize: Math.min(
						256 * 1024 * 1024,
						this.adapter.limits.maxBufferSize
					),
				},
			});

			this.canvas = canvas;
			this.context = canvas.getContext('webgpu') as GPUCanvasContext;
			if (!this.context) {
				this.dispose();
				return false;
			}

			this.format = navigator.gpu.getPreferredCanvasFormat();
			this.context.configure({
				device: this.device,
				format: this.format,
				alphaMode: 'premultiplied',
			});

			this.capabilities = {
				maxBufferSize: this.device.limits.maxBufferSize,
				maxStorageBufferBindingSize: this.device.limits.maxStorageBufferBindingSize,
				maxComputeWorkgroupSizeX: this.device.limits.maxComputeWorkgroupSizeX,
				maxComputeInvocationsPerWorkgroup: this.device.limits.maxComputeInvocationsPerWorkgroup,
				supportsFloat32Filterable: this.device.features.has('float32-filterable'),
				supportsTimestampQuery: this.device.features.has('timestamp-query'),
			};

			// Handle device loss
			this.onDeviceLost = opts?.onDeviceLost ?? null;
			this.device.lost.then((info) => {
				console.error(`[GPUContext] Device lost: ${info.reason} — ${info.message}`);
				if (info.reason !== 'destroyed') {
					this.onDeviceLost?.();
				}
			});

			return true;
		} catch (e) {
			console.warn('[GPUContext] Init failed:', e);
			this.dispose();
			return false;
		}
	}

	getCurrentTexture(): GPUTexture | null {
		return this.context?.getCurrentTexture() ?? null;
	}

	dispose(): void {
		this.device?.destroy();
		this.device = null;
		this.adapter = null;
		this.context = null;
		this.canvas = null;
		this.capabilities = null;
	}
}
