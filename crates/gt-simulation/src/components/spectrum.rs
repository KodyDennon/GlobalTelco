use gt_common::types::{EntityId, FrequencyBand, Money, Tick};
use serde::{Deserialize, Serialize};

/// A spectrum license grants exclusive use of a frequency band in a region.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpectrumLicense {
    pub band: FrequencyBand,
    pub region_id: EntityId,
    pub owner: EntityId,
    pub bandwidth_mhz: f64,
    pub start_tick: Tick,
    pub duration_ticks: Tick,
    pub auction_price: Money,
}

impl SpectrumLicense {
    pub fn new(
        band: FrequencyBand,
        region_id: EntityId,
        owner: EntityId,
        bandwidth_mhz: f64,
        start_tick: Tick,
        duration_ticks: Tick,
        auction_price: Money,
    ) -> Self {
        Self {
            band,
            region_id,
            owner,
            bandwidth_mhz,
            start_tick,
            duration_ticks,
            auction_price,
        }
    }

    /// Tick at which this license expires.
    pub fn end_tick(&self) -> Tick {
        self.start_tick + self.duration_ticks
    }

    /// Whether the license is active at the given tick.
    pub fn is_active(&self, tick: Tick) -> bool {
        tick >= self.start_tick && tick < self.end_tick()
    }

    /// Cost per tick for holding this license (amortized auction price).
    pub fn cost_per_tick(&self) -> Money {
        if self.duration_ticks == 0 {
            return 0;
        }
        self.auction_price / self.duration_ticks as Money
    }
}

/// An active or pending spectrum auction for a frequency band in a region.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpectrumAuction {
    pub band: FrequencyBand,
    pub region_id: EntityId,
    pub bandwidth_mhz: f64,
    pub start_tick: Tick,
    pub end_tick: Tick,
    /// (bidder_corp_id, bid_amount)
    pub bids: Vec<(EntityId, Money)>,
}

impl SpectrumAuction {
    pub fn new(
        band: FrequencyBand,
        region_id: EntityId,
        bandwidth_mhz: f64,
        start_tick: Tick,
        duration_ticks: Tick,
    ) -> Self {
        Self {
            band,
            region_id,
            bandwidth_mhz,
            start_tick,
            end_tick: start_tick + duration_ticks,
            bids: Vec::new(),
        }
    }

    /// Place or update a bid from a corporation.
    pub fn place_bid(&mut self, bidder: EntityId, amount: Money) {
        if let Some(existing) = self.bids.iter_mut().find(|(b, _)| *b == bidder) {
            existing.1 = amount;
        } else {
            self.bids.push((bidder, amount));
        }
    }

    /// Get the highest bid (bidder, amount), if any.
    pub fn highest_bid(&self) -> Option<(EntityId, Money)> {
        self.bids
            .iter()
            .max_by_key(|(_, amount)| *amount)
            .copied()
    }

    /// Whether this auction has ended at the given tick.
    pub fn is_ended(&self, tick: Tick) -> bool {
        tick >= self.end_tick
    }

    /// Ticks remaining until auction ends.
    pub fn ticks_remaining(&self, tick: Tick) -> Tick {
        self.end_tick.saturating_sub(tick)
    }
}
