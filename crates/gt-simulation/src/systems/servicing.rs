use gt_common::types::{SatelliteStatus, ServiceType};

use crate::world::GameWorld;

/// Servicing system — processes active satellite service missions (refuel/repair).
/// Runs after debris.
pub fn run(world: &mut GameWorld) {
    let tick = world.current_tick();

    let mission_ids: Vec<usize> = (0..world.service_missions.len()).rev().collect();

    for idx in mission_ids {
        if idx >= world.service_missions.len() {
            continue;
        }

        let mission = &mut world.service_missions[idx];
        mission.ticks_remaining = mission.ticks_remaining.saturating_sub(1);

        if mission.ticks_remaining == 0 {
            let sat_id = mission.satellite_id;
            let service_type = mission.service_type;
            let cost = mission.cost;

            // Apply service
            match service_type {
                ServiceType::Refuel => {
                    if let Some(sat) = world.satellites.get_mut(&sat_id) {
                        sat.fuel_remaining = sat.fuel_capacity;
                        if sat.status == SatelliteStatus::Decaying {
                            sat.status = SatelliteStatus::Operational;
                            sat.altitude_km = sat.base_altitude_km;
                        }
                    }
                }
                ServiceType::Repair => {
                    if let Some(health) = world.healths.get_mut(&sat_id) {
                        health.condition = 1.0;
                    }
                }
            }

            let owner = world
                .ownerships
                .get(&sat_id)
                .map(|o| o.owner)
                .unwrap_or(0);

            world.event_queue.push(
                tick,
                gt_common::events::GameEvent::SatelliteServiced {
                    satellite_id: sat_id,
                    service_type: format!("{:?}", service_type),
                    cost,
                },
            );

            // Charge cost
            if let Some(fin) = world.financials.get_mut(&owner) {
                fin.cash -= cost;
            }

            // Remove completed mission
            world.service_missions.swap_remove(idx);
        }
    }
}
