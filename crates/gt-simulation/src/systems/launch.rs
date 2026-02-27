use gt_common::types::{EntityId, RocketType, SatelliteStatus};

use crate::world::GameWorld;

/// Launch system — processes launch queues and sends satellites to orbit.
/// Runs after manufacturing.
pub fn run(world: &mut GameWorld) {
    let tick = world.current_tick();

    let pad_ids: Vec<EntityId> = world.launch_pads.keys().copied().collect();

    for pad_id in pad_ids {
        let pad = match world.launch_pads.get_mut(&pad_id) {
            Some(p) => p,
            None => continue,
        };

        // Check construction complete
        if let Some(construction) = world.constructions.get(&pad_id) {
            if !construction.is_complete(tick) {
                continue;
            }
        }

        // Cooldown
        if pad.cooldown_remaining > 0 {
            pad.cooldown_remaining -= 1;
            continue;
        }

        // Process next launch in queue
        if pad.launch_queue.is_empty() {
            continue;
        }

        let (rocket_type_str, satellite_ids) = pad.launch_queue.remove(0);
        let rocket_type = parse_rocket_type(&rocket_type_str);
        let owner = pad.owner;
        let reusable = pad.reusable;

        process_launch(world, tick, owner, rocket_type, &satellite_ids, reusable);

        // Set cooldown
        let cooldown = rocket_type.cooldown_ticks();
        let cooldown = if reusable {
            (cooldown as f64 * 0.6) as u64 // -40% cooldown with reusability
        } else {
            cooldown
        };

        if let Some(pad) = world.launch_pads.get_mut(&pad_id) {
            pad.cooldown_remaining = cooldown;
        }
    }

    // Process contract launches (no pad needed)
    let contract_launches: Vec<(String, Vec<EntityId>)> =
        world.pending_contract_launches.drain(..).collect();

    for (rocket_type_str, satellite_ids) in contract_launches {
        let rocket_type = parse_rocket_type(&rocket_type_str);

        // Find owner from first satellite
        let owner = satellite_ids
            .first()
            .and_then(|id| world.ownerships.get(id))
            .map(|o| o.owner)
            .unwrap_or(0);

        // Charge launch cost (higher for contract: 1.5x)
        let cost = (rocket_type.launch_cost() as f64 * 1.5) as i64;
        if let Some(fin) = world.financials.get_mut(&owner) {
            fin.cash -= cost;
        }

        process_launch(world, tick, owner, rocket_type, &satellite_ids, false);
    }
}

fn process_launch(
    world: &mut GameWorld,
    tick: u64,
    owner: EntityId,
    rocket_type: RocketType,
    satellite_ids: &[EntityId],
    reusable: bool,
) {
    let sat_count = satellite_ids.len() as u32;

    world.event_queue.push(
        tick,
        gt_common::events::GameEvent::LaunchAttempted {
            owner,
            rocket_type: format!("{:?}", rocket_type),
            satellite_count: sat_count,
        },
    );

    // Check payload capacity
    let total_mass: f64 = satellite_ids
        .iter()
        .filter_map(|id| world.satellites.get(id))
        .map(|s| s.mass_kg)
        .sum();

    if total_mass > rocket_type.payload_capacity_kg() {
        world.event_queue.push(
            tick,
            gt_common::events::GameEvent::LaunchFailed {
                owner,
                satellite_count: sat_count,
                debris_created: 0,
            },
        );
        return;
    }

    // Charge launch cost (for owned pad launches)
    let cost = rocket_type.launch_cost();
    let cost = if reusable {
        (cost as f64 * 0.35) as i64 // -65% cost with reusability
    } else {
        cost
    };
    if let Some(fin) = world.financials.get_mut(&owner) {
        fin.cash -= cost;
    }

    // Reliability roll
    let reliability = rocket_type.reliability();
    let roll = world.deterministic_random();

    if roll > reliability {
        // Launch failure
        let debris_created = satellite_ids.len() as u32 * 2; // 2 debris per sat

        // Destroy satellites
        for sat_id in satellite_ids {
            if let Some(sat) = world.satellites.get_mut(sat_id) {
                sat.status = SatelliteStatus::Dead;
            }
        }

        // Add debris to LEO-low shell (launch failures happen at low alt)
        for shell in &mut world.orbital_shells {
            if shell.min_altitude_km <= 300.0 && shell.max_altitude_km > 300.0 {
                shell.debris_count += debris_created;
                shell.collision_probability = shell.debris_count as f64 * 1e-7;
                break;
            }
        }

        world.event_queue.push(
            tick,
            gt_common::events::GameEvent::LaunchFailed {
                owner,
                satellite_count: sat_count,
                debris_created,
            },
        );
    } else {
        // Launch success — satellites go operational
        for sat_id in satellite_ids {
            if let Some(sat) = world.satellites.get_mut(sat_id) {
                sat.status = SatelliteStatus::Operational;
                sat.launched_tick = tick;
            }
        }

        world.event_queue.push(
            tick,
            gt_common::events::GameEvent::LaunchSucceeded {
                owner,
                satellite_count: sat_count,
            },
        );

        // Emit individual satellite operational events
        for sat_id in satellite_ids {
            world.event_queue.push(
                tick,
                gt_common::events::GameEvent::SatelliteOperational {
                    satellite_id: *sat_id,
                    owner,
                },
            );
        }
    }
}

fn parse_rocket_type(s: &str) -> RocketType {
    match s {
        "Small" => RocketType::Small,
        "Medium" => RocketType::Medium,
        "Heavy" => RocketType::Heavy,
        "SuperHeavy" => RocketType::SuperHeavy,
        _ => RocketType::Medium,
    }
}
