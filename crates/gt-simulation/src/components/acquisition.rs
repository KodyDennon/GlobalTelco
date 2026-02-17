use gt_common::types::{EntityId, Money, Tick};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AcquisitionStatus {
    Pending,
    Accepted,
    Rejected,
    Expired,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcquisitionProposal {
    pub acquirer: EntityId,
    pub target: EntityId,
    pub offer: Money,
    pub status: AcquisitionStatus,
    pub tick: Tick,
}

impl AcquisitionProposal {
    pub fn new(acquirer: EntityId, target: EntityId, offer: Money, tick: Tick) -> Self {
        Self {
            acquirer,
            target,
            offer,
            status: AcquisitionStatus::Pending,
            tick,
        }
    }
}
