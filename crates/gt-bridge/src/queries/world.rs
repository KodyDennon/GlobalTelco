//! World, region, city, and population queries.

use gt_simulation::world::GameWorld;

pub fn query_static_definitions() -> String {
    use gt_common::types::{NodeType, EdgeType};

    let node_types: std::collections::HashMap<u8, String> = NodeType::ALL
        .iter()
        .map(|&t| (t as u8, t.to_string()))
        .collect();

    let edge_types: std::collections::HashMap<u8, String> = EdgeType::ALL
        .iter()
        .map(|&t| (t as u8, t.to_string()))
        .collect();

    serde_json::json!({
        "node_types": node_types,
        "edge_types": edge_types,
    }).to_string()
}

pub fn query_world_info(world: &GameWorld) -> String {
    let info = serde_json::json!({
        "tick": world.current_tick(),
        "speed": world.speed(),
        "entity_count": world.entity_count(),
        "region_count": world.regions.len(),
        "city_count": world.cities.len(),
        "corporation_count": world.corporations.len(),
        "infra_node_count": world.infra_nodes.len(),
        "infra_edge_count": world.infra_edges.len(),
        "player_corp_id": world.player_corp_id().unwrap_or(0),
        "cell_spacing_km": world.cell_spacing_km,
        "sandbox": world.config().sandbox,
    });
    serde_json::to_string(&info).unwrap_or_default()
}

pub fn query_regions(world: &GameWorld) -> String {
    let regions: Vec<serde_json::Value> = world
        .regions
        .iter()
        .map(|(&id, r)| {
            let boundary: Vec<serde_json::Value> = r
                .boundary_polygon
                .iter()
                .map(|&(lat, lon)| serde_json::json!([lat, lon]))
                .collect();
            serde_json::json!({
                "id": id,
                "name": r.name,
                "center_lat": r.center_lat,
                "center_lon": r.center_lon,
                "population": r.population,
                "gdp": r.gdp,
                "development": r.development,
                "tax_rate": r.tax_rate,
                "regulatory_strictness": r.regulatory_strictness,
                "disaster_risk": r.disaster_risk,
                "cell_count": r.cells.len(),
                "city_ids": r.city_ids,
                "boundary_polygon": boundary,
            })
        })
        .collect();
    serde_json::to_string(&regions).unwrap_or_default()
}

pub fn query_cities(world: &GameWorld) -> String {
    let cities: Vec<serde_json::Value> = world
        .cities
        .iter()
        .map(|(&id, c)| {
            let pos = world.positions.get(&id);
            let cell_positions: Vec<serde_json::Value> = c
                .cells
                .iter()
                .filter_map(|&ci| {
                    let (lat, lon) = world.grid_cell_positions.get(ci)?;
                    Some(serde_json::json!({"index": ci, "lat": lat, "lon": lon}))
                })
                .collect();
            serde_json::json!({
                "id": id,
                "name": c.name,
                "region_id": c.region_id,
                "cell_index": c.cell_index,
                "cells": c.cells,
                "cell_positions": cell_positions,
                "population": c.population,
                "growth_rate": c.growth_rate,
                "development": c.development,
                "telecom_demand": c.telecom_demand,
                "infrastructure_satisfaction": c.infrastructure_satisfaction,
                "employment_rate": c.employment_rate,
                "jobs_available": c.jobs_available,
                "birth_rate": c.birth_rate,
                "death_rate": c.death_rate,
                "migration_pressure": c.migration_pressure,
                "x": pos.map(|p| p.x).unwrap_or(0.0),
                "y": pos.map(|p| p.y).unwrap_or(0.0),
            })
        })
        .collect();
    serde_json::to_string(&cities).unwrap_or_default()
}
