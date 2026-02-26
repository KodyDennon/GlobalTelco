use crate::world::GameWorld;
use gt_common::types::{CreditRating, EntityId};

pub fn run(world: &mut GameWorld) {
    let tick = world.current_tick();

    // Process debt payments
    let mut debt_ids: Vec<u64> = world.debt_instruments.keys().copied().collect();
    debt_ids.sort_unstable();
    let mut paid_off = Vec::new();

    for debt_id in debt_ids {
        if let Some(debt) = world.debt_instruments.get_mut(&debt_id) {
            let payment = debt.process_payment();
            if payment > 0 {
                let holder = debt.holder;
                // Principal portion of payment reduces outstanding debt
                let interest_portion = (debt.principal as f64 * debt.interest_rate / 365.0) as i64;
                let principal_portion = (payment - interest_portion).max(0);
                if let Some(fin) = world.financials.get_mut(&holder) {
                    fin.debt = (fin.debt - principal_portion).max(0);
                }
            }
            if debt.is_paid_off() {
                paid_off.push(debt_id);
            }
        }
    }

    for id in paid_off {
        world.debt_instruments.remove(&id);
    }

    // Sandbox mode: keep player corporation flush with cash
    let is_sandbox = world.config().sandbox;
    if is_sandbox {
        if let Some(player_id) = world.player_corp_id() {
            if let Some(fin) = world.financials.get_mut(&player_id) {
                // Ensure player always has at least 10M cash in sandbox
                if fin.cash < 10_000_000 {
                    fin.cash = 10_000_000;
                }
                // Clear any debt
                fin.debt = 0;
            }
        }
    }

    // Update credit ratings based on financial health
    let mut corp_ids: Vec<u64> = world.corporations.keys().copied().collect();
    corp_ids.sort_unstable();
    for &corp_id in &corp_ids {
        let (cash, debt, revenue, cost) = match world.financials.get(&corp_id) {
            Some(f) => (f.cash, f.debt, f.revenue_per_tick, f.cost_per_tick),
            None => continue,
        };

        let debt_ratio = if revenue > 0 {
            debt as f64 / (revenue as f64 * 365.0)
        } else if debt > 0 {
            10.0
        } else {
            0.0
        };

        let profit_margin = if revenue > 0 {
            (revenue - cost) as f64 / revenue as f64
        } else {
            -1.0
        };

        let cash_ratio = cash as f64 / (cost as f64 * 30.0).max(1.0);

        let new_rating = if debt_ratio < 0.5 && profit_margin > 0.2 && cash_ratio > 5.0 {
            CreditRating::AAA
        } else if debt_ratio < 1.0 && profit_margin > 0.1 && cash_ratio > 3.0 {
            CreditRating::AA
        } else if debt_ratio < 2.0 && profit_margin > 0.05 && cash_ratio > 1.0 {
            CreditRating::A
        } else if debt_ratio < 3.0 && profit_margin > 0.0 {
            CreditRating::BBB
        } else if debt_ratio < 5.0 && cash > 0 {
            CreditRating::BB
        } else if cash > 0 {
            CreditRating::B
        } else if cash > -cost * 30 {
            CreditRating::CCC
        } else {
            CreditRating::D
        };

        if let Some(corp) = world.corporations.get_mut(&corp_id) {
            corp.credit_rating = new_rating;
        }

        // Enhanced insolvency detection
        let is_insolvent = new_rating == CreditRating::D && cash < -cost * 90 && debt > cost * 90;
        let is_player = world
            .corporations
            .get(&corp_id)
            .map(|c| c.is_player)
            .unwrap_or(false);

        if is_insolvent {
            // Skip insolvency for player in sandbox mode
            if is_player && is_sandbox {
                continue;
            }
            if is_player {
                // Player gets a warning and must choose bailout or bankruptcy via command
                world.event_queue.push(
                    tick,
                    gt_common::events::GameEvent::InsolvencyWarning {
                        corporation: corp_id,
                    },
                );
            } else {
                // AI auto-decides based on archetype
                let archetype = world
                    .ai_states
                    .get(&corp_id)
                    .map(|a| a.archetype);

                let take_bailout = match archetype {
                    Some(gt_common::types::AIArchetype::AggressiveExpander)
                    | Some(gt_common::types::AIArchetype::TechInnovator) => true,
                    Some(gt_common::types::AIArchetype::BudgetOperator)
                    | Some(gt_common::types::AIArchetype::DefensiveConsolidator) => {
                        debt < cost * 200 // Only bailout if debt isn't too extreme
                    }
                    None => false,
                };

                if take_bailout {
                    // AI takes bailout
                    let bailout_amount = cost * 180;
                    let interest_rate = 0.30;
                    let debt_inst =
                        crate::components::DebtInstrument::new(corp_id, bailout_amount, interest_rate, 365);
                    let payment = debt_inst.payment_per_tick;
                    let loan_id = world.allocate_entity();
                    world.debt_instruments.insert(loan_id, debt_inst);

                    if let Some(fin) = world.financials.get_mut(&corp_id) {
                        fin.cash += bailout_amount;
                        fin.debt += bailout_amount;
                        fin.cost_per_tick += payment;
                    }
                    if let Some(corp) = world.corporations.get_mut(&corp_id) {
                        corp.credit_rating = CreditRating::CCC;
                    }

                    world.event_queue.push(
                        tick,
                        gt_common::events::GameEvent::BailoutTaken {
                            corporation: corp_id,
                            amount: bailout_amount,
                            interest_rate,
                        },
                    );
                } else {
                    // AI declares bankruptcy
                    world.event_queue.push(
                        tick,
                        gt_common::events::GameEvent::BankruptcyDeclared {
                            corporation: corp_id,
                        },
                    );

                    // Create auction for assets
                    let assets: Vec<u64> = world
                        .corp_infra_nodes
                        .get(&corp_id)
                        .cloned()
                        .unwrap_or_default();

                    if !assets.is_empty() {
                        let auction_id = world.allocate_entity();
                        let auction =
                            crate::components::Auction::new(corp_id, assets.clone(), tick, 50);
                        world.auctions.insert(auction_id, auction);

                        world.event_queue.push(
                            tick,
                            gt_common::events::GameEvent::AuctionStarted {
                                auction: auction_id,
                                seller: corp_id,
                                asset_count: assets.len() as u32,
                            },
                        );
                    }

                    // Zero out finances
                    if let Some(fin) = world.financials.get_mut(&corp_id) {
                        fin.cash = 0;
                        fin.revenue_per_tick = 0;
                        fin.cost_per_tick = 0;
                        fin.debt = 0;
                    }

                    // Remove debts
                    let debts: Vec<u64> = world
                        .debt_instruments
                        .iter()
                        .filter(|(_, d)| d.holder == corp_id)
                        .map(|(&id, _)| id)
                        .collect();
                    for id in debts {
                        world.debt_instruments.remove(&id);
                    }
                }
            }
        }
    }

    // Track deep negative cash for AI bankruptcy liquidation
    check_ai_bankruptcy_liquidation(world);
}

// ─── AI Bankruptcy Liquidation ──────────────────────────────────────────────

/// Track AI corporations with sustained deep negative cash (< -500_000).
/// After 50 consecutive ticks in this state, liquidate the corporation entirely,
/// removing it and all its assets. This frees market space for new AI spawning.
fn check_ai_bankruptcy_liquidation(world: &mut GameWorld) {
    let tick = world.current_tick();

    // Collect AI corps and their cash positions
    let ai_corps: Vec<(EntityId, i64)> = world
        .ai_states
        .keys()
        .copied()
        .filter_map(|id| {
            let cash = world.financials.get(&id)?.cash;
            Some((id, cash))
        })
        .collect();

    let mut to_liquidate: Vec<EntityId> = Vec::new();

    for (corp_id, cash) in ai_corps {
        if cash < -500_000 {
            // Increment bankruptcy counter
            if let Some(ai) = world.ai_states.get_mut(&corp_id) {
                ai.bankruptcy_ticks += 1;
                if ai.bankruptcy_ticks >= 50 {
                    to_liquidate.push(corp_id);
                }
            }
        } else {
            // Reset counter if cash recovers
            if let Some(ai) = world.ai_states.get_mut(&corp_id) {
                ai.bankruptcy_ticks = 0;
            }
        }
    }

    for corp_id in to_liquidate {
        // Emit bankruptcy event
        world.event_queue.push(
            tick,
            gt_common::events::GameEvent::BankruptcyDeclared {
                corporation: corp_id,
            },
        );

        // Remove all infrastructure nodes
        let nodes = world.corp_infra_nodes.remove(&corp_id).unwrap_or_default();
        for node_id in &nodes {
            world.infra_nodes.remove(node_id);
            world.positions.remove(node_id);
            world.healths.remove(node_id);
            world.capacities.remove(node_id);
            world.ownerships.remove(node_id);
            world.constructions.remove(node_id);
            world.network.remove_node(*node_id);
        }

        // Remove all edges owned by this corp
        let edge_ids: Vec<EntityId> = world
            .infra_edges
            .iter()
            .filter(|(_, e)| e.owner == corp_id)
            .map(|(&id, _)| id)
            .collect();
        for edge_id in edge_ids {
            world.infra_edges.remove(&edge_id);
        }

        // Remove debt instruments
        let debts: Vec<EntityId> = world
            .debt_instruments
            .iter()
            .filter(|(_, d)| d.holder == corp_id)
            .map(|(&id, _)| id)
            .collect();
        for id in debts {
            world.debt_instruments.remove(&id);
        }

        // Remove contracts involving this corp
        let contracts: Vec<EntityId> = world
            .contracts
            .iter()
            .filter(|(_, c)| c.from == corp_id || c.to == corp_id)
            .map(|(&id, _)| id)
            .collect();
        for id in contracts {
            world.contracts.remove(&id);
        }

        // Remove all corporation components
        world.corporations.remove(&corp_id);
        world.financials.remove(&corp_id);
        world.ai_states.remove(&corp_id);
        world.policies.remove(&corp_id);
        world.workforces.remove(&corp_id);
        world.covert_ops.remove(&corp_id);
        world.achievements.remove(&corp_id);
    }
}
