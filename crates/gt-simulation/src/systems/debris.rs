use gt_common::types::SatelliteStatus;

use crate::world::GameWorld;

/// Debris system — handles orbital debris accumulation, collision risk, and Kessler cascade.
/// Runs after disaster.
pub fn run(world: &mut GameWorld) {
    let tick = world.current_tick();

    // Update collision probabilities per shell
    for shell in &mut world.orbital_shells {
        shell.collision_probability = shell.debris_count as f64 * 1e-7;

        // Kessler cascade: if above threshold, debris grows each tick
        if shell.debris_count >= shell.kessler_threshold {
            if !shell.cascade_active {
                shell.cascade_active = true;
                world.event_queue.push(
                    tick,
                    gt_common::events::GameEvent::KesslerCascadeStarted {
                        shell_min_km: shell.min_altitude_km,
                        shell_max_km: shell.max_altitude_km,
                        debris_count: shell.debris_count,
                    },
                );
            }
            // Cascade: +1% debris per tick (exponential growth)
            let growth = (shell.debris_count as f64 * 0.01).max(1.0) as u32;
            shell.debris_count += growth;
        }

        // Natural decay for very low LEO (< 400km)
        if shell.max_altitude_km <= 400.0 && shell.debris_count > 0 {
            // ~1 debris piece decays per 100 ticks
            if tick % 100 == 0 {
                shell.debris_count = shell.debris_count.saturating_sub(1);
            }
        }
    }

    // Collision check for each operational satellite
    let sat_ids: Vec<u64> = world
        .satellites
        .iter()
        .filter(|(_, s)| s.status == SatelliteStatus::Operational)
        .map(|(id, _)| *id)
        .collect();

    let mut collisions = Vec::new();

    for sat_id in &sat_ids {
        let altitude = match world.satellites.get(sat_id) {
            Some(s) => s.altitude_km,
            None => continue,
        };

        // Find the shell for this altitude
        let collision_prob = world
            .orbital_shells
            .iter()
            .find(|s| altitude >= s.min_altitude_km && altitude < s.max_altitude_km)
            .map(|s| s.collision_probability)
            .unwrap_or(0.0);

        if collision_prob > 0.0 {
            let roll = world.deterministic_random();
            if roll < collision_prob {
                collisions.push(*sat_id);
            }
        }
    }

    // Process collisions
    for sat_id in collisions {
        let (owner, altitude) = {
            let owner = world.ownerships.get(&sat_id).map(|o| o.owner).unwrap_or(0);
            let altitude = world
                .satellites
                .get(&sat_id)
                .map(|s| s.altitude_km)
                .unwrap_or(500.0);
            (owner, altitude)
        };

        // Destroy satellite
        if let Some(sat) = world.satellites.get_mut(&sat_id) {
            sat.status = SatelliteStatus::Dead;
        }

        // Generate debris (3-10 fragments)
        let debris_count = 3 + (world.deterministic_random() * 7.0) as u32;

        // Add debris to shell
        for shell in &mut world.orbital_shells {
            if altitude >= shell.min_altitude_km && altitude < shell.max_altitude_km {
                shell.debris_count += debris_count;
                shell.collision_probability = shell.debris_count as f64 * 1e-7;
                break;
            }
        }

        world.event_queue.push(
            tick,
            gt_common::events::GameEvent::DebrisCollision {
                satellite_id: sat_id,
                owner,
                debris_created: debris_count,
            },
        );
    }
}
