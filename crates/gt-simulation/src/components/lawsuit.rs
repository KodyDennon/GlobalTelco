use gt_common::types::{EntityId, Money, Tick};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LawsuitType {
    PatentInfringement,
    OwnershipDispute,
    SabotageClaim,
    RegulatoryComplaint,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LawsuitOutcome {
    DamagesAwarded(f64),
    ForcedLicensing,
    AssetForfeiture(EntityId),
    Dismissed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LawsuitStatus {
    Active,
    Settled,
    Resolved,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lawsuit {
    pub id: EntityId,
    pub plaintiff: EntityId,
    pub defendant: EntityId,
    pub lawsuit_type: LawsuitType,
    pub damages_claimed: Money,
    pub filing_cost: Money,
    pub filed_tick: Tick,
    pub resolution_tick: Tick, // filed_tick + 20-50
    pub status: LawsuitStatus,
    pub outcome: Option<LawsuitOutcome>,
    /// Whether the defendant has actively defended this lawsuit.
    /// Defended lawsuits have reduced damages (20% reduction).
    #[serde(default)]
    pub defended: bool,
}

impl Lawsuit {
    /// Create a new lawsuit. Filing cost is 10% of damages claimed.
    /// Resolution tick is filed_tick + resolution_ticks (should be 20-50).
    pub fn new(
        id: EntityId,
        plaintiff: EntityId,
        defendant: EntityId,
        lawsuit_type: LawsuitType,
        damages_claimed: Money,
        filed_tick: Tick,
        resolution_ticks: Tick,
    ) -> Self {
        let filing_cost = damages_claimed / 10; // 10% of damages claimed
        Self {
            id,
            plaintiff,
            defendant,
            lawsuit_type,
            damages_claimed,
            filing_cost,
            filed_tick,
            resolution_tick: filed_tick + resolution_ticks,
            status: LawsuitStatus::Active,
            outcome: None,
            defended: false,
        }
    }

    /// Check if the lawsuit is ready for resolution.
    pub fn is_ready_for_resolution(&self, current_tick: Tick) -> bool {
        self.status == LawsuitStatus::Active && current_tick >= self.resolution_tick
    }

    /// Settle the lawsuit. Defendant pays 60% of claimed damages.
    pub fn settle(&mut self) -> Money {
        self.status = LawsuitStatus::Settled;
        let settlement = (self.damages_claimed as f64 * 0.6) as Money;
        self.outcome = Some(LawsuitOutcome::DamagesAwarded(settlement as f64));
        settlement
    }

    /// Resolve the lawsuit with a given outcome.
    pub fn resolve(&mut self, outcome: LawsuitOutcome) {
        self.status = LawsuitStatus::Resolved;
        self.outcome = Some(outcome);
    }
}
