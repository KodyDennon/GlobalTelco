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
                if let Some(fin) = world.financials.get_mut(&holder) {
                    fin.debt = (fin.debt
                        - (payment - (debt.principal as f64 * debt.interest_rate / 365.0) as i64)
                            .max(0))
                    .max(0);
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

        // Bankruptcy check
        if new_rating == CreditRating::D && cash < -cost * 90 {
            world.event_queue.push(
                tick,
                gt_common::events::GameEvent::Bankruptcy {
                    corporation: corp_id,
                },
            );
        }
    }
}
