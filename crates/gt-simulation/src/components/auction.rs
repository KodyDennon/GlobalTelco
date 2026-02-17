use gt_common::types::{EntityId, Money, Tick};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuctionStatus {
    Open,
    Closed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Auction {
    pub seller: EntityId,
    pub assets: Vec<EntityId>,
    pub bids: Vec<(EntityId, Money)>,
    pub start_tick: Tick,
    pub end_tick: Tick,
    pub status: AuctionStatus,
}

impl Auction {
    pub fn new(seller: EntityId, assets: Vec<EntityId>, start_tick: Tick, duration: Tick) -> Self {
        Self {
            seller,
            assets,
            bids: Vec::new(),
            start_tick,
            end_tick: start_tick + duration,
            status: AuctionStatus::Open,
        }
    }

    pub fn place_bid(&mut self, bidder: EntityId, amount: Money) {
        // Replace existing bid from same bidder, or add new
        if let Some(existing) = self.bids.iter_mut().find(|(b, _)| *b == bidder) {
            existing.1 = amount;
        } else {
            self.bids.push((bidder, amount));
        }
    }

    pub fn highest_bid(&self) -> Option<(EntityId, Money)> {
        self.bids
            .iter()
            .max_by_key(|(_, amount)| *amount)
            .copied()
    }
}
