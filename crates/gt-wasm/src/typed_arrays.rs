//! WASM typed-array exports for hot-path rendering (zero-copy deck.gl data).

use js_sys::{Float64Array, Uint32Array, Uint8Array};
use wasm_bindgen::prelude::*;

use crate::WasmBridge;

#[wasm_bindgen]
impl WasmBridge {
    /// Returns infrastructure node data as parallel typed arrays.
    /// Output: { count, ids: Uint32Array, owners: Uint32Array, positions: Float64Array,
    ///           stats: Float64Array, node_types: Uint32Array, network_levels: Uint32Array,
    ///           construction_flags: Uint8Array }
    /// positions layout: [lon0, lat0, lon1, lat1, ...] (2 floats per node)
    /// stats layout: [health0, utilization0, throughput0, ...] (3 floats per node)
    pub fn get_infra_nodes_typed(&self) -> JsValue {
        let arrays = gt_bridge::queries::build_infra_arrays(&self.world);

        let obj = js_sys::Object::new();
        let _ = js_sys::Reflect::set(
            &obj,
            &"count".into(),
            &JsValue::from(arrays.ids.len() as u32),
        );
        let _ = js_sys::Reflect::set(
            &obj,
            &"ids".into(),
            &Uint32Array::from(&arrays.ids[..]).into(),
        );
        let _ = js_sys::Reflect::set(
            &obj,
            &"owners".into(),
            &Uint32Array::from(&arrays.owners[..]).into(),
        );
        let _ = js_sys::Reflect::set(
            &obj,
            &"positions".into(),
            &Float64Array::from(&arrays.positions[..]).into(),
        );
        let _ = js_sys::Reflect::set(
            &obj,
            &"stats".into(),
            &Float64Array::from(&arrays.stats[..]).into(),
        );
        let _ = js_sys::Reflect::set(
            &obj,
            &"node_types".into(),
            &Uint32Array::from(&arrays.node_types[..]).into(),
        );
        let _ = js_sys::Reflect::set(
            &obj,
            &"network_levels".into(),
            &Uint32Array::from(&arrays.network_levels[..]).into(),
        );
        let _ = js_sys::Reflect::set(
            &obj,
            &"construction_flags".into(),
            &Uint8Array::from(&arrays.construction_flags[..]).into(),
        );
        obj.into()
    }

    /// Returns infrastructure edge data as parallel typed arrays.
    /// endpoints layout: [src_lon0, src_lat0, dst_lon0, dst_lat0, ...] (4 floats per edge)
    /// stats layout: [bandwidth0, utilization0, ...] (2 floats per edge)
    pub fn get_infra_edges_typed(&self) -> JsValue {
        let arrays = gt_bridge::queries::build_edge_arrays(&self.world);

        let obj = js_sys::Object::new();
        let _ = js_sys::Reflect::set(
            &obj,
            &"count".into(),
            &JsValue::from(arrays.ids.len() as u32),
        );
        let _ = js_sys::Reflect::set(
            &obj,
            &"ids".into(),
            &Uint32Array::from(&arrays.ids[..]).into(),
        );
        let _ = js_sys::Reflect::set(
            &obj,
            &"owners".into(),
            &Uint32Array::from(&arrays.owners[..]).into(),
        );
        let _ = js_sys::Reflect::set(
            &obj,
            &"endpoints".into(),
            &Float64Array::from(&arrays.endpoints[..]).into(),
        );
        let _ = js_sys::Reflect::set(
            &obj,
            &"stats".into(),
            &Float64Array::from(&arrays.stats[..]).into(),
        );
        let _ = js_sys::Reflect::set(
            &obj,
            &"edge_types".into(),
            &Uint32Array::from(&arrays.edge_types[..]).into(),
        );
        obj.into()
    }

    /// Returns all corporation summary data as typed arrays for fast leaderboard rendering.
    /// ids: Uint32Array, cash/revenue/cost: Float64Array (3 floats per corp)
    pub fn get_corporations_typed(&self) -> JsValue {
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

        let obj = js_sys::Object::new();
        let _ = js_sys::Reflect::set(&obj, &"count".into(), &JsValue::from(count as u32));
        let _ = js_sys::Reflect::set(
            &obj,
            &"ids".into(),
            &Uint32Array::from(&ids[..]).into(),
        );
        let _ = js_sys::Reflect::set(
            &obj,
            &"financials".into(),
            &Float64Array::from(&financials[..]).into(),
        );
        let _ = js_sys::Reflect::set(
            &obj,
            &"name_offsets".into(),
            &Uint32Array::from(&name_offsets[..]).into(),
        );
        let _ = js_sys::Reflect::set(
            &obj,
            &"names_packed".into(),
            &Uint8Array::from(&names_packed[..]).into(),
        );
        obj.into()
    }

    /// Hot-path typed array query for satellite positions (orbital overlay).
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
