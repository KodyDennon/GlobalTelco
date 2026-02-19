use crate::world::GameWorld;
use gt_common::types::CreditRating;

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
}
