use crate::components::lawsuit::{LawsuitOutcome, LawsuitStatus};
use crate::world::GameWorld;
use gt_common::types::EntityId;

/// Legal system: resolves active lawsuits and applies financial outcomes.
///
/// Resolution timing: 20-50 ticks after filing (set at filing time).
/// Filing cost: 10% of damages claimed (deducted at filing time).
/// Settlement: defendant pays 60% of claimed damages.
///
/// Resolution outcomes (deterministic based on RNG):
///   - 40% chance: Damages awarded (50-100% of claimed)
///   - 20% chance: Forced licensing
///   - 10% chance: Asset forfeiture
///   - 30% chance: Dismissed
pub fn run(world: &mut GameWorld) {
    let tick = world.current_tick();

    let lawsuit_ids: Vec<EntityId> = world.lawsuits.keys().copied().collect();
    let mut resolved: Vec<(EntityId, LawsuitOutcome)> = Vec::new();

    for &lawsuit_id in &lawsuit_ids {
        let lawsuit = match world.lawsuits.get(&lawsuit_id) {
            Some(l) if l.status == LawsuitStatus::Active => l.clone(),
            _ => continue,
        };

        // Check if ready for resolution
        if !lawsuit.is_ready_for_resolution(tick) {
            continue;
        }

        // Deterministic outcome based on lawsuit ID and tick
        let outcome_roll =
            ((tick.wrapping_mul(lawsuit_id) >> 8) % 100) as f64 / 100.0;

        // Defended lawsuits get 20% damage reduction and higher dismissal chance
        let defense_modifier = if lawsuit.defended { 0.8 } else { 1.0 };
        let dismiss_threshold = if lawsuit.defended { 0.65 } else { 0.7 };

        let outcome = if outcome_roll < 0.4 {
            // 40% chance: Damages awarded (50-100% of claimed, reduced if defended)
            let damage_pct = 0.5 + (outcome_roll / 0.4) * 0.5; // Scale 0.0-1.0 within this range
            let damages = (lawsuit.damages_claimed as f64 * damage_pct * defense_modifier) as i64;
            LawsuitOutcome::DamagesAwarded(damages as f64)
        } else if outcome_roll < 0.6 {
            // 20% chance: Forced licensing
            LawsuitOutcome::ForcedLicensing
        } else if outcome_roll < dismiss_threshold {
            // 10% (5% if defended) chance: Asset forfeiture (forfeit defendant's entity)
            LawsuitOutcome::AssetForfeiture(lawsuit.defendant)
        } else {
            // 30% (35% if defended) chance: Dismissed
            LawsuitOutcome::Dismissed
        };

        resolved.push((lawsuit_id, outcome));
    }

    // Apply resolutions
    for (lawsuit_id, outcome) in resolved {
        let lawsuit = match world.lawsuits.get(&lawsuit_id) {
            Some(l) => l.clone(),
            None => continue,
        };

        // Apply financial outcomes
        match &outcome {
            LawsuitOutcome::DamagesAwarded(amount) => {
                let damages = *amount as i64;
                // Defendant pays, plaintiff receives
                if let Some(fin) = world.financials.get_mut(&lawsuit.defendant) {
                    fin.cash -= damages;
                }
                if let Some(fin) = world.financials.get_mut(&lawsuit.plaintiff) {
                    fin.cash += damages;
                }
            }
            LawsuitOutcome::ForcedLicensing => {
                // Defendant must pay a licensing fee (20% of claimed damages per tick for 50 ticks)
                // Simplified: one-time payment of 20% of claimed damages
                let fee = lawsuit.damages_claimed / 5;
                if let Some(fin) = world.financials.get_mut(&lawsuit.defendant) {
                    fin.cash -= fee;
                }
                if let Some(fin) = world.financials.get_mut(&lawsuit.plaintiff) {
                    fin.cash += fee;
                }
            }
            LawsuitOutcome::AssetForfeiture(_target) => {
                // Transfer a portion of defendant's assets value to plaintiff
                // Simplified: defendant pays 80% of claimed damages
                let penalty = (lawsuit.damages_claimed as f64 * 0.8) as i64;
                if let Some(fin) = world.financials.get_mut(&lawsuit.defendant) {
                    fin.cash -= penalty;
                }
                if let Some(fin) = world.financials.get_mut(&lawsuit.plaintiff) {
                    fin.cash += penalty;
                }
            }
            LawsuitOutcome::Dismissed => {
                // No financial impact — plaintiff loses filing cost (already deducted)
            }
        }

        // Update lawsuit status
        if let Some(l) = world.lawsuits.get_mut(&lawsuit_id) {
            l.resolve(outcome.clone());
        }

        world.event_queue.push(
            tick,
            gt_common::events::GameEvent::LawsuitResolved {
                lawsuit_id,
                plaintiff: lawsuit.plaintiff,
                defendant: lawsuit.defendant,
                outcome: format!("{:?}", outcome),
            },
        );
    }

    // Cleanup resolved/settled lawsuits older than 100 ticks past resolution
    let to_remove: Vec<EntityId> = world
        .lawsuits
        .iter()
        .filter(|(_, l)| {
            (l.status == LawsuitStatus::Resolved || l.status == LawsuitStatus::Settled)
                && tick > l.resolution_tick + 100
        })
        .map(|(&id, _)| id)
        .collect();

    for id in to_remove {
        world.lawsuits.shift_remove(&id);
    }
}
