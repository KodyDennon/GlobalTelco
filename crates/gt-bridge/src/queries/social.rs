//! Alliance, legal, patent, grant, stock market, co-ownership, and spectrum queries.

use gt_common::types::EntityId;
use gt_simulation::world::GameWorld;

pub fn query_grants(world: &GameWorld, corp_id: EntityId) -> String {
    use gt_simulation::components::grant::GrantStatus;
    let tick = world.current_tick();
    let grants: Vec<serde_json::Value> = world
        .grants
        .iter()
        .filter(|(_, g)| {
            g.status == GrantStatus::Available
            || g.awarded_corp == Some(corp_id)
        })
        .map(|(&id, g)| {
            let region_name = world.regions.get(&g.region_id).map(|r| r.name.as_str()).unwrap_or("Unknown");
            serde_json::json!({
                "id": id,
                "region_id": g.region_id,
                "region_name": region_name,
                "required_coverage": g.required_coverage_pct,
                "current_coverage": g.progress,
                "reward": g.reward_cash,
                "deadline_tick": g.deadline_tick,
                "ticks_remaining": g.deadline_tick.saturating_sub(tick),
                "status": match g.status {
                    GrantStatus::Available => "available",
                    GrantStatus::Awarded => "active",
                    GrantStatus::Completed => "completed",
                    GrantStatus::Expired => "failed",
                },
                "is_holder": g.awarded_corp == Some(corp_id),
            })
        })
        .collect();
    serde_json::to_string(&grants).unwrap_or_default()
}

pub fn query_spectrum_licenses(world: &GameWorld) -> String {
    let tick = world.current_tick();
    let licenses: Vec<serde_json::Value> = world
        .spectrum_licenses
        .iter()
        .filter(|(_, l)| l.is_active(tick))
        .map(|(&id, l)| {
            let region_name = world
                .regions
                .get(&l.region_id)
                .map(|r| r.name.as_str())
                .unwrap_or("Unknown");
            let owner_name = world
                .corporations
                .get(&l.owner)
                .map(|c| c.name.as_str())
                .unwrap_or("Unknown");
            serde_json::json!({
                "id": id,
                "band": l.band,
                "band_name": l.band.display_name(),
                "band_category": l.band.category(),
                "region_id": l.region_id,
                "region_name": region_name,
                "owner": l.owner,
                "owner_name": owner_name,
                "bandwidth_mhz": l.bandwidth_mhz,
                "start_tick": l.start_tick,
                "end_tick": l.end_tick(),
                "cost_per_tick": l.cost_per_tick(),
                "coverage_radius_km": l.band.coverage_radius_km(),
            })
        })
        .collect();
    serde_json::to_string(&licenses).unwrap_or_default()
}

pub fn query_spectrum_auctions(world: &GameWorld) -> String {
    let tick = world.current_tick();
    let auctions: Vec<serde_json::Value> = world
        .spectrum_auctions
        .iter()
        .filter(|(_, a)| !a.is_ended(tick))
        .map(|(&id, a)| {
            let region_name = world
                .regions
                .get(&a.region_id)
                .map(|r| r.name.as_str())
                .unwrap_or("Unknown");
            let (highest_bidder, current_bid) = a.highest_bid().unwrap_or((0, 0));
            let bidder_name = world
                .corporations
                .get(&highest_bidder)
                .map(|c| c.name.as_str())
                .unwrap_or("None");
            serde_json::json!({
                "id": id,
                "band": a.band,
                "band_name": a.band.display_name(),
                "band_category": a.band.category(),
                "region_id": a.region_id,
                "region_name": region_name,
                "bandwidth_mhz": a.bandwidth_mhz,
                "current_bid": current_bid,
                "highest_bidder": highest_bidder,
                "bidder_name": bidder_name,
                "end_tick": a.end_tick,
                "ticks_remaining": a.ticks_remaining(tick),
                "coverage_radius_km": a.band.coverage_radius_km(),
            })
        })
        .collect();
    serde_json::to_string(&auctions).unwrap_or_default()
}

pub fn query_available_spectrum(world: &GameWorld, region_id: EntityId) -> String {
    use gt_common::types::FrequencyBand;
    let tick = world.current_tick();

    let licensed_bands: std::collections::HashSet<gt_common::types::FrequencyBand> = world
        .spectrum_licenses
        .values()
        .filter(|l| l.region_id == region_id && l.is_active(tick))
        .map(|l| l.band)
        .collect();

    let auction_bands: std::collections::HashSet<gt_common::types::FrequencyBand> = world
        .spectrum_auctions
        .values()
        .filter(|a| a.region_id == region_id && !a.is_ended(tick))
        .map(|a| a.band)
        .collect();

    let available: Vec<serde_json::Value> = FrequencyBand::all()
        .iter()
        .filter(|b| {
            !licensed_bands.contains(b) && !auction_bands.contains(b)
        })
        .map(|b| {
            serde_json::json!({
                "band": b,
                "band_name": b.display_name(),
                "band_category": b.category(),
                "coverage_radius_km": b.coverage_radius_km(),
                "max_bandwidth_mhz": b.max_bandwidth_mhz(),
                "min_bid": b.cost_per_mhz() * b.max_bandwidth_mhz() as i64,
            })
        })
        .collect();
    serde_json::to_string(&available).unwrap_or_default()
}

pub fn query_co_ownership_proposals(world: &GameWorld, corp_id: EntityId) -> String {
    let proposals: Vec<serde_json::Value> = world
        .co_ownership_proposals
        .iter()
        .filter(|(_, &(proposer, target, _))| proposer == corp_id || target == corp_id)
        .map(|(&node_id, &(proposer, target, share))| {
            let node = world.infra_nodes.get(&node_id);
            let proposer_name = world.corporations.get(&proposer).map(|c| c.name.as_str()).unwrap_or("Unknown");
            let target_name = world.corporations.get(&target).map(|c| c.name.as_str()).unwrap_or("Unknown");
            serde_json::json!({
                "id": node_id, // Use node_id as proposal ID
                "node_id": node_id,
                "node_type": node.map(|n| n.node_type).unwrap_or(gt_common::types::NodeType::CellTower),
                "from_corp": proposer,
                "from_name": proposer_name,
                "to_corp": target,
                "to_name": target_name,
                "share_pct": share * 100.0,
                "direction": if proposer == corp_id { "outgoing" } else { "incoming" },
            })
        })
        .collect();
    serde_json::to_string(&proposals).unwrap_or_default()
}

pub fn query_pending_upgrade_votes(world: &GameWorld, corp_id: EntityId) -> String {
    let votes: Vec<serde_json::Value> = world
        .pending_upgrade_votes
        .iter()
        .filter(|(&node_id, _)| {
            // Player must be an owner or co-owner to see/vote
            world.ownerships.get(&node_id).map(|o| {
                o.owner == corp_id || o.co_owners.iter().any(|(id, _)| *id == corp_id)
            }).unwrap_or(false)
        })
        .map(|(&node_id, (proposer, votes_map, _))| {
            let node = world.infra_nodes.get(&node_id);
            let proposer_name = world.corporations.get(proposer).map(|c| c.name.as_str()).unwrap_or("Unknown");

            // Map votes_map keys to strings for JSON
            let votes_data: std::collections::HashMap<String, bool> = votes_map
                .iter()
                .map(|(cid, val)| (cid.to_string(), *val))
                .collect();

            serde_json::json!({
                "node_id": node_id,
                "node_type": node.map(|n| n.node_type).unwrap_or(gt_common::types::NodeType::CellTower),
                "proposer_id": proposer,
                "proposer_name": proposer_name,
                "votes": votes_data,
                "has_voted": votes_map.contains_key(&corp_id),
            })
        })
        .collect();
    serde_json::to_string(&votes).unwrap_or_default()
}

pub fn query_alliances(world: &GameWorld, corp_id: EntityId) -> String {
    let alliances: Vec<serde_json::Value> = world
        .alliances
        .iter()
        .filter(|(_, a)| a.member_corp_ids.contains(&corp_id))
        .map(|(&id, a)| {
            let member_names: Vec<String> = a
                .member_corp_ids
                .iter()
                .filter_map(|cid| world.corporations.get(cid).map(|c| c.name.clone()))
                .collect();
            let trust_map: std::collections::HashMap<String, f64> = a
                .trust_scores
                .iter()
                .map(|(cid, &score)| (cid.to_string(), score))
                .collect();
            serde_json::json!({
                "id": id,
                "name": a.name,
                "member_corp_ids": a.member_corp_ids,
                "member_names": member_names,
                "trust_scores": trust_map,
                "revenue_share_pct": a.revenue_share_pct,
                "formed_tick": a.formed_tick,
            })
        })
        .collect();
    serde_json::to_string(&alliances).unwrap_or_default()
}

pub fn query_lawsuits(world: &GameWorld, corp_id: EntityId) -> String {
    let lawsuits: Vec<serde_json::Value> = world
        .lawsuits
        .iter()
        .filter(|(_, l)| l.plaintiff == corp_id || l.defendant == corp_id)
        .map(|(&id, l)| {
            let plaintiff_name = world
                .corporations
                .get(&l.plaintiff)
                .map(|c| c.name.as_str())
                .unwrap_or("Unknown");
            let defendant_name = world
                .corporations
                .get(&l.defendant)
                .map(|c| c.name.as_str())
                .unwrap_or("Unknown");
            serde_json::json!({
                "id": id,
                "plaintiff": l.plaintiff,
                "plaintiff_name": plaintiff_name,
                "defendant": l.defendant,
                "defendant_name": defendant_name,
                "lawsuit_type": l.lawsuit_type,
                "damages_claimed": l.damages_claimed,
                "filing_cost": l.filing_cost,
                "filed_tick": l.filed_tick,
                "resolution_tick": l.resolution_tick,
                "status": l.status,
                "outcome": l.outcome.as_ref(),
            })
        })
        .collect();
    serde_json::to_string(&lawsuits).unwrap_or_default()
}

pub fn query_stock_market(world: &GameWorld, corp_id: EntityId) -> String {
    let sm = world.stock_market.get(&corp_id);
    let data = serde_json::json!({
        "public": sm.map(|s| s.public).unwrap_or(false),
        "total_shares": sm.map(|s| s.total_shares).unwrap_or(0),
        "share_price": sm.map(|s| s.share_price).unwrap_or(0),
        "dividends_per_share": sm.map(|s| s.dividends_per_share).unwrap_or(0),
        "ipo_tick": sm.and_then(|s| s.ipo_tick),
        "shareholder_satisfaction": sm.map(|s| s.shareholder_satisfaction).unwrap_or(0.0),
        "board_votes": sm.map(|s| {
            s.board_votes.iter().map(|v| serde_json::json!({
                "proposal": v.proposal,
                "votes_for": v.votes_for,
                "votes_against": v.votes_against,
                "deadline_tick": v.deadline_tick,
            })).collect::<Vec<_>>()
        }).unwrap_or_default(),
    });
    serde_json::to_string(&data).unwrap_or_default()
}

pub fn query_region_pricing(world: &GameWorld, corp_id: EntityId) -> String {
    let pricing: Vec<serde_json::Value> = world
        .region_pricing
        .iter()
        .filter(|((cid, _), _)| *cid == corp_id)
        .map(|((_, region_id), rp)| {
            let region_name = world
                .regions
                .get(region_id)
                .map(|r| r.name.as_str())
                .unwrap_or("Unknown");
            serde_json::json!({
                "region_id": region_id,
                "region_name": region_name,
                "tier": rp.tier,
                "price_per_unit": rp.price_per_unit,
            })
        })
        .collect();
    serde_json::to_string(&pricing).unwrap_or_default()
}

pub fn query_maintenance_priorities(world: &GameWorld, corp_id: EntityId) -> String {
    let node_ids = world
        .corp_infra_nodes
        .get(&corp_id)
        .cloned()
        .unwrap_or_default();
    let priorities: Vec<serde_json::Value> = node_ids
        .iter()
        .filter_map(|&id| {
            let mp = world.maintenance_priorities.get(&id)?;
            Some(serde_json::json!({
                "node_id": id,
                "priority": mp.tier,
                "auto_repair": mp.auto_repair,
            }))
        })
        .collect();
    serde_json::to_string(&priorities).unwrap_or_default()
}
