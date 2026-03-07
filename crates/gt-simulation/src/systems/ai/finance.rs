//! AI financial management.
//!
//! Handles loan-taking when cash is low and early debt repayment when flush.
//! Debt tolerance and repayment aggressiveness vary by archetype.

use crate::components::*;
use crate::world::GameWorld;
use gt_common::types::*;

// ─── Public Entry Point ──────────────────────────────────────────────────────

/// Manage finances: take loans if cash-strapped, repay early if flush.
pub fn manage(world: &mut GameWorld, corp_id: EntityId, ai: &AiState, tick: Tick) {
    let fin = match world.financials.get(&corp_id) {
        Some(f) => f.clone(),
        None => return,
    };

    let credit_rating = world
        .corporations
        .get(&corp_id)
        .map(|c| c.credit_rating)
        .unwrap_or(CreditRating::D);

    let cash_runway = if fin.cost_per_tick > 0 {
        fin.cash as f64 / fin.cost_per_tick as f64
    } else {
        999.0
    };

    maybe_take_loan(world, corp_id, ai, &fin, credit_rating, cash_runway, tick);
    maybe_repay_early(world, corp_id, ai, &fin, cash_runway);
}

// ─── Loan Taking ─────────────────────────────────────────────────────────────

fn maybe_take_loan(
    world: &mut GameWorld,
    corp_id: EntityId,
    ai: &AiState,
    fin: &Financial,
    credit_rating: CreditRating,
    cash_runway: f64,
    tick: Tick,
) {
    let max_debt_ratio = match ai.archetype {
        AIArchetype::AggressiveExpander => 3.0,
        AIArchetype::DefensiveConsolidator => 0.5,
        AIArchetype::TechInnovator => 2.0,
        AIArchetype::BudgetOperator => 0.3,
        AIArchetype::SatellitePioneer => 2.5,
    };

    let current_debt_ratio = if fin.cash > 0 {
        fin.debt as f64 / fin.cash as f64
    } else {
        0.0
    };

    if cash_runway >= 30.0
        || current_debt_ratio >= max_debt_ratio
        || credit_rating == CreditRating::D
        || credit_rating == CreditRating::CCC
    {
        return;
    }

    let loan_amount = fin.cost_per_tick * 90;
    if loan_amount <= 0 {
        return;
    }

    let interest_rate = match credit_rating {
        CreditRating::AAA => 0.03,
        CreditRating::AA => 0.04,
        CreditRating::A => 0.05,
        CreditRating::BBB => 0.07,
        CreditRating::BB => 0.10,
        CreditRating::B => 0.15,
        _ => return,
    };

    let debt = DebtInstrument::new(corp_id, loan_amount, interest_rate, 365);
    let payment = debt.payment_per_tick;
    let loan_id = world.allocate_entity();
    world.debt_instruments.insert(loan_id, debt);

    if let Some(f) = world.financials.get_mut(&corp_id) {
        f.cash += loan_amount;
        f.debt += loan_amount;
        f.cost_per_tick += payment;
    }

    world.event_queue.push(
        tick,
        gt_common::events::GameEvent::LoanTaken {
            corporation: corp_id,
            amount: loan_amount,
        },
    );
}

// ─── Early Repayment ─────────────────────────────────────────────────────────

fn maybe_repay_early(
    world: &mut GameWorld,
    corp_id: EntityId,
    ai: &AiState,
    fin: &Financial,
    cash_runway: f64,
) {
    if cash_runway <= 180.0 || fin.debt <= 0 {
        return;
    }

    // Only conservative archetypes repay early
    if !matches!(
        ai.archetype,
        AIArchetype::DefensiveConsolidator | AIArchetype::BudgetOperator
    ) {
        return;
    }

    // Find a loan to repay (sorted for determinism)
    let mut candidate_loans: Vec<(EntityId, Money)> = world
        .debt_instruments
        .iter()
        .filter(|(_, d)| d.holder == corp_id && d.principal > 0)
        .map(|(&id, d)| (id, d.principal))
        .collect();
    candidate_loans.sort_unstable_by_key(|t| t.0);

    if let Some((loan_id, principal)) = candidate_loans.first().copied() {
        let repay_amount = principal.min(fin.cash / 4); // Max 25% of cash
        if repay_amount <= 0 {
            return;
        }

        if let Some(debt) = world.debt_instruments.get_mut(&loan_id) {
            debt.principal -= repay_amount;
            if let Some(f) = world.financials.get_mut(&corp_id) {
                f.cash -= repay_amount;
                f.debt = (f.debt - repay_amount).max(0);
                if debt.is_paid_off() {
                    f.cost_per_tick = (f.cost_per_tick - debt.payment_per_tick).max(0);
                }
            }
            if debt.is_paid_off() {
                world.debt_instruments.shift_remove(&loan_id);
            }
        }
    }
}
