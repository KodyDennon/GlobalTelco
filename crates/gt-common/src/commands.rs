use serde::{Deserialize, Serialize};
use crate::types::{EntityId, GameSpeed, Money};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Command {
    // Infrastructure
    BuildNode { node_type: crate::types::NodeType, parcel: EntityId },
    BuildEdge { edge_type: crate::types::EdgeType, from: EntityId, to: EntityId },
    UpgradeNode { entity: EntityId },
    DecommissionNode { entity: EntityId },

    // Workforce
    HireEmployee { corporation: EntityId, role: String },
    FireEmployee { entity: EntityId },
    AssignTeam { team: EntityId, target: EntityId },

    // Finance
    TakeLoan { corporation: EntityId, amount: Money },
    RepayLoan { loan: EntityId, amount: Money },
    SetBudget { corporation: EntityId, category: String, amount: Money },

    // Contracts
    ProposeContract { from: EntityId, to: EntityId, terms: String },
    AcceptContract { contract: EntityId },
    RejectContract { contract: EntityId },

    // Research
    StartResearch { corporation: EntityId, tech: String },
    CancelResearch { corporation: EntityId },

    // Policy
    SetPolicy { corporation: EntityId, policy: String, value: String },

    // Game control
    SetSpeed(GameSpeed),
    TogglePause,
    SaveGame { slot: u32 },
    LoadGame { slot: u32 },
}
