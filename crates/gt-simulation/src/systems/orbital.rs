use gt_common::types::{SatelliteStatus, Tick};

use crate::world::GameWorld;

/// Orbital mechanics system — advances satellite positions each tick.
/// Runs after construction, before satellite_network.
pub fn run(world: &mut GameWorld) {
    let tick = world.current_tick();
    // Approximate tick duration: 1 tick ≈ 1 game-minute ≈ 60 real seconds at Normal speed
    let tick_duration_seconds = 60.0;

    let sat_ids: Vec<_> = world.satellites.keys().copied().collect();

    let mut dead_sats = Vec::new();
    let mut decaying_sats = Vec::new();

    for sat_id in sat_ids {
        let sat = match world.satellites.get_mut(&sat_id) {
            Some(s) => s,
            None => continue,
        };

        match sat.status {
            SatelliteStatus::Manufacturing | SatelliteStatus::AwaitingLaunch => continue,
            SatelliteStatus::Dead => continue,
            SatelliteStatus::Deorbiting => {
                // Rapid altitude decrease
                sat.altitude_km -= 50.0;
                if sat.altitude_km < 100.0 {
                    sat.status = SatelliteStatus::Dead;
                    dead_sats.push(sat_id);
                }
                update_position(world, sat_id, tick, tick_duration_seconds);
                continue;
            }
            SatelliteStatus::Operational | SatelliteStatus::Decaying => {}
        }

        // Advance mean anomaly based on orbital period
        let mean_motion = sat.mean_motion_deg_per_sec();
        sat.mean_anomaly_deg += mean_motion * tick_duration_seconds;
        while sat.mean_anomaly_deg >= 360.0 {
            sat.mean_anomaly_deg -= 360.0;
        }

        // Station-keeping fuel consumption (only operational)
        if sat.status == SatelliteStatus::Operational {
            sat.fuel_remaining -= sat.station_keeping_rate;
            if sat.fuel_remaining <= 0.0 {
                sat.fuel_remaining = 0.0;
                sat.status = SatelliteStatus::Decaying;
                decaying_sats.push((sat_id, sat.constellation_id));
            }
        }

        // Decaying satellites lose altitude
        if sat.status == SatelliteStatus::Decaying {
            // Lose ~1km per tick due to atmospheric drag (simplified)
            let drag_rate = if sat.altitude_km < 400.0 { 5.0 } else { 1.0 };
            sat.altitude_km -= drag_rate;

            if sat.altitude_km < 150.0 {
                sat.status = SatelliteStatus::Dead;
                dead_sats.push(sat_id);
            }
        }

        // Update position component from orbital elements
        update_position(world, sat_id, tick, tick_duration_seconds);
    }

    // Process dead satellites — add debris, emit events
    for sat_id in &dead_sats {
        if let Some(sat) = world.satellites.get(sat_id) {
            let owner = world
                .ownerships
                .get(sat_id)
                .map(|o| o.owner)
                .unwrap_or(0);
            let altitude = sat.altitude_km.max(200.0);

            // Add 1 debris to the appropriate shell
            add_debris_for_altitude(world, altitude, 1);

            world.event_queue.push(
                tick,
                gt_common::events::GameEvent::SatelliteDead {
                    satellite_id: *sat_id,
                    owner,
                },
            );
        }
    }

    // Emit decaying events
    for (sat_id, _constellation_id) in &decaying_sats {
        let owner = world
            .ownerships
            .get(sat_id)
            .map(|o| o.owner)
            .unwrap_or(0);
        world.event_queue.push(
            tick,
            gt_common::events::GameEvent::SatelliteDecaying {
                satellite_id: *sat_id,
                owner,
            },
        );
    }

    // Update constellation operational counts
    update_constellation_counts(world);
}

fn update_position(world: &mut GameWorld, sat_id: u64, tick: Tick, tick_duration_seconds: f64) {
    if let Some(sat) = world.satellites.get(&sat_id) {
        let elapsed = tick as f64 * tick_duration_seconds;
        let (lon, lat) = sat.sub_satellite_point(elapsed);

        if let Some(pos) = world.positions.get_mut(&sat_id) {
            pos.x = lon;
            pos.y = lat;
        }
    }
}

fn add_debris_for_altitude(world: &mut GameWorld, altitude_km: f64, count: u32) {
    for shell in &mut world.orbital_shells {
        if altitude_km >= shell.min_altitude_km && altitude_km < shell.max_altitude_km {
            shell.debris_count += count;
            // Recalculate collision probability
            // Base rate: 1e-7 per debris item per satellite per tick
            shell.collision_probability = shell.debris_count as f64 * 1e-7;
            break;
        }
    }
}

fn update_constellation_counts(world: &mut GameWorld) {
    let constellation_ids: Vec<_> = world.constellations.keys().copied().collect();

    for const_id in constellation_ids {
        let sat_ids = match world.constellations.get(&const_id) {
            Some(c) => c.satellite_ids.clone(),
            None => continue,
        };

        let operational = sat_ids
            .iter()
            .filter(|id| {
                world
                    .satellites
                    .get(id)
                    .map(|s| s.status == SatelliteStatus::Operational)
                    .unwrap_or(false)
            })
            .count() as u32;

        if let Some(constellation) = world.constellations.get_mut(&const_id) {
            constellation.operational_count = operational;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::satellite::Satellite;
    use crate::components::Position;
    use gt_common::types::{OrbitType, SatelliteStatus};

    fn make_test_satellite(altitude_km: f64, inclination_deg: f64) -> Satellite {
        Satellite {
            orbit_type: OrbitType::LEO,
            altitude_km,
            base_altitude_km: altitude_km,
            inclination_deg,
            ascending_node_deg: 0.0,
            mean_anomaly_deg: 0.0,
            fuel_remaining: 1.0,
            fuel_capacity: 100.0,
            station_keeping_rate: 0.0001,
            status: SatelliteStatus::Operational,
            constellation_id: 0,
            mass_kg: 260.0,
            max_isl_links: 4,
            coverage_cone_half_angle_deg: 45.0,
            launched_tick: 0,
            plane_index: 0,
            index_in_plane: 0,
        }
    }

    #[test]
    fn test_satellite_position_advances() {
        let mut world = GameWorld::new(gt_common::types::WorldConfig::default());
        let sat_id = 1000;

        world
            .satellites
            .insert(sat_id, make_test_satellite(550.0, 53.0));
        world.positions.insert(
            sat_id,
            Position::new(0.0, 0.0),
        );

        // Run orbital system
        run(&mut world);

        // Mean anomaly should have advanced
        let sat = world.satellites.get(&sat_id).unwrap();
        assert!(sat.mean_anomaly_deg > 0.0, "Mean anomaly should advance");
    }

    #[test]
    fn test_satellite_fuel_depletion() {
        let mut world = GameWorld::new(gt_common::types::WorldConfig::default());
        let sat_id = 1001;

        let mut sat = make_test_satellite(550.0, 53.0);
        sat.fuel_remaining = 0.00005; // Almost empty
        sat.station_keeping_rate = 0.0001;
        world.satellites.insert(sat_id, sat);
        world.positions.insert(
            sat_id,
            Position::new(0.0, 0.0),
        );
        world.ownerships.insert(
            sat_id,
            crate::components::Ownership {
                owner: 1,
                co_owners: Vec::new(),
            },
        );

        run(&mut world);

        let sat = world.satellites.get(&sat_id).unwrap();
        assert_eq!(
            sat.status,
            SatelliteStatus::Decaying,
            "Satellite should be decaying after fuel depletion"
        );
    }
}
