use serde::{Deserialize, Serialize};
use crate::types::{EntityId, Money, Tick};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GameEvent {
    // Construction
    ConstructionStarted { entity: EntityId, tick: Tick },
    ConstructionCompleted { entity: EntityId, tick: Tick },

    // Infrastructure
    NodeBuilt { entity: EntityId, owner: EntityId },
    EdgeBuilt { entity: EntityId, from: EntityId, to: EntityId },
    NodeDestroyed { entity: EntityId },

    // Finance
    RevenueEarned { corporation: EntityId, amount: Money },
    CostIncurred { corporation: EntityId, amount: Money },
    LoanTaken { corporation: EntityId, amount: Money },
    Bankruptcy { corporation: EntityId },

    // Corporation
    CorporationFounded { entity: EntityId, name: String },
    CorporationMerged { absorbed: EntityId, absorber: EntityId },

    // Contract
    ContractProposed { entity: EntityId, from: EntityId, to: EntityId },
    ContractAccepted { entity: EntityId },
    ContractExpired { entity: EntityId },

    // Research
    ResearchStarted { corporation: EntityId, tech: String },
    ResearchCompleted { corporation: EntityId, tech: String },

    // Disaster
    DisasterStruck { region: EntityId, severity: f64 },

    // Regulation
    RegulationChanged { region: EntityId, description: String },

    // Market
    MarketShiftOccurred { description: String },
}
