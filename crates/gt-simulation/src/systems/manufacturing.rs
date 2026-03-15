use gt_common::types::{EntityId, NodeType, OrbitType, SatelliteStatus};

use crate::components::satellite::Satellite;
use crate::components::{InfraNode, Ownership, Position};
use crate::world::GameWorld;

/// Manufacturing system — satellite and terminal factories produce units.
/// Runs after ftth, before launch.
pub fn run(world: &mut GameWorld) {
    let tick = world.current_tick();

    // Process satellite factories
    let factory_ids: Vec<EntityId> = world.satellite_factories.keys().copied().collect();

    for factory_id in factory_ids {
        let factory = match world.satellite_factories.get(&factory_id) {
            Some(f) => f.clone(),
            None => continue,
        };

        // Check factory node is operational (not under construction)
        if let Some(construction) = world.constructions.get(&factory_id) {
            if !construction.is_complete(tick) {
                continue;
            }
        }

        if factory.queue.is_empty() {
            continue;
        }

        let (constellation_id, _remaining) = factory.queue[0];
        let production_rate = factory.tier.satellite_production_rate();

        let Some(factory_mut) = world.satellite_factories.get_mut(&factory_id) else {
            continue;
        };
        factory_mut.production_progress += production_rate;

        if factory_mut.production_progress >= 1.0 {
            factory_mut.production_progress -= 1.0;

            // Produce one satellite
            let sat_id = world.allocate_entity();

            // Get constellation details
            let (orbit_type, altitude_km, inclination_deg, plane_idx, idx_in_plane) =
                if let Some(constellation) = world.constellations.get(&constellation_id) {
                    let existing_count = constellation.satellite_ids.len() as u32;
                    let spp = constellation.sats_per_plane.max(1);
                    let plane = existing_count / spp;
                    let idx = existing_count % spp;
                    (
                        constellation.orbit_type,
                        constellation.target_altitude_km,
                        constellation.target_inclination_deg,
                        plane,
                        idx,
                    )
                } else {
                    (OrbitType::LEO, 550.0, 53.0, 0, 0)
                };

            // Create satellite component
            let node_type = match orbit_type {
                OrbitType::LEO => NodeType::LEO_Satellite,
                OrbitType::MEO => NodeType::MEO_Satellite,
                OrbitType::GEO => NodeType::GEO_Satellite,
                OrbitType::HEO => NodeType::HEO_Satellite,
            };

            let (fuel_cap, sk_rate, mass, max_isl, cone_angle) = match orbit_type {
                OrbitType::LEO => (100.0, 0.0002, 260.0, 4, 45.0),
                OrbitType::MEO => (200.0, 0.0001, 500.0, 2, 30.0),
                OrbitType::GEO => (500.0, 0.00005, 2000.0, 0, 20.0),
                OrbitType::HEO => (300.0, 0.00015, 800.0, 2, 35.0),
            };

            // Distribute ascending nodes evenly
            let raan = if let Some(constellation) = world.constellations.get(&constellation_id) {
                let num_planes = constellation.num_planes.max(1) as f64;
                (plane_idx as f64 / num_planes) * 360.0
            } else {
                0.0
            };

            // Distribute mean anomaly evenly within plane
            let mean_anomaly = if let Some(constellation) = world.constellations.get(&constellation_id)
            {
                let spp = constellation.sats_per_plane.max(1) as f64;
                (idx_in_plane as f64 / spp) * 360.0
            } else {
                0.0
            };

            let satellite = Satellite {
                orbit_type,
                altitude_km,
                base_altitude_km: altitude_km,
                inclination_deg,
                ascending_node_deg: raan,
                mean_anomaly_deg: mean_anomaly,
                fuel_remaining: fuel_cap,
                fuel_capacity: fuel_cap,
                station_keeping_rate: sk_rate,
                status: SatelliteStatus::AwaitingLaunch,
                constellation_id,
                mass_kg: mass,
                max_isl_links: max_isl,
                coverage_cone_half_angle_deg: cone_angle,
                launched_tick: 0,
                plane_index: plane_idx,
                index_in_plane: idx_in_plane,
            };

            world.satellites.insert(sat_id, satellite);
            world.positions.insert(sat_id, Position::new(0.0, 0.0));
            world.infra_nodes.insert(
                sat_id,
                InfraNode::new(node_type, 0, factory.owner),
            );
            world.ownerships.insert(
                sat_id,
                Ownership {
                    owner: factory.owner,
                    co_owners: Vec::new(),
                },
            );

            // Add to constellation
            if let Some(constellation) = world.constellations.get_mut(&constellation_id) {
                constellation.satellite_ids.push(sat_id);
            }

            // Decrement queue
            let Some(factory_mut) = world.satellite_factories.get_mut(&factory_id) else {
                continue;
            };
            if let Some(first) = factory_mut.queue.first_mut() {
                first.1 = first.1.saturating_sub(1);
                if first.1 == 0 {
                    factory_mut.queue.remove(0);
                }
            }

            world.event_queue.push(
                tick,
                gt_common::events::GameEvent::SatelliteManufactured {
                    satellite_id: sat_id,
                    owner: factory.owner,
                    constellation_id,
                },
            );
        }
    }

    // Process terminal factories
    let term_factory_ids: Vec<EntityId> = world.terminal_factories.keys().copied().collect();

    for factory_id in term_factory_ids {
        if let Some(construction) = world.constructions.get(&factory_id) {
            if !construction.is_complete(tick) {
                continue;
            }
        }

        let factory = match world.terminal_factories.get_mut(&factory_id) {
            Some(f) => f,
            None => continue,
        };

        // Respect production target if set
        if let Some(target) = factory.production_target {
            if factory.produced_stored >= target {
                continue;
            }
        }

        let rate = factory.tier.terminal_production_rate();
        factory.produced_stored += rate;
    }
}
