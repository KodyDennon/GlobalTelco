use crate::components::patent::LicenseType;
use crate::world::GameWorld;
use gt_common::types::EntityId;

pub fn run(world: &mut GameWorld) {
    let tick = world.current_tick();

    // Only run every 5 ticks for performance
    if !tick.is_multiple_of(5) {
        return;
    }

    // --- Royalty collection ---
    // Collect per-tick royalties from all active Royalty-type licenses
    let mut royalty_transfers: Vec<(EntityId, EntityId, i64)> = Vec::new(); // (licensee, holder, amount)

    let license_ids: Vec<EntityId> = world.licenses.keys().copied().collect();
    for license_id in &license_ids {
        let license = match world.licenses.get(license_id) {
            Some(l) if l.is_active(tick) => l,
            _ => continue,
        };

        if let LicenseType::Royalty = license.license_type {
            let patent = match world.patents.get(&license.patent_id) {
                Some(p) => p,
                None => continue,
            };

            let royalty = patent.royalty_per_tick() * 5; // multiply by interval (every 5 ticks)
            if royalty > 0 {
                royalty_transfers.push((license.licensee_corp, patent.holder_corp, royalty));
            }
        }
    }

    for (licensee, holder, amount) in royalty_transfers {
        // Deduct from licensee
        if let Some(fin) = world.financials.get_mut(&licensee) {
            fin.cash -= amount;
        }
        // Pay to holder
        if let Some(fin) = world.financials.get_mut(&holder) {
            fin.cash += amount;
        }
    }

    // --- Lease expiration ---
    let mut expired_licenses: Vec<(EntityId, EntityId, EntityId)> = Vec::new(); // (license_id, patent_id, licensee)

    for (&license_id, license) in world.licenses.iter() {
        if let LicenseType::Lease { expires_tick } = license.license_type {
            if tick >= expires_tick {
                expired_licenses.push((license_id, license.patent_id, license.licensee_corp));
            }
        }
    }

    for (license_id, patent_id, licensee) in expired_licenses {
        world.licenses.shift_remove(&license_id);

        // Remove from tech_research licensed_to
        if let Some(patent) = world.patents.get(&patent_id) {
            let tech_id = patent.tech_id;
            if let Some(tech) = world.tech_research.get_mut(&tech_id) {
                tech.licensed_to.retain(|&id| id != licensee);
            }
        }

        world.event_queue.push(
            tick,
            gt_common::events::GameEvent::LicenseRevoked {
                license_id,
                patent_id,
                licensee,
            },
        );
    }
}
