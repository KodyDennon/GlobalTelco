/**
 * InfraPipeline — GPU render pipeline for infrastructure nodes and edges.
 * Uses instanced rendering: one draw call per type (nodes, edges).
 * Buffers are updated incrementally (dirty ranges only).
 */

import infraShaderSource from '../shaders/infra.wgsl?raw';
import { Camera2D } from '../camera/Camera2D';

// Per-node instance data: mercator_pos(2f) + radius_color(4f) + id_flags(2u) = 32 bytes
const NODE_INSTANCE_SIZE = 32;
// Per-edge instance data: src(2f) + dst(2f) + width_color(4f) + id_util(2f) = 40 bytes
const EDGE_INSTANCE_SIZE = 40;

const MAX_NODES = 50000;
const MAX_EDGES = 50000;

export interface InfraRenderData {
	// Node data from typed arrays
	nodeCount: number;
	nodeIds: Uint32Array;
	nodeLons: Float64Array;
	nodeLats: Float64Array;
	nodeTypes: Uint32Array;
	nodeOwners: Uint32Array;
	nodeHealth: Float64Array;
	nodeUtil: Float64Array;
	nodeFlags: Uint32Array; // bit 0 = under_construction

	// Edge data from typed arrays
	edgeCount: number;
	edgeIds: Uint32Array;
	edgeSrcX: Float64Array;
	edgeSrcY: Float64Array;
	edgeDstX: Float64Array;
	edgeDstY: Float64Array;
	edgeTypes: Uint32Array;
	edgeUtil: Float64Array;
}

// Node type → base color (RGB floats)
const NODE_COLORS: Record<number, [number, number, number]> = {
	// Default blue
};

function getNodeColor(typeId: number, ownerId: number): [number, number, number] {
	// Corp-based coloring
	const CORP_COLORS: [number, number, number][] = [
		[0.06, 0.52, 0.98],  // blue
		[0.94, 0.27, 0.27],  // red
		[0.06, 0.73, 0.51],  // green
		[0.96, 0.62, 0.04],  // amber
		[0.55, 0.36, 0.96],  // purple
		[0.93, 0.29, 0.60],  // pink
		[0.08, 0.72, 0.65],  // teal
		[0.98, 0.45, 0.09],  // orange
	];
	return CORP_COLORS[ownerId % CORP_COLORS.length] ?? [0.06, 0.52, 0.98];
}

function getNodeRadius(typeId: number, zoom: number): number {
	// Scale radius with zoom
	const base = 6;
	return Math.max(3, base * Math.pow(1.2, zoom - 3));
}

function getEdgeColor(typeId: number): [number, number, number] {
	return [0.4, 0.4, 0.5]; // neutral gray
}

function getEdgeWidth(typeId: number, zoom: number): number {
	return Math.max(1, 2 * Math.pow(1.1, zoom - 3));
}

export class InfraPipeline {
	private device: GPUDevice;
	private nodePipeline: GPURenderPipeline | null = null;
	private edgePipeline: GPURenderPipeline | null = null;
	private uniformBuffer: GPUBuffer;
	private uniformBindGroup: GPUBindGroup | null = null;
	private nodeInstanceBuffer: GPUBuffer;
	private edgeInstanceBuffer: GPUBuffer;
	private nodeCount = 0;
	private edgeCount = 0;

	// Staging arrays for CPU-side data assembly
	private nodeStagingArray = new ArrayBuffer(MAX_NODES * NODE_INSTANCE_SIZE);
	private nodeStagingF32 = new Float32Array(this.nodeStagingArray);
	private nodeStagingU32 = new Uint32Array(this.nodeStagingArray);
	private edgeStagingArray = new ArrayBuffer(MAX_EDGES * EDGE_INSTANCE_SIZE);
	private edgeStagingF32 = new Float32Array(this.edgeStagingArray);
	private edgeStagingU32 = new Uint32Array(this.edgeStagingArray);

	constructor(device: GPUDevice, format: GPUTextureFormat) {
		this.device = device;

		// Uniform buffer (matrix + viewport + time + zoom + selected + hovered + pad)
		this.uniformBuffer = device.createBuffer({
			size: 96, // 64 (mat4) + 8 (viewport) + 4 (time) + 4 (zoom) + 4 (selected) + 4 (hovered) + 8 (pad) = 96
			usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST,
		});

		this.nodeInstanceBuffer = device.createBuffer({
			size: MAX_NODES * NODE_INSTANCE_SIZE,
			usage: GPUBufferUsage.VERTEX | GPUBufferUsage.COPY_DST,
		});

		this.edgeInstanceBuffer = device.createBuffer({
			size: MAX_EDGES * EDGE_INSTANCE_SIZE,
			usage: GPUBufferUsage.VERTEX | GPUBufferUsage.COPY_DST,
		});

		this.initPipelines(format);
	}

	private initPipelines(format: GPUTextureFormat): void {
		const shaderModule = this.device.createShaderModule({ code: infraShaderSource });

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

		const pipelineLayout = this.device.createPipelineLayout({ bindGroupLayouts: [uniformBGL] });

		// Node pipeline (instanced circles)
		this.nodePipeline = this.device.createRenderPipeline({
			layout: pipelineLayout,
			vertex: {
				module: shaderModule,
				entryPoint: 'vs_node',
				buffers: [{
					arrayStride: NODE_INSTANCE_SIZE,
					stepMode: 'instance',
					attributes: [
						{ shaderLocation: 0, offset: 0, format: 'float32x2' },   // mercator_pos
						{ shaderLocation: 1, offset: 8, format: 'float32x4' },   // radius_color
						{ shaderLocation: 2, offset: 24, format: 'uint32x2' },   // id_flags
					],
				}],
			},
			fragment: {
				module: shaderModule,
				entryPoint: 'fs_node',
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

		// Edge pipeline (line quads)
		this.edgePipeline = this.device.createRenderPipeline({
			layout: pipelineLayout,
			vertex: {
				module: shaderModule,
				entryPoint: 'vs_edge',
				buffers: [{
					arrayStride: EDGE_INSTANCE_SIZE,
					stepMode: 'instance',
					attributes: [
						{ shaderLocation: 0, offset: 0, format: 'float32x2' },   // src_mercator
						{ shaderLocation: 1, offset: 8, format: 'float32x2' },   // dst_mercator
						{ shaderLocation: 2, offset: 16, format: 'float32x4' },  // width_color
						{ shaderLocation: 3, offset: 32, format: 'float32x2' },  // id_util
					],
				}],
			},
			fragment: {
				module: shaderModule,
				entryPoint: 'fs_edge',
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

	/** Update instance buffers from typed array data. */
	updateData(data: InfraRenderData, zoom: number): void {
		// Pack node instance data
		this.nodeCount = Math.min(data.nodeCount, MAX_NODES);
		for (let i = 0; i < this.nodeCount; i++) {
			const base = i * 8; // 32 bytes / 4 = 8 float32s
			const [mx, my] = Camera2D.lonLatToMercator(data.nodeLons[i], data.nodeLats[i]);
			const color = getNodeColor(data.nodeTypes[i], data.nodeOwners[i]);
			const radius = getNodeRadius(data.nodeTypes[i], zoom);

			this.nodeStagingF32[base] = mx;
			this.nodeStagingF32[base + 1] = my;
			this.nodeStagingF32[base + 2] = radius;
			this.nodeStagingF32[base + 3] = color[0];
			this.nodeStagingF32[base + 4] = color[1];
			this.nodeStagingF32[base + 5] = color[2];
			this.nodeStagingU32[base + 6] = data.nodeIds[i];
			this.nodeStagingU32[base + 7] = data.nodeFlags?.[i] ?? 0;
		}

		if (this.nodeCount > 0) {
			this.device.queue.writeBuffer(
				this.nodeInstanceBuffer,
				0,
				this.nodeStagingArray,
				0,
				this.nodeCount * NODE_INSTANCE_SIZE,
			);
		}

		// Pack edge instance data
		this.edgeCount = Math.min(data.edgeCount, MAX_EDGES);
		for (let i = 0; i < this.edgeCount; i++) {
			const base = i * 10; // 40 bytes / 4 = 10 float32s
			const [sx, sy] = Camera2D.lonLatToMercator(data.edgeSrcX[i], data.edgeSrcY[i]);
			const [dx, dy] = Camera2D.lonLatToMercator(data.edgeDstX[i], data.edgeDstY[i]);
			const color = getEdgeColor(data.edgeTypes[i]);
			const width = getEdgeWidth(data.edgeTypes[i], zoom);

			this.edgeStagingF32[base] = sx;
			this.edgeStagingF32[base + 1] = sy;
			this.edgeStagingF32[base + 2] = dx;
			this.edgeStagingF32[base + 3] = dy;
			this.edgeStagingF32[base + 4] = width;
			this.edgeStagingF32[base + 5] = color[0];
			this.edgeStagingF32[base + 6] = color[1];
			this.edgeStagingF32[base + 7] = color[2];
			// Write edge ID as uint32 (not f32) to avoid precision loss.
			// The shader uses bitcast<u32>() to recover the exact bits.
			this.edgeStagingU32[base + 8] = data.edgeIds[i];
			this.edgeStagingF32[base + 9] = data.edgeUtil[i];
		}

		if (this.edgeCount > 0) {
			this.device.queue.writeBuffer(
				this.edgeInstanceBuffer,
				0,
				this.edgeStagingArray,
				0,
				this.edgeCount * EDGE_INSTANCE_SIZE,
			);
		}
	}

	/** Update uniform buffer with current camera + state. */
	updateUniforms(camera: Camera2D, time: number, selectedId: number, hoveredId: number): void {
		const vs = camera.getViewState();
		const matrix = camera.getMatrix();
		const buf = new ArrayBuffer(96);
		const f32 = new Float32Array(buf);
		const u32 = new Uint32Array(buf);

		// mat4x4 (64 bytes)
		f32.set(matrix, 0);
		// viewport (8 bytes)
		f32[16] = vs.width;
		f32[17] = vs.height;
		// time (4 bytes)
		f32[18] = time;
		// zoom (4 bytes)
		f32[19] = vs.zoom;
		// selected_id (4 bytes)
		u32[20] = selectedId;
		// hovered_id (4 bytes)
		u32[21] = hoveredId;

		this.device.queue.writeBuffer(this.uniformBuffer, 0, buf);
	}

	/** Encode render commands into a render pass. */
	render(pass: GPURenderPassEncoder): void {
		if (!this.uniformBindGroup) return;

		// Draw edges first (behind nodes)
		if (this.edgeCount > 0 && this.edgePipeline) {
			pass.setPipeline(this.edgePipeline);
			pass.setBindGroup(0, this.uniformBindGroup);
			pass.setVertexBuffer(0, this.edgeInstanceBuffer);
			pass.draw(6, this.edgeCount); // 6 vertices per quad instance
		}

		// Draw nodes
		if (this.nodeCount > 0 && this.nodePipeline) {
			pass.setPipeline(this.nodePipeline);
			pass.setBindGroup(0, this.uniformBindGroup);
			pass.setVertexBuffer(0, this.nodeInstanceBuffer);
			pass.draw(6, this.nodeCount); // 6 vertices per circle quad instance
		}
	}

	dispose(): void {
		this.uniformBuffer.destroy();
		this.nodeInstanceBuffer.destroy();
		this.edgeInstanceBuffer.destroy();
	}
}
