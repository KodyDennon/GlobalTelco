//! Corporation data, financials, workforce, research, and related queries.

use gt_common::types::EntityId;
use gt_simulation::world::GameWorld;

pub fn query_corporation_data(world: &GameWorld, corp_id: EntityId) -> String {
    let corp = world.corporations.get(&corp_id);
    let fin = world.financials.get(&corp_id);
    let wf = world.workforces.get(&corp_id);
    let node_count = world
        .corp_infra_nodes
        .get(&corp_id)
        .map(|n| n.len())
        .unwrap_or(0);

    let is_player = world.player_corp_id() == Some(corp_id);

    let data = serde_json::json!({
        "id": corp_id,
        "name": corp.map(|c| c.name.as_str()).unwrap_or("Unknown"),
        "is_player": is_player,
        "credit_rating": corp.map(|c| c.credit_rating),
        "cash": fin.map(|f| f.cash).unwrap_or(0),
        "revenue_per_tick": fin.map(|f| f.revenue_per_tick).unwrap_or(0),
        "cost_per_tick": fin.map(|f| f.cost_per_tick).unwrap_or(0),
        "debt": fin.map(|f| f.debt).unwrap_or(0),
        "profit_per_tick": fin.map(|f| f.revenue_per_tick - f.cost_per_tick).unwrap_or(0),
        "employee_count": wf.map(|w| w.employee_count).unwrap_or(0),
        "morale": wf.map(|w| w.morale).unwrap_or(0.0),
        "infrastructure_count": node_count,
    });
    serde_json::to_string(&data).unwrap_or_default()
}

pub fn query_all_corporations(world: &GameWorld) -> String {
    let player_id = world.player_corp_id().unwrap_or(0);
    let corps: Vec<serde_json::Value> = world
        .corporations
        .iter()
        .map(|(&id, corp)| {
            let fin = world.financials.get(&id);
            let is_player = player_id == id;

            let intel_level = if is_player {
                3
            } else {
                world.intel_levels.get(&(player_id, id)).copied().unwrap_or(0)
            };

            if intel_level >= 2 {
                // Detailed data
                serde_json::json!({
                    "id": id,
                    "name": corp.name,
                    "is_player": is_player,
                    "credit_rating": corp.credit_rating,
                    "cash": fin.map(|f| f.cash).unwrap_or(0),
                    "revenue": fin.map(|f| f.revenue_per_tick).unwrap_or(0),
                    "cost": fin.map(|f| f.cost_per_tick).unwrap_or(0),
                    "intel_level": intel_level,
                })
            } else if intel_level == 1 {
                // Obfuscated data (rounded to nearest 100k or 10k)
                let cash = fin.map(|f| (f.cash / 100_000) * 100_000).unwrap_or(0);
                let rev = fin.map(|f| (f.revenue_per_tick / 10_000) * 10_000).unwrap_or(0);
                serde_json::json!({
                    "id": id,
                    "name": corp.name,
                    "is_player": is_player,
                    "credit_rating": corp.credit_rating,
                    "cash": cash,
                    "revenue": rev,
                    "cost": null,
                    "intel_level": intel_level,
                })
            } else {
                // Basic data
                serde_json::json!({
                    "id": id,
                    "name": corp.name,
                    "is_player": is_player,
                    "credit_rating": null,
                    "cash": null,
                    "revenue": null,
                    "cost": null,
                    "intel_level": intel_level,
                })
            }
        })
        .collect();
    serde_json::to_string(&corps).unwrap_or_default()
}

pub fn query_research_state(world: &GameWorld) -> String {
    let research: Vec<serde_json::Value> = world
        .tech_research
        .iter()
        .map(|(&id, r)| {
            let researcher_name = r
                .researcher
                .and_then(|rid| world.corporations.get(&rid).map(|c| c.name.clone()));
            let patent_owner_name = r
                .patent_owner
                .and_then(|oid| world.corporations.get(&oid).map(|c| c.name.clone()));
            let patent_data = world.patents.values().find(|p| p.tech_id == id);
            let per_unit_price = patent_data.map(|p| p.per_unit_price).unwrap_or(0);
            let lease_duration = patent_data.map(|p| p.lease_duration).unwrap_or(0);
            let patent_license_type = patent_data
                .map(|p| p.license_type);

            serde_json::json!({
                "id": id,
                "category": r.category,
                "category_name": r.category.display_name(),
                "name": r.name,
                "description": r.description,
                "progress": r.progress,
                "total_cost": r.total_cost,
                "progress_pct": r.progress_pct(),
                "researcher": r.researcher,
                "researcher_name": researcher_name,
                "completed": r.completed,
                "patent_status": r.patent_status,
                "patent_owner": r.patent_owner,
                "patent_owner_name": patent_owner_name,
                "license_price": r.license_price,
                "prerequisites": r.prerequisites,
                "throughput_bonus": r.throughput_bonus,
                "cost_reduction": r.cost_reduction,
                "reliability_bonus": r.reliability_bonus,
                "independent_tier": r.independent_tier,
                "per_unit_price": per_unit_price,
                "lease_duration": lease_duration,
                "patent_license_type": patent_license_type,
            })
        })
        .collect();
    serde_json::to_string(&research).unwrap_or_default()
}

pub fn query_contracts(world: &GameWorld, corp_id: EntityId) -> String {
    let contracts: Vec<serde_json::Value> = world
        .contracts
        .iter()
        .filter(|(_, c)| c.from == corp_id || c.to == corp_id)
        .map(|(&id, c)| {
            let from_name = world
                .corporations
                .get(&c.from)
                .map(|corp| corp.name.as_str())
                .unwrap_or("Unknown");
            let to_name = world
                .corporations
                .get(&c.to)
                .map(|corp| corp.name.as_str())
                .unwrap_or("Unknown");
            let sla_status = if c.sla_current_performance >= c.sla_target {
                "ok"
            } else if c.sla_current_performance >= c.sla_target - 5.0 {
                "at_risk"
            } else {
                "breach"
            };
            let traffic_current = world
                .traffic_matrix
                .contract_traffic
                .get(&id)
                .copied()
                .unwrap_or(0.0);
            let traffic_capacity_pct = if c.capacity > 0.0 {
                (traffic_current / c.capacity * 100.0).min(100.0)
            } else {
                0.0
            };
            let price_per_unit = if c.capacity > 0.0 {
                c.price_per_tick as f64 / c.capacity
            } else {
                0.0
            };
            let transit_amount = (traffic_current * price_per_unit) as i64;
            let (transit_revenue, transit_cost) = if c.from == corp_id {
                (transit_amount, 0_i64)
            } else {
                (0_i64, transit_amount)
            };
            serde_json::json!({
                "id": id,
                "contract_type": c.contract_type,
                "from": c.from,
                "to": c.to,
                "from_name": from_name,
                "to_name": to_name,
                "capacity": c.capacity,
                "price_per_tick": c.price_per_tick,
                "start_tick": c.start_tick,
                "end_tick": c.end_tick,
                "status": c.status,
                "penalty": c.penalty,
                "sla_target": c.sla_target,
                "sla_current_performance": c.sla_current_performance,
                "sla_status": sla_status,
                "sla_penalty_accrued": c.sla_penalty_accrued,
                "traffic_current": traffic_current,
                "traffic_capacity_pct": traffic_capacity_pct,
                "transit_revenue": transit_revenue,
                "transit_cost": transit_cost,
            })
        })
        .collect();
    serde_json::to_string(&contracts).unwrap_or_default()
}

pub fn query_debt_instruments(world: &GameWorld, corp_id: EntityId) -> String {
    let debts: Vec<serde_json::Value> = world
        .debt_instruments
        .iter()
        .filter(|(_, d)| d.holder == corp_id)
        .map(|(&id, d)| {
            serde_json::json!({
                "id": id,
                "principal": d.principal,
                "interest_rate": d.interest_rate,
                "remaining_ticks": d.remaining_ticks,
                "payment_per_tick": d.payment_per_tick,
                "is_paid_off": d.is_paid_off(),
            })
        })
        .collect();
    serde_json::to_string(&debts).unwrap_or_default()
}

pub fn query_notifications(world: &mut GameWorld) -> String {
    let events = world.event_queue.drain();
    let notifications: Vec<serde_json::Value> = events
        .iter()
        .map(|(tick, event)| {
            serde_json::json!({
                "tick": tick,
                "event": serde_json::to_value(event).unwrap_or(serde_json::Value::Null),
            })
        })
        .collect();
    serde_json::to_string(&notifications).unwrap_or_default()
}

pub fn query_covert_ops(world: &GameWorld, corp_id: EntityId) -> String {
    let ops = world.covert_ops.get(&corp_id);
    let data = serde_json::json!({
        "security_level": ops.map(|o| o.security_level).unwrap_or(0),
        "active_missions": ops.map(|o| o.active_missions.len()).unwrap_or(0),
        "detection_count": ops.map(|o| o.detection_history.len()).unwrap_or(0),
    });
    serde_json::to_string(&data).unwrap_or_default()
}

pub fn query_lobbying_campaigns(world: &GameWorld, corp_id: EntityId) -> String {
    let campaigns: Vec<serde_json::Value> = world
        .lobbying_campaigns
        .iter()
        .filter(|(_, c)| c.corporation == corp_id)
        .map(|(&id, c)| {
            let region_name = world
                .regions
                .get(&c.region)
                .map(|r| r.name.as_str())
                .unwrap_or("Unknown");
            serde_json::json!({
                "id": id,
                "region": c.region,
                "region_name": region_name,
                "policy": c.policy,
                "budget_spent": c.budget_spent,
                "budget_total": c.budget_total,
                "influence": c.influence,
                "threshold": c.influence_threshold(),
                "active": c.active,
            })
        })
        .collect();
    serde_json::to_string(&campaigns).unwrap_or_default()
}

pub fn query_achievements(world: &GameWorld, corp_id: EntityId) -> String {
    let tracker = world.achievements.get(&corp_id);
    let data = serde_json::json!({
        "unlocked": tracker.map(|t| t.unlocked.iter().cloned().collect::<Vec<_>>()).unwrap_or_default(),
        "progress": tracker.map(|t| t.progress.clone()).unwrap_or_default(),
    });
    serde_json::to_string(&data).unwrap_or_default()
}

pub fn query_victory_state(world: &GameWorld) -> String {
    let state = world.victory_state.as_ref();
    let data = serde_json::json!({
        "domination_score": state.map(|s| s.domination_score).unwrap_or(0.0),
        "tech_score": state.map(|s| s.tech_score).unwrap_or(0.0),
        "wealth_score": state.map(|s| s.wealth_score).unwrap_or(0.0),
        "infrastructure_score": state.map(|s| s.infrastructure_score).unwrap_or(0.0),
        "total_score": state.map(|s| s.total_score).unwrap_or(0.0),
        "victory_type": state.and_then(|s| s.victory_type.clone()),
    });
    serde_json::to_string(&data).unwrap_or_default()
}

pub fn query_auctions(world: &GameWorld) -> String {
    let auctions: Vec<serde_json::Value> = world
        .auctions
        .iter()
        .map(|(&id, a)| {
            let seller_name = world
                .corporations
                .get(&a.seller)
                .map(|c| c.name.as_str())
                .unwrap_or("Unknown");
            let highest = a.highest_bid();
            serde_json::json!({
                "id": id,
                "seller": a.seller,
                "seller_name": seller_name,
                "asset_count": a.assets.len(),
                "bid_count": a.bids.len(),
                "highest_bid": highest.map(|(_, amt)| amt).unwrap_or(0),
                "highest_bidder": highest.map(|(id, _)| id).unwrap_or(0),
                "start_tick": a.start_tick,
                "end_tick": a.end_tick,
                "status": a.status,
            })
        })
        .collect();
    serde_json::to_string(&auctions).unwrap_or_default()
}

pub fn query_acquisition_proposals(world: &GameWorld) -> String {
    let proposals: Vec<serde_json::Value> = world
        .acquisition_proposals
        .iter()
        .map(|(&id, p)| {
            let acquirer_name = world
                .corporations
                .get(&p.acquirer)
                .map(|c| c.name.as_str())
                .unwrap_or("Unknown");
            let target_name = world
                .corporations
                .get(&p.target)
                .map(|c| c.name.as_str())
                .unwrap_or("Unknown");
            serde_json::json!({
                "id": id,
                "acquirer": p.acquirer,
                "acquirer_name": acquirer_name,
                "target": p.target,
                "target_name": target_name,
                "offer": p.offer,
                "status": p.status,
                "tick": p.tick,
            })
        })
        .collect();
    serde_json::to_string(&proposals).unwrap_or_default()
}
