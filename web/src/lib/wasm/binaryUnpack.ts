/**
 * Binary unpacking for typed arrays transferred via Tauri IPC.
 *
 * Format: `[u32 LE count][raw array data...]`
 * Each field is packed sequentially in little-endian byte order.
 * ArrayBuffer.slice() creates aligned copies for typed array views.
 */

import type { InfraNodesTyped, InfraEdgesTyped, CorporationsTyped } from './types';
import type { SatelliteArrays } from './bridge';

const EMPTY_F64 = new Float64Array(0);
const EMPTY_U32 = new Uint32Array(0);
const EMPTY_U8 = new Uint8Array(0);

/**
 * Unpack infra node binary data.
 *
 * Layout: [4: count][count*4: ids][count*4: owners][count*16: positions]
 *         [count*24: stats][count*1: node_types][count*4: network_levels]
 *         [count*1: construction_flags]
 */
export function unpackInfraNodes(buffer: ArrayBuffer): InfraNodesTyped {
	if (buffer.byteLength < 4) {
		return { count: 0, ids: EMPTY_U32, owners: EMPTY_U32, positions: EMPTY_F64, stats: EMPTY_F64, node_types: EMPTY_U8, network_levels: EMPTY_U32, construction_flags: EMPTY_U8, cell_indices: EMPTY_U32 };
	}

	const view = new DataView(buffer);
	const count = view.getUint32(0, true);
	if (count === 0) {
		return { count: 0, ids: EMPTY_U32, owners: EMPTY_U32, positions: EMPTY_F64, stats: EMPTY_F64, node_types: EMPTY_U8, network_levels: EMPTY_U32, construction_flags: EMPTY_U8, cell_indices: EMPTY_U32 };
	}

	let offset = 4;

	const ids = new Uint32Array(buffer.slice(offset, offset + count * 4));
	offset += count * 4;

	const owners = new Uint32Array(buffer.slice(offset, offset + count * 4));
	offset += count * 4;

	const positions = new Float64Array(buffer.slice(offset, offset + count * 16));
	offset += count * 16;

	const stats = new Float64Array(buffer.slice(offset, offset + count * 24));
	offset += count * 24;

	const node_types = new Uint8Array(buffer.slice(offset, offset + count));
	offset += count;

	const network_levels = new Uint32Array(buffer.slice(offset, offset + count * 4));
	offset += count * 4;

	const construction_flags = new Uint8Array(buffer.slice(offset, offset + count));
	offset += count;

	const cell_indices = new Uint32Array(buffer.slice(offset, offset + count * 4));

	return { count, ids, owners, positions, stats, node_types, network_levels, construction_flags, cell_indices };
}

/**
 * Unpack edge binary data.
 *
 * Layout: [4: count][count*4: ids][count*4: owners][count*32: endpoints]
 *         [count*16: stats][count*1: edge_types][count*1: deployment_types]
 *         [variable: waypoints_data][count*4: waypoint_offsets][count*1: waypoint_lengths]
 * 
 * NOTE: For Tauri IPC, we might need a more rigid packing for variable waypoints.
 * Currently, waypoints_data size is inferred from the remaining buffer after offsets/lengths.
 * Let's assume the Rust side sends waypoints_data last or with a size prefix.
 */
export function unpackInfraEdges(buffer: ArrayBuffer): InfraEdgesTyped {
	if (buffer.byteLength < 4) {
		return { 
            count: 0, 
            ids: EMPTY_U32, 
            owners: EMPTY_U32, 
            endpoints: EMPTY_F64, 
            stats: EMPTY_F64, 
            edge_types: EMPTY_U8,
            deployment_types: EMPTY_U8,
            waypoints_data: EMPTY_F64,
            waypoint_offsets: EMPTY_U32,
            waypoint_lengths: EMPTY_U8
        };
	}

	const view = new DataView(buffer);
	const count = view.getUint32(0, true);
	if (count === 0) {
		return { 
            count: 0, 
            ids: EMPTY_U32, 
            owners: EMPTY_U32, 
            endpoints: EMPTY_F64, 
            stats: EMPTY_F64, 
            edge_types: EMPTY_U8,
            deployment_types: EMPTY_U8,
            waypoints_data: EMPTY_F64,
            waypoint_offsets: EMPTY_U32,
            waypoint_lengths: EMPTY_U8
        };
	}

	let offset = 4;

	const ids = new Uint32Array(buffer.slice(offset, offset + count * 4));
	offset += count * 4;

	const owners = new Uint32Array(buffer.slice(offset, offset + count * 4));
	offset += count * 4;

	const endpoints = new Float64Array(buffer.slice(offset, offset + count * 32));
	offset += count * 32;

	const stats = new Float64Array(buffer.slice(offset, offset + count * 16));
	offset += count * 16;

	const edge_types = new Uint8Array(buffer.slice(offset, offset + count));
	offset += count;

    const deployment_types = new Uint8Array(buffer.slice(offset, offset + count));
    offset += count;

    // We need to know how many waypoint points there are in total
    // Rust side must prefix waypoints_data with count
    const waypoints_point_count = view.getUint32(offset, true);
    offset += 4;
    
    const waypoints_data = new Float64Array(buffer.slice(offset, offset + waypoints_point_count * 16));
    offset += waypoints_point_count * 16;

    const waypoint_offsets = new Uint32Array(buffer.slice(offset, offset + count * 4));
    offset += count * 4;

    const waypoint_lengths = new Uint8Array(buffer.slice(offset, offset + count));

	return { count, ids, owners, endpoints, stats, edge_types, deployment_types, waypoints_data, waypoint_offsets, waypoint_lengths };
}

/**
 * Unpack satellite binary data.
 *
 * Layout: [4: count][count*4: ids][count*4: owners][count*16: positions]
 *         [count*8: altitudes][count*4: orbit_types][count*4: statuses]
 *         [count*8: fuel_levels]
 */
export function unpackSatellites(buffer: ArrayBuffer): SatelliteArrays | null {
	if (buffer.byteLength < 4) return null;

	const view = new DataView(buffer);
	const count = view.getUint32(0, true);
	if (count === 0) return null;

	let offset = 4;

	const ids = new Uint32Array(buffer.slice(offset, offset + count * 4));
	offset += count * 4;

	const owners = new Uint32Array(buffer.slice(offset, offset + count * 4));
	offset += count * 4;

	const positions = new Float64Array(buffer.slice(offset, offset + count * 16));
	offset += count * 16;

	const altitudes = new Float64Array(buffer.slice(offset, offset + count * 8));
	offset += count * 8;

	const orbitTypes = new Uint32Array(buffer.slice(offset, offset + count * 4));
	offset += count * 4;

	const statuses = new Uint32Array(buffer.slice(offset, offset + count * 4));
	offset += count * 4;

	const fuelLevels = new Float64Array(buffer.slice(offset, offset + count * 8));

	return { ids, owners, positions, altitudes, orbitTypes, statuses, fuelLevels };
}

/**
 * Unpack corporation binary data.
 *
 * Layout: [4: count][count*4: ids][count*24: financials][count*8: name_offsets]
 *         [variable: names_packed]
 */
export function unpackCorporations(buffer: ArrayBuffer): CorporationsTyped {
	if (buffer.byteLength < 4) {
		return { count: 0, ids: EMPTY_U32, financials: EMPTY_F64, name_offsets: EMPTY_U32, names_packed: EMPTY_U8 };
	}

	const view = new DataView(buffer);
	const count = view.getUint32(0, true);
	if (count === 0) {
		return { count: 0, ids: EMPTY_U32, financials: EMPTY_F64, name_offsets: EMPTY_U32, names_packed: EMPTY_U8 };
	}

	let offset = 4;

	const ids = new Uint32Array(buffer.slice(offset, offset + count * 4));
	offset += count * 4;

	const financials = new Float64Array(buffer.slice(offset, offset + count * 24));
	offset += count * 24;

	const name_offsets = new Uint32Array(buffer.slice(offset, offset + count * 8));
	offset += count * 8;

	const names_packed = new Uint8Array(buffer.slice(offset));

	return { count, ids, financials, name_offsets, names_packed };
}
