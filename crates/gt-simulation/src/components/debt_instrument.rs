use gt_common::types::{EntityId, Money};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebtInstrument {
    pub holder: EntityId,
    pub principal: Money,
    pub interest_rate: f64,
    pub remaining_ticks: u64,
    pub payment_per_tick: Money,
}

impl DebtInstrument {
    pub fn new(
        holder: EntityId,
        principal: Money,
        interest_rate: f64,
        duration_ticks: u64,
    ) -> Self {
        // Calculate fixed payment per tick (amortized)
        let rate_per_tick = interest_rate / 365.0; // approximate daily rate
        let payment = if rate_per_tick == 0.0 {
            principal / duration_ticks as i64
        } else {
            let r = rate_per_tick;
            let n = duration_ticks as f64;
            let pmt = principal as f64 * (r * (1.0 + r).powf(n)) / ((1.0 + r).powf(n) - 1.0);
            pmt as Money
        };

        Self {
            holder,
            principal,
            interest_rate,
            remaining_ticks: duration_ticks,
            payment_per_tick: payment.max(1),
        }
    }

    pub fn is_paid_off(&self) -> bool {
        self.remaining_ticks == 0 || self.principal <= 0
    }

    pub fn process_payment(&mut self) -> Money {
        if self.is_paid_off() {
            return 0;
        }
        let interest = (self.principal as f64 * self.interest_rate / 365.0) as Money;
        let principal_payment = (self.payment_per_tick - interest).max(0);
        self.principal = (self.principal - principal_payment).max(0);
        self.remaining_ticks = self.remaining_ticks.saturating_sub(1);
        self.payment_per_tick
    }
}
