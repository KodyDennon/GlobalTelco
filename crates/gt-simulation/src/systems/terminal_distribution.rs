use gt_common::types::EntityId;

use crate::world::GameWorld;

/// Terminal distribution system — warehouses distribute terminals to cities,
/// cities adopt based on terminal availability + satellite coverage.
/// Runs after launch.
pub fn run(world: &mut GameWorld) {
    let tick = world.current_tick();

    let warehouse_ids: Vec<EntityId> = world.warehouses.keys().copied().collect();

    for wh_id in warehouse_ids {
        let warehouse = match world.warehouses.get(&wh_id) {
            Some(w) => w.clone(),
            None => continue,
        };

        // Check construction complete
        if let Some(construction) = world.constructions.get(&wh_id) {
            if !construction.is_complete(tick) {
                continue;
            }
        }

        if warehouse.terminal_inventory == 0 {
            continue;
        }

        let region_id = warehouse.region_id;
        let owner = warehouse.owner;

        // Find cities in this region
        let city_ids: Vec<EntityId> = world
            .cities
            .iter()
            .filter(|(_, city)| city.region_id == region_id)
            .map(|(id, _)| *id)
            .collect();

        if city_ids.is_empty() {
            continue;
        }

        let mut terminals_remaining = warehouse.distribution_rate.min(warehouse.terminal_inventory);

        // Distribute terminals evenly across cities, favoring underserved
        let terminals_per_city = (terminals_remaining / city_ids.len() as u32).max(1);

        for city_id in &city_ids {
            if terminals_remaining == 0 {
                break;
            }

            let to_deploy = terminals_per_city.min(terminals_remaining);

            // Create or update subscription for this city/corp
            let sub_key = (*city_id, owner);
            let sub = world
                .satellite_subscriptions
                .entry(sub_key)
                .or_insert_with(|| crate::components::SatelliteSubscription {
                    city_id: *city_id,
                    corp_id: owner,
                    subscribers: 0,
                    terminals_deployed: 0,
                    monthly_rate: 50, // default rate
                });

            sub.terminals_deployed += to_deploy;
            terminals_remaining -= to_deploy;

            world.event_queue.push(
                tick,
                gt_common::events::GameEvent::TerminalsDeployed {
                    city_id: *city_id,
                    count: to_deploy,
                    owner,
                },
            );
        }

        // Deduct from warehouse
        let wh_mut = world.warehouses.get_mut(&wh_id).unwrap();
        let used = warehouse.distribution_rate.min(warehouse.terminal_inventory) - terminals_remaining;
        wh_mut.terminal_inventory -= used;

        // Subscriber adoption: subscribers = min(terminals, demand * coverage_factor)
        // First pass: compute coverage and adoption changes
        let mut adoption_updates: Vec<(EntityId, u32)> = Vec::new();
        for city_id in &city_ids {
            let sub_key = (*city_id, owner);
            let (terminals_deployed, current_subs) = match world.satellite_subscriptions.get(&sub_key) {
                Some(sub) => (sub.terminals_deployed, sub.subscribers),
                None => continue,
            };

            let has_coverage = city_has_satellite_coverage(world, *city_id, owner);
            if !has_coverage {
                continue;
            }

            let city_pop = world
                .populations
                .get(city_id)
                .map(|p| p.count)
                .unwrap_or(0) as u32;

            let demand_fraction = if city_pop < 50_000 { 0.20 } else { 0.05 };
            let max_demand = (city_pop as f64 * demand_fraction) as u32;

            let potential = terminals_deployed.min(max_demand);
            let new_subs = potential.saturating_sub(current_subs);

            if new_subs > 0 {
                adoption_updates.push((*city_id, new_subs));
            }
        }

        // Second pass: apply updates
        for (city_id, new_subs) in adoption_updates {
            let sub_key = (city_id, owner);
            if let Some(sub) = world.satellite_subscriptions.get_mut(&sub_key) {
                sub.subscribers += new_subs;
            }
            world.event_queue.push(
                tick,
                gt_common::events::GameEvent::SatelliteSubscribersGained {
                    city_id,
                    owner,
                    new_subscribers: new_subs,
                },
            );
        }
    }
}

fn city_has_satellite_coverage(world: &GameWorld, city_id: EntityId, owner: EntityId) -> bool {
    let city_pos = match world.positions.get(&city_id) {
        Some(p) => p,
        None => return false,
    };

    // Check if any operational satellite owned by this corp covers this city
    for (sat_id, sat) in &world.satellites {
        if sat.status != gt_common::types::SatelliteStatus::Operational {
            continue;
        }
        let sat_owner = world.ownerships.get(sat_id).map(|o| o.owner).unwrap_or(0);
        if sat_owner != owner {
            continue;
        }

        if let Some(sat_pos) = world.positions.get(sat_id) {
            let dist = haversine_km(city_pos.x, city_pos.y, sat_pos.x, sat_pos.y);
            let footprint = sat.footprint_radius_km(25.0); // 25 deg min elevation
            if dist < footprint {
                return true;
            }
        }
    }

    false
}

fn haversine_km(lon1: f64, lat1: f64, lon2: f64, lat2: f64) -> f64 {
    let r = 6371.0;
    let dlat = (lat2 - lat1).to_radians();
    let dlon = (lon2 - lon1).to_radians();
    let a = (dlat / 2.0).sin().powi(2)
        + lat1.to_radians().cos() * lat2.to_radians().cos() * (dlon / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().asin();
    r * c
}
