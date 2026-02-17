use crate::world::GameWorld;

pub fn run(world: &mut GameWorld) {
    let tick = world.current_tick();

    // Regulatory changes happen every 100 ticks
    if !tick.is_multiple_of(100) {
        return;
    }

    let mut region_ids: Vec<u64> = world.regions.keys().copied().collect();
    region_ids.sort_unstable();

    for region_id in region_ids {
        let roll = world.deterministic_random();

        // 10% chance of regulatory change per region per check
        if roll < 0.10 {
            let change_type = world.deterministic_random();
            let description;

            if change_type < 0.3 {
                // Tax rate change
                let delta = (world.deterministic_random() - 0.5) * 0.05; // +/- 2.5%
                if let Some(region) = world.regions.get_mut(&region_id) {
                    region.tax_rate = (region.tax_rate + delta).clamp(0.05, 0.45);
                    description = format!(
                        "Tax rate adjusted to {:.1}% in {}",
                        region.tax_rate * 100.0,
                        region.name
                    );
                } else {
                    continue;
                }
            } else if change_type < 0.6 {
                // Regulatory strictness change
                let delta = (world.deterministic_random() - 0.5) * 0.1;
                if let Some(region) = world.regions.get_mut(&region_id) {
                    region.regulatory_strictness =
                        (region.regulatory_strictness + delta).clamp(0.1, 0.9);
                    description = format!("Regulatory environment changed in {}", region.name);
                } else {
                    continue;
                }
            } else {
                // Zoning change: unlock some protected land for development
                let region_cells = world
                    .regions
                    .get(&region_id)
                    .map(|r| r.cells.clone())
                    .unwrap_or_default();

                let parcels_to_rezone: Vec<u64> = region_cells
                    .iter()
                    .filter_map(|ci| world.cell_to_parcel.get(ci).copied())
                    .filter(|pid| {
                        world
                            .land_parcels
                            .get(pid)
                            .map(|p| p.zoning == crate::components::ZoningType::Protected)
                            .unwrap_or(false)
                    })
                    .take(1)
                    .collect();

                if let Some(&parcel_id) = parcels_to_rezone.first() {
                    if let Some(parcel) = world.land_parcels.get_mut(&parcel_id) {
                        parcel.zoning = crate::components::ZoningType::Mixed;
                    }
                }

                description = format!(
                    "Zoning regulations updated in {}",
                    world
                        .regions
                        .get(&region_id)
                        .map(|r| r.name.as_str())
                        .unwrap_or("Unknown")
                );
            }

            world.event_queue.push(
                tick,
                gt_common::events::GameEvent::RegulationChanged {
                    region: region_id,
                    description,
                },
            );
        }
    }
}
