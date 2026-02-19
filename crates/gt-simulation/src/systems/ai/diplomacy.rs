//! AI diplomacy — auctions, acquisitions, espionage, and lobbying.
//!
//! Handles all AI interactions with other corporations beyond contracts:
//! bidding on asset auctions, evaluating acquisition offers,
//! conducting espionage (Aggressive Expanders only), and lobbying regulators.

use crate::components::*;
use crate::world::GameWorld;
use gt_common::types::*;

// ─── Auction Bidding ─────────────────────────────────────────────────────────

/// Bid on active asset auctions if the assets are worth acquiring.
pub fn bid_on_auctions(
    world: &mut GameWorld,
    corp_id: EntityId,
    ai: &AiState,
    fin: &Financial,
    tick: Tick,
) {
    let mut auction_ids: Vec<EntityId> = world
        .auctions
        .iter()
        .filter(|(_, a)| {
            a.status == crate::components::AuctionStatus::Open && a.seller != corp_id
        })
        .map(|(&id, _)| id)
        .collect();
    auction_ids.sort_unstable();

    for auction_id in auction_ids {
        let auction = match world.auctions.get(&auction_id) {
            Some(a) => a.clone(),
            None => continue,
        };

        let asset_value: i64 = auction
            .assets
            .iter()
            .filter_map(|&id| world.infra_nodes.get(&id).map(|n| n.construction_cost))
            .sum();

        let (willingness, bid_multiplier) = match ai.archetype {
            AIArchetype::AggressiveExpander => (0.8, 0.6),
            AIArchetype::DefensiveConsolidator => (0.4, 0.3),
            AIArchetype::TechInnovator => (0.5, 0.4),
            AIArchetype::BudgetOperator => (0.6, 0.25),
        };

        let check =
            ((tick.wrapping_mul(corp_id).wrapping_mul(auction_id) >> 8) % 100) as f64 / 100.0;
        if check > willingness {
            continue;
        }

        let bid = (asset_value as f64 * bid_multiplier) as i64;
        if bid > 0 && fin.cash > bid * 2 {
            if let Some(a) = world.auctions.get_mut(&auction_id) {
                a.place_bid(corp_id, bid);
                world.event_queue.push(
                    tick,
                    gt_common::events::GameEvent::AuctionBidPlaced {
                        auction: auction_id,
                        bidder: corp_id,
                        amount: bid,
                    },
                );
            }
        }
    }
}

// ─── Acquisition Evaluation ──────────────────────────────────────────────────

/// Evaluate incoming acquisition proposals — accept if premium meets threshold.
pub fn evaluate_acquisitions(
    world: &mut GameWorld,
    corp_id: EntityId,
    ai: &AiState,
    tick: Tick,
) {
    let mut proposals: Vec<(EntityId, EntityId, i64)> = world
        .acquisition_proposals
        .iter()
        .filter(|(_, p)| {
            p.target == corp_id
                && p.status == crate::components::AcquisitionStatus::Pending
        })
        .map(|(&id, p)| (id, p.acquirer, p.offer))
        .collect();
    proposals.sort_unstable_by_key(|t| t.0);

    for (proposal_id, _acquirer, offer) in proposals {
        let node_count = world
            .corp_infra_nodes
            .get(&corp_id)
            .map(|n| n.len())
            .unwrap_or(0);

        let asset_value: i64 = world
            .corp_infra_nodes
            .get(&corp_id)
            .unwrap_or(&Vec::new())
            .iter()
            .filter_map(|&id| world.infra_nodes.get(&id).map(|n| n.construction_cost))
            .sum();
        let cash = world.financials.get(&corp_id).map(|f| f.cash).unwrap_or(0);
        let book_value = asset_value + cash;

        let required_premium = match ai.archetype {
            AIArchetype::AggressiveExpander => 2.0,
            AIArchetype::DefensiveConsolidator => 1.3,
            AIArchetype::TechInnovator => 1.5,
            AIArchetype::BudgetOperator => 1.2,
        };

        let accept = offer >= (book_value as f64 * required_premium) as i64
            || (node_count <= 1 && offer > 0);

        if let Some(p) = world.acquisition_proposals.get_mut(&proposal_id) {
            if accept {
                p.status = crate::components::AcquisitionStatus::Accepted;
                let acquirer = p.acquirer;
                let target = p.target;

                if let Some(fin) = world.financials.get_mut(&acquirer) {
                    fin.cash -= offer;
                }
                world.transfer_corporation_assets(target, acquirer);
                world.event_queue.push(
                    tick,
                    gt_common::events::GameEvent::AcquisitionAccepted { acquirer, target },
                );
            } else {
                p.status = crate::components::AcquisitionStatus::Rejected;
                let acquirer = p.acquirer;
                let target = p.target;
                world.event_queue.push(
                    tick,
                    gt_common::events::GameEvent::AcquisitionRejected { acquirer, target },
                );
            }
        }
    }
}

// ─── Espionage ───────────────────────────────────────────────────────────────

/// Aggressive Expanders conduct espionage on competitors.
pub fn conduct_espionage(
    world: &mut GameWorld,
    corp_id: EntityId,
    fin: &Financial,
    tick: Tick,
) {
    let has_active = world
        .covert_ops
        .get(&corp_id)
        .map(|c| !c.active_missions.is_empty())
        .unwrap_or(false);

    if has_active || fin.cash < 5_000_000 {
        return;
    }

    // Only every ~50 ticks
    let check = ((tick.wrapping_mul(corp_id) >> 5) % 50) as u64;
    if check != 0 {
        return;
    }

    let mut competitors: Vec<EntityId> = world
        .corporations
        .keys()
        .copied()
        .filter(|&id| id != corp_id)
        .collect();
    competitors.sort_unstable();

    if let Some(&target) = competitors.first() {
        let target_security = world
            .covert_ops
            .get(&target)
            .map(|c| c.security_level)
            .unwrap_or(0);
        let cost = 100_000 + target_security as i64 * 100_000;

        if fin.cash > cost * 3 {
            if let Some(f) = world.financials.get_mut(&corp_id) {
                f.cash -= cost;
            }

            let region = world.regions.keys().copied().next().unwrap_or(0);

            let mission = crate::components::Mission {
                mission_type: crate::components::MissionType::Espionage,
                target,
                region,
                start_tick: tick,
                duration: 20,
                cost,
                success_chance: 0.7,
                completed: false,
            };

            world
                .covert_ops
                .entry(corp_id)
                .or_insert_with(crate::components::CovertOps::new)
                .active_missions
                .push(mission);
        }
    }
}

// ─── Lobbying ────────────────────────────────────────────────────────────────

/// Lobby regulators for favorable policies based on archetype preferences.
pub fn lobby(
    world: &mut GameWorld,
    corp_id: EntityId,
    ai: &AiState,
    fin: &Financial,
    tick: Tick,
) {
    let has_active = world
        .lobbying_campaigns
        .values()
        .any(|c| c.corporation == corp_id && c.active);
    if has_active {
        return;
    }

    let lobby_willingness = match ai.archetype {
        AIArchetype::AggressiveExpander => 0.5,
        AIArchetype::DefensiveConsolidator => 0.2,
        AIArchetype::TechInnovator => 0.3,
        AIArchetype::BudgetOperator => 0.7,
    };

    let check = ((tick.wrapping_mul(corp_id) >> 6) % 100) as f64 / 100.0;
    if check > lobby_willingness || fin.cash < 2_000_000 {
        return;
    }

    // Pick a region where we have infrastructure
    let corp_regions: Vec<EntityId> = world
        .corp_infra_nodes
        .get(&corp_id)
        .unwrap_or(&Vec::new())
        .iter()
        .filter_map(|&nid| world.positions.get(&nid).and_then(|p| p.region_id))
        .collect();

    if let Some(&region_id) = corp_regions.first() {
        let policy = match ai.archetype {
            AIArchetype::BudgetOperator => gt_common::types::LobbyPolicy::ReduceTax,
            AIArchetype::AggressiveExpander => {
                gt_common::types::LobbyPolicy::IncreasedCompetitorBurden
            }
            AIArchetype::TechInnovator => gt_common::types::LobbyPolicy::SubsidyRequest,
            AIArchetype::DefensiveConsolidator => gt_common::types::LobbyPolicy::RelaxZoning,
        };

        let budget = fin.cash / 10;
        let campaign =
            crate::components::LobbyingCampaign::new(corp_id, region_id, policy, budget, tick);
        let campaign_id = world.allocate_entity();
        world.lobbying_campaigns.insert(campaign_id, campaign);
    }
}
