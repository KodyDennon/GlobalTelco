use crate::world::GameWorld;
use gt_common::types::EntityId;

/// Alliance system: updates trust scores and auto-dissolves alliances with low trust.
///
/// Trust scoring:
///   - +0.01/tick if both members are profitable
///   - -0.02/tick if one member is failing (negative net income)
///
/// Dissolution:
///   - Auto-dissolve if any member's trust < 0.2
///   - Auto-dissolve if fewer than 2 members remain
pub fn run(world: &mut GameWorld) {
    let tick = world.current_tick();

    // Only run every 5 ticks to reduce overhead
    if !tick.is_multiple_of(5) {
        return;
    }

    let alliance_ids: Vec<EntityId> = world.alliances.keys().copied().collect();
    let mut to_dissolve: Vec<(EntityId, String)> = Vec::new();

    for &alliance_id in &alliance_ids {
        let alliance = match world.alliances.get(&alliance_id) {
            Some(a) => a.clone(),
            None => continue,
        };

        // Determine profitability of each member
        let mut member_profitable: Vec<(EntityId, bool)> = Vec::new();
        for &member_id in &alliance.member_corp_ids {
            let profitable = world
                .financials
                .get(&member_id)
                .map(|f| f.net_income() > 0)
                .unwrap_or(false);
            member_profitable.push((member_id, profitable));
        }

        // Update trust scores based on mutual profitability
        // For each member pair, adjust trust:
        //   - Both profitable: +0.01 per tick cycle (run every 5 ticks, so +0.05)
        //   - One failing: -0.02 per tick cycle (so -0.10)
        let all_profitable = member_profitable.iter().all(|(_, p)| *p);
        let any_failing = member_profitable.iter().any(|(_, p)| !*p);

        if let Some(alliance) = world.alliances.get_mut(&alliance_id) {
            for &member_id in &alliance.member_corp_ids.clone() {
                let trust = alliance.trust_scores.entry(member_id).or_insert(0.5);
                if all_profitable {
                    *trust = (*trust + 0.01).min(1.0);
                } else if any_failing {
                    *trust = (*trust - 0.02).max(0.0);
                }
            }

            // Check for low trust dissolution
            if alliance.has_low_trust(0.2) {
                let low_trust_member = alliance
                    .trust_scores
                    .iter()
                    .filter(|(_, &t)| t < 0.2)
                    .map(|(&id, _)| id)
                    .next();
                let reason = if let Some(member_id) = low_trust_member {
                    format!("Trust fell below threshold for member {}", member_id)
                } else {
                    "Trust fell below threshold".to_string()
                };
                to_dissolve.push((alliance_id, reason));
            }

            // Check for defunct alliance (fewer than 2 members)
            if alliance.is_defunct() {
                to_dissolve.push((
                    alliance_id,
                    "Fewer than 2 members remaining".to_string(),
                ));
            }
        }
    }

    // Process dissolutions
    for (alliance_id, reason) in to_dissolve {
        world.alliances.shift_remove(&alliance_id);
        world.event_queue.push(
            tick,
            gt_common::events::GameEvent::AllianceDissolved {
                alliance_id,
                reason,
            },
        );
    }
}
