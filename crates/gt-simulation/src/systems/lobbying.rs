use crate::world::GameWorld;
use gt_common::types::{EntityId, LobbyPolicy};

pub fn run(world: &mut GameWorld) {
    let tick = world.current_tick();

    // Only run every 10 ticks
    if !tick.is_multiple_of(10) {
        return;
    }

    let mut campaign_ids: Vec<EntityId> = world.lobbying_campaigns.keys().copied().collect();
    campaign_ids.sort_unstable();

    let mut succeeded: Vec<(EntityId, EntityId, EntityId, LobbyPolicy)> = Vec::new(); // (campaign_id, corp, region, policy)
    let mut scandals: Vec<(EntityId, EntityId)> = Vec::new(); // (campaign_id, corp)
    let mut failed: Vec<EntityId> = Vec::new();

    for campaign_id in &campaign_ids {
        let campaign = match world.lobbying_campaigns.get(campaign_id) {
            Some(c) if c.active => c.clone(),
            _ => continue,
        };

        // Spend budget incrementally
        let spend_per_cycle = (campaign.budget_total / 20).max(10_000);
        let remaining_budget = campaign.budget_total - campaign.budget_spent;
        let actual_spend = spend_per_cycle.min(remaining_budget);

        if actual_spend <= 0 {
            failed.push(*campaign_id);
            continue;
        }

        // Deduct from corp
        if let Some(fin) = world.financials.get_mut(&campaign.corporation) {
            fin.cash -= actual_spend;
        }

        // Calculate influence gain with diminishing returns
        let influence_gain = actual_spend as f64
            / (1.0 + campaign.budget_spent as f64 / 1_000_000.0)
            / 1_000_000.0;

        if let Some(c) = world.lobbying_campaigns.get_mut(campaign_id) {
            c.budget_spent += actual_spend;
            c.influence += influence_gain;
        }

        let new_influence = campaign.influence + influence_gain;

        // Check for scandal (5% chance per $500K spent, increasing)
        let scandal_chance = campaign.budget_spent as f64 / 500_000.0 * 0.05;
        let rng_val =
            ((tick.wrapping_mul(*campaign_id) >> 12) % 1000) as f64 / 1000.0;
        if rng_val < scandal_chance {
            scandals.push((*campaign_id, campaign.corporation));
            continue;
        }

        // Check if influence threshold reached
        if new_influence >= campaign.influence_threshold() {
            succeeded.push((
                *campaign_id,
                campaign.corporation,
                campaign.region,
                campaign.policy,
            ));
        }
    }

    // Process successes
    for (campaign_id, corp_id, region_id, policy) in succeeded {
        let effect = match policy {
            LobbyPolicy::ReduceTax => {
                if let Some(region) = world.regions.get_mut(&region_id) {
                    region.tax_rate = (region.tax_rate - 0.1).max(0.05);
                }
                "Tax reduced by 10%"
            }
            LobbyPolicy::RelaxZoning => {
                if let Some(region) = world.regions.get_mut(&region_id) {
                    region.regulatory_strictness =
                        (region.regulatory_strictness - 0.15).max(0.1);
                }
                "Zoning relaxed"
            }
            LobbyPolicy::FastTrackPermits => "Construction speed +20%",
            LobbyPolicy::IncreasedCompetitorBurden => {
                if let Some(region) = world.regions.get_mut(&region_id) {
                    region.regulatory_strictness =
                        (region.regulatory_strictness + 0.1).min(1.0);
                }
                "Competitor burden increased"
            }
            LobbyPolicy::SubsidyRequest => {
                if let Some(fin) = world.financials.get_mut(&corp_id) {
                    fin.cash += 1_000_000;
                }
                "Subsidy of $1M received"
            }
        };

        world.event_queue.push(
            tick,
            gt_common::events::GameEvent::LobbyingSucceeded {
                corporation: corp_id,
                region: region_id,
                effect: effect.to_string(),
            },
        );

        if let Some(c) = world.lobbying_campaigns.get_mut(&campaign_id) {
            c.active = false;
        }
    }

    // Process scandals
    for (campaign_id, corp_id) in scandals {
        if let Some(corp) = world.corporations.get_mut(&corp_id) {
            let loss = 10.0;
            corp.reputation = (corp.reputation - loss).max(0.0);
            world.event_queue.push(
                tick,
                gt_common::events::GameEvent::ScandalOccurred {
                    corporation: corp_id,
                    reputation_loss: loss,
                },
            );
        }

        if let Some(c) = world.lobbying_campaigns.get_mut(&campaign_id) {
            c.active = false;
        }
    }

    // Process failures
    for campaign_id in failed {
        let corp_id = world
            .lobbying_campaigns
            .get(&campaign_id)
            .map(|c| c.corporation);
        let region = world
            .lobbying_campaigns
            .get(&campaign_id)
            .map(|c| c.region);

        if let (Some(corp), Some(reg)) = (corp_id, region) {
            world.event_queue.push(
                tick,
                gt_common::events::GameEvent::LobbyingFailed {
                    corporation: corp,
                    region: reg,
                },
            );
        }

        if let Some(c) = world.lobbying_campaigns.get_mut(&campaign_id) {
            c.active = false;
        }
    }

    // Cleanup inactive campaigns older than 200 ticks
    let to_remove: Vec<EntityId> = world
        .lobbying_campaigns
        .iter()
        .filter(|(_, c)| !c.active && tick > c.start_tick + 200)
        .map(|(&id, _)| id)
        .collect();
    for id in to_remove {
        world.lobbying_campaigns.remove(&id);
    }
}
