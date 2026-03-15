//! Constellation, orbital, launch, terminal, and debris queries.

use gt_common::types::EntityId;
use gt_simulation::world::GameWorld;

pub fn query_satellite_inventory(world: &GameWorld, corp_id: EntityId) -> String {
    let inventory: Vec<serde_json::Value> = world
        .satellites
        .iter()
        .filter(|(_, s)| {
            let owner = world.ownerships.get(&s.constellation_id).map(|o| o.owner).unwrap_or(0);
            owner == corp_id && s.status == gt_common::types::SatelliteStatus::AwaitingLaunch
        })
        .map(|(&id, s)| {
            let const_name = world.constellations.get(&s.constellation_id).map(|c| c.name.as_str()).unwrap_or("Independent");
            serde_json::json!({
                "id": id,
                "constellation_id": s.constellation_id,
                "constellation_name": const_name,
                "orbit_type": s.orbit_type,
                "mass_kg": s.mass_kg,
            })
        })
        .collect();
    serde_json::to_string(&inventory).unwrap_or_default()
}

pub fn query_constellation_data(world: &GameWorld, corp_id: EntityId) -> String {
    let constellations: Vec<serde_json::Value> = world
        .constellations
        .iter()
        .filter(|(_, c)| c.owner == corp_id)
        .map(|(&id, c)| {
            serde_json::json!({
                "id": id,
                "name": c.name,
                "orbit_type": c.orbit_type,
                "target_altitude_km": c.target_altitude_km,
                "target_inclination_deg": c.target_inclination_deg,
                "num_planes": c.num_planes,
                "sats_per_plane": c.sats_per_plane,
                "total_target": c.num_planes * c.sats_per_plane,
                "operational_count": c.operational_count,
                "satellite_ids": c.satellite_ids,
            })
        })
        .collect();
    serde_json::to_string(&constellations).unwrap_or_default()
}

pub fn query_orbital_view(world: &GameWorld) -> String {
    let sats: Vec<serde_json::Value> = world
        .satellites
        .iter()
        .map(|(&id, sat)| {
            let pos = world.positions.get(&id);
            let owner = world.ownerships.get(&id).map(|o| o.owner).unwrap_or(0);
            serde_json::json!({
                "id": id,
                "owner": owner,
                "lon": pos.map(|p| p.x).unwrap_or(0.0),
                "lat": pos.map(|p| p.y).unwrap_or(0.0),
                "altitude_km": sat.altitude_km,
                "orbit_type": sat.orbit_type,
                "status": sat.status,
                "fuel_remaining": sat.fuel_remaining,
                "fuel_capacity": sat.fuel_capacity,
                "constellation_id": sat.constellation_id,
            })
        })
        .collect();
    serde_json::to_string(&sats).unwrap_or_default()
}

pub fn query_launch_schedule(world: &GameWorld, corp_id: EntityId) -> String {
    let launches: Vec<serde_json::Value> = world
        .launch_pads
        .iter()
        .filter(|(_, lp)| lp.owner == corp_id)
        .map(|(&id, lp)| {
            serde_json::json!({
                "launch_pad_id": id,
                "cooldown_remaining": lp.cooldown_remaining,
                "reusable": lp.reusable,
                "queue": lp.launch_queue.iter().map(|(rt, sats)| {
                    serde_json::json!({
                        "rocket_type": rt,
                        "satellite_count": sats.len(),
                    })
                }).collect::<Vec<_>>(),
            })
        })
        .collect();
    serde_json::to_string(&launches).unwrap_or_default()
}

pub fn query_terminal_inventory(world: &GameWorld, corp_id: EntityId) -> String {
    let factories: Vec<serde_json::Value> = world
        .terminal_factories
        .iter()
        .filter(|(_, tf)| tf.owner == corp_id)
        .map(|(&id, tf)| {
            serde_json::json!({
                "factory_id": id,
                "tier": tf.tier,
                "produced_stored": tf.produced_stored,
                "production_progress": tf.production_progress,
            })
        })
        .collect();

    let warehouses: Vec<serde_json::Value> = world
        .warehouses
        .iter()
        .filter(|(_, wh)| wh.owner == corp_id)
        .map(|(&id, wh)| {
            serde_json::json!({
                "warehouse_id": id,
                "region_id": wh.region_id,
                "terminal_inventory": wh.terminal_inventory,
                "distribution_rate": wh.distribution_rate,
            })
        })
        .collect();

    serde_json::json!({ "factories": factories, "warehouses": warehouses }).to_string()
}

pub fn query_debris_status(world: &GameWorld) -> String {
    let shells: Vec<serde_json::Value> = world
        .orbital_shells
        .iter()
        .enumerate()
        .map(|(i, shell)| {
            serde_json::json!({
                "index": i,
                "min_altitude_km": shell.min_altitude_km,
                "max_altitude_km": shell.max_altitude_km,
                "debris_count": shell.debris_count,
                "collision_probability": shell.collision_probability,
                "kessler_threshold": shell.kessler_threshold,
                "cascade_active": shell.cascade_active,
            })
        })
        .collect();
    serde_json::to_string(&shells).unwrap_or_default()
}
