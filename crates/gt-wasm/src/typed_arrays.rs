//! WASM typed-array exports for hot-path rendering (zero-copy deck.gl data).

use js_sys::{Float64Array, Uint32Array, Uint8Array};
use wasm_bindgen::prelude::*;

use crate::WasmBridge;

#[wasm_bindgen]
impl WasmBridge {
    /// Returns infrastructure node data as a flat js_sys::Array.
    /// Layout: [count, ids, owners, positions, stats, node_types, network_levels, construction_flags]
    /// positions: Float64Array [lon0, lat0, lon1, lat1, ...] (2 floats per node)
    /// Stats: Float64Array [health0, utilization0, throughput0, ...] (3 floats per node)
    pub fn get_infra_nodes_typed(&self) -> js_sys::Array {
        let arrays = gt_bridge::queries::build_infra_arrays(&self.world);
        self.pack_infra_arrays(arrays)
    }

    pub fn get_infra_nodes_typed_viewport(
        &self,
        west: f64,
        south: f64,
        east: f64,
        north: f64,
        min_level: u8,
    ) -> js_sys::Array {
        let arrays =
            gt_bridge::queries::build_infra_arrays_viewport(&self.world, west, south, east, north, min_level);
        self.pack_infra_arrays(arrays)
    }

    fn pack_infra_arrays(&self, arrays: gt_bridge::InfraArrays) -> js_sys::Array {
        let result = js_sys::Array::new();
        result.push(&JsValue::from(arrays.ids.len() as u32));
        result.push(&Uint32Array::from(&arrays.ids[..]).into());
        result.push(&Uint32Array::from(&arrays.owners[..]).into());
        result.push(&Float64Array::from(&arrays.positions[..]).into());
        result.push(&Float64Array::from(&arrays.stats[..]).into());
        result.push(&Uint8Array::from(&arrays.node_types[..]).into());
        result.push(&Uint32Array::from(&arrays.network_levels[..]).into());
        result.push(&Uint8Array::from(&arrays.construction_flags[..]).into());
        result.push(&Uint32Array::from(&arrays.cell_indices[..]).into());
        result
    }

    /// Returns infrastructure edge data as a flat js_sys::Array.
    /// Layout: [count, ids, owners, endpoints, stats, edge_types, deployment_types, waypoints_data, waypoint_offsets, waypoint_lengths]
    /// endpoints: Float64Array [src_lon0, src_lat0, dst_lon0, dst_lat0, ...] (4 floats per edge)
    /// stats: Float64Array [bandwidth0, utilization0, ...] (2 floats per edge)
    pub fn get_infra_edges_typed(&self) -> js_sys::Array {
        let arrays = gt_bridge::queries::build_edge_arrays(&self.world);
        self.pack_edge_arrays(arrays)
    }

    pub fn get_infra_edges_typed_viewport(
        &self,
        west: f64,
        south: f64,
        east: f64,
        north: f64,
        min_level: u8,
    ) -> js_sys::Array {
        let arrays =
            gt_bridge::queries::build_edge_arrays_viewport(&self.world, west, south, east, north, min_level);
        self.pack_edge_arrays(arrays)
    }

    fn pack_edge_arrays(&self, arrays: gt_bridge::EdgeArrays) -> js_sys::Array {
        let result = js_sys::Array::new();
        result.push(&JsValue::from(arrays.ids.len() as u32));
        result.push(&Uint32Array::from(&arrays.ids[..]).into());
        result.push(&Uint32Array::from(&arrays.owners[..]).into());
        result.push(&Float64Array::from(&arrays.endpoints[..]).into());
        result.push(&Float64Array::from(&arrays.stats[..]).into());
        result.push(&Uint8Array::from(&arrays.edge_types[..]).into());
        result.push(&Uint8Array::from(&arrays.deployment_types[..]).into());
        result.push(&Float64Array::from(&arrays.waypoints_data[..]).into());
        result.push(&Uint32Array::from(&arrays.waypoint_offsets[..]).into());
        result.push(&Uint8Array::from(&arrays.waypoint_lengths[..]).into());
        result
    }

    /// Returns all corporation summary data as a flat js_sys::Array.
    /// Layout: [count, ids, financials, name_offsets, names_packed]
    /// financials: Float64Array [cash, revenue, cost] per corp (3 floats per corp)
    pub fn get_corporations_typed(&self) -> js_sys::Array {
        let w = &self.world;
        let count = w.corporations.len();
        let mut ids = Vec::with_capacity(count);
        let mut financials = Vec::with_capacity(count * 3);
        let mut names_packed = Vec::new();
        let mut name_offsets = Vec::with_capacity(count * 2);

        for (&cid, corp) in &w.corporations {
            ids.push(cid as u32);
            let fin = w.financials.get(&cid);
            financials.push(fin.map(|f| f.cash as f64).unwrap_or(0.0));
            financials.push(fin.map(|f| f.revenue_per_tick as f64).unwrap_or(0.0));
            financials.push(fin.map(|f| f.cost_per_tick as f64).unwrap_or(0.0));

            let name_bytes = corp.name.as_bytes();
            name_offsets.push(names_packed.len() as u32);
            name_offsets.push(name_bytes.len() as u32);
            names_packed.extend_from_slice(name_bytes);
        }

        let result = js_sys::Array::new();
        result.push(&JsValue::from(count as u32));
        result.push(&Uint32Array::from(&ids[..]).into());
        result.push(&Float64Array::from(&financials[..]).into());
        result.push(&Uint32Array::from(&name_offsets[..]).into());
        result.push(&Uint8Array::from(&names_packed[..]).into());
        result
    }

    /// Hot-path typed array query for satellite positions (orbital overlay).
    /// Layout: [ids, owners, positions, altitudes, orbit_types, statuses, fuel_levels]
    pub fn get_satellite_arrays(&self) -> js_sys::Array {
        let arrays = gt_bridge::queries::build_satellite_arrays(&self.world);

        let result = js_sys::Array::new();
        result.push(&Uint32Array::from(&arrays.ids[..]).into());
        result.push(&Uint32Array::from(&arrays.owners[..]).into());
        result.push(&Float64Array::from(&arrays.positions[..]).into());
        result.push(&Float64Array::from(&arrays.altitudes[..]).into());
        result.push(&Uint32Array::from(&arrays.orbit_types[..]).into());
        result.push(&Uint32Array::from(&arrays.statuses[..]).into());
        result.push(&Float64Array::from(&arrays.fuel_levels[..]).into());
        result
    }
}
