//! Binary packing for typed arrays transferred via Tauri IPC.
//!
//! Format: `[u32 LE count][raw array data...]`
//! Each array field is packed sequentially in little-endian byte order.
//! The frontend unpacks these via `DataView` / typed array slicing.

use gt_bridge::{EdgeArrays, InfraArrays, SatelliteArrays};

/// Pack InfraArrays into a single byte buffer.
///
/// Layout:
/// ```text
/// [4: count as u32 LE]
/// [count*4: ids as u32 LE]
/// [count*4: owners as u32 LE]
/// [count*16: positions as f64 LE (2 per node)]
/// [count*24: stats as f64 LE (3 per node)]
/// [count*1: node_types as u8]
/// [count*4: network_levels as u32 LE]
/// [count*1: construction_flags as u8]
/// [count*4: cell_indices as u32 LE]
/// ```
pub fn pack_infra_arrays(arrays: &InfraArrays) -> Vec<u8> {
    let count = arrays.ids.len();
    // Total size: 4 + count*(4+4+16+24+1+4+1+4) = 4 + count*58
    let total = 4 + count * 58;
    let mut buf = Vec::with_capacity(total);

    buf.extend_from_slice(&(count as u32).to_le_bytes());

    for &id in &arrays.ids {
        buf.extend_from_slice(&id.to_le_bytes());
    }
    for &owner in &arrays.owners {
        buf.extend_from_slice(&owner.to_le_bytes());
    }
    for &pos in &arrays.positions {
        buf.extend_from_slice(&pos.to_le_bytes());
    }
    for &stat in &arrays.stats {
        buf.extend_from_slice(&stat.to_le_bytes());
    }
    buf.extend_from_slice(&arrays.node_types);
    for &nl in &arrays.network_levels {
        buf.extend_from_slice(&nl.to_le_bytes());
    }
    buf.extend_from_slice(&arrays.construction_flags);
    for &ci in &arrays.cell_indices {
        buf.extend_from_slice(&ci.to_le_bytes());
    }

    buf
}

/// Pack EdgeArrays into a single byte buffer.
///
/// Layout:
/// ```text
/// [4: count as u32 LE]
/// [count*4: ids as u32 LE]
/// [count*4: owners as u32 LE]
/// [count*32: endpoints as f64 LE (4 per edge)]
/// [count*16: stats as f64 LE (2 per edge)]
/// [count*1: edge_types as u8]
/// [count*1: deployment_types as u8]
/// [4: waypoints_count as u32 LE]
/// [waypoints_count*16: waypoints_data as f64 LE (2 per point)]
/// [count*4: waypoint_offsets as u32 LE]
/// [count*1: waypoint_lengths as u8]
/// ```
pub fn pack_edge_arrays(arrays: &EdgeArrays) -> Vec<u8> {
    let count = arrays.ids.len();
    let waypoints_count = arrays.waypoints_data.len() / 2;
    
    // Base size: 4 + count*(4+4+32+16+1+1+4+1) + 4 + waypoints_count*16
    let total = 4 + count * 63 + 4 + waypoints_count * 16;
    let mut buf = Vec::with_capacity(total);

    buf.extend_from_slice(&(count as u32).to_le_bytes());

    for &id in &arrays.ids {
        buf.extend_from_slice(&id.to_le_bytes());
    }
    for &owner in &arrays.owners {
        buf.extend_from_slice(&owner.to_le_bytes());
    }
    for &ep in &arrays.endpoints {
        buf.extend_from_slice(&ep.to_le_bytes());
    }
    for &stat in &arrays.stats {
        buf.extend_from_slice(&stat.to_le_bytes());
    }
    buf.extend_from_slice(&arrays.edge_types);
    buf.extend_from_slice(&arrays.deployment_types);

    // Waypoints
    buf.extend_from_slice(&(waypoints_count as u32).to_le_bytes());
    for &w in &arrays.waypoints_data {
        buf.extend_from_slice(&w.to_le_bytes());
    }
    for &off in &arrays.waypoint_offsets {
        buf.extend_from_slice(&off.to_le_bytes());
    }
    buf.extend_from_slice(&arrays.waypoint_lengths);

    buf
}

/// Pack SatelliteArrays into a single byte buffer.
///
/// Layout:
/// ```text
/// [4: count as u32 LE]
/// [count*4: ids as u32 LE]
/// [count*4: owners as u32 LE]
/// [count*16: positions as f64 LE (2 per sat)]
/// [count*8: altitudes as f64 LE]
/// [count*4: orbit_types as u32 LE]
/// [count*4: statuses as u32 LE]
/// [count*8: fuel_levels as f64 LE]
/// ```
pub fn pack_satellite_arrays(arrays: &SatelliteArrays) -> Vec<u8> {
    let count = arrays.ids.len();
    // Total: 4 + count*(4+4+16+8+4+4+8) = 4 + count*48
    let total = 4 + count * 48;
    let mut buf = Vec::with_capacity(total);

    buf.extend_from_slice(&(count as u32).to_le_bytes());

    for &id in &arrays.ids {
        buf.extend_from_slice(&id.to_le_bytes());
    }
    for &owner in &arrays.owners {
        buf.extend_from_slice(&owner.to_le_bytes());
    }
    for &pos in &arrays.positions {
        buf.extend_from_slice(&pos.to_le_bytes());
    }
    for &alt in &arrays.altitudes {
        buf.extend_from_slice(&alt.to_le_bytes());
    }
    for &ot in &arrays.orbit_types {
        buf.extend_from_slice(&ot.to_le_bytes());
    }
    for &st in &arrays.statuses {
        buf.extend_from_slice(&st.to_le_bytes());
    }
    for &fl in &arrays.fuel_levels {
        buf.extend_from_slice(&fl.to_le_bytes());
    }

    buf
}

/// Pack corporation typed data into a single byte buffer.
///
/// Layout:
/// ```text
/// [4: count as u32 LE]
/// [count*4: ids as u32 LE]
/// [count*24: financials as f64 LE (3 per corp: cash, revenue, cost)]
/// [count*8: name_offsets as u32 LE (2 per corp: offset, length)]
/// [variable: names_packed as UTF-8 bytes]
/// ```
pub fn pack_corporations_typed(
    ids: &[u32],
    financials: &[f64],
    name_offsets: &[u32],
    names_packed: &[u8],
) -> Vec<u8> {
    let count = ids.len();
    let total = 4 + count * 4 + count * 24 + count * 8 + names_packed.len();
    let mut buf = Vec::with_capacity(total);

    buf.extend_from_slice(&(count as u32).to_le_bytes());

    for &id in ids {
        buf.extend_from_slice(&id.to_le_bytes());
    }
    for &f in financials {
        buf.extend_from_slice(&f.to_le_bytes());
    }
    for &off in name_offsets {
        buf.extend_from_slice(&off.to_le_bytes());
    }
    buf.extend_from_slice(names_packed);

    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pack_infra_round_trip() {
        let arrays = InfraArrays {
            ids: vec![1, 2],
            owners: vec![10, 20],
            positions: vec![1.0, 2.0, 3.0, 4.0],
            stats: vec![0.9, 0.5, 100.0, 0.8, 0.3, 200.0],
            node_types: vec![5, 6],
            network_levels: vec![0, 1],
            construction_flags: vec![0, 1],
            cell_indices: vec![0, 0],
        };
        let buf = pack_infra_arrays(&arrays);

        // Verify count
        let count = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]);
        assert_eq!(count, 2);

        // Verify first id
        let id0 = u32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]);
        assert_eq!(id0, 1);
    }

    #[test]
    fn pack_edge_round_trip() {
        let arrays = EdgeArrays {
            ids: vec![100],
            owners: vec![10],
            endpoints: vec![1.0, 2.0, 3.0, 4.0],
            stats: vec![500.0, 0.7],
            edge_types: vec![3],
            deployment_types: vec![1],
            waypoints_data: vec![1.5, 2.5],
            waypoint_offsets: vec![0],
            waypoint_lengths: vec![1],
        };
        let buf = pack_edge_arrays(&arrays);
        let count = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]);
        assert_eq!(count, 1);
    }

    #[test]
    fn pack_satellite_round_trip() {
        let arrays = SatelliteArrays {
            ids: vec![50],
            owners: vec![1],
            positions: vec![10.0, 20.0],
            altitudes: vec![550.0],
            orbit_types: vec![0],
            statuses: vec![1],
            fuel_levels: vec![0.95],
        };
        let buf = pack_satellite_arrays(&arrays);
        let count = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]);
        assert_eq!(count, 1);
    }
}
