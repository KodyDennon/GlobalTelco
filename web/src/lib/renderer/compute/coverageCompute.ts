/**
 * CoverageCompute — GPU compute shader for per-cell coverage calculation.
 * Replaces the CPU-side O(cells * nodes) coverage system with a GPU dispatch.
 * With 10,000 cells and 5,000 nodes: dispatches 157 workgroups of 64 threads.
 */

import coverageShaderSource from '../shaders/coverage.compute.wgsl?raw';

const MAX_NODES = 50000;
const MAX_CELLS = 100000;

// GPU struct sizes (must match WGSL)
const NODE_STRUCT_SIZE = 32; // 8 x f32/u32
const CELL_STRUCT_SIZE = 8;  // 2 x f32
const PARAMS_SIZE = 16;      // 2 x u32 + 8 bytes padding (16-byte uniform alignment)

export interface CoverageComputeInput {
	nodes: {
		count: number;
		lons: Float64Array;
		lats: Float64Array;
		coverageRadius: Float32Array; // Mercator units
		bandwidth: Float32Array;
		owners: Uint32Array;
		active: Uint32Array; // 0 or 1
	};
	cells: {
		count: number;
		lons: Float64Array;
		lats: Float64Array;
	};
}

export class CoverageCompute {
	private device: GPUDevice;
	private pipeline: GPUComputePipeline;
	private bindGroupLayout: GPUBindGroupLayout;

	private nodeBuffer: GPUBuffer;
	private cellBuffer: GPUBuffer;
	private coverageBuffer: GPUBuffer;
	private paramsBuffer: GPUBuffer;
	private readbackBuffer: GPUBuffer;

	private nodeStagingArray = new ArrayBuffer(MAX_NODES * NODE_STRUCT_SIZE);
	private nodeStagingF32 = new Float32Array(this.nodeStagingArray);
	private nodeStagingU32 = new Uint32Array(this.nodeStagingArray);

	private cellStagingArray = new ArrayBuffer(MAX_CELLS * CELL_STRUCT_SIZE);
	private cellStagingF32 = new Float32Array(this.cellStagingArray);

	private lastCellCount = 0;

	constructor(device: GPUDevice) {
		this.device = device;

		const shaderModule = device.createShaderModule({ code: coverageShaderSource });

		this.bindGroupLayout = device.createBindGroupLayout({
			entries: [
				{ binding: 0, visibility: GPUShaderStage.COMPUTE, buffer: { type: 'read-only-storage' } },
				{ binding: 1, visibility: GPUShaderStage.COMPUTE, buffer: { type: 'read-only-storage' } },
				{ binding: 2, visibility: GPUShaderStage.COMPUTE, buffer: { type: 'storage' } },
				{ binding: 3, visibility: GPUShaderStage.COMPUTE, buffer: { type: 'uniform' } },
			],
		});

		this.pipeline = device.createComputePipeline({
			layout: device.createPipelineLayout({ bindGroupLayouts: [this.bindGroupLayout] }),
			compute: { module: shaderModule, entryPoint: 'main' },
		});

		this.nodeBuffer = device.createBuffer({
			size: MAX_NODES * NODE_STRUCT_SIZE,
			usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST,
		});

		this.cellBuffer = device.createBuffer({
			size: MAX_CELLS * CELL_STRUCT_SIZE,
			usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST,
		});

		this.coverageBuffer = device.createBuffer({
			size: MAX_CELLS * 4, // f32 per cell
			usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_SRC,
		});

		this.readbackBuffer = device.createBuffer({
			size: MAX_CELLS * 4,
			usage: GPUBufferUsage.MAP_READ | GPUBufferUsage.COPY_DST,
		});

		this.paramsBuffer = device.createBuffer({
			size: PARAMS_SIZE,
			usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST,
		});
	}

	/** Upload node + cell data and dispatch the compute shader. Returns coverage values. */
	async compute(input: CoverageComputeInput): Promise<Float32Array> {
		const nodeCount = Math.min(input.nodes.count, MAX_NODES);
		const cellCount = Math.min(input.cells.count, MAX_CELLS);
		this.lastCellCount = cellCount;

		// Pack node data
		for (let i = 0; i < nodeCount; i++) {
			const base = i * 8;
			this.nodeStagingF32[base] = input.nodes.lons[i];
			this.nodeStagingF32[base + 1] = input.nodes.lats[i];
			this.nodeStagingF32[base + 2] = input.nodes.coverageRadius[i];
			this.nodeStagingF32[base + 3] = input.nodes.bandwidth[i];
			this.nodeStagingU32[base + 4] = input.nodes.owners[i];
			this.nodeStagingU32[base + 5] = input.nodes.active[i];
			this.nodeStagingU32[base + 6] = 0;
			this.nodeStagingU32[base + 7] = 0;
		}

		// Pack cell data
		for (let i = 0; i < cellCount; i++) {
			const base = i * 2;
			this.cellStagingF32[base] = input.cells.lons[i];
			this.cellStagingF32[base + 1] = input.cells.lats[i];
		}

		// Upload params (padded to 16 bytes for uniform buffer alignment)
		const params = new Uint32Array([nodeCount, cellCount, 0, 0]);

		this.device.queue.writeBuffer(this.nodeBuffer, 0, this.nodeStagingArray, 0, nodeCount * NODE_STRUCT_SIZE);
		this.device.queue.writeBuffer(this.cellBuffer, 0, this.cellStagingArray, 0, cellCount * CELL_STRUCT_SIZE);
		this.device.queue.writeBuffer(this.paramsBuffer, 0, params);

		// Create bind group
		const bindGroup = this.device.createBindGroup({
			layout: this.bindGroupLayout,
			entries: [
				{ binding: 0, resource: { buffer: this.nodeBuffer, size: nodeCount * NODE_STRUCT_SIZE } },
				{ binding: 1, resource: { buffer: this.cellBuffer, size: cellCount * CELL_STRUCT_SIZE } },
				{ binding: 2, resource: { buffer: this.coverageBuffer, size: cellCount * 4 } },
				{ binding: 3, resource: { buffer: this.paramsBuffer } },
			],
		});

		// Dispatch compute
		const encoder = this.device.createCommandEncoder();
		const pass = encoder.beginComputePass();
		pass.setPipeline(this.pipeline);
		pass.setBindGroup(0, bindGroup);
		pass.dispatchWorkgroups(Math.ceil(cellCount / 64));
		pass.end();

		// Copy results for readback
		encoder.copyBufferToBuffer(this.coverageBuffer, 0, this.readbackBuffer, 0, cellCount * 4);
		this.device.queue.submit([encoder.finish()]);

		// Map and read results
		await this.readbackBuffer.mapAsync(GPUMapMode.READ, 0, cellCount * 4);
		const resultArray = new Float32Array(this.readbackBuffer.getMappedRange(0, cellCount * 4).slice(0));
		this.readbackBuffer.unmap();

		return resultArray;
	}

	dispose(): void {
		this.nodeBuffer.destroy();
		this.cellBuffer.destroy();
		this.coverageBuffer.destroy();
		this.readbackBuffer.destroy();
		this.paramsBuffer.destroy();
	}
}
