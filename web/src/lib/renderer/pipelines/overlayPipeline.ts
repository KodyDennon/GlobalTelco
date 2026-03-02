/**
 * OverlayPipeline — GPU render pipeline for coverage/demand/utilization heatmap overlays.
 * Each cell is an instanced colored quad positioned at its Mercator center.
 */

import overlayShaderSource from '../shaders/overlay.wgsl?raw';
import { Camera2D } from '../camera/Camera2D';

// Per-cell instance: mercator_pos(2f) + value(f) + pad(f) = 16 bytes
const CELL_INSTANCE_SIZE = 16;
const MAX_CELLS = 100000;

export interface OverlayCellData {
	cellCount: number;
	lons: Float64Array;
	lats: Float64Array;
	values: Float32Array; // 0..1 normalized
}

export class OverlayPipeline {
	private device: GPUDevice;
	private pipeline: GPURenderPipeline | null = null;
	private uniformBuffer: GPUBuffer;
	private uniformBindGroup: GPUBindGroup | null = null;
	private instanceBuffer: GPUBuffer;
	private cellCount = 0;

	private stagingArray = new ArrayBuffer(MAX_CELLS * CELL_INSTANCE_SIZE);
	private stagingF32 = new Float32Array(this.stagingArray);

	constructor(device: GPUDevice, format: GPUTextureFormat) {
		this.device = device;

		// Uniform: matrix(64) + viewport(8) + cell_size(4) + pad(4) = 80
		this.uniformBuffer = device.createBuffer({
			size: 80,
			usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST,
		});

		this.instanceBuffer = device.createBuffer({
			size: MAX_CELLS * CELL_INSTANCE_SIZE,
			usage: GPUBufferUsage.VERTEX | GPUBufferUsage.COPY_DST,
		});

		this.initPipeline(format);
	}

	private initPipeline(format: GPUTextureFormat): void {
		const shaderModule = this.device.createShaderModule({ code: overlayShaderSource });

		const uniformBGL = this.device.createBindGroupLayout({
			entries: [{
				binding: 0,
				visibility: GPUShaderStage.VERTEX | GPUShaderStage.FRAGMENT,
				buffer: { type: 'uniform' },
			}],
		});

		this.uniformBindGroup = this.device.createBindGroup({
			layout: uniformBGL,
			entries: [{ binding: 0, resource: { buffer: this.uniformBuffer } }],
		});

		this.pipeline = this.device.createRenderPipeline({
			layout: this.device.createPipelineLayout({ bindGroupLayouts: [uniformBGL] }),
			vertex: {
				module: shaderModule,
				entryPoint: 'vs_overlay',
				buffers: [{
					arrayStride: CELL_INSTANCE_SIZE,
					stepMode: 'instance',
					attributes: [
						{ shaderLocation: 0, offset: 0, format: 'float32x2' },  // mercator_pos
						{ shaderLocation: 1, offset: 8, format: 'float32' },    // value
						{ shaderLocation: 2, offset: 12, format: 'float32' },   // pad
					],
				}],
			},
			fragment: {
				module: shaderModule,
				entryPoint: 'fs_overlay',
				targets: [{
					format,
					blend: {
						color: { srcFactor: 'src-alpha', dstFactor: 'one-minus-src-alpha', operation: 'add' },
						alpha: { srcFactor: 'one', dstFactor: 'one-minus-src-alpha', operation: 'add' },
					},
				}],
			},
			primitive: { topology: 'triangle-list' },
		});
	}

	/** Update cell data for overlay rendering. */
	updateData(data: OverlayCellData): void {
		this.cellCount = Math.min(data.cellCount, MAX_CELLS);
		for (let i = 0; i < this.cellCount; i++) {
			const base = i * 4; // 16 bytes / 4 = 4 float32s
			const [mx, my] = Camera2D.lonLatToMercator(data.lons[i], data.lats[i]);
			this.stagingF32[base] = mx;
			this.stagingF32[base + 1] = my;
			this.stagingF32[base + 2] = data.values[i];
			this.stagingF32[base + 3] = 0; // pad
		}

		if (this.cellCount > 0) {
			this.device.queue.writeBuffer(this.instanceBuffer, 0, this.stagingArray, 0, this.cellCount * CELL_INSTANCE_SIZE);
		}
	}

	/** Update uniforms with camera matrix and cell sizing. */
	updateUniforms(camera: Camera2D, cellSizeMercator: number): void {
		const vs = camera.getViewState();
		const matrix = camera.getMatrix();
		const buf = new ArrayBuffer(80);
		const f32 = new Float32Array(buf);

		f32.set(matrix, 0);
		f32[16] = vs.width;
		f32[17] = vs.height;
		f32[18] = cellSizeMercator;
		f32[19] = 0; // pad

		this.device.queue.writeBuffer(this.uniformBuffer, 0, buf);
	}

	/** Encode overlay render commands. */
	render(pass: GPURenderPassEncoder): void {
		if (this.cellCount === 0 || !this.pipeline || !this.uniformBindGroup) return;

		pass.setPipeline(this.pipeline);
		pass.setBindGroup(0, this.uniformBindGroup);
		pass.setVertexBuffer(0, this.instanceBuffer);
		pass.draw(6, this.cellCount);
	}

	dispose(): void {
		this.uniformBuffer.destroy();
		this.instanceBuffer.destroy();
	}
}
