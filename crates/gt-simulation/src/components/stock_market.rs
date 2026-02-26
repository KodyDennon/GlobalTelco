use gt_common::types::{Money, Tick};
use serde::{Deserialize, Serialize};

/// A board vote proposal for a public corporation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardVote {
    pub proposal: String,
    pub votes_for: u32,
    pub votes_against: u32,
    pub deadline_tick: Tick,
}

/// Stock market component attached to corporations.
/// Tracks share price, dividends, IPO status, and shareholder satisfaction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockMarket {
    pub total_shares: u32,
    pub share_price: Money,
    pub dividends_per_share: Money,
    pub public: bool,
    pub ipo_tick: Option<Tick>,
    pub shareholder_satisfaction: f64,
    pub board_votes: Vec<BoardVote>,
}

impl StockMarket {
    pub fn new() -> Self {
        Self {
            total_shares: 1000,
            share_price: 100,
            dividends_per_share: 0,
            public: false,
            ipo_tick: None,
            shareholder_satisfaction: 0.5,
            board_votes: Vec::new(),
        }
    }

    /// Market capitalization = share_price * total_shares
    pub fn market_cap(&self) -> Money {
        self.share_price * self.total_shares as Money
    }
}

impl Default for StockMarket {
    fn default() -> Self {
        Self::new()
    }
}
